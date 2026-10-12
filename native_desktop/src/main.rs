mod ingress;
mod panel;
mod placement;
mod tray;

use gtk::{gio, glib, prelude::*};
use rqs_lib::service::{
    client::{controller::PanelController, NativeClient},
    process::BusAddress,
};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};
use tokio_util::sync::CancellationToken;

fn follow<T: Clone + 'static>(
    mut updates: tokio::sync::watch::Receiver<T>,
    stop: CancellationToken,
    consume: impl Fn(T) + 'static,
) -> glib::JoinHandle<()> {
    consume(updates.borrow_and_update().clone());
    glib::spawn_future_local(async move {
        loop {
            tokio::select! {_=stop.cancelled()=>break,changed=updates.changed()=>if changed.is_err(){break}}
            consume(updates.borrow_and_update().clone());
        }
    })
}

struct Options {
    preview: bool,
    output: Option<PathBuf>,
    page: String,
}
impl Options {
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, &'static str> {
        let mut args = args.into_iter();
        let mut options = Self {
            preview: false,
            output: None,
            page: "nearby".into(),
        };
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--preview")=>options.preview=true,
                Some("--preview-output")=>{options.preview=true;options.output=Some(PathBuf::from(args.next().ok_or("Missing preview output")?));},
                Some("--preview-page")=>{options.page=args.next().and_then(|arg|arg.into_string().ok()).filter(|page|["nearby","incoming","transfers"].contains(&page.as_str())).ok_or("Invalid preview page")?;},
                _=>return Err("Usage: rquickshare [--preview [--preview-page nearby|incoming|transfers] [--preview-output PATH]]"),
            }
        }
        if !options.preview && options.page != "nearby" {
            return Err("Preview page requires preview mode");
        }
        if options
            .output
            .as_ref()
            .is_some_and(|path| !path.is_absolute())
        {
            return Err("Preview output must be absolute");
        }
        Ok(options)
    }
}

fn main() {
    if let Err(message) = run() {
        eprintln!("rQuickShare: {message}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), &'static str> {
    let options = Options::parse(std::env::args_os().skip(1))?;
    gtk::init().map_err(|_| "GTK display is unavailable")?;
    if !options.preview && !gtk4_layer_shell::is_supported() {
        return Err(
            "This desktop does not support GTK4 Layer Shell. Use --preview only for review.",
        );
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|_| "Cannot create native runtime")?;
    let ingress = Arc::new(ingress::Ingress::new());
    let (controller, tray) = if options.preview {
        (None, None)
    } else {
        let directory = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or("Missing session runtime directory")?;
        let address = BusAddress::session(&directory).map_err(|_| "Invalid session bus address")?;
        let connected=runtime.block_on(async {
            let stream=address.connect().await.map_err(|_|"Session bus unavailable")?;
            let client=NativeClient::connect(stream).await.map_err(|_|"Native daemon unavailable or incompatible. Start the uninstalled development daemon explicitly.")?;
            match tray::Tray::connect(&address).await {
                Ok(tray)=>Ok((PanelController::spawn(client),tray)),
                Err(_)=>{let _=client.close().await;Err("KDE tray watcher unavailable or changed")}
            }
        })?;
        (Some(connected.0), Some(connected.1))
    };
    let application = gtk::Application::new(
        Some("io.github.morozhenko_apps.RQuickShare.Native"),
        gio::ApplicationFlags::NON_UNIQUE,
    );
    let handle = controller.as_ref().map(PanelController::handle);
    let tray_updates = tray.as_ref().map(|tray| tray.updates.clone());
    let stop = CancellationToken::new();
    let stop_ui = stop.clone();
    let retained: Rc<RefCell<Option<Rc<panel::Panel>>>> = Rc::new(RefCell::new(None));
    let retained_ui = retained.clone();
    let ingress_ui = ingress.clone();
    let runtime_handle = runtime.handle().clone();
    let hold: Rc<RefCell<Option<gio::ApplicationHoldGuard>>> = Rc::new(RefCell::new(None));
    let hold_ui = hold.clone();
    let tasks = Rc::new(RefCell::new(Vec::new()));
    let tasks_ui = tasks.clone();
    let failure = Rc::new(Cell::new(None));
    let failure_ui = failure.clone();
    application.connect_activate(move |application| {
        if retained_ui.borrow().is_some() {
            return;
        }
        *hold_ui.borrow_mut() = Some(application.hold());
        let panel = panel::Panel::new(
            application,
            handle.clone(),
            runtime_handle.clone(),
            ingress_ui.clone(),
            !options.preview,
        );
        if options.preview {
            panel.preview(&options.page);
            if let Some(output) = options.output.clone() {
                let application = application.clone();
                let failure = failure_ui.clone();
                panel.window.add_tick_callback(move |window, _| {
                    let width = window.width();
                    let height = window.height();
                    if width <= 0 || height <= 0 {
                        return glib::ControlFlow::Continue;
                    }
                    let paintable = gtk::WidgetPaintable::new(Some(window));
                    let snapshot = gtk::Snapshot::new();
                    paintable.snapshot(&snapshot, f64::from(width), f64::from(height));
                    let Some(node) = snapshot.to_node() else {
                        return glib::ControlFlow::Continue;
                    };
                    if let Some(renderer) = window.renderer() {
                        let texture = renderer.render_texture(&node, None);
                        if texture.save_to_png(&output).is_err() {
                            failure.set(Some("Cannot save preview image"));
                        }
                    } else {
                        failure.set(Some("Preview renderer unavailable"));
                    }
                    application.quit();
                    glib::ControlFlow::Break
                });
            }
            panel.window.present();
        } else {
            if let Some(handle) = &handle {
                let weak = Rc::downgrade(&panel);
                tasks_ui.borrow_mut().push(follow(
                    handle.subscribe(),
                    stop_ui.clone(),
                    move |state| {
                        if let Some(panel) = weak.upgrade() {
                            panel.update(state);
                        }
                    },
                ));
            }
            let weak = Rc::downgrade(&panel);
            tasks_ui.borrow_mut().push(follow(
                ingress_ui.subscribe(),
                stop_ui.clone(),
                move |state| {
                    if let Some(panel) = weak.upgrade() {
                        panel.ingress_update(state);
                    }
                },
            ));
            if let Some(updates) = tray_updates.clone() {
                let weak = Rc::downgrade(&panel);
                let application = application.clone();
                let failure = failure_ui.clone();
                tasks_ui
                    .borrow_mut()
                    .push(follow(updates, stop_ui.clone(), move |state| {
                        if state.failed {
                            failure.set(Some("Tray watcher changed. Restart the panel."));
                            application.quit();
                            return;
                        }
                        if let (Some(panel), Some(point)) = (weak.upgrade(), state.activation) {
                            if !panel.show_at(point) {
                                eprintln!(
                                    "rQuickShare: Invalid tray position or monitor too small"
                                );
                            }
                        }
                    }));
            }
        }
        *retained_ui.borrow_mut() = Some(panel);
    });
    let exit = application.run_with_args::<&str>(&[]);
    if let Some(panel) = retained.borrow_mut().take() {
        panel.close();
    }
    stop.cancel();
    for task in tasks.borrow_mut().drain(..) {
        task.abort();
    }
    hold.borrow_mut().take();
    runtime.block_on(async {
        // Close controller first so ingress cannot wait forever on a service call.
        if let Some(controller) = controller {
            let _ = controller.close().await;
        }
        ingress.close().await;
        if let Some(tray) = tray {
            let _ = tray.close().await;
        }
    });
    if let Some(message) = failure.get() {
        return Err(message);
    }
    if exit != glib::ExitCode::SUCCESS {
        return Err("GTK application failed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn given_preview_arguments_when_parsed_then_review_is_explicit_and_paths_are_local() {
        // Arrange / Act / Assert
        for args in [
            vec!["--unknown"],
            vec!["--preview-output"],
            vec!["--preview-output", "relative.png"],
            vec!["--preview-page", "bad"],
            vec!["--preview-page", "incoming"],
        ] {
            assert!(Options::parse(args.into_iter().map(Into::into)).is_err());
        }
        let options = Options::parse(
            [
                "--preview-output",
                "/tmp/panel.png",
                "--preview-page",
                "incoming",
            ]
            .into_iter()
            .map(Into::into),
        )
        .unwrap();
        assert!(options.preview);
        assert_eq!(options.page, "incoming");
        assert_eq!(options.output, Some(PathBuf::from("/tmp/panel.png")));
        assert!(!Options::parse([]).unwrap().preview);
    }
}
