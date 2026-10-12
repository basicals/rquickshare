use std::sync::atomic::{AtomicBool, Ordering};

use super::*;

async fn scenario(operation: impl Future<Output = ()>) {
    tokio::time::timeout(std::time::Duration::from_secs(5), operation)
        .await
        .expect("lifecycle scenario deadlocked");
}

#[tokio::test]
async fn given_started_instance_when_both_entrypoints_restart_then_existing_lifetime_is_preserved()
{
    scenario(async {
        // Arrange
        let mut core = RQS::default();
        let (tracker, token, guard) = core.begin_startup().unwrap();
        // Act
        assert!(core.run().await.is_err());
        assert!(core.run_managed().await.is_err());
        // Assert
        assert!(!token.is_cancelled());
        assert!(!tracker.is_closed());
        assert!(core.tracker.is_some());
        guard.disarm();
        core.stop().await;
        assert!(token.is_cancelled());
    })
    .await;
}

#[tokio::test]
async fn given_partial_startup_when_initialization_fails_then_workers_are_joined_before_error_returns(
) {
    scenario(async {
        // Arrange
        let mut core = RQS::default();
        let (tracker, token, guard) = core.begin_startup().unwrap();
        let joined = Arc::new(AtomicBool::new(false));
        let marker = Arc::clone(&joined);
        tracker.spawn(async move {
            token.cancelled().await;
            marker.store(true, Ordering::SeqCst);
        });
        // Act
        let result = core
            .complete_startup::<()>(Err(anyhow!("controlled initialization failure")))
            .await;
        // Assert
        assert_eq!(
            result.unwrap_err().to_string(),
            "controlled initialization failure"
        );
        assert!(joined.load(Ordering::SeqCst));
        assert!(tracker.is_empty() && tracker.is_closed());
        assert!(core.tracker.is_none() && core.ctoken.is_none());
        drop(guard);
    })
    .await;
}

#[tokio::test]
async fn given_abandoned_startup_guard_when_dropped_then_started_workers_cancel_and_stop_can_join()
{
    scenario(async {
        // Arrange
        let mut core = RQS::default();
        let (tracker, token, guard) = core.begin_startup().unwrap();
        let worker_token = token.clone();
        let joined = tracker.spawn(async move {
            worker_token.cancelled().await;
        });
        // Act
        drop(guard);
        core.stop().await;
        // Assert
        assert!(token.is_cancelled());
        joined.await.unwrap();
        assert!(tracker.is_closed() && tracker.is_empty());
        assert!(core.tracker.is_none());
    })
    .await;
}

#[tokio::test]
async fn given_unclosed_tracker_when_stopped_twice_then_stop_closes_and_joins_once() {
    scenario(async {
        // Arrange
        let mut core = RQS::default();
        let (tracker, token, guard) = core.begin_startup().unwrap();
        let worker_token = token.clone();
        tracker.spawn(async move {
            worker_token.cancelled().await;
        });
        guard.disarm();
        // Act
        core.stop().await;
        core.stop().await;
        // Assert
        assert!(token.is_cancelled());
        assert!(tracker.is_closed() && tracker.is_empty());
        assert!(core.ctoken.is_none());
    })
    .await;
}

#[tokio::test]
async fn given_tcp_manager_exit_when_observed_then_error_panic_and_unexpected_success_cancel_transports(
) {
    // Arrange
    for already_stopped in [false, true] {
        for exit in ["ok", "error", "panic"] {
            let token = CancellationToken::new();
            if already_stopped {
                token.cancel();
            }
            // Act
            let failed = supervise_tcp(
                async {
                    match exit {
                        "ok" => Ok(()),
                        "error" => Err(anyhow!("controlled failure")),
                        _ => panic!("controlled supervisor panic"),
                    }
                },
                token.clone(),
            )
            .await;
            // Assert
            assert_eq!(failed, exit != "ok" || !already_stopped);
            assert!(token.is_cancelled());
        }
    }
}

#[tokio::test]
async fn given_occupied_port_when_startup_fails_then_state_is_reset_without_starting_other_transports(
) {
    scenario(async {
        // Arrange
        let occupied = TcpListener::bind("0.0.0.0:0").await.unwrap();
        let mut core = RQS::new(
            Visibility::Invisible,
            Some(occupied.local_addr().unwrap().port().into()),
            None,
        );
        // Act
        let result = core.run().await;
        // Assert
        assert!(result.is_err());
        assert!(core.tracker.is_none() && core.ctoken.is_none());
        core.stop().await;
        assert!(occupied.local_addr().is_ok());
    })
    .await;
}
