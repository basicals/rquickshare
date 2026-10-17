use tokio::sync::oneshot;

use super::*;

async fn scenario(operation: impl Future<Output = ()>) {
    tokio::time::timeout(std::time::Duration::from_secs(5), operation)
        .await
        .expect("task ownership deadlocked");
}

#[tokio::test]
async fn given_task_budget_when_full_or_invalid_then_admission_is_bounded_and_completion_returns_capacity(
) {
    scenario(async {
        // Arrange
        let parent = CancellationToken::new();
        for limit in [0, Semaphore::MAX_PERMITS + 1] {
            assert!(matches!(
                SessionTasks::new(&parent, limit),
                Err(SessionError::Invalid)
            ));
        }
        let group = SessionTasks::new(&parent, 1).unwrap();
        let (release, wait) = oneshot::channel();
        let first = group
            .spawn(async move {
                let _ = wait.await;
            })
            .unwrap();
        // Act / Assert
        assert!(matches!(group.spawn(async {}), Err(SessionError::Full)));
        release.send(()).unwrap();
        first.await.unwrap();
        group.spawn(async {}).unwrap().await.unwrap();
        group.stop().await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn given_owned_worker_when_group_stops_twice_then_cancellation_is_joined_without_cancelling_caller(
) {
    scenario(async {
        // Arrange
        let parent = CancellationToken::new();
        let group = SessionTasks::new(&parent, 2).unwrap();
        let lifetime = group.lifetime();
        let completed = Arc::new(AtomicBool::new(false));
        let marker = Arc::clone(&completed);
        let (started, ready) = oneshot::channel();
        let worker = group
            .spawn(async move {
                started.send(()).unwrap();
                lifetime.cancelled().await;
                marker.store(true, Ordering::SeqCst);
            })
            .unwrap();
        ready.await.unwrap();
        // Act
        group.stop().await.unwrap();
        group.stop().await.unwrap();
        // Assert
        assert!(completed.load(Ordering::SeqCst));
        assert!(!parent.is_cancelled());
        worker.await.unwrap();
        assert!(matches!(group.spawn(async {}), Err(SessionError::Closed)));
    })
    .await;
}

#[tokio::test]
async fn given_migrated_worker_when_bridge_finishes_then_group_keeps_worker_owned_until_service_stop(
) {
    scenario(async {
        // Arrange
        let parent = CancellationToken::new();
        let group = SessionTasks::new(&parent, 2).unwrap();
        let lifetime = group.lifetime();
        let completed = Arc::new(AtomicBool::new(false));
        let marker = Arc::clone(&completed);
        let worker = group
            .spawn(async move {
                lifetime.cancelled().await;
                marker.store(true, Ordering::SeqCst);
            })
            .unwrap();
        // Act
        group.spawn(async {}).unwrap().await.unwrap();
        // Assert
        assert!(!completed.load(Ordering::SeqCst));
        assert!(!worker.is_finished());
        group.stop().await.unwrap();
        assert!(completed.load(Ordering::SeqCst));
        worker.await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn given_panicking_worker_when_supervised_then_sibling_cancels_and_stop_reports_loss_after_joining(
) {
    scenario(async {
        // Arrange
        let parent = CancellationToken::new();
        let group = SessionTasks::new(&parent, 2).unwrap();
        let lifetime = group.lifetime();
        let completed = Arc::new(AtomicBool::new(false));
        let marker = Arc::clone(&completed);
        let sibling = group
            .spawn(async move {
                lifetime.cancelled().await;
                marker.store(true, Ordering::SeqCst);
            })
            .unwrap();
        // Act
        group
            .spawn(async {
                panic!("controlled worker panic");
            })
            .unwrap()
            .await
            .unwrap();
        // Assert
        assert_eq!(group.stop().await, Err(SessionError::OwnerLost));
        assert!(completed.load(Ordering::SeqCst));
        assert!(!parent.is_cancelled());
        sibling.await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn given_closed_admission_when_late_callback_arrives_then_it_cannot_spawn_after_join_barrier()
{
    scenario(async {
        // Arrange
        let group = SessionTasks::new(&CancellationToken::new(), 2).unwrap();
        let late_callback = group.clone();
        group.stop().await.unwrap();
        let polled = Arc::new(AtomicBool::new(false));
        let marker = Arc::clone(&polled);
        // Act
        let result = late_callback.spawn(async move {
            marker.store(true, Ordering::SeqCst);
        });
        // Assert
        assert!(matches!(result, Err(SessionError::Closed)));
        assert!(!polled.load(Ordering::SeqCst));
        assert_eq!(Arc::strong_count(&polled), 1);
    })
    .await;
}

#[tokio::test]
async fn given_parent_shutdown_when_group_is_active_then_children_stop_and_late_admission_fails() {
    scenario(async {
        // Arrange
        let parent = CancellationToken::new();
        let group = SessionTasks::new(&parent, 1).unwrap();
        let lifetime = group.lifetime();
        let worker = group
            .spawn(async move {
                lifetime.cancelled().await;
            })
            .unwrap();
        // Act
        parent.cancel();
        assert!(matches!(group.spawn(async {}), Err(SessionError::Closed)));
        group.stop().await.unwrap();
        // Assert
        worker.await.unwrap();
        assert!(matches!(group.spawn(async {}), Err(SessionError::Closed)));
    })
    .await;
}
