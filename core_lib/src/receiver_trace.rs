//! Explicit local discovery diagnostics: static stage names and monotonic time only.
use std::{ffi::OsStr, sync::OnceLock, time::Instant};
#[derive(Clone, Copy)]
pub enum Event {
    Start,
    Visible,
    Hidden,
    AdvertisementStarted,
    AdvertisementFailed,
    AdvertisementStopped,
    AdvertisementImmediate,
    AdvertisementDeferred,
    AdvertisementPeriodic,
    GattRead,
    GattHeaderReady,
    MdnsRegister,
    MdnsResend,
}
impl Event {
    fn name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Visible => "visible",
            Self::Hidden => "hidden",
            Self::AdvertisementStarted => "advertisement_started",
            Self::AdvertisementFailed => "advertisement_failed",
            Self::AdvertisementStopped => "advertisement_stopped",
            Self::AdvertisementImmediate => "advertisement_immediate",
            Self::AdvertisementDeferred => "advertisement_deferred",
            Self::AdvertisementPeriodic => "advertisement_periodic",
            Self::GattRead => "gatt_slot0_read",
            Self::GattHeaderReady => "gatt_header_ready",
            Self::MdnsRegister => "mdns_register",
            Self::MdnsResend => "mdns_resend",
        }
    }
}
fn enabled(value: Option<&OsStr>) -> bool {
    value == Some(OsStr::new("1"))
}
fn line(event: Event, elapsed: u128) -> String {
    format!("rqs-receiver event={} elapsed_ms={elapsed}", event.name())
}
pub fn event(event: Event) {
    static CLOCK: OnceLock<(bool, Instant)> = OnceLock::new();
    let (on, start) = CLOCK.get_or_init(|| {
        (
            enabled(std::env::var_os("RQUICKSHARE_RECEIVER_TRACE").as_deref()),
            Instant::now(),
        )
    });
    if *on {
        eprintln!("{}", line(event, start.elapsed().as_millis()));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn given_diagnostic_flag_and_stages_when_formatted_then_only_explicit_opt_in_and_allowlisted_numeric_output_exist(
    ) {
        for value in [
            None,
            Some(""),
            Some("0"),
            Some("true"),
            Some(" 1"),
            Some("1 "),
        ] {
            assert!(!enabled(value.map(OsStr::new)));
        }
        assert!(enabled(Some(OsStr::new("1"))));
        for stage in [
            Event::Start,
            Event::Visible,
            Event::Hidden,
            Event::AdvertisementStarted,
            Event::AdvertisementFailed,
            Event::AdvertisementStopped,
            Event::AdvertisementImmediate,
            Event::AdvertisementDeferred,
            Event::AdvertisementPeriodic,
            Event::GattRead,
            Event::GattHeaderReady,
            Event::MdnsRegister,
            Event::MdnsResend,
        ] {
            for elapsed in [0, 1, u128::MAX] {
                let message = line(stage, elapsed);
                assert_eq!(message.split_whitespace().count(), 3);
                assert!(message.ends_with(&format!("elapsed_ms={elapsed}")));
                assert!(!message.contains('/'));
            }
        }
    }
}
