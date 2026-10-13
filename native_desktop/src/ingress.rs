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
