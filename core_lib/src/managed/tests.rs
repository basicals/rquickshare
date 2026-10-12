use std::sync::Arc;

use super::*;
use crate::file_source::{OpenedFile, OpenedSelection, SelectionLimits};
use crate::OutboundPayload;

fn info(id: &str) -> SendInfo {
    SendInfo {
        id: id.into(),
        name: "peer".into(),
        addr: "127.0.0.1:1".into(),
        ob: OutboundPayload::Text("private".into()),
    }
}

fn selection() -> Arc<OpenedSelection> {
    let file = tempfile::NamedTempFile::new().unwrap();
    Arc::new(
        OpenedSelection::from_files(
            vec![
                OpenedFile::adopt(std::fs::File::open(file.path()).unwrap(), "input".into())
                    .unwrap(),
            ],
            SelectionLimits::default(),
        )
        .unwrap(),
    )
}

#[tokio::test]
async fn given_full_queue_when_admission_retries_then_rejected_input_is_retained_until_capacity_returns(
) {
    // Arrange
    let (sender, mut receiver) = ManagedSender::channel().unwrap();
    let mut controls = Vec::new();
    for index in 0..QUEUE_CAPACITY {
        controls.push(sender.try_send(info(&index.to_string())).unwrap());
    }
    let mut input = info("retry");
    input.ob = OutboundPayload::OpenedFiles(selection());
    let weak = match &input.ob {
        OutboundPayload::OpenedFiles(files) => Arc::downgrade(files),
        _ => unreachable!(),
    };
    // Act
    let rejection = sender.try_send(input).err().unwrap();
    // Assert
    assert_eq!(rejection.kind, RejectionKind::Full);
    assert_eq!(rejection.request.id, "retry");
    assert!(weak.upgrade().is_some());
    assert!(!format!("{rejection:?}").contains("retry"));
    receiver.recv().await.unwrap().release();
    controls[0].wait_released().await.unwrap();
    let retry = sender.try_send(*rejection.request).unwrap();
    for control in controls.iter().skip(1) {
        receiver.recv().await.unwrap().release();
        control.wait_released().await.unwrap();
    }
    receiver.recv().await.unwrap().release();
    retry.wait_released().await.unwrap();
    assert!(weak.upgrade().is_none());
}

#[tokio::test]
async fn given_closed_receiver_when_submitted_then_closure_is_observable_and_input_returns() {
    // Arrange
    let (sender, mut receiver) = ManagedSender::channel().unwrap();
    receiver.close();
    // Act
    sender.closed().await;
    let rejection = sender.try_send(info("unsent")).err().unwrap();
    // Assert
    assert!(sender.is_closed());
    assert_eq!(rejection.kind, RejectionKind::Closed);
    assert_eq!(rejection.request.id, "unsent");
    assert!(receiver.recv().await.is_none());
}

#[tokio::test]
async fn given_accepted_control_when_cancelled_before_receive_then_job_already_observes_cancellation(
) {
    // Arrange
    let (sender, mut receiver) = ManagedSender::channel().unwrap();
    let control = sender.try_send(info("early")).unwrap();
    // Act
    control.cancel();
    let job = receiver.recv().await.unwrap();
    // Assert
    assert!(job.cancellation.is_cancelled());
    job.release();
    control.wait_released().await.unwrap();
}

#[tokio::test]
async fn given_service_owned_send_when_control_is_dropped_then_job_is_not_cancelled() {
    // Arrange
    let (sender, mut receiver) = ManagedSender::channel().unwrap();
    let control = sender.try_send(info("survives")).unwrap();
    // Act
    drop(control);
    let job = receiver.recv().await.unwrap();
    // Assert
    assert!(!job.cancellation.is_cancelled());
    assert_eq!(job.info.id, "survives");
    job.release();
}

#[tokio::test]
async fn given_owned_payload_when_release_is_acknowledged_then_job_drops_its_reference_first() {
    // Arrange
    let (sender, mut receiver) = ManagedSender::channel().unwrap();
    for external_owner in [false, true] {
        let files = selection();
        let weak = Arc::downgrade(&files);
        let retained = external_owner.then(|| Arc::clone(&files));
        let mut input = info("payload");
        input.ob = OutboundPayload::OpenedFiles(files);
        let control = sender.try_send(input).unwrap();
        // Act
        receiver.recv().await.unwrap().release();
        control.wait_released().await.unwrap();
        // Assert
        assert_eq!(weak.upgrade().is_some(), external_owner);
        drop(retained);
        assert!(weak.upgrade().is_none());
    }
}

#[tokio::test]
async fn given_lost_dispatch_owner_when_waiting_then_no_release_or_success_is_invented() {
    // Arrange
    for acknowledged in [false, true] {
        let (sender, mut receiver) = ManagedSender::channel().unwrap();
        let control = sender.try_send(info("lost")).unwrap();
        let job = receiver.recv().await.unwrap();
        // Act
        if acknowledged {
            job.release();
        } else {
            drop(job);
        }
        // Assert
        assert_eq!(
            control.wait_released().await,
            if acknowledged {
                Ok(())
            } else {
                Err(ControlLost)
            }
        );
    }
}

#[tokio::test]
async fn given_completed_dispatch_when_cancel_and_wait_repeat_then_release_remains_idempotent() {
    // Arrange
    let (sender, mut receiver) = ManagedSender::channel().unwrap();
    let control = sender.try_send(info("repeat")).unwrap();
    let other = control.clone();
    // Act
    control.cancel();
    other.cancel();
    receiver.recv().await.unwrap().release();
    // Assert
    control.wait_released().await.unwrap();
    other.wait_released().await.unwrap();
    control.cancel();
    control.wait_released().await.unwrap();
}

#[tokio::test]
async fn given_legacy_receiver_when_closed_then_buffered_inputs_are_drained_with_identity_preserved(
) {
    // Arrange
    let (sender, receiver) = mpsc::channel(2);
    sender.send(info("first")).await.unwrap();
    sender.send(info("second")).await.unwrap();
    let mut receiver: OutboundReceiver = receiver.into();
    // Act
    receiver.close();
    let first = receiver.recv().await.unwrap();
    let second = receiver.try_recv().unwrap();
    // Assert
    assert_eq!(first.info.id, "first");
    assert_eq!(second.info.id, "second");
    assert!(!first.cancellation.is_cancelled());
    assert!(first.completion.is_none());
    assert!(receiver.recv().await.is_none());
    assert!(sender.send(info("closed")).await.is_err());
}
