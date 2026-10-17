//! Epoch-local presentation records. They never own consent or payload lifetime.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum NoticeKind {
    Request,
    Finished,
    Failed,
    Disconnected,
}
impl NoticeKind {
    pub fn wire(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Finished => "finished",
            Self::Failed => "failed",
            Self::Disconnected => "disconnected",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "request" => Self::Request,
            "finished" => Self::Finished,
            "failed" => Self::Failed,
            "disconnected" => Self::Disconnected,
            _ => return None,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Notice {
    pub transfer: TransferId,
    pub kind: NoticeKind,
    pub round: u64,
    pub revision: u64,
}
pub(super) struct Record {
    notice: Notice,
    dismissed: bool,
}
impl Coordinator {
    pub(super) fn notice_event(&mut self, id: TransferId) {
        let view = &self.transfers[&id].view;
        let event = match view.phase {
            Phase::AwaitingConsent if view.direction == Direction::Inbound => view
                .consent_revision
                .map(|round| (NoticeKind::Request, round)),
            Phase::Finished => Some((NoticeKind::Finished, 0)),
            Phase::Failed => Some((NoticeKind::Failed, 0)),
            Phase::Disconnected => Some((NoticeKind::Disconnected, 0)),
            _ => None,
        };
        let Some((kind, round)) = event else {
            self.notices.remove(&id);
            return;
        };
        if self
            .notices
            .get(&id)
            .is_some_and(|old| old.notice.kind == kind && old.notice.round == round)
        {
            return;
        }
        self.notices.insert(
            id,
            Record {
                notice: Notice {
                    transfer: id,
                    kind,
                    round,
                    revision: self.revision,
                },
                dismissed: false,
            },
        );
    }
    pub(super) fn pending_notices(&self) -> Vec<Notice> {
        self.notices
            .values()
            .filter(|record| !record.dismissed)
            .map(|record| record.notice.clone())
            .collect()
    }
    /// Exact replay is harmless; stale identities cannot dismiss a newer event.
    pub fn dismiss_notices(&mut self, client: ClientId, notices: &[Notice]) -> Result<(), Error> {
        self.client(client)?;
        if notices.is_empty() || notices.len() > 16 {
            return Err(Error::InvalidInput);
        }
        let mut unique = BTreeSet::new();
        for notice in notices {
            if notice.transfer.0 != self.epoch
                || notice.transfer.1 == 0
                || notice.revision == 0
                || notice.revision > self.revision
                || (notice.kind == NoticeKind::Request) != (notice.round > 0)
                || notice.round > notice.revision
                || !unique.insert(notice)
            {
                return Err(Error::InvalidInput);
            }
        }
        let changed = notices.iter().any(|notice| {
            self.notices
                .get(&notice.transfer)
                .is_some_and(|record| record.notice == *notice && !record.dismissed)
        });
        if changed {
            self.advance()?;
            for notice in notices {
                if let Some(record) = self.notices.get_mut(&notice.transfer) {
                    if record.notice == *notice {
                        record.dismissed = true;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
