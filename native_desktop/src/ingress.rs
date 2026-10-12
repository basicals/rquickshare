//! One tracked acquisition owner; shutdown joins filesystem work before runtime exit.

use rqs_lib::{
    file_source::{OpenedSelection, SelectionLimits},
    service::client::controller::PanelHandle,
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
}
impl Ingress {
    pub fn new() -> Self {
        let (state, _) = watch::channel(IngressState::default());
        Self {
            task: Mutex::new(None),
            state,
        }
    }
    pub fn subscribe(&self) -> watch::Receiver<IngressState> {
        self.state.subscribe()
    }
    pub fn start(&self, runtime: &Handle, paths: Vec<PathBuf>, controller: PanelHandle) -> bool {
        let mut task = self.task.lock().expect("ingress ownership");
        if task.as_ref().is_some_and(|task| !task.is_finished()) {
            return false;
        }
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
        }));
        true
    }
    pub async fn close(&self) {
        let task = self.task.lock().expect("ingress ownership").take();
        if let Some(task) = task {
            let _ = task.await;
        }
    }
}
