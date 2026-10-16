//! Current core policy and explicit saved visibility intent; no second expiry owner.
use serde::{Deserialize, Serialize};
use zbus::zvariant::Type;

use super::native_settings::Base;

pub const INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Visibility1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VisibilityView {
    pub epoch: String,
    pub available: bool,
    pub revision: u64,
    pub running: bool,
    pub mode: u8,
}
impl VisibilityView {
    pub(super) fn project(epoch: &str, reader: Option<&crate::VisibilityReader>) -> Self {
        let observed = reader.map(crate::VisibilityReader::observation);
        let (mode, revision, running) =
            observed.unwrap_or((crate::Visibility::Invisible, 0, false));
        Self {
            epoch: epoch.into(),
            available: observed.is_some(),
            revision,
            running,
            mode: if running { mode as u8 } else { 1 },
        }
    }
    pub fn valid(&self, epoch: u128) -> bool {
        self.epoch == format!("{epoch:032x}")
            && self.mode <= 2
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
        for case in 0..4 {
            let mut view = unavailable.clone();
            match case {
                0 => view.epoch = "bad".into(),
                1 => view.mode = 3,
                2 => view.running = true,
                3 => view.mode = 0,
                _ => unreachable!(),
            }
            assert!(!view.valid(123));
        }
        assert_eq!(VisibilityView::SIGNATURE.to_string(), "(sbtby)");
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
