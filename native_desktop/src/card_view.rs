//! Pure presentation choices; consent and transfer ownership stay in the controller.
use rqs_lib::service::{Direction, Phase};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Mode {
    #[default]
    Compact,
    Full,
}
impl Mode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "compact" => Some(Self::Compact),
            "full" => Some(Self::Full),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Full => "full",
        }
    }
}

pub fn compact_index(transfers: impl IntoIterator<Item = (Direction, Phase)>) -> Option<usize> {
    let mut active = None;
    for (index, (direction, phase)) in transfers.into_iter().enumerate() {
        if direction == Direction::Inbound && phase == Phase::AwaitingConsent {
            return Some(index);
        }
        if active.is_none() && !phase.is_terminal() {
            active = Some(index);
        }
    }
    active
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn given_bounded_current_transfers_when_projected_then_pending_inbound_precedes_active_and_terminal_is_not_pending(
    ) {
        // Arrange / Act / Assert
        assert_eq!(compact_index([]), None);
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
            for direction in [Direction::Inbound, Direction::Outbound] {
                assert_eq!(
                    compact_index([(direction, phase)]),
                    if phase.is_terminal() { None } else { Some(0) }
                );
                assert_eq!(
                    compact_index([
                        (direction, phase),
                        (Direction::Inbound, Phase::AwaitingConsent)
                    ]),
                    if direction == Direction::Inbound && phase == Phase::AwaitingConsent {
                        Some(0)
                    } else {
                        Some(1)
                    }
                );
            }
        }
        assert_eq!(
            compact_index([
                (Direction::Inbound, Phase::Finished),
                (Direction::Outbound, Phase::Sending),
                (Direction::Inbound, Phase::Receiving)
            ]),
            Some(1)
        );
        assert_eq!(
            compact_index([(Direction::Inbound, Phase::AwaitingConsent); 64]),
            Some(0)
        );
        assert_eq!(Mode::parse("full"), Some(Mode::Full));
        assert_eq!(Mode::parse("compact"), Some(Mode::default()));
        for value in ["", "Full", "full\0", "unknown"] {
            assert_eq!(Mode::parse(value), None);
        }
    }
}
