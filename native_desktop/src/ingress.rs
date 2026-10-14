//! One tracked acquisition owner; shutdown joins filesystem work before runtime exit.

use rqs_lib::{
    file_source::{OpenedSelection, SelectionLimits},
    service::client::{
        controller::PanelHandle,
        desktop::{PreparationGate, PreparationPermit},
    },
};
use std::{path::PathBuf, sync::Mutex};
use tokio::{runtime::Handle, sync::watch, task::JoinHandle};
type ImageReply = tokio::sync::oneshot::Receiver<(
    Result<rqs_lib::clipboard_image::CapturedImage, rqs_lib::clipboard_image::ImageError>,
    PreparationPermit,
)>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IngressState {
    pub busy: bool,
    pub failed: bool,
}

pub struct Ingress {
    task: Mutex<Option<JoinHandle<()>>>,
    state: watch::Sender<IngressState>,
    pub gate: PreparationGate,
}
impl Ingress {
    pub fn start_image(
        &self,
        runtime: &Handle,
        bytes: Vec<u8>,
        cancel: tokio_util::sync::CancellationToken,
        permit: PreparationPermit,
    ) -> ImageReply {
        self.image_job(runtime, permit, move || {
            rqs_lib::clipboard_image::capture(&bytes, &cancel)
        })
    }
    fn image_job(
        &self,
        runtime: &Handle,
        permit: PreparationPermit,
        work: impl FnOnce() -> Result<
                rqs_lib::clipboard_image::CapturedImage,
                rqs_lib::clipboard_image::ImageError,
            > + Send
            + 'static,
    ) -> ImageReply {
        let (reply, receive) = tokio::sync::oneshot::channel();
        let mut task = self.task.lock().expect("ingress ownership");
        self.state.send_replace(IngressState {
            busy: true,
            failed: false,
        });
        let state = self.state.clone();
        let previous = task.take();
        *task = Some(runtime.spawn(async move {
            if let Some(previous) = previous {
                let _ = previous.await;
            }
            // The blocking worker owns the permit even if this asynchronous wait is dropped.
            let result = tokio::task::spawn_blocking(move || (work(), permit)).await;
            state.send_replace(IngressState {
                busy: false,
                failed: false,
            });
            if let Ok(result) = result {
                let _ = reply.send(result);
            }
        }));
        receive
    }
    pub fn start_directory(
        &self,
        runtime: &Handle,
        path: PathBuf,
        permit: PreparationPermit,
    ) -> tokio::sync::oneshot::Receiver<Result<(std::fs::File, String), ()>> {
        let (reply, receive) = tokio::sync::oneshot::channel();
        let mut task = self.task.lock().expect("ingress ownership");
        self.state.send_replace(IngressState {
            busy: true,
            failed: false,
        });
        let state = self.state.clone();
        let previous = task.take();
        *task = Some(runtime.spawn(async move {
            if let Some(previous) = previous {
                let _ = previous.await;
            }
            let directory = tokio::task::spawn_blocking(move || {
                let file = std::fs::File::from(
                    rustix::fs::open(
                        path,
                        rustix::fs::OFlags::PATH
                            | rustix::fs::OFlags::DIRECTORY
                            | rustix::fs::OFlags::CLOEXEC,
                        rustix::fs::Mode::empty(),
                    )
                    .map_err(|_| ())?,
                );
                let record = rqs_lib::service::native_settings::DirectoryRecord::acquire(&file)
                    .map_err(|_| ())?;
                Ok((file, record.path().to_string_lossy().into_owned()))
            })
            .await
            .unwrap_or(Err(()));
            state.send_replace(IngressState {
                busy: false,
                failed: false,
            });
            drop(permit);
            let _ = reply.send(directory);
        }));
        receive
    }
    pub fn new() -> Self {
        let (state, _) = watch::channel(IngressState::default());
        Self {
            task: Mutex::new(None),
            state,
            gate: PreparationGate::new(),
        }
    }
    pub fn subscribe(&self) -> watch::Receiver<IngressState> {
        self.state.subscribe()
    }
    pub fn start(
        &self,
        runtime: &Handle,
        paths: Vec<PathBuf>,
        controller: PanelHandle,
        permit: PreparationPermit,
    ) {
        let mut task = self.task.lock().expect("ingress ownership");
        self.state.send_replace(IngressState {
            busy: true,
            failed: false,
        });
        let state = self.state.clone();
        let previous = task.take();
        *task = Some(runtime.spawn(async move {
            if let Some(previous) = previous {
                let _ = previous.await;
            }
            let source = tokio::task::spawn_blocking(move || {
                OpenedSelection::acquire(&paths, SelectionLimits::default())
            })
            .await;
            let failed = match source {
                Ok(Ok(source)) => match controller.choose(source) {
                    Ok(completion) => completion.wait().await.is_err(),
                    Err(_) => true,
                },
                _ => true,
            };
            state.send_replace(IngressState {
                busy: false,
                failed,
            });
            drop(permit);
        }));
    }
    pub async fn close(&self) {
        self.gate.close();
        let task = self.task.lock().expect("ingress ownership").take();
        if let Some(task) = task {
            let _ = task.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn given_dropped_image_wait_when_worker_is_cancelled_then_gate_remains_owned_and_shutdown_joins(
    ) {
        // Arrange: explicit boundary events hold actual blocking ownership, never a timed sleep.
        let owner = Ingress::new();
        let permit = owner.gate.try_acquire().unwrap();
        let cancel = tokio_util::sync::CancellationToken::new();
        let worker_cancel = cancel.clone();
        let (started, observed) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let receive = owner.image_job(&Handle::current(), permit, move || {
            started.send(()).unwrap();
            wait.recv().unwrap();
            rqs_lib::clipboard_image::capture(b"invalid", &worker_cancel)
        });
        observed.await.unwrap();
        // Act / Assert
        drop(receive);
        cancel.cancel();
        assert!(owner.gate.try_acquire().is_none());
        assert!(owner.subscribe().borrow().busy);
        let close = owner.close();
        tokio::pin!(close);
        assert!(futures::poll!(&mut close).is_pending());
        release.send(()).unwrap();
        close.await;
        assert!(!owner.subscribe().borrow().busy);
        assert!(owner.gate.try_acquire().is_none()); // Shutdown permanently closes admission.
    }
    #[tokio::test]
    async fn given_invalid_png_when_actual_worker_completes_then_reply_retains_permit_until_gui_drops_it(
    ) {
        // Arrange / Act
        let owner = Ingress::new();
        let receive = owner.start_image(
            &Handle::current(),
            vec![1],
            tokio_util::sync::CancellationToken::new(),
            owner.gate.try_acquire().unwrap(),
        );
        let (result, permit) = receive.await.unwrap();
        // Assert
        assert_eq!(
            result.unwrap_err(),
            rqs_lib::clipboard_image::ImageError::InvalidPng
        );
        assert!(owner.gate.try_acquire().is_none());
        assert!(!owner.subscribe().borrow().busy);
        drop(permit);
        assert!(owner.gate.try_acquire().is_some());
        owner.close().await;
    }
    #[tokio::test]
    async fn given_directory_choice_when_acquired_and_owner_closes_then_fd_is_retained_and_filesystem_work_is_joined(
    ) {
        use std::os::unix::fs::MetadataExt;
        // Arrange
        let directory = tempfile::tempdir().unwrap();
        let owner = Ingress::new();
        let permit = owner.gate.try_acquire().unwrap();
        let receive =
            owner.start_directory(&Handle::current(), directory.path().to_owned(), permit);
        // Act
        owner.close().await;
        let (file, display) = receive.await.unwrap().unwrap();
        // Assert
        assert_eq!(
            file.metadata().unwrap().ino(),
            directory.path().metadata().unwrap().ino()
        );
        assert_eq!(display, directory.path().to_string_lossy());
        assert_eq!(
            rustix::io::fcntl_getfd(&file).unwrap(),
            rustix::io::FdFlags::CLOEXEC
        );
        assert!(!owner.subscribe().borrow().busy);
        assert!(owner.gate.try_acquire().is_none());
    }
}
