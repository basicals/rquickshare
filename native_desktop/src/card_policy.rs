//! Bounded, replay-safe presentation state. Dismissal never answers daemon consent.
use crate::desktop_guard::SafetyState;
use rqs_lib::service::{Direction, Phase};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notice {
    pub epoch: u128,
    pub id: String,
    pub round: Option<u64>,
    pub phase: Phase,
    pub revision: u64,
}
pub struct Row<'a> {
    pub id: &'a str,
    pub direction: Direction,
    pub phase: Phase,
    pub round: Option<u64>,
    pub notice_revision: Option<u64>,
}
struct Record {
    notice: Option<Notice>,
    dismissed: bool,
}
#[derive(Default)]
pub struct CardPolicy {
    epoch: Option<u128>,
    records: BTreeMap<String, Record>,
    shown: Option<Notice>,
}
pub enum Effect {
    None,
    Hide,
    Show(Notice),
}
pub struct Surface {
    pub visible: bool,
    pub automatic: bool,
    pub busy: bool,
    pub healthy: bool,
    pub safety: SafetyState,
}
impl CardPolicy {
    pub fn dismiss(&mut self) -> Vec<Notice> {
        let mut dismissed = Vec::new();
        for record in self.records.values_mut() {
            if !record.dismissed {
                if let Some(notice) = &record.notice {
                    dismissed.push(notice.clone());
                }
            }
            record.dismissed = true;
        }
        self.shown = None;
        dismissed
    }
    pub fn manual(&mut self) {
        self.shown = None;
    }
    pub fn reconcile(&mut self, epoch: u128, rows: &[Row<'_>], surface: Surface) -> Effect {
        if self.epoch != Some(epoch) {
            self.records.clear();
            self.epoch = Some(epoch);
        }
        let mut previous = std::mem::take(&mut self.records);
        for row in rows {
            let old = previous.remove(row.id);
            let incoming = row.direction == Direction::Inbound
                && row.phase == Phase::AwaitingConsent
                && row.round.is_some();
            let outcome = matches!(
                row.phase,
                Phase::Finished | Phase::Failed | Phase::Disconnected
            );
            let keep_progress = !row.phase.is_terminal()
                && self
                    .shown
                    .as_ref()
                    .is_some_and(|notice| notice.epoch == epoch && notice.id == row.id);
            let notice = if (incoming || outcome) && row.notice_revision.is_some() {
                Some(Notice {
                    epoch,
                    id: row.id.to_owned(),
                    round: if incoming { row.round } else { None },
                    phase: row.phase,
                    revision: row.notice_revision.unwrap(),
                })
            } else if keep_progress && !incoming {
                Some(Notice {
                    epoch,
                    id: row.id.to_owned(),
                    round: None,
                    phase: row.phase,
                    revision: self.shown.as_ref().unwrap().revision,
                })
            } else {
                None
            };
            let dismissed = old
                .as_ref()
                .is_some_and(|old| old.notice == notice && old.dismissed);
            self.records
                .insert(row.id.to_owned(), Record { notice, dismissed });
        }
        let hide = surface.visible
            && (!surface.safety.manual_allowed()
                || (surface.automatic
                    && (!surface.healthy || !surface.safety.automatic_allowed())));
        if hide {
            self.shown = None;
            return Effect::Hide;
        }
        if !surface.healthy
            || !surface.safety.automatic_allowed()
            || (surface.visible && !surface.automatic)
        {
            return Effect::None;
        }
        let candidate = self
            .records
            .values()
            .filter(|record| !record.dismissed)
            .filter_map(|record| record.notice.as_ref())
            .min_by_key(|notice| {
                if notice.phase == Phase::AwaitingConsent {
                    0
                } else {
                    1
                }
            })
            .cloned();
        if surface.busy {
            if surface.automatic
                && surface.visible
                && !self.records.values().any(|record| {
                    record.notice.as_ref() == self.shown.as_ref() && !record.dismissed
                })
            {
                self.shown = None;
                return Effect::Hide;
            }
            return Effect::None;
        }
        match candidate {
            Some(notice) if self.shown.as_ref() != Some(&notice) || !surface.visible => {
                self.shown = Some(notice.clone());
                Effect::Show(notice)
            }
            Some(_) => Effect::None,
            None if surface.automatic && surface.visible => {
                self.shown = None;
                Effect::Hide
            }
            None => Effect::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn safe() -> Surface {
        Surface {
            visible: false,
            automatic: false,
            busy: false,
            healthy: true,
            safety: SafetyState {
                locked: Some(false),
                inhibited: Some(false),
                closed: false,
            },
        }
    }
    fn row(id: &str, phase: Phase, round: Option<u64>) -> Row<'_> {
        Row {
            id,
            direction: Direction::Inbound,
            phase,
            round,
            notice_revision: if phase == Phase::AwaitingConsent {
                round
            } else {
                None
            },
        }
    }
    fn pending_row(id: &str, phase: Phase) -> Row<'_> {
        let mut row = row(id, phase, None);
        row.notice_revision = Some(100);
        row
    }
    fn shown(effect: Effect, id: &str, phase: Phase) -> Notice {
        let Effect::Show(notice) = effect else {
            panic!("Expected current notice")
        };
        assert_eq!(notice.id, id);
        assert_eq!(notice.phase, phase);
        notice
    }
    #[test]
    fn given_epochs_rounds_and_desktop_safety_when_events_repeat_then_only_current_undismissed_notice_can_present(
    ) {
        // Arrange / Act / Assert
        for locked in [None, Some(false), Some(true)] {
            for inhibited in [None, Some(false), Some(true)] {
                for closed in [false, true] {
                    let mut policy = CardPolicy::default();
                    let mut surface = safe();
                    surface.safety = SafetyState {
                        locked,
                        inhibited,
                        closed,
                    };
                    let effect =
                        policy.reconcile(1, &[row("a", Phase::AwaitingConsent, Some(1))], surface);
                    assert_eq!(
                        matches!(effect, Effect::Show(_)),
                        !closed && locked == Some(false) && inhibited == Some(false)
                    );
                }
            }
        }
        let mut policy = CardPolicy::default();
        shown(
            policy.reconcile(1, &[row("a", Phase::AwaitingConsent, Some(1))], safe()),
            "a",
            Phase::AwaitingConsent,
        );
        let mut visible = safe();
        visible.visible = true;
        visible.automatic = true;
        assert!(matches!(
            policy.reconcile(1, &[row("a", Phase::AwaitingConsent, Some(1))], visible),
            Effect::None
        ));
        policy.dismiss();
        assert!(matches!(
            policy.reconcile(1, &[row("a", Phase::AwaitingConsent, Some(1))], safe()),
            Effect::None
        ));
        shown(
            policy.reconcile(1, &[row("a", Phase::AwaitingConsent, Some(2))], safe()),
            "a",
            Phase::AwaitingConsent,
        );
        shown(
            policy.reconcile(1, &[row("a", Phase::Receiving, None)], safe()),
            "a",
            Phase::Receiving,
        );
        shown(
            policy.reconcile(1, &[pending_row("a", Phase::Finished)], safe()),
            "a",
            Phase::Finished,
        );
        policy.dismiss();
        assert!(matches!(
            policy.reconcile(1, &[pending_row("a", Phase::Finished)], safe()),
            Effect::None
        ));
        shown(
            policy.reconcile(2, &[row("a", Phase::AwaitingConsent, Some(1))], safe()),
            "a",
            Phase::AwaitingConsent,
        );
        let mut manual = safe();
        manual.visible = true;
        assert!(matches!(
            policy.reconcile(2, &[row("a", Phase::AwaitingConsent, Some(1))], manual),
            Effect::None
        ));
        let mut locked = safe();
        locked.visible = true;
        locked.safety.locked = Some(true);
        assert!(matches!(
            policy.reconcile(2, &[row("a", Phase::AwaitingConsent, Some(1))], locked),
            Effect::Hide
        ));
        let mut policy = CardPolicy::default();
        assert!(matches!(
            policy.reconcile(1, &[row("done", Phase::Finished, None)], safe()),
            Effect::None
        ));
        assert!(matches!(
            policy.reconcile(1, &[row("progress", Phase::Sending, None)], safe()),
            Effect::None
        ));
        shown(
            policy.reconcile(
                1,
                &[
                    pending_row("progress", Phase::Failed),
                    row("request", Phase::AwaitingConsent, Some(1)),
                ],
                safe(),
            ),
            "request",
            Phase::AwaitingConsent,
        );
        assert_eq!(policy.records.len(), 2);
        let mut busy = safe();
        busy.busy = true;
        assert!(matches!(
            policy.reconcile(1, &[row("request", Phase::AwaitingConsent, Some(2))], busy),
            Effect::None
        ));
        assert_eq!(policy.records.len(), 1);
        shown(
            policy.reconcile(
                1,
                &[row("request", Phase::AwaitingConsent, Some(2))],
                safe(),
            ),
            "request",
            Phase::AwaitingConsent,
        );
        let mut visible = safe();
        visible.visible = true;
        visible.automatic = true;
        assert!(matches!(
            policy.reconcile(1, &[row("request", Phase::Rejected, None)], visible),
            Effect::Hide
        ));
        for phase in [
            Phase::Queued,
            Phase::Active,
            Phase::AwaitingConsent,
            Phase::Receiving,
            Phase::Sending,
            Phase::CancelRequested,
            Phase::Finished,
            Phase::Rejected,
            Phase::Cancelled,
            Phase::Failed,
            Phase::Disconnected,
        ] {
            let mut policy = CardPolicy::default();
            assert!(matches!(
                policy.reconcile(1, &[row("initial", phase, None)], safe()),
                Effect::None
            ));
        }
        for phase in [Phase::Finished, Phase::Failed, Phase::Disconnected] {
            let mut policy = CardPolicy::default();
            policy.reconcile(1, &[row("transfer", Phase::Sending, None)], safe());
            let notice = shown(
                policy.reconcile(1, &[pending_row("transfer", phase)], safe()),
                "transfer",
                phase,
            );
            assert_eq!(notice.epoch, 1);
            assert_eq!(notice.round, None);
            let mut visible = safe();
            visible.visible = true;
            visible.automatic = true;
            assert!(matches!(
                policy.reconcile(1, &[pending_row("transfer", phase)], visible),
                Effect::None
            ));
        }
        let mut policy = CardPolicy::default();
        let notice = shown(
            policy.reconcile(3, &[row("round", Phase::AwaitingConsent, Some(7))], safe()),
            "round",
            Phase::AwaitingConsent,
        );
        assert_eq!((notice.epoch, notice.round), (3, Some(7)));
        let mut busy = safe();
        busy.visible = true;
        busy.automatic = true;
        busy.busy = true;
        assert!(matches!(
            policy.reconcile(3, &[row("round", Phase::AwaitingConsent, Some(8))], busy),
            Effect::Hide
        ));
        let mut manual = safe();
        manual.visible = true;
        manual.safety.inhibited = Some(true);
        assert!(matches!(
            policy.reconcile(3, &[row("round", Phase::AwaitingConsent, Some(8))], manual),
            Effect::None
        ));
        let mut automatic = safe();
        automatic.visible = true;
        automatic.automatic = true;
        automatic.healthy = false;
        assert!(matches!(
            policy.reconcile(
                3,
                &[row("round", Phase::AwaitingConsent, Some(8))],
                automatic
            ),
            Effect::Hide
        ));
        for healthy in [false, true] {
            let mut policy = CardPolicy::default();
            let mut surface = safe();
            surface.healthy = healthy;
            assert_eq!(
                matches!(
                    policy.reconcile(
                        1,
                        &[row("request", Phase::AwaitingConsent, Some(1))],
                        surface
                    ),
                    Effect::Show(_)
                ),
                healthy
            );
        }
        let mut policy = CardPolicy::default();
        let ids = (0..64)
            .map(|index| format!("request-{index:03}"))
            .collect::<Vec<_>>();
        let rows = ids
            .iter()
            .map(|id| row(id, Phase::AwaitingConsent, Some(1)))
            .collect::<Vec<_>>();
        shown(
            policy.reconcile(1, &rows, safe()),
            "request-000",
            Phase::AwaitingConsent,
        );
        assert_eq!(policy.records.len(), 64);
        policy.dismiss();
        assert!(matches!(policy.reconcile(1, &rows, safe()), Effect::None));
        policy.reconcile(1, &[], safe());
        assert!(policy.records.is_empty());
        policy.manual();
        assert!(policy.shown.is_none());
    }
    #[test]
    fn given_authoritative_pending_outcome_when_renderer_starts_then_it_presents_and_daemon_dismissal_retires_it(
    ) {
        // Arrange / Act
        let mut policy = CardPolicy::default();
        let notice = shown(
            policy.reconcile(42, &[pending_row("done", Phase::Finished)], safe()),
            "done",
            Phase::Finished,
        );
        // Assert
        assert_eq!(notice.revision, 100);
        let dismissed = policy.dismiss();
        assert_eq!(dismissed, vec![notice]);
        assert!(matches!(
            policy.reconcile(42, &[pending_row("done", Phase::Finished)], safe()),
            Effect::None
        ));
        policy.reconcile(42, &[row("done", Phase::Finished, None)], safe());
        assert!(policy.records["done"].notice.is_none());
        let mut fresh = CardPolicy::default();
        assert!(matches!(
            fresh.reconcile(42, &[row("done", Phase::Finished, None)], safe()),
            Effect::None
        ));
        let pending = pending_row("failure", Phase::Failed);
        shown(
            fresh.reconcile(42, &[pending], safe()),
            "failure",
            Phase::Failed,
        );
    }
}
