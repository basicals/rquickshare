use super::*;

#[test]
fn given_rounds_progress_and_terminal_phases_when_dismissed_or_replaced_then_notice_authority_stays_exact(
) {
    // Arrange
    let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
    let client = coordinator.connect().unwrap();
    let id = coordinator.register_inbound().unwrap();
    assert!(coordinator.pending_notices().is_empty());
    // Act / Assert
    coordinator.observe(id, Phase::AwaitingConsent).unwrap();
    let first = coordinator.pending_notices()[0].clone();
    let before = coordinator.snapshot(client).unwrap();
    coordinator
        .dismiss_notices(client, std::slice::from_ref(&first))
        .unwrap();
    let dismissed_revision = coordinator.revision;
    assert!(coordinator.pending_notices().is_empty());
    assert_eq!(
        coordinator.snapshot(client).unwrap().transfers,
        before.transfers
    );
    coordinator.observe(id, Phase::AwaitingConsent).unwrap();
    coordinator
        .dismiss_notices(client, std::slice::from_ref(&first))
        .unwrap();
    assert_eq!(coordinator.revision, dismissed_revision);
    coordinator.observe(id, Phase::Receiving).unwrap();
    assert!(coordinator.pending_notices().is_empty());
    coordinator.observe(id, Phase::AwaitingConsent).unwrap();
    let second = coordinator.pending_notices()[0].clone();
    assert!(second.round > first.round);
    coordinator.dismiss_notices(client, &[first]).unwrap();
    assert_eq!(coordinator.pending_notices(), vec![second]);
    coordinator.cancel(client, id).unwrap();
    assert!(coordinator.pending_notices().is_empty());
    for phase in [
        Phase::Finished,
        Phase::Failed,
        Phase::Disconnected,
        Phase::Rejected,
        Phase::Cancelled,
    ] {
        let id = coordinator.register_inbound().unwrap();
        coordinator.observe(id, phase).unwrap();
        let pending = coordinator.pending_notices();
        assert_eq!(
            pending.iter().any(|notice| notice.transfer == id),
            matches!(phase, Phase::Finished | Phase::Failed | Phase::Disconnected)
        );
        coordinator.owner_closed(id).unwrap();
        coordinator.source_released(id).unwrap();
    }
    for _ in 0..80 {
        let id = coordinator.register_inbound().unwrap();
        coordinator.observe(id, Phase::Finished).unwrap();
        coordinator.owner_closed(id).unwrap();
    }
    coordinator.maintain_history().unwrap();
    assert!(coordinator
        .notices
        .keys()
        .all(|id| coordinator.transfers.contains_key(id)));
    assert!(coordinator.notices.len() <= crate::service::retention::RECENT_TERMINALS);
}

#[test]
fn given_invalid_duplicate_and_stale_batches_when_dismissed_then_validation_is_atomic_and_consent_unchanged(
) {
    // Arrange
    let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
    let client = coordinator.connect().unwrap();
    let id = coordinator.register_inbound().unwrap();
    coordinator.observe(id, Phase::AwaitingConsent).unwrap();
    let notice = coordinator.pending_notices()[0].clone();
    let original = coordinator.snapshot(client).unwrap();
    let mut invalid = Vec::new();
    for (transfer, kind, round, revision) in [
        (
            TransferId(456, 1),
            NoticeKind::Request,
            notice.round,
            notice.revision,
        ),
        (
            TransferId(123, 0),
            NoticeKind::Request,
            notice.round,
            notice.revision,
        ),
        (id, NoticeKind::Request, 0, notice.revision),
        (id, NoticeKind::Finished, notice.round, notice.revision),
        (id, NoticeKind::Request, notice.round, 0),
        (id, NoticeKind::Request, notice.round, u64::MAX),
        (id, NoticeKind::Request, u64::MAX, notice.revision),
    ] {
        invalid.push(vec![
            notice.clone(),
            Notice {
                transfer,
                kind,
                round,
                revision,
            },
        ]);
    }
    invalid.extend([vec![], vec![notice.clone(); 2], vec![notice.clone(); 17]]);
    // Act / Assert
    for batch in invalid {
        assert_eq!(
            coordinator.dismiss_notices(client, &batch),
            Err(Error::InvalidInput)
        );
        assert_eq!(coordinator.snapshot(client).unwrap(), original);
    }
    assert_eq!(
        coordinator.dismiss_notices(ClientId(123, 9999), &[notice]),
        Err(Error::UnknownClient)
    );
    assert_eq!(NoticeKind::parse("private arbitrary text"), None);
    for kind in [
        NoticeKind::Request,
        NoticeKind::Finished,
        NoticeKind::Failed,
        NoticeKind::Disconnected,
    ] {
        assert_eq!(NoticeKind::parse(kind.wire()), Some(kind));
    }
}
