//! Current core policy and explicit saved visibility intent; no second expiry owner.
use serde::{Deserialize, Serialize};
use zbus::zvariant::Type;

use super::native_settings::Base;

pub const INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Visibility2";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VisibilityView {
    pub epoch: String,
    pub available: bool,
    pub revision: u64,
    pub running: bool,
    pub mode: u8,
    pub remaining_ms: u64,
}
impl VisibilityView {
    pub(super) fn project(epoch: &str, reader: Option<&crate::VisibilityReader>) -> Self {
        let observed = reader.map(crate::VisibilityReader::timed_observation);
        let (mode, revision, running, remaining_ms) =
            observed.unwrap_or((crate::Visibility::Invisible, 0, false, 0));
        Self {
            epoch: epoch.into(),
            available: observed.is_some(),
            revision,
            running,
            mode: if running { mode as u8 } else { 1 },
            remaining_ms,
        }
    }
    pub fn valid(&self, epoch: u128) -> bool {
        self.epoch == format!("{epoch:032x}")
            && self.mode <= 2
            && self.remaining_ms <= 60_000
            && ((self.running && self.mode == 2) || self.remaining_ms == 0)
            && (self.running || self.mode == 1)
            && (self.available || (!self.running && self.revision == 0))
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VisibilityIntent {
    pub epoch: String,
    pub revision: u64,
    pub token: String,
    pub request: String,
    pub mode: u8,
}
impl std::fmt::Debug for VisibilityIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisibilityIntent")
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}
impl VisibilityIntent {
    pub fn valid(&self, epoch: u128) -> bool {
        self.epoch == format!("{epoch:032x}")
            && self.mode <= 2
            && super::settings_service::valid_hex(&self.token, 64)
            && super::settings_service::valid_hex(&self.request, 32)
    }
    pub(super) fn base(&self) -> Base {
        Base {
            revision: self.revision,
            token: self.token.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn given_policy_observation_when_projected_or_validated_then_stopped_and_unavailable_never_claim_visible(
    ) {
        // Arrange
        let epoch = format!("{:032x}", 123);
        let unavailable = VisibilityView::project(&epoch, None);
        // Act / Assert
        assert!(unavailable.valid(123));
        assert!(!unavailable.available);
        assert!(!unavailable.running);
        assert_eq!(unavailable.mode, 1);
        for mode in [
            crate::Visibility::Visible,
            crate::Visibility::Invisible,
            crate::Visibility::Temporarily,
        ] {
            let policy = crate::visibility::VisibilityPolicy::new(mode);
            let reader = policy.subscribe();
            assert!(!policy.set_running(crate::Visibility::Visible));
            assert_eq!(reader.current(), mode);
            let stopped = VisibilityView::project(&epoch, Some(&reader));
            assert!(stopped.valid(123));
            assert!(stopped.available);
            assert!(!stopped.running);
            assert_eq!(stopped.mode, 1);
        }
        for case in 0..6 {
            let mut view = unavailable.clone();
            match case {
                0 => view.epoch = "bad".into(),
                1 => view.mode = 3,
                2 => view.running = true,
                3 => view.mode = 0,
                4 => view.remaining_ms = 1,
                5 => {
                    view.available = true;
                    view.running = true;
                    view.mode = 2;
                    view.remaining_ms = 60_001;
                }
                _ => unreachable!(),
            }
            assert!(!view.valid(123));
        }
        assert_eq!(VisibilityView::SIGNATURE.to_string(), "(sbtbyt)");
        assert_eq!(VisibilityIntent::SIGNATURE.to_string(), "(stssy)");
        let intent = VisibilityIntent {
            epoch,
            revision: 0,
            token: "a".repeat(64),
            request: "b".repeat(32),
            mode: 2,
        };
        assert!(intent.valid(123));
        assert!(!format!("{intent:?}").contains(&intent.token));
    }
}

#[cfg(test)]
mod timing_tests {
    use std::time::Duration;

    use tokio_util::sync::CancellationToken;

    use super::*;
    #[tokio::test(start_paused = true)]
    async fn given_core_deadline_when_sampled_renewed_stopped_or_elapsed_then_one_snapshot_is_bounded_and_never_restarts_the_minute(
    ) {
        // Arrange: the real policy owns the deadline; no transport or wall clock.
        let policy = crate::visibility::VisibilityPolicy::new(crate::Visibility::Temporarily);
        let reader = policy.subscribe();
        let epoch = format!("{:032x}", 123);
        let worker = policy.start(CancellationToken::new()).unwrap();
        // Act / Assert
        for (advance, expected) in [(0, 60_000), (1, 59_999), (999, 59_000), (58_999, 1), (1, 0)] {
            tokio::time::advance(Duration::from_millis(advance)).await;
            let view = VisibilityView::project(&epoch, Some(&reader));
            assert_eq!(view.remaining_ms, expected);
            assert_eq!(view.mode, 2);
            assert!(view.running && view.valid(123));
        }
        policy.set(crate::Visibility::Temporarily, false).unwrap();
        assert_eq!(
            VisibilityView::project(&epoch, Some(&reader)).remaining_ms,
            0
        );
        let previous = VisibilityView::project(&epoch, Some(&reader)).revision;
        policy.set(crate::Visibility::Temporarily, true).unwrap();
        let renewed = VisibilityView::project(&epoch, Some(&reader));
        assert_eq!(renewed.remaining_ms, 60_000);
        assert!(renewed.revision > previous);
        for mode in [crate::Visibility::Visible, crate::Visibility::Invisible] {
            policy.set(mode, true).unwrap();
            let view = VisibilityView::project(&epoch, Some(&reader));
            assert_eq!(view.remaining_ms, 0);
            assert_eq!(view.mode, mode as u8);
        }
        drop(worker);
        let stopped = VisibilityView::project(&epoch, Some(&reader));
        assert!(!stopped.running);
        assert_eq!(stopped.remaining_ms, 0);
    }
}
