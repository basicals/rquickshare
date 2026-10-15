use super::*;

const MODES: [Visibility; 3] = [
    Visibility::Visible,
    Visibility::Invisible,
    Visibility::Temporarily,
];

#[tokio::test]
async fn given_independent_unstarted_owners_when_modes_change_then_readers_observe_only_their_owner(
) {
    // Arrange
    for initial in MODES {
        let policy = VisibilityPolicy::new(initial);
        let other = VisibilityPolicy::new(initial);
        let mut reader = policy.subscribe();
        let mut cloned = reader.clone();
        assert!(!reader.has_changed().unwrap());
        // Act / Assert
        for mode in MODES {
            policy.set(mode, true).unwrap();
            reader.changed().await.unwrap();
            assert_eq!(reader.current_and_update(), mode);
            assert!(cloned.has_changed().unwrap());
            assert_eq!(cloned.current_and_update(), mode);
            assert!(!reader.has_changed().unwrap());
            assert_eq!(other.subscribe().current(), initial);
            let snapshot = *policy.sender.borrow();
            assert!(!snapshot.running);
            assert_eq!(snapshot.deadline, None);
        }
        drop(policy);
        assert!(reader.changed().await.is_err());
        assert!(reader.has_changed().is_err());
        assert_eq!(reader.current(), Visibility::Temporarily);
    }
}

#[tokio::test(start_paused = true)]
async fn given_running_modes_when_config_repeats_or_explicit_command_renews_then_only_intent_advances(
) {
    for mode in MODES {
        // Arrange
        let policy = VisibilityPolicy::new(mode);
        let lifetime = CancellationToken::new();
        let worker = policy.start(lifetime.clone()).unwrap();
        let before = *policy.sender.borrow();
        let mut reader = policy.subscribe();
        tokio::time::advance(Duration::from_secs(10)).await;
        // Act / Assert
        policy.set(mode, false).unwrap();
        assert_eq!(*policy.sender.borrow(), before);
        assert!(!reader.has_changed().unwrap());
        policy.set(mode, true).unwrap();
        let after = *policy.sender.borrow();
        assert_eq!(after.revision, before.revision + 1);
        assert_eq!(after.mode, mode);
        assert!(reader.has_changed().unwrap());
        reader.current_and_update();
        assert_eq!(
            after.deadline,
            before
                .deadline
                .map(|deadline| deadline + Duration::from_secs(10))
        );
        lifetime.cancel();
        worker.await;
    }
}

#[tokio::test(start_paused = true)]
async fn given_exhausted_revision_when_start_or_command_is_admitted_then_hidden_cannot_wrap() {
    // Arrange
    for running in [false, true] {
        let policy = VisibilityPolicy::new(Visibility::Visible);
        let lifetime = CancellationToken::new();
        let worker = running.then(|| policy.start(lifetime.clone()).unwrap());
        policy
            .sender
            .send_modify(|snapshot| snapshot.revision = u64::MAX);
        // Act
        assert_eq!(
            policy.set(Visibility::Temporarily, true),
            Err(VisibilityError::RevisionExhausted)
        );
        // Assert
        let snapshot = *policy.sender.borrow();
        assert_eq!(snapshot.revision, u64::MAX);
        assert_eq!(snapshot.mode, Visibility::Invisible);
        assert_eq!(snapshot.deadline, None);
        assert_eq!(snapshot.running, running);
        assert_eq!(
            policy.start(CancellationToken::new()).err(),
            Some(if running {
                VisibilityError::AlreadyRunning
            } else {
                VisibilityError::RevisionExhausted
            })
        );
        assert_eq!(*policy.sender.borrow(), snapshot);
        lifetime.cancel();
        if let Some(worker) = worker {
            worker.await;
        }
    }
}

#[tokio::test(start_paused = true)]
async fn given_matching_deadline_when_before_at_or_after_boundary_then_only_due_policy_expires() {
    for offset in [-1_i8, 0, 1] {
        // Arrange
        let policy = VisibilityPolicy::new(Visibility::Temporarily);
        let lifetime = CancellationToken::new();
        let worker = policy.start(lifetime.clone()).unwrap();
        let before = *policy.sender.borrow();
        let deadline = before.deadline.unwrap();
        let now = match offset {
            -1 => deadline - Duration::from_nanos(1),
            0 => deadline,
            _ => deadline + Duration::from_nanos(1),
        };
        // Act
        policy.expire(before, now);
        // Assert
        let after = *policy.sender.borrow();
        if offset < 0 {
            assert_eq!(after, before);
        } else {
            assert_eq!(after.mode, Visibility::Invisible);
            assert_eq!(after.deadline, None);
            assert_eq!(after.revision, before.revision + 1);
            assert!(after.running);
            policy.expire(before, now);
            assert_eq!(*policy.sender.borrow(), after);
        }
        lifetime.cancel();
        worker.await;
    }
}

#[tokio::test(start_paused = true)]
async fn given_superseded_or_inactive_window_when_old_expiry_fires_then_new_policy_is_preserved() {
    for mode in MODES {
        // Arrange
        let policy = VisibilityPolicy::new(Visibility::Temporarily);
        let lifetime = CancellationToken::new();
        let worker = policy.start(lifetime.clone()).unwrap();
        let old = *policy.sender.borrow();
        // Act: same-time renewal deliberately has the same deadline, a new revision.
        policy.set(mode, true).unwrap();
        let current = *policy.sender.borrow();
        policy.expire(old, old.deadline.unwrap() + TEMPORARY_DURATION);
        // Assert
        assert_eq!(*policy.sender.borrow(), current);
        if mode == Visibility::Temporarily {
            assert_eq!(old.deadline, current.deadline);
            let mut wrong_deadline = current;
            wrong_deadline.deadline = Some(current.deadline.unwrap() + TEMPORARY_DURATION);
            policy.expire(wrong_deadline, wrong_deadline.deadline.unwrap());
            assert_eq!(*policy.sender.borrow(), current);
        }
        lifetime.cancel();
        worker.await;
        let stopped = *policy.sender.borrow();
        policy.expire(current, old.deadline.unwrap() + TEMPORARY_DURATION);
        assert_eq!(*policy.sender.borrow(), stopped);
        assert!(!stopped.running);
        assert_eq!(stopped.deadline, None);
    }
}

#[tokio::test(start_paused = true)]
async fn given_delayed_first_poll_without_transports_when_deadline_passes_then_worker_expires_original_window(
) {
    // Arrange
    let policy = VisibilityPolicy::new(Visibility::Temporarily);
    let lifetime = CancellationToken::new();
    let worker = policy.start(lifetime.clone()).unwrap();
    let started = *policy.sender.borrow();
    let mut reader = policy.subscribe();
    assert_eq!(started.deadline, Some(Instant::now() + TEMPORARY_DURATION));
    tokio::time::advance(TEMPORARY_DURATION + Duration::from_secs(1)).await;
    // Act
    let task = tokio::spawn(worker);
    reader.changed().await.unwrap();
    // Assert
    assert_eq!(reader.current(), Visibility::Invisible);
    assert!(policy.sender.borrow().running);
    assert!(!lifetime.is_cancelled());
    lifetime.cancel();
    task.await.unwrap();
    assert!(!policy.sender.borrow().running);
}

#[tokio::test(start_paused = true)]
async fn given_cancelled_worker_when_restarted_then_fresh_window_has_one_joined_owner() {
    // Arrange
    let policy = VisibilityPolicy::new(Visibility::Temporarily);
    let lifetime = CancellationToken::new();
    let worker = policy.start(lifetime.clone()).unwrap();
    let first = *policy.sender.borrow();
    // Act / Assert
    assert_eq!(
        policy.start(CancellationToken::new()).err(),
        Some(VisibilityError::AlreadyRunning)
    );
    assert_eq!(*policy.sender.borrow(), first);
    tokio::time::advance(Duration::from_secs(10)).await;
    lifetime.cancel();
    worker.await;
    let stopped = *policy.sender.borrow();
    assert_eq!(stopped.mode, Visibility::Temporarily);
    assert!(!stopped.running);
    assert_eq!(stopped.deadline, None);
    let next_lifetime = CancellationToken::new();
    let task = tokio::spawn(policy.start(next_lifetime.clone()).unwrap());
    assert_eq!(
        policy.sender.borrow().deadline,
        Some(first.deadline.unwrap() + Duration::from_secs(10))
    );
    let mut reader = policy.subscribe();
    tokio::time::advance(TEMPORARY_DURATION).await;
    reader.changed().await.unwrap();
    assert_eq!(reader.current(), Visibility::Invisible);
    next_lifetime.cancel();
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn given_unpolled_or_polled_worker_when_abandoned_then_guard_hides_and_cancels_core_lifetime()
{
    for polled in [false, true] {
        // Arrange
        let policy = VisibilityPolicy::new(Visibility::Temporarily);
        let lifetime = CancellationToken::new();
        let mut worker = Box::pin(policy.start(lifetime.clone()).unwrap());
        // Act
        if polled {
            assert!(futures::poll!(worker.as_mut()).is_pending());
        }
        drop(worker);
        // Assert
        assert!(lifetime.is_cancelled());
        assert_eq!(policy.subscribe().current(), Visibility::Invisible);
        assert!(!policy.sender.borrow().running);
        assert_eq!(policy.sender.borrow().deadline, None);
    }
}
