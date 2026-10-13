mod card_view;
mod cli;
mod ingress;
mod panel;
mod placement;
mod tray;

use gtk::{gio, glib, prelude::*};
use rqs_lib::service::{
    client::{controller::PanelController, desktop::DesktopServer, NativeClient},
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
    mode: card_view::Mode,
}
impl Options {
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, &'static str> {
        let mut args = args.into_iter();
        let mut options = Self {
            preview: false,
            output: None,
            page: "nearby".into(),
            mode: card_view::Mode::Compact,
        };
        let mut preview_options = false;
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--preview")=>options.preview=true,
                Some("--preview-output")=>{options.preview=true;options.output=Some(PathBuf::from(args.next().ok_or("Missing preview output")?));},
                Some("--preview-mode")=>{preview_options=true;options.mode=args.next().and_then(|arg|arg.into_string().ok()).and_then(|value|card_view::Mode::parse(&value)).ok_or("Invalid preview mode")?;},
                Some("--preview-page")=>{preview_options=true;options.page=args.next().and_then(|arg|arg.into_string().ok()).filter(|page|["nearby","incoming","transfers"].contains(&page.as_str())).ok_or("Invalid preview page")?;},
                _=>return Err("Usage: rquickshare [--preview [--preview-page nearby|incoming|transfers] [--preview-mode compact|full] [--preview-output PATH]]"),
            }
        }
        if !options.preview && preview_options {
            return Err("Preview options require preview mode");
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
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "send") {
        let paths = cli::paths(args.into_iter().skip(1))?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|_| "Cannot create native runtime")?;
        let address = session_address()?;
        let executable =
            std::env::current_exe().map_err(|_| "Cannot locate native panel executable")?;
        runtime.block_on(cli::send(paths, &address, &executable))?;
        println!("Files prepared. Choose a recipient in the panel to send.");
        return Ok(());
    }
    let options = Options::parse(args)?;
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
    let (controller, tray, desktop) = if options.preview {
        (None, None, None)
    } else {
        let address = session_address()?;
        let connected = runtime.block_on(async {
            let stream = address.connect().await.map_err(|_| "Session bus unavailable")?;
            let client = NativeClient::connect(stream).await.map_err(|_| "Native daemon unavailable or incompatible. Start the uninstalled development daemon explicitly.")?;
            let controller = PanelController::spawn(client);
            let mut updates = controller.handle().subscribe();
            if updates.wait_for(|state| state.view.is_some() || state.disconnected).await.is_err() || updates.borrow().disconnected {
                let _ = controller.close().await;
                return Err("Cannot obtain complete native service state");
            }
            let tray = match tray::Tray::connect(&address).await {
                Ok(tray) => tray,
                Err(_) => {let _ = controller.close().await;return Err("KDE tray watcher unavailable or changed");}
            };
            let desktop = match address.connect().await {
                Ok(stream) => DesktopServer::connect(stream, controller.handle(), ingress.gate.clone()).await,
                Err(_) => {let _ = controller.close().await;let _ = tray.close().await;return Err("Session bus unavailable");}
            };
            match desktop {
                Ok(desktop) => Ok((controller, tray, desktop)),
                Err(_) => {let _ = controller.close().await;let _ = tray.close().await;Err("Another panel owns the desktop service or registration failed")}
            }
        })?;
        (Some(connected.0), Some(connected.1), Some(connected.2))
    };
    let application = gtk::Application::new(
        Some("io.github.morozhenko_apps.RQuickShare.Native"),
        gio::ApplicationFlags::NON_UNIQUE,
    );
    let handle = controller.as_ref().map(PanelController::handle);
    let tray_updates = tray.as_ref().map(|tray| tray.updates.clone());
    let desktop_handle = desktop.as_ref().map(DesktopServer::handle);
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
            panel.set_mode(options.mode);
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
            if let Some(desktop) = &desktop_handle {
                panel.bind_desktop(desktop.clone());
                let weak = Rc::downgrade(&panel);
                let application = application.clone();
                let failure = failure_ui.clone();
                tasks_ui.borrow_mut().push(follow(
                    desktop.subscribe(),
                    stop_ui.clone(),
                    move |state| {
                        if state.closed {
                            failure
                                .set(Some("Desktop service connection lost. Restart the panel."));
                            application.quit();
                            return;
                        }
                        if let Some(panel) = weak.upgrade() {
                            if !panel.desktop_update(state) {
                                failure.set(Some(
                                    "Cannot place recipient panel on an available monitor",
                                ));
                                application.quit();
                            }
                        }
                    },
                ));
            }
            let weak = Rc::downgrade(&panel);
            tasks_ui.borrow_mut().push(follow(
                ingress_ui.gate.subscribe(),
                stop_ui.clone(),
                move |_| {
                    if let Some(panel) = weak.upgrade() {
                        panel.render();
                    }
                },
            ));
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
        if let Some(desktop) = desktop {
            let _ = desktop.close().await;
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

fn session_address() -> Result<BusAddress, &'static str> {
    let directory = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or("Missing session runtime directory")?;
    BusAddress::session(&directory).map_err(|_| "Invalid session bus address")
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
            vec!["--preview-mode"],
            vec!["--preview-mode", "invalid"],
            vec!["--preview-mode", "full"],
            vec!["--preview-mode", "compact"],
            vec!["--preview-page", "nearby"],
        ] {
            assert!(Options::parse(args.into_iter().map(Into::into)).is_err());
        }
        let options = Options::parse(
            [
                "--preview-output",
                "/tmp/panel.png",
                "--preview-page",
                "incoming",
                "--preview-mode",
                "full",
            ]
            .into_iter()
            .map(Into::into),
        )
        .unwrap();
        assert!(options.preview);
        assert_eq!(options.page, "incoming");
        assert_eq!(options.mode, card_view::Mode::Full);
        assert_eq!(options.output, Some(PathBuf::from("/tmp/panel.png")));
        assert!(!Options::parse([]).unwrap().preview);
    }
}
