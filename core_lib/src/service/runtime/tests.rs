use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};

use futures::FutureExt;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::file_source::{OpenedFile, SelectionLimits};
use crate::managed::{acknowledge_release, OutboundJob, OutboundReceiver};
use crate::sessions::{consent_command, SessionLease};

struct Fixture {
    runtime: NativeRuntime,
    receiver: OutboundReceiver,
    discovery: broadcast::Sender<EndpointInfo>,
}
fn fixture() -> Fixture {
    let (sender, receiver) = ManagedSender::channel().unwrap();
    let events = sender.sessions().subscribe_fresh().unwrap();
    let (discovery, observed) = broadcast::channel(128);
    Fixture {
        runtime: NativeRuntime::assemble(
            RQS::default(),
            Coordinator::new(123, Limits::default()).unwrap(),
            sender,
            events,
            observed,
        ),
        receiver,
        discovery,
    }
}

#[test]
fn given_existing_discovery_when_name_or_route_changes_then_shared_revision_changes_only_for_effective_updates(
) {
    // Arrange
    let mut fixture = fixture();
    let runtime = &mut fixture.runtime;
    let client = runtime.connect().unwrap();
    let mut info = advertisement("device");
    runtime.discovered(info.clone()).unwrap();
    let first = runtime.snapshot(client).unwrap();
    let id = runtime.endpoints()[0].id.clone();
    // Act / Assert
    runtime.discovered(info.clone()).unwrap();
    assert_eq!(runtime.snapshot(client).unwrap().revision, first.revision);
    info.name = Some("Renamed device".into());
    runtime.discovered(info.clone()).unwrap();
    let second = runtime.snapshot(client).unwrap();
    assert_eq!(second.revision, first.revision + 1);
    assert_eq!(
        runtime.endpoints()[0],
        EndpointSnapshot {
            id: id.clone(),
            name: "Renamed device".into()
        }
    );
    info.port = Some("4321".into());
    runtime.discovered(info.clone()).unwrap();
    assert_eq!(
        runtime.snapshot(client).unwrap().revision,
        second.revision + 1
    );
    assert_eq!(runtime.routes[&id].addr.port(), 4321);
    info.present = Some(false);
    runtime.discovered(info.clone()).unwrap();
    let removed = runtime.snapshot(client).unwrap();
    assert_eq!(removed.revision, second.revision + 2);
    assert!(runtime.routes.is_empty() && runtime.coordinator.endpoints.is_empty());
    runtime.discovered(info).unwrap();
    assert_eq!(runtime.snapshot(client).unwrap().revision, removed.revision);
}

#[test]
fn given_exhausted_revision_when_discovery_mutates_then_existing_route_and_endpoint_sequence_remain_unchanged(
) {
    // Arrange
    for change in ["name", "route", "remove", "insert"] {
        let mut fixture = fixture();
        let runtime = &mut fixture.runtime;
        let mut info = advertisement("device");
        runtime.discovered(info.clone()).unwrap();
        let before = runtime.endpoints();
        let id = before[0].id.clone();
        let addr = runtime.routes[&id].addr;
        let sequence = runtime.endpoint_sequence;
        runtime.coordinator.revision = u64::MAX;
        match change {
            "name" => info.name = Some("Changed".into()),
            "route" => info.port = Some("4321".into()),
            "remove" => info.present = Some(false),
            _ => info.fullname = "new-device".into(),
        }
        // Act / Assert
        assert_eq!(
            runtime.discovered(info),
            Err(RuntimeError::Coordinator(Error::Exhausted))
        );
        assert_eq!(runtime.endpoints(), before);
        assert_eq!(runtime.routes[&id].addr, addr);
        assert_eq!(runtime.endpoint_sequence, sequence);
        assert_eq!(
            runtime
                .coordinator
                .endpoints
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec![id]
        );
    }
}
fn advertisement(fullname: &str) -> EndpointInfo {
    EndpointInfo {
        fullname: fullname.into(),
        name: Some("Peer".into()),
        ip: Some("127.0.0.1".into()),
        port: Some("1234".into()),
        present: Some(true),
        ..Default::default()
    }
}
fn selection() -> OpenedSelection {
    let source = tempfile::NamedTempFile::new().unwrap();
    source.as_file().set_len(3).unwrap();
    OpenedSelection::from_files(
        vec![OpenedFile::adopt(File::open(source.path()).unwrap(), "private.txt".into()).unwrap()],
        SelectionLimits::default(),
    )
    .unwrap()
}
fn send(runtime: &mut NativeRuntime, client: ClientId, request: &str) -> TransferId {
    let selected = runtime.create_selection(client, selection()).unwrap();
    runtime
        .send_selection(
            client,
            selected,
            &runtime.endpoints()[0].id.clone(),
            request,
        )
        .unwrap()
}
fn view(runtime: &mut NativeRuntime, client: ClientId, id: TransferId) -> TransferSnapshot {
    runtime
        .snapshot(client)
        .unwrap()
        .transfers
        .into_iter()
        .find(|transfer| transfer.id == id)
        .unwrap()
}
fn finish(job: OutboundJob, state: State) {
    let OutboundJob {
        info,
        completion,
        mut session,
        ..
    } = job;
    session.as_mut().unwrap().observe(&state);
    drop(info);
    drop(session);
    acknowledge_release(completion);
}
async fn waiting(
    runtime: &mut NativeRuntime,
    client: ClientId,
) -> (Option<SessionLease>, TransferId, u64) {
    let mut lease = runtime
        .sender
        .sessions()
        .open(
            crate::channel::TransferType::Inbound,
            None,
            CancellationToken::new(),
        )
        .unwrap();
    lease.observe(&State::WaitingForUserConsent);
    runtime.step().await.unwrap();
    let transfer = runtime
        .snapshot(client)
        .unwrap()
        .transfers
        .last()
        .unwrap()
        .clone();
    (Some(lease), transfer.id, transfer.consent_revision.unwrap())
}
async fn scenario(operation: impl Future<Output = ()>) {
    // Watchdog only; progress waits on channels, owned tasks and I/O.
    tokio::time::timeout(std::time::Duration::from_secs(5), operation)
        .await
        .expect("runtime scenario deadlocked");
}

#[test]
fn given_unusable_discovery_when_updated_then_only_bounded_numeric_routes_are_exposed() {
    // Arrange
    let mut fixture = fixture();
    let runtime = &mut fixture.runtime;
    // Act / Assert
    for (ip, port) in [
        ("hostname.invalid", "42"),
        ("127.0.0.1", "0"),
        ("127.0.0.1", "65536"),
        ("", "42"),
    ] {
        let mut info = advertisement("peer");
        info.ip = Some(ip.into());
        info.port = Some(port.into());
        runtime.discovered(info).unwrap();
        assert!(runtime.endpoints().is_empty());
    }
    runtime.coordinator.limits.endpoints = 1;
    runtime.discovered(advertisement("peer")).unwrap();
    let first = runtime.endpoints()[0].clone();
    assert!(!first.id.contains("peer"));
    runtime.discovered(advertisement("excess")).unwrap();
    assert_eq!(runtime.endpoints(), vec![first.clone()]);
    let mut updated = advertisement("peer");
    updated.ip = Some("::1".into());
    updated.name = Some("Updated".into());
    runtime.discovered(updated).unwrap();
    assert_eq!(runtime.endpoints()[0].id, first.id);
    assert_eq!(runtime.routes[&first.id].addr.to_string(), "[::1]:1234");
    let mut removed = advertisement("peer");
    removed.present = Some(false);
    runtime.discovered(removed).unwrap();
    assert!(runtime.endpoints().is_empty());
    runtime.discovered(advertisement("peer")).unwrap();
    assert_ne!(runtime.endpoints()[0].id, first.id);
}

#[tokio::test]
async fn given_queued_files_when_caller_disconnects_then_dispatch_is_fifo_once_and_resources_follow_core(
) {
    // Arrange
    let mut fixture = fixture();
    let runtime = &mut fixture.runtime;
    runtime.discovered(advertisement("peer")).unwrap();
    let client = runtime.connect().unwrap();
    let first = send(runtime, client, "first");
    let second = send(runtime, client, "second");
    runtime.disconnect(client).unwrap();
    let observer = runtime.connect().unwrap();
    // Act
    runtime.step().await.unwrap();
    runtime.step().await.unwrap();
    let first_job = fixture.receiver.recv().await.unwrap();
    let second_job = fixture.receiver.recv().await.unwrap();
    // Assert
    assert!(first_job.info.id.ends_with(&format!("{:016x}", first.1)));
    assert!(second_job.info.id.ends_with(&format!("{:016x}", second.1)));
    assert_eq!(first_job.info.addr, "127.0.0.1:1234");
    assert!(fixture.receiver.try_recv().is_err());
    let weak = match &first_job.info.ob {
        OutboundPayload::OpenedFiles(files) => Arc::downgrade(files),
        _ => unreachable!(),
    };
    assert_eq!(
        runtime
            .snapshot(observer)
            .unwrap()
            .resources
            .reserved_handles,
        4
    );
    finish(first_job, State::Finished);
    finish(second_job, State::Rejected);
    runtime.step().await.unwrap();
    assert_eq!(view(runtime, observer, first).phase, Phase::Finished);
    assert_eq!(view(runtime, observer, second).phase, Phase::Rejected);
    assert!(weak.upgrade().is_none());
    assert_eq!(
        runtime
            .snapshot(observer)
            .unwrap()
            .resources
            .reserved_handles,
        0
    );
}

#[tokio::test]
async fn given_full_queue_when_pending_send_is_cancelled_then_no_duplicate_dispatch_or_file_leak_occurs(
) {
    // Arrange
    let mut fixture = fixture();
    let runtime = &mut fixture.runtime;
    runtime.discovered(advertisement("peer")).unwrap();
    let client = runtime.connect().unwrap();
    for index in 0..10 {
        send(runtime, client, &index.to_string());
        runtime.dispatch().unwrap();
    }
    let transfer = send(runtime, client, "pending");
    assert!(!runtime.dispatch().unwrap());
    let weak = match &runtime.pending.as_ref().unwrap().request.ob {
        OutboundPayload::OpenedFiles(files) => Arc::downgrade(files),
        _ => unreachable!(),
    };
    // Act
    runtime.cancel(client, transfer).unwrap();
    runtime.controls().unwrap();
    // Assert
    assert!(runtime.pending.is_none());
    assert!(weak.upgrade().is_none());
    assert_eq!(view(runtime, client, transfer).phase, Phase::Cancelled);
    let mut count = 0;
    while let Ok(job) = fixture.receiver.try_recv() {
        count += 1;
        finish(job, State::Finished);
    }
    assert_eq!(count, 10);
    runtime.step().await.unwrap();
    assert_eq!(
        runtime.snapshot(client).unwrap().resources.reserved_handles,
        0
    );
}

#[tokio::test]
async fn given_pending_backpressure_when_capacity_returns_then_same_payload_is_submitted_once() {
    // Arrange
    let mut fixture = fixture();
    let runtime = &mut fixture.runtime;
    runtime.discovered(advertisement("peer")).unwrap();
    let client = runtime.connect().unwrap();
    for index in 0..10 {
        send(runtime, client, &index.to_string());
        runtime.dispatch().unwrap();
    }
    let transfer = send(runtime, client, "pending");
    runtime.dispatch().unwrap();
    let id = runtime.pending.as_ref().unwrap().request.id.clone();
    // Act
    finish(fixture.receiver.recv().await.unwrap(), State::Finished);
    runtime.step().await.unwrap();
    // Assert
    assert!(runtime.pending.is_none());
    let mut ids = Vec::new();
    while let Ok(job) = fixture.receiver.try_recv() {
        ids.push(job.info.id.clone());
        finish(job, State::Finished);
    }
    assert_eq!(ids.len(), 10);
    assert_eq!(ids.iter().filter(|entry| **entry == id).count(), 1);
    runtime.step().await.unwrap();
    assert_eq!(view(runtime, client, transfer).phase, Phase::Finished);
}

#[tokio::test]
async fn given_removed_endpoint_when_queued_or_pending_then_unsent_work_fails_and_releases_resources(
) {
    // Arrange / Act / Assert
    for full in [false, true] {
        let mut fixture = fixture();
        let runtime = &mut fixture.runtime;
        runtime.discovered(advertisement("peer")).unwrap();
        let client = runtime.connect().unwrap();
        if full {
            for index in 0..10 {
                send(runtime, client, &index.to_string());
                runtime.dispatch().unwrap();
            }
        }
        let transfer = send(runtime, client, "removed");
        if full {
            runtime.dispatch().unwrap();
            assert!(runtime.pending.is_some());
        }
        let mut removed = advertisement("peer");
        removed.present = Some(false);
        fixture.discovery.send(removed).unwrap();
        runtime.step().await.unwrap();
        assert_eq!(view(runtime, client, transfer).phase, Phase::Failed);
        assert!(runtime.pending.is_none());
        let mut count = 0;
        while let Ok(job) = fixture.receiver.try_recv() {
            count += 1;
            finish(job, State::Finished);
        }
        assert_eq!(count, if full { 10 } else { 0 });
    }
}

#[tokio::test]
async fn given_inbound_answer_when_queued_then_only_processed_receipt_acknowledges_exact_intent() {
    // Arrange
    let mut fixture = fixture();
    let runtime = &mut fixture.runtime;
    let client = runtime.connect().unwrap();
    let (mut lease, transfer, revision) = waiting(runtime, client).await;
    // Act
    runtime.respond(client, transfer, revision, true).unwrap();
    runtime.step().await.unwrap();
    let intent = view(runtime, client, transfer).pending_control.unwrap();
    assert!(runtime.receipts.next().now_or_never().is_none());
    let command = consent_command(&mut lease).await.unwrap();
    assert_eq!(
        lease
            .as_mut()
            .unwrap()
            .apply(command, &State::WaitingForUserConsent),
        Some(true)
    );
    let (processed, result) = runtime.receipts.next().await.unwrap();
    runtime.receipt(processed, result).unwrap();
    // Assert
    assert_eq!(processed, intent);
    assert!(view(runtime, client, transfer).pending_control.is_none());
    assert_eq!(
        view(runtime, client, transfer).phase,
        Phase::AwaitingConsent
    );
    lease.as_mut().unwrap().observe(&State::Finished);
    drop(lease);
    runtime.step().await.unwrap();
    assert_eq!(view(runtime, client, transfer).phase, Phase::Finished);
}

#[tokio::test]
async fn given_cancel_superseding_answer_when_late_receipt_arrives_then_cancel_remains_truthful() {
    // Arrange
    let mut fixture = fixture();
    let runtime = &mut fixture.runtime;
    let client = runtime.connect().unwrap();
    let (mut lease, transfer, revision) = waiting(runtime, client).await;
    runtime.respond(client, transfer, revision, false).unwrap();
    runtime.step().await.unwrap();
    let command = consent_command(&mut lease).await.unwrap();
    lease
        .as_mut()
        .unwrap()
        .apply(command, &State::WaitingForUserConsent);
    // Act
    runtime.cancel(client, transfer).unwrap();
    let cancel = view(runtime, client, transfer).pending_control.unwrap();
    let (old, result) = runtime.receipts.next().await.unwrap();
    runtime.receipt(old, result).unwrap();
    // Assert
    assert_eq!(
        view(runtime, client, transfer).pending_control,
        Some(cancel)
    );
    runtime.controls().unwrap();
    assert!(lease.as_ref().unwrap().cancellation().is_cancelled());
    assert_eq!(
        view(runtime, client, transfer).phase,
        Phase::CancelRequested
    );
    lease.as_mut().unwrap().observe(&State::Cancelled);
    drop(lease);
    runtime.step().await.unwrap();
    assert_eq!(view(runtime, client, transfer).phase, Phase::Cancelled);
}

#[tokio::test]
async fn given_second_consent_round_when_old_revision_is_replayed_then_only_new_round_can_be_answered(
) {
    // Arrange
    let mut fixture = fixture();
    let runtime = &mut fixture.runtime;
    let client = runtime.connect().unwrap();
    let (mut lease, transfer, old) = waiting(runtime, client).await;
    // Act
    lease.as_mut().unwrap().observe(&State::ReceivingFiles);
    lease
        .as_mut()
        .unwrap()
        .observe(&State::WaitingForUserConsent);
    runtime.step().await.unwrap();
    // Assert
    let new = view(runtime, client, transfer).consent_revision.unwrap();
    assert_ne!(new, old);
    assert_eq!(
        runtime.respond(client, transfer, old, true),
        Err(Error::StaleConsent)
    );
    runtime.respond(client, transfer, new, false).unwrap();
    runtime.step().await.unwrap();
    let command = consent_command(&mut lease).await.unwrap();
    assert_eq!(
        lease
            .as_mut()
            .unwrap()
            .apply(command, &State::WaitingForUserConsent),
        Some(false)
    );
    lease.as_mut().unwrap().observe(&State::Rejected);
    drop(lease);
    runtime.step().await.unwrap();
    assert_eq!(view(runtime, client, transfer).phase, Phase::Rejected);
}

#[tokio::test]
async fn given_state_racing_answer_when_core_rejects_stale_consent_then_no_false_acknowledgement_occurs(
) {
    // Arrange
    let mut fixture = fixture();
    let runtime = &mut fixture.runtime;
    let client = runtime.connect().unwrap();
    let (mut lease, transfer, revision) = waiting(runtime, client).await;
    runtime.respond(client, transfer, revision, true).unwrap();
    // Act: core advances between the adapter's last observation and command delivery.
    lease.as_mut().unwrap().observe(&State::ReceivingFiles);
    assert!(!runtime.controls().unwrap());
    // Assert
    assert!(view(runtime, client, transfer).pending_control.is_some());
    assert!(!runtime.deferred.is_empty());
    runtime.step().await.unwrap();
    assert_eq!(view(runtime, client, transfer).phase, Phase::Receiving);
    assert!(view(runtime, client, transfer).pending_control.is_none());
    lease.as_mut().unwrap().observe(&State::Finished);
    drop(lease);
}

#[tokio::test]
async fn given_owner_loss_or_event_lag_when_polled_then_new_work_is_permanently_paused() {
    // Arrange / Act / Assert
    for lag in [false, true] {
        let mut fixture = fixture();
        let runtime = &mut fixture.runtime;
        let client = runtime.connect().unwrap();
        let hub = runtime.sender.sessions();
        if lag {
            for _ in 0..257 {
                drop(
                    hub.open(
                        crate::channel::TransferType::Inbound,
                        None,
                        CancellationToken::new(),
                    )
                    .unwrap(),
                );
            }
        } else {
            drop(
                hub.open(
                    crate::channel::TransferType::Inbound,
                    None,
                    CancellationToken::new(),
                )
                .unwrap(),
            );
        }
        assert_eq!(runtime.step().await, Err(RuntimeError::Unsynchronized));
        assert!(!runtime.snapshot(client).unwrap().synchronized);
        assert!(matches!(
            runtime.coordinator.next_send(),
            Err(Error::Unsynchronized)
        ));
    }
}

#[tokio::test]
async fn given_lost_processing_receipt_when_owner_disappears_then_adapter_does_not_acknowledge() {
    // Arrange
    let mut fixture = fixture();
    let runtime = &mut fixture.runtime;
    let client = runtime.connect().unwrap();
    let (lease, transfer, revision) = waiting(runtime, client).await;
    runtime.respond(client, transfer, revision, true).unwrap();
    runtime.controls().unwrap();
    let intent = view(runtime, client, transfer).pending_control.unwrap();
    // Act
    drop(lease);
    let (delivered, result) = runtime.receipts.next().await.unwrap();
    // Assert
    assert_eq!(result, Err(SessionError::OwnerLost));
    assert_eq!(
        runtime.receipt(delivered, result),
        Err(RuntimeError::Unsynchronized)
    );
    assert_eq!(
        view(runtime, client, transfer).pending_control,
        Some(intent)
    );
}

#[tokio::test]
async fn given_closed_core_queue_when_polled_then_health_loss_is_not_delivery() {
    // Arrange
    let mut fixture = fixture();
    let runtime = &mut fixture.runtime;
    let client = runtime.connect().unwrap();
    fixture.receiver.close();
    // Act / Assert
    assert_eq!(runtime.step().await, Err(RuntimeError::CoreUnavailable));
    assert!(!runtime.snapshot(client).unwrap().synchronized);
}

#[tokio::test]
async fn given_empty_runtime_when_step_is_cancelled_then_no_work_or_resource_is_created() {
    // Arrange
    let mut fixture = fixture();
    let runtime = &mut fixture.runtime;
    let client = runtime.connect().unwrap();
    let before = runtime.snapshot(client).unwrap();
    // Act
    assert!(runtime.step().now_or_never().is_none());
    // Assert
    assert_eq!(runtime.snapshot(client).unwrap(), before);
    assert!(fixture.receiver.try_recv().is_err());
}

#[tokio::test]
async fn given_tracked_core_when_shutdown_returns_then_workers_are_joined_before_stopped_snapshot()
{
    scenario(async {
        // Arrange
        let mut fixture = fixture();
        let runtime = &mut fixture.runtime;
        let client = runtime.connect().unwrap();
        let (tracker, lifetime, guard) = runtime.core.begin_startup().unwrap();
        let joined = Arc::new(AtomicBool::new(false));
        let mark = joined.clone();
        tracker.spawn(async move {
            lifetime.cancelled().await;
            mark.store(true, Ordering::SeqCst);
        });
        guard.disarm();
        runtime.create_selection(client, selection()).unwrap();
        // Act
        runtime.shutdown().await.unwrap();
        // Assert
        assert!(joined.load(Ordering::SeqCst));
        assert!(tracker.is_empty() && tracker.is_closed());
        assert_eq!(
            runtime.snapshot(client).unwrap().lifecycle,
            Lifecycle::Stopped
        );
        assert_eq!(
            runtime.snapshot(client).unwrap().resources.reserved_handles,
            0
        );
        assert_eq!(
            runtime.create_selection(client, selection()),
            Err(Error::NotRunning)
        );
        assert_eq!(
            runtime.step().await,
            Err(RuntimeError::Coordinator(Error::NotRunning))
        );
    })
    .await;
}

#[tokio::test]
async fn given_lost_observation_when_shutdown_fails_then_core_barrier_is_still_awaited() {
    scenario(async {
        // Arrange
        let mut fixture = fixture();
        let runtime = &mut fixture.runtime;
        let (tracker, lifetime, guard) = runtime.core.begin_startup().unwrap();
        let joined = Arc::new(AtomicBool::new(false));
        let mark = joined.clone();
        tracker.spawn(async move {
            lifetime.cancelled().await;
            mark.store(true, Ordering::SeqCst);
        });
        guard.disarm();
        assert_eq!(runtime.fault(), Err(RuntimeError::Unsynchronized));
        // Act / Assert
        assert_eq!(runtime.shutdown().await, Err(RuntimeError::Unsynchronized));
        assert!(joined.load(Ordering::SeqCst));
        assert!(tracker.is_empty());
    })
    .await;
}

#[test]
fn given_every_protocol_state_when_mapped_then_coarse_direction_and_terminal_outcomes_are_preserved(
) {
    // Arrange / Act / Assert
    use crate::channel::TransferType::{Inbound, Outbound};
    for direction in [Inbound, Outbound] {
        for (state, expected) in [
            (State::Initial, Phase::Active),
            (State::ReceivedConnectionRequest, Phase::Active),
            (State::SentUkeyServerInit, Phase::Active),
            (State::SentUkeyClientInit, Phase::Active),
            (State::SentUkeyClientFinish, Phase::Active),
            (State::SentPairedKeyEncryption, Phase::Active),
            (State::ReceivedUkeyClientFinish, Phase::Active),
            (State::SentConnectionResponse, Phase::Active),
            (State::SentPairedKeyResult, Phase::Active),
            (State::SentIntroduction, Phase::Active),
            (State::ReceivedPairedKeyResult, Phase::Active),
            (State::ReceivingFiles, Phase::Receiving),
            (State::SendingFiles, Phase::Sending),
            (State::Finished, Phase::Finished),
            (State::Rejected, Phase::Rejected),
            (State::Cancelled, Phase::Cancelled),
            (State::Disconnected, Phase::Disconnected),
        ] {
            assert_eq!(phase(&state, &direction), expected);
        }
        assert_eq!(
            phase(&State::WaitingForUserConsent, &direction),
            if direction == Inbound {
                Phase::AwaitingConsent
            } else {
                Phase::Active
            }
        );
    }
}

#[tokio::test]
async fn given_real_tcp_worker_when_peer_closes_or_service_stops_then_runtime_preserves_outcome_and_joins(
) {
    scenario(async {
        for stop in [false, true] {
            // Arrange: real protocol/manager, only the remote socket is controlled.
            let mut fixture = fixture();
            let peer = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = peer.local_addr().unwrap();
            let mut info = advertisement("real-peer");
            info.port = Some(address.port().to_string());
            fixture.runtime.discovered(info).unwrap();
            let client = fixture.runtime.connect().unwrap();
            let transfer = send(&mut fixture.runtime, client, "real");
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let mut server = crate::manager::TcpServer::new(
                *b"TEST",
                listener,
                fixture.runtime.core.message_sender.clone(),
                fixture.receiver,
                crate::hdl::BwuRouter::new(),
            )
            .unwrap();
            let (tracker, lifetime, guard) = fixture.runtime.core.begin_startup().unwrap();
            let worker_lifetime = lifetime.clone();
            tracker.spawn(async move {
                crate::supervise_tcp(server.run(worker_lifetime), lifetime).await;
            });
            guard.disarm();
            fixture.runtime.step().await.unwrap();
            let (socket, _) = peer.accept().await.unwrap();
            // Act / Assert
            if stop {
                fixture.runtime.shutdown().await.unwrap();
                assert_eq!(
                    view(&mut fixture.runtime, client, transfer).phase,
                    Phase::Cancelled
                );
                drop(socket);
            } else {
                drop(socket);
                while !view(&mut fixture.runtime, client, transfer)
                    .phase
                    .is_terminal()
                {
                    fixture.runtime.step().await.unwrap();
                }
                assert_eq!(
                    view(&mut fixture.runtime, client, transfer).phase,
                    Phase::Disconnected
                );
                fixture.runtime.shutdown().await.unwrap();
            }
            assert!(tracker.is_closed() && tracker.is_empty());
            let snapshot = fixture.runtime.snapshot(client).unwrap();
            assert_eq!(snapshot.lifecycle, Lifecycle::Stopped);
            assert_eq!(snapshot.resources.reserved_handles, 0);
            assert!(fixture.runtime.bindings.is_empty());
        }
    })
    .await;
}

#[tokio::test]
async fn given_started_core_when_runtime_preparation_rejects_then_unchanged_ownership_is_returned()
{
    scenario(async {
        // Arrange
        let mut core = RQS::default();
        let (tracker, lifetime, guard) = core.begin_startup().unwrap();
        let parent = lifetime.clone();
        let joined = Arc::new(AtomicBool::new(false));
        let mark = joined.clone();
        tracker.spawn(async move {
            lifetime.cancelled().await;
            mark.store(true, Ordering::SeqCst);
        });
        guard.disarm();
        // Act
        let mut rejected = NativeRuntime::new(123, Limits::default(), core)
            .err()
            .unwrap();
        // Assert
        assert_eq!(rejected.error, RuntimeError::CoreUnavailable);
        assert!(!parent.is_cancelled());
        assert!(!joined.load(Ordering::SeqCst));
        rejected.core.stop().await;
        assert!(joined.load(Ordering::SeqCst));
        assert!(tracker.is_closed() && tracker.is_empty());
    })
    .await;
}

#[tokio::test]
async fn given_discovery_loss_when_polled_then_endpoint_history_is_not_treated_as_current() {
    // Arrange
    let mut fixture = fixture();
    let client = fixture.runtime.connect().unwrap();
    for index in 0..129 {
        fixture
            .discovery
            .send(advertisement(&index.to_string()))
            .unwrap();
    }
    // Act / Assert
    assert_eq!(
        fixture.runtime.step().await,
        Err(RuntimeError::Unsynchronized)
    );
    assert!(!fixture.runtime.snapshot(client).unwrap().synchronized);
    assert!(fixture.runtime.endpoints().is_empty());
}

#[tokio::test]
async fn given_buffered_inbound_open_when_shutdown_begins_then_admission_closes_before_coordinator_drain(
) {
    scenario(async {
        // Arrange: the inbound owner exists before the runtime has polled its Opened event.
        let mut fixture = fixture();
        let runtime = &mut fixture.runtime;
        let client = runtime.connect().unwrap();
        let hub = runtime.sender.sessions();
        let mut lease = hub
            .open(
                crate::channel::TransferType::Inbound,
                None,
                CancellationToken::new(),
            )
            .unwrap();
        let (tracker, lifetime, guard) = runtime.core.begin_startup().unwrap();
        tracker.spawn(async move {
            lifetime.cancelled().await;
            lease.observe(&State::Cancelled);
        });
        guard.disarm();
        // Act
        runtime.shutdown().await.unwrap();
        // Assert
        let snapshot = runtime.snapshot(client).unwrap();
        assert_eq!(snapshot.lifecycle, Lifecycle::Stopped);
        assert!(snapshot.synchronized);
        assert_eq!(snapshot.transfers.len(), 1);
        assert_eq!(snapshot.transfers[0].phase, Phase::Cancelled);
        assert!(matches!(
            hub.open(
                crate::channel::TransferType::Inbound,
                None,
                CancellationToken::new()
            ),
            Err(SessionError::Closed)
        ));
        assert!(tracker.is_empty());
    })
    .await;
}

#[tokio::test]
async fn given_invalid_runtime_configuration_when_prepared_then_core_is_returned_without_starting()
{
    // Arrange / Act
    let rejected = NativeRuntime::new(0, Limits::default(), RQS::default())
        .err()
        .unwrap();
    // Assert
    assert_eq!(
        rejected.error,
        RuntimeError::Coordinator(Error::InvalidInput)
    );
    assert!(rejected.core.tracker.is_none());
    assert!(!format!("{rejected:?}").contains("visibility"));
}

#[tokio::test]
async fn given_bind_failure_when_start_finishes_then_owned_runtime_can_shutdown_and_cannot_restart()
{
    scenario(async {
        // Arrange: bind failure precedes optional BLE initialization.
        let occupied = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
        let core = RQS::new(
            crate::Visibility::Invisible,
            Some(occupied.local_addr().unwrap().port().into()),
            None,
        );
        let mut runtime = NativeRuntime::new(123, Limits::default(), core).unwrap();
        assert_eq!(runtime.connect(), Err(Error::NotRunning));
        // Act
        assert_eq!(runtime.start().await, Err(RuntimeError::CoreUnavailable));
        // Assert
        assert!(runtime.core.tracker.is_none() && runtime.core.ctoken.is_none());
        assert!(runtime.sender.is_closed());
        runtime.shutdown().await.unwrap();
        assert_eq!(runtime.coordinator.lifecycle, Lifecycle::Stopped);
        assert_eq!(runtime.start().await, Err(RuntimeError::CoreUnavailable));
    })
    .await;
}

#[tokio::test]
async fn given_abandoned_startup_guard_when_runtime_is_retained_then_shutdown_can_join_it() {
    scenario(async {
        // Arrange: use the same RQS startup owner/guard as its real start method.
        let mut runtime = NativeRuntime::new(123, Limits::default(), RQS::default()).unwrap();
        let (tracker, lifetime, guard) = runtime.core.begin_startup().unwrap();
        let joined = Arc::new(AtomicBool::new(false));
        let mark = joined.clone();
        tracker.spawn(async move {
            lifetime.cancelled().await;
            mark.store(true, Ordering::SeqCst);
        });
        // Act
        drop(guard);
        runtime.shutdown().await.unwrap();
        // Assert
        assert!(joined.load(Ordering::SeqCst));
        assert!(tracker.is_closed() && tracker.is_empty());
        assert_eq!(runtime.coordinator.lifecycle, Lifecycle::Stopped);
        assert_eq!(runtime.start().await, Err(RuntimeError::CoreUnavailable));
    })
    .await;
}

#[tokio::test]
async fn given_outbound_close_and_release_when_delivered_in_either_order_then_history_waits_for_both(
) {
    // Arrange / Act / Assert
    for release_first in [false, true] {
        let mut fixture = fixture();
        let runtime = &mut fixture.runtime;
        runtime.coordinator.limits.transfers = 2;
        runtime.discovered(advertisement("peer")).unwrap();
        let client = runtime.connect().unwrap();
        let id = send(runtime, client, "send");
        runtime.dispatch().unwrap();
        runtime.disconnect(client).unwrap();
        let OutboundJob {
            info,
            completion,
            mut session,
            ..
        } = fixture.receiver.try_recv().unwrap();
        session.as_mut().unwrap().observe(&State::Finished);
        runtime.drain_events().unwrap();
        let other = runtime.coordinator.register_inbound().unwrap();
        runtime.coordinator.observe(other, Phase::Finished).unwrap();
        runtime.coordinator.owner_closed(other).unwrap();
        assert!(!runtime.coordinator.maintain_history().unwrap());
        drop(info);
        if release_first {
            acknowledge_release(completion);
            let (transfer, result) = runtime.releases.next().await.unwrap();
            runtime.release(transfer, result).unwrap();
            assert!(!runtime.coordinator.maintain_history().unwrap());
            drop(session);
            runtime.drain_events().unwrap();
        } else {
            drop(session);
            runtime.drain_events().unwrap();
            assert!(!runtime.coordinator.maintain_history().unwrap());
            acknowledge_release(completion);
            let (transfer, result) = runtime.releases.next().await.unwrap();
            runtime.release(transfer, result).unwrap();
        }
        assert!(runtime.coordinator.maintain_history().unwrap());
        assert!(!runtime.coordinator.transfers.contains_key(&id));
        assert!(runtime.coordinator.transfers.contains_key(&other));
    }
}

#[tokio::test]
async fn given_terminal_inbound_with_late_consent_receipt_when_closed_then_retirement_waits_for_receipt(
) {
    // Arrange
    let mut fixture = fixture();
    let runtime = &mut fixture.runtime;
    runtime.coordinator.limits.transfers = 2;
    let client = runtime.connect().unwrap();
    let (mut lease, id, revision) = waiting(runtime, client).await;
    runtime.respond(client, id, revision, true).unwrap();
    runtime.controls().unwrap();
    assert!(runtime.coordinator.transfers[&id].control_receipt);
    lease.as_mut().unwrap().observe(&State::Cancelled);
    drop(lease);
    runtime.drain_events().unwrap();
    let other = runtime.coordinator.register_inbound().unwrap();
    runtime.coordinator.observe(other, Phase::Finished).unwrap();
    runtime.coordinator.owner_closed(other).unwrap();
    // Act / Assert
    assert!(!runtime.coordinator.maintain_history().unwrap());
    let (intent, result) = runtime.receipts.next().await.unwrap();
    runtime.receipt(intent, result).unwrap();
    assert!(runtime.coordinator.maintain_history().unwrap());
    assert!(!runtime.coordinator.transfers.contains_key(&id));
    assert!(runtime.inflight.is_empty());
}
