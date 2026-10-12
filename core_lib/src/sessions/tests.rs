use super::*;

fn hub() -> SessionHub {
    SessionHub::new(17).unwrap()
}
fn lease(hub: &SessionHub) -> SessionLease {
    hub.open(TransferType::Inbound, None, CancellationToken::new())
        .unwrap()
}
fn waiting(hub: &SessionHub) -> (SessionLease, SessionControl, ConsentToken) {
    let mut session = lease(hub);
    session.observe(&State::WaitingForUserConsent);
    let control = session.control();
    let token = control.snapshot().consent.unwrap();
    (session, control, token)
}
fn states() -> Vec<State> {
    vec![
        State::Initial,
        State::ReceivedConnectionRequest,
        State::SentUkeyServerInit,
        State::SentUkeyClientInit,
        State::SentUkeyClientFinish,
        State::SentPairedKeyEncryption,
        State::ReceivedUkeyClientFinish,
        State::SentConnectionResponse,
        State::SentPairedKeyResult,
        State::SentIntroduction,
        State::ReceivedPairedKeyResult,
        State::WaitingForUserConsent,
        State::ReceivingFiles,
        State::SendingFiles,
        State::Disconnected,
        State::Rejected,
        State::Cancelled,
        State::Finished,
    ]
}

#[test]
fn given_zero_epoch_or_budget_when_hub_is_created_then_invalid_configuration_is_rejected() {
    // Arrange / Act / Assert
    for (epoch, limit) in [(0, 1), (1, 0), (0, 0)] {
        assert!(matches!(
            SessionHub::with_limit(epoch, limit),
            Err(SessionError::Invalid)
        ));
    }
}

#[test]
fn given_different_run_epochs_when_sessions_are_opened_then_old_capabilities_cannot_alias() {
    // Arrange
    let left = SessionHub::new(1).unwrap();
    let right = SessionHub::new(2).unwrap();
    // Act
    let one = lease(&left);
    let two = lease(&right);
    // Assert
    assert_ne!(one.id, two.id);
    assert_ne!(one.event_id(), two.event_id());
}

#[test]
fn given_completed_incarnation_when_new_session_opens_then_identity_is_never_reused() {
    // Arrange
    let hub = hub();
    let old = lease(&hub);
    let id = old.id;
    // Act
    drop(old);
    let current = lease(&hub);
    // Assert
    assert_ne!(current.id, id);
    assert_eq!(current.id.sequence, id.sequence + 1);
}

#[test]
fn given_exact_session_budget_when_owner_drops_then_retained_controls_do_not_hold_the_slot() {
    // Arrange
    let hub = SessionHub::with_limit(1, 1).unwrap();
    let owner = lease(&hub);
    let control = owner.control();
    // Act / Assert
    assert!(matches!(
        hub.open(TransferType::Inbound, None, CancellationToken::new()),
        Err(SessionError::Full)
    ));
    drop(owner);
    assert!(control.is_closed());
    let next = lease(&hub);
    assert_ne!(next.id, control.snapshot().id);
    assert_eq!(hub.active().unwrap().len(), 1);
}

#[test]
fn given_exhausted_session_sequence_when_opened_then_no_partial_registration_or_alias_occurs() {
    // Arrange
    let hub = hub();
    hub.0.lock().unwrap().sequence = u64::MAX - 1;
    let last = lease(&hub);
    assert_eq!(last.id.sequence, u64::MAX);
    drop(last);
    // Act / Assert
    assert!(matches!(
        hub.open(TransferType::Inbound, None, CancellationToken::new()),
        Err(SessionError::Exhausted)
    ));
    assert!(hub.active().unwrap().is_empty());
    assert_eq!(hub.0.lock().unwrap().sequence, u64::MAX);
}

#[test]
fn given_live_hub_when_closed_twice_then_all_owners_cancel_and_admission_stays_closed() {
    // Arrange
    let hub = hub();
    let owner = lease(&hub);
    // Act
    hub.close();
    hub.close();
    // Assert
    assert!(owner.cancellation().is_cancelled());
    assert!(matches!(
        hub.open(TransferType::Inbound, None, CancellationToken::new()),
        Err(SessionError::Closed)
    ));
    assert_eq!(hub.active().unwrap().len(), 1);
    drop(owner);
    assert!(hub.active().unwrap().is_empty());
}

#[test]
fn given_outbound_correlation_when_session_is_registered_then_capability_is_independent_and_debug_is_redacted(
) {
    // Arrange
    let hub = hub();
    let private = "secret/path/and/peer";
    // Act
    let owner = hub
        .open(
            TransferType::Outbound,
            Some(private.into()),
            CancellationToken::new(),
        )
        .unwrap();
    // Assert
    assert_eq!(owner.event_id(), private);
    assert!(!format!("{owner:?}").contains(private));
    assert!(!format!("{:?}", owner.control().snapshot()).contains(private));
    assert_eq!(owner.id.sequence, 1);
}

#[test]
fn given_protocol_phase_or_direction_when_responded_then_only_inbound_current_wait_accepts_commands(
) {
    // Arrange
    for direction in [TransferType::Inbound, TransferType::Outbound] {
        for state in states() {
            let hub = hub();
            let mut owner = hub
                .open(direction.clone(), None, CancellationToken::new())
                .unwrap();
            owner.observe(&state);
            let control = owner.control();
            let token = control.snapshot().consent.unwrap_or(ConsentToken {
                session: owner.id,
                round: 1,
            });
            // Act
            let result = control.respond(token, 1, true);
            // Assert
            match (&direction, &state) {
                (TransferType::Inbound, State::WaitingForUserConsent) => assert!(result.is_ok()),
                (TransferType::Outbound, _) => {
                    assert!(matches!(result, Err(SessionError::WrongDirection)))
                }
                _ => assert!(matches!(result, Err(SessionError::Stale))),
            }
        }
    }
}

#[test]
fn given_invalid_command_or_foreign_token_when_responded_then_mailbox_remains_empty() {
    // Arrange
    let hub = hub();
    let (mut owner, control, token) = waiting(&hub);
    let other = lease(&hub);
    // Act / Assert
    for (token, command, error) in [
        (token, 0, SessionError::Invalid),
        (
            ConsentToken {
                session: other.id,
                round: token.round,
            },
            1,
            SessionError::Stale,
        ),
        (
            ConsentToken {
                round: token.round + 1,
                ..token
            },
            1,
            SessionError::Stale,
        ),
    ] {
        assert!(matches!(control.respond(token, command, true), Err(actual) if actual == error));
        assert!(owner.mailbox.try_recv().is_err());
    }
}

#[tokio::test]
async fn given_queued_consent_when_waiting_for_receipt_then_only_handler_validation_acknowledges() {
    // Arrange
    let hub = hub();
    let (mut owner, control, token) = waiting(&hub);
    let mut receipt = control.respond(token, 7, true).unwrap();
    // Act / Assert
    assert!(matches!(
        receipt.0.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    let command = owner.mailbox.recv().await.unwrap();
    assert_eq!(
        owner.apply(command, &State::WaitingForUserConsent),
        Some(true)
    );
    receipt.wait().await.unwrap();
    assert_eq!(control.snapshot().state, State::WaitingForUserConsent);
}

#[tokio::test]
async fn given_accepted_consent_when_exact_command_replays_after_progress_then_receipt_replays_without_side_effect(
) {
    // Arrange
    for later in [
        State::WaitingForUserConsent,
        State::ReceivingFiles,
        State::Finished,
    ] {
        let hub = hub();
        let (mut owner, control, token) = waiting(&hub);
        let first = control.respond(token, 3, true).unwrap();
        let command = owner.mailbox.recv().await.unwrap();
        assert_eq!(
            owner.apply(command, &State::WaitingForUserConsent),
            Some(true)
        );
        first.wait().await.unwrap();
        owner.observe(&later);
        // Act
        control
            .respond(token, 3, true)
            .unwrap()
            .wait()
            .await
            .unwrap();
        // Assert
        assert!(owner.mailbox.try_recv().is_err());
        assert_eq!(control.snapshot().state, later);
    }
}

#[tokio::test]
async fn given_accepted_decision_when_command_or_answer_conflicts_then_replay_is_rejected() {
    // Arrange
    let hub = hub();
    let (mut owner, control, token) = waiting(&hub);
    let receipt = control.respond(token, 1, true).unwrap();
    let command = owner.mailbox.recv().await.unwrap();
    owner.apply(command, &State::WaitingForUserConsent);
    receipt.wait().await.unwrap();
    // Act / Assert
    for (command, accept) in [(1, false), (2, true), (2, false)] {
        assert!(matches!(
            control.respond(token, command, accept),
            Err(SessionError::Conflict)
        ));
        assert!(owner.mailbox.try_recv().is_err());
    }
}

#[tokio::test]
async fn given_full_consent_mailbox_when_processing_frees_capacity_then_exact_retry_obtains_receipt(
) {
    // Arrange
    let hub = hub();
    let (mut owner, control, token) = waiting(&hub);
    let receipt = control.respond(token, 2, false).unwrap();
    // Act / Assert
    assert!(matches!(
        control.respond(token, 2, false),
        Err(SessionError::Full)
    ));
    let command = owner.mailbox.recv().await.unwrap();
    assert_eq!(
        owner.apply(command, &State::WaitingForUserConsent),
        Some(false)
    );
    receipt.wait().await.unwrap();
    control
        .respond(token, 2, false)
        .unwrap()
        .wait()
        .await
        .unwrap();
    assert!(owner.mailbox.try_recv().is_err());
}

#[tokio::test]
async fn given_cancelled_session_when_pending_consent_is_processed_then_no_consent_is_applied() {
    // Arrange
    let hub = hub();
    let (mut owner, control, token) = waiting(&hub);
    let receipt = control.respond(token, 1, true).unwrap();
    // Act
    control.cancel();
    let command = owner.mailbox.recv().await.unwrap();
    // Assert
    assert_eq!(owner.apply(command, &State::WaitingForUserConsent), None);
    assert_eq!(receipt.wait().await, Err(SessionError::Cancelled));
    assert!(matches!(
        control.respond(token, 1, true),
        Err(SessionError::Cancelled)
    ));
}

#[tokio::test]
async fn given_actual_handler_phase_changed_when_queued_consent_is_checked_then_stale_command_has_no_effect(
) {
    // Arrange
    for state in states()
        .into_iter()
        .filter(|state| *state != State::WaitingForUserConsent)
    {
        let hub = hub();
        let (mut owner, control, token) = waiting(&hub);
        let receipt = control.respond(token, 1, true).unwrap();
        // Act
        let command = owner.mailbox.recv().await.unwrap();
        // Assert
        assert_eq!(owner.apply(command, &state), None);
        assert_eq!(receipt.wait().await, Err(SessionError::Stale));
    }
}

#[tokio::test]
async fn given_unprocessed_command_when_session_owner_disappears_then_receipt_reports_loss() {
    // Arrange
    let hub = hub();
    let (owner, control, token) = waiting(&hub);
    let receipt = control.respond(token, 1, true).unwrap();
    // Act
    drop(owner);
    // Assert
    assert_eq!(receipt.wait().await, Err(SessionError::OwnerLost));
    assert_eq!(control.snapshot().state, State::WaitingForUserConsent);
}

#[tokio::test]
async fn given_dropped_receipt_when_handler_processes_then_accepted_decision_is_not_rolled_back() {
    // Arrange
    let hub = hub();
    let (mut owner, control, token) = waiting(&hub);
    drop(control.respond(token, 1, true).unwrap());
    // Act
    let command = owner.mailbox.recv().await.unwrap();
    // Assert
    assert_eq!(
        owner.apply(command, &State::WaitingForUserConsent),
        Some(true)
    );
    control
        .respond(token, 1, true)
        .unwrap()
        .wait()
        .await
        .unwrap();
}

#[test]
fn given_repeated_wait_and_new_round_when_state_updates_then_only_new_round_invalidates_old_tokens()
{
    // Arrange
    let hub = hub();
    let (mut owner, control, token) = waiting(&hub);
    // Act / Assert
    owner.observe(&State::WaitingForUserConsent);
    assert_eq!(control.snapshot().consent, Some(token));
    owner.observe(&State::ReceivingFiles);
    owner.observe(&State::WaitingForUserConsent);
    let next = control.snapshot().consent.unwrap();
    assert_ne!(next, token);
    assert_eq!(next.round, token.round + 1);
    assert!(matches!(
        control.respond(token, 1, true),
        Err(SessionError::Stale)
    ));
}

#[test]
fn given_exhausted_consent_round_when_waiting_begins_then_owner_cancels_without_token_alias() {
    // Arrange
    let hub = hub();
    let mut owner = lease(&hub);
    owner.round = u64::MAX - 1;
    owner.observe(&State::WaitingForUserConsent);
    assert_eq!(owner.control().snapshot().consent.unwrap().round, u64::MAX);
    assert!(!owner.cancellation().is_cancelled());
    owner.observe(&State::ReceivingFiles);
    // Act
    owner.observe(&State::WaitingForUserConsent);
    // Assert
    assert!(owner.cancellation().is_cancelled());
    assert!(owner.control().snapshot().consent.is_none());
    assert_eq!(
        owner.control().snapshot().state,
        State::WaitingForUserConsent
    );
    assert_eq!(owner.round, u64::MAX);
}

#[tokio::test]
async fn given_live_status_when_observed_then_watch_publishes_actual_state_and_token() {
    // Arrange
    let hub = hub();
    let mut owner = lease(&hub);
    let mut control = owner.control();
    // Act
    owner.observe(&State::WaitingForUserConsent);
    let snapshot = control.changed().await.unwrap();
    // Assert
    assert_eq!(snapshot.id, owner.id);
    assert_eq!(snapshot.state, State::WaitingForUserConsent);
    assert_eq!(snapshot.consent, control.snapshot().consent);
    assert!(snapshot.consent.is_some());
}

#[test]
fn given_terminal_session_when_cancel_and_late_states_arrive_then_every_terminal_outcome_is_absorbing(
) {
    // Arrange
    for terminal in [
        State::Finished,
        State::Rejected,
        State::Cancelled,
        State::Disconnected,
    ] {
        for later in states() {
            let hub = hub();
            let mut owner = lease(&hub);
            owner.observe(&terminal);
            // Act
            owner.control().cancel();
            owner.observe(&later);
            // Assert
            assert_eq!(owner.control().snapshot().state, terminal);
            assert!(owner.control().snapshot().consent.is_none());
        }
    }
}

#[tokio::test]
async fn given_retained_control_when_owner_closes_then_watch_and_mailbox_close_without_fake_terminal(
) {
    // Arrange
    let hub = hub();
    let (owner, mut control, token) = waiting(&hub);
    control.status.borrow_and_update();
    // Act
    drop(owner);
    // Assert
    assert!(control.is_closed());
    assert!(matches!(control.changed().await, Err(SessionError::Closed)));
    assert!(matches!(
        control.respond(token, 1, true),
        Err(SessionError::Closed)
    ));
    assert_eq!(control.snapshot().state, State::WaitingForUserConsent);
    assert!(hub.active().unwrap().is_empty());
}

#[test]
fn given_client_control_when_dropped_then_service_owned_session_stays_live_and_uncancelled() {
    // Arrange
    let hub = hub();
    let owner = lease(&hub);
    // Act
    drop(owner.control());
    // Assert
    assert!(!owner.cancellation().is_cancelled());
    assert_eq!(hub.active().unwrap().len(), 1);
    assert!(!owner.control().is_closed());
}

#[tokio::test]
async fn given_consent_queued_before_first_ack_when_first_command_applies_then_duplicate_replays_and_conflict_fails(
) {
    // Arrange
    for (command_id, accept, expected) in [
        (1, true, Ok(())),
        (1, false, Err(SessionError::Conflict)),
        (2, true, Err(SessionError::Conflict)),
    ] {
        let hub = hub();
        let (mut owner, control, token) = waiting(&hub);
        let first = control.respond(token, 1, true).unwrap();
        let first_command = owner.mailbox.recv().await.unwrap();
        let second = control.respond(token, command_id, accept).unwrap();
        // Act
        assert_eq!(
            owner.apply(first_command, &State::WaitingForUserConsent),
            Some(true)
        );
        owner.observe(&State::ReceivingFiles);
        let queued = owner.mailbox.recv().await.unwrap();
        // Assert
        assert_eq!(owner.apply(queued, &State::ReceivingFiles), None);
        first.wait().await.unwrap();
        assert_eq!(second.wait().await, expected);
        assert_eq!(control.snapshot().state, State::ReceivingFiles);
    }
}

#[tokio::test]
async fn given_consent_queued_in_old_round_when_new_wait_begins_then_old_answer_cannot_accept_new_round(
) {
    // Arrange
    let hub = hub();
    let (mut owner, control, old) = waiting(&hub);
    let obsolete = control.respond(old, 1, true).unwrap();
    // Act
    owner.observe(&State::ReceivingFiles);
    owner.observe(&State::WaitingForUserConsent);
    let current = control.snapshot().consent.unwrap();
    let stale = owner.mailbox.recv().await.unwrap();
    // Assert
    assert_eq!(owner.apply(stale, &State::WaitingForUserConsent), None);
    assert_eq!(obsolete.wait().await, Err(SessionError::Stale));
    let live = control.respond(current, 1, false).unwrap();
    let command = owner.mailbox.recv().await.unwrap();
    assert_eq!(
        owner.apply(command, &State::WaitingForUserConsent),
        Some(false)
    );
    live.wait().await.unwrap();
}

#[test]
fn given_short_lived_session_when_observed_later_then_lifecycle_order_survives_owner_drop() {
    // Arrange
    let hub = SessionHub::new(7).unwrap();
    let mut events = hub.subscribe_fresh().unwrap();
    let mut lease = hub
        .open(TransferType::Inbound, None, CancellationToken::new())
        .unwrap();
    let id = lease.control().snapshot().id;
    // Act
    lease.observe(&State::WaitingForUserConsent);
    lease.observe(&State::Finished);
    lease.observe(&State::Disconnected);
    drop(lease);
    // Assert
    assert!(
        matches!(events.try_recv().unwrap(), SessionEvent::Opened(control) if control.snapshot().id == id)
    );
    assert!(
        matches!(events.try_recv().unwrap(), SessionEvent::Changed(s) if s.id == id && s.consent.is_some())
    );
    assert!(
        matches!(events.try_recv().unwrap(), SessionEvent::Changed(s) if s.state == State::Finished)
    );
    assert!(
        matches!(events.try_recv().unwrap(), SessionEvent::Closed(s) if s.state == State::Finished)
    );
    assert!(matches!(
        events.try_recv(),
        Err(broadcast::error::TryRecvError::Empty)
    ));
}

#[test]
fn given_late_subscription_or_owner_loss_when_read_then_no_recovery_or_terminal_is_invented() {
    // Arrange
    let hub = SessionHub::new(7).unwrap();
    let mut events = hub.subscribe_fresh().unwrap();
    let lease = hub
        .open(TransferType::Inbound, None, CancellationToken::new())
        .unwrap();
    // Act / Assert
    assert!(matches!(hub.subscribe_fresh(), Err(SessionError::Stale)));
    drop(lease);
    assert!(matches!(
        events.try_recv().unwrap(),
        SessionEvent::Opened(_)
    ));
    assert!(
        matches!(events.try_recv().unwrap(), SessionEvent::Closed(s) if s.state == State::Initial)
    );
    hub.close();
    assert!(matches!(hub.subscribe_fresh(), Err(SessionError::Closed)));
}

#[test]
fn given_more_than_bounded_events_when_unpolled_then_loss_is_explicit() {
    // Arrange
    let hub = SessionHub::new(7).unwrap();
    let mut events = hub.subscribe_fresh().unwrap();
    // Act
    for _ in 0..=EVENT_LIMIT / 2 {
        drop(
            hub.open(TransferType::Inbound, None, CancellationToken::new())
                .unwrap(),
        );
    }
    // Assert
    assert!(matches!(
        events.try_recv(),
        Err(broadcast::error::TryRecvError::Lagged(2))
    ));
}

#[tokio::test]
async fn given_full_hub_when_lease_drops_or_hub_closes_then_readiness_wakes_without_timer() {
    use futures::FutureExt;
    // Arrange
    let hub = SessionHub::with_limit(7, 1).unwrap();
    let lease = hub
        .open(TransferType::Inbound, None, CancellationToken::new())
        .unwrap();
    let available = hub.available();
    tokio::pin!(available);
    // Act / Assert
    assert!(available.as_mut().now_or_never().is_none());
    drop(lease);
    assert_eq!(available.await, Ok(()));
    let _lease = hub
        .open(TransferType::Inbound, None, CancellationToken::new())
        .unwrap();
    let available = hub.available();
    tokio::pin!(available);
    assert!(available.as_mut().now_or_never().is_none());
    hub.close();
    assert_eq!(available.await, Err(SessionError::Closed));
}
