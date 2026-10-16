mod card_policy;
mod card_view;
mod cli;
mod desktop_guard;
mod ingress;
mod lifecycle;
mod login;
mod panel;
mod placement;
mod portal;
#[cfg(test)]
mod test_bus;
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

#[derive(Clone)]
struct Options {
    preview: bool,
    service: bool,
    output: Option<PathBuf>,
    page: String,
    mode: card_view::Drawer,
}
impl Options {
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Self, &'static str> {
        let mut args = args.into_iter();
        let mut options = Self {
            preview: false,
            service: false,
            output: None,
            page: "nearby".into(),
            mode: card_view::Drawer::Closed,
        };
        let mut preview_options = false;
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--preview")=>options.preview=true,
                Some("--service") if !options.service=>options.service=true,
                Some("--preview-output")=>{options.preview=true;options.output=Some(PathBuf::from(args.next().ok_or("Missing preview output")?));},
                Some("--preview-mode")=>{preview_options=true;options.mode=args.next().and_then(|arg|arg.into_string().ok()).and_then(|value|card_view::Drawer::parse(&value)).ok_or("Invalid preview mode")?;},
                Some("--preview-page")=>{preview_options=true;options.page=args.next().and_then(|arg|arg.into_string().ok()).filter(|page|["nearby","incoming","transfers"].contains(&page.as_str())).ok_or("Invalid preview page")?;},
                _=>return Err("Usage: rquickshare [--preview [--preview-page nearby|incoming|transfers] [--preview-mode compact|full] [--preview-output PATH]]"),
            }
        }
        if options.preview && options.service {
            return Err("Service and preview modes are separate");
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

enum RunError {
    Cli(cli::Failure),
    Gui(&'static str),
}
impl From<&'static str> for RunError {
    fn from(message: &'static str) -> Self {
        Self::Gui(message)
    }
}
impl From<cli::Failure> for RunError {
    fn from(error: cli::Failure) -> Self {
        Self::Cli(error)
    }
}
fn main() {
    if let Err(error) = run() {
        let message = match &error {
            RunError::Cli(error) => error.message,
            RunError::Gui(message) => message,
        };
        eprintln!("rQuickShare: {message}");
        if let RunError::Cli(error) = error {
            // A failed activation is not retried just to display its own error.
            if !matches!(
                error.code,
                rqs_lib::service::client::desktop::ErrorCode::PanelUnavailable
            ) {
                if let (Ok(address), Ok(runtime)) = (
                    session_address(),
                    tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build(),
                ) {
                    let _ = runtime.block_on(cli::show_error(error.code, &address));
                }
            }
        }
        std::process::exit(1);
    }
}
fn run() -> Result<(), RunError> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() == 1 && args[0] == "--help" {
        println!("rquickshare\nLaunch the native tray client.\nrquickshare --service\nRun the foreground panel worker (manager/development).\nrquickshare send -- FILE_OR_LOCAL_URI...\nPrepare files for an explicit recipient selection and Send.\nrquickshare --preview [--preview-page nearby|incoming|transfers] [--preview-mode compact|full] [--preview-output PATH]");
        return Ok(());
    }
    if args.first().is_some_and(|arg| arg == "send") {
        use rqs_lib::service::client::desktop::ErrorCode;
        let paths = cli::paths(args.into_iter().skip(1))
            .map_err(|message| cli::Failure::new(ErrorCode::InvalidSelection, message))?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|_| {
                cli::Failure::new(ErrorCode::PanelUnavailable, "Cannot create native runtime")
            })?;
        let address = session_address()
            .map_err(|message| cli::Failure::new(ErrorCode::PanelUnavailable, message))?;
        runtime.block_on(cli::send(paths, &address))?;
        println!("Files prepared. Choose a recipient in the panel to send.");
        return Ok(());
    }
    let options = Options::parse(args)?;
    if !options.preview && !options.service {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|_| "Cannot create native runtime")?;
        return runtime
            .block_on(cli::open(&session_address()?))
            .map_err(RunError::Gui);
    }
    run_worker(options)
}
fn run_worker(mut options: Options) -> Result<(), RunError> {
    let _ownership = if options.preview {
        None
    } else {
        Some(
            rqs_lib::service::process::LaunchGuard::acquire_desktop(
                &rqs_lib::service::process::runtime_directory()
                    .map_err(|_| "Invalid session runtime directory")?,
            )
            .map_err(|_| {
                "Another native panel owns this session or runtime access is unavailable"
            })?,
        )
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|_| "Cannot create native runtime")?;
    let shutdown = runtime.block_on(async { lifecycle::Shutdown::new() })?;
    let process_stop = shutdown.stop.clone();
    let result = (|| loop {
        if process_stop.is_cancelled() {
            return Ok(());
        }
        let (reconnect, mode) = run_gui(&runtime, options.clone(), process_stop.clone())?;
        if !reconnect || process_stop.is_cancelled() {
            return Ok(());
        }
        options.mode = mode;
    })();
    runtime.block_on(shutdown.close());
    result
}
async fn close_controller(controller: Option<PanelController>) {
    if let Some(controller) = controller {
        let _ = controller.close().await;
    }
}
fn run_gui(
    runtime: &tokio::runtime::Runtime,
    options: Options,
    process_stop: CancellationToken,
) -> Result<(bool, card_view::Drawer), &'static str> {
    gtk::init().map_err(|_| "GTK display is unavailable")?;
    if !options.preview && !gtk4_layer_shell::is_supported() {
        return Err(
            "This desktop does not support GTK4 Layer Shell. Use --preview only for review.",
        );
    }
    let ingress = Arc::new(ingress::Ingress::new());
    let (controller, mut tray, desktop, guard) = if options.preview {
        (None, None, None, None)
    } else {
        let address = session_address()?;
        let connected = runtime.block_on(async {
            let controller = async {
                let client = tokio::select! { biased;
                    _ = process_stop.cancelled() => None,
                    client = async {
                        let stream = address.connect().await.ok()?;
                        rqs_lib::service::activation::activate_daemon(stream, process_stop.clone()).await.ok()?;
                        NativeClient::connect(address.connect().await.ok()?).await.ok()
                    } => client,
                }?;
                let controller = PanelController::spawn(client);
                let mut updates = controller.handle().subscribe();
                let complete = tokio::select! { biased;
                    _ = process_stop.cancelled() => false,
                    result = updates.wait_for(|state| state.view.is_some() || state.disconnected) => result.is_ok(),
                } && !updates.borrow().disconnected;
                if complete { Some(controller) } else { let _ = controller.close().await; None }
            }.await;
            if process_stop.is_cancelled() { close_controller(controller).await; return Ok(None); }
            let tray = match tray::Tray::connect_when_ready(&address, process_stop.clone()).await {
                Ok(tray) => tray,
                Err(_) => {
                    close_controller(controller).await;
                    return if process_stop.is_cancelled() { Ok(None) } else { Err("KDE tray watcher unavailable or changed") };
                }
            };
            let guard = match tokio::select! { biased;
                _ = process_stop.cancelled() => Err(()),
                guard = desktop_guard::DesktopGuard::connect(&address) => guard.map_err(|_| ()),
            } {
                Ok(guard) => guard,
                Err(_) => {
                    close_controller(controller).await;
                    let _ = tray.close().await;
                    return if process_stop.is_cancelled() { Ok(None) } else { Err("Cannot observe desktop privacy state") };
                }
            };
            if process_stop.is_cancelled() {
                close_controller(controller).await; guard.close().await; let _ = tray.close().await; return Ok(None);
            }
            let desktop = tokio::select! { biased;
                _ = process_stop.cancelled() => Err(()),
                desktop = async {
                    let stream = address.connect().await.map_err(|_| ())?;
                    DesktopServer::connect_optional(stream, controller.as_ref().map(PanelController::handle), ingress.gate.clone()).await.map_err(|_| ())
                } => desktop,
            };
            match desktop {
                Ok(desktop) if !process_stop.is_cancelled() => Ok(Some((controller, tray, desktop, guard))),
                outcome => {
                    close_controller(controller).await; guard.close().await; let _ = tray.close().await;
                    if let Ok(desktop) = outcome { let _ = desktop.close().await; }
                    if process_stop.is_cancelled() { Ok(None) } else { Err("Another panel owns the desktop service or registration failed") }
                }
            }
        })?;
        let Some(connected) = connected else {
            return Ok((false, options.mode));
        };
        (
            connected.0,
            Some(connected.1),
            Some(connected.2),
            Some(connected.3),
        )
    };
    let login = if options.preview {
        None
    } else {
        Some(runtime.block_on(login::Owner::connect(
            &session_address()?,
            process_stop.clone(),
        )))
    };
    let login_handle = login.as_ref().map(login::Owner::handle);
    let login_updates = login_handle.as_ref().map(login::Handle::subscribe);
    let application = gtk::Application::new(
        Some("io.github.morozhenko_apps.RQuickShare.Native"),
        gio::ApplicationFlags::NON_UNIQUE,
    );
    let handle = controller.as_ref().map(PanelController::handle);
    let tray_updates = tray.as_ref().map(|tray| tray.updates.clone());
    let tray_menu = tray.as_ref().map(|tray| tray.menu.clone());
    let tray_commands = Rc::new(RefCell::new(
        tray.as_mut().and_then(|tray| tray.take_commands()),
    ));
    let desktop_handle = desktop.as_ref().map(DesktopServer::handle);
    let safety_updates = guard.as_ref().map(|guard| guard.updates.clone());
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
        let quit = application.clone();
        let process_stop = process_stop.clone();
        tasks_ui.borrow_mut().push(glib::spawn_future_local(async move { process_stop.cancelled().await; quit.quit(); }));
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
        if let Some(handle) = &login_handle { panel.bind_login(handle.clone()); }
        if let Some(updates) = login_updates.clone() {
            let weak = Rc::downgrade(&panel);
            tasks_ui.borrow_mut().push(follow(updates, stop_ui.clone(), move |_| {
                if let Some(panel) = weak.upgrade() { panel.refresh_login(); }
            }));
        }
        if let Some(menu) = &tray_menu { panel.bind_tray_menu(menu.clone()); }
        if let Some(mut commands) = tray_commands.borrow_mut().take() {
            let weak = Rc::downgrade(&panel); let end = stop_ui.clone();
            tasks_ui.borrow_mut().push(glib::spawn_future_local(async move {
                'events: loop {
                    let actions = tokio::select! { biased; _ = end.cancelled() => break, actions = commands.recv() => match actions { Some(actions) => actions, None => break } };
                    let Some(panel) = weak.upgrade() else { break };
                    for action in actions { if panel.tray_action(action) { break 'events; } }
                }
                commands.close();
            }));
        }
        if options.preview {
            panel.set_drawer(options.mode);
            panel.preview(&options.page);
            if let Some(output) = options.output.clone() {
                let application = application.clone();
                let failure = failure_ui.clone();
                let preview = Rc::downgrade(&panel);
                panel.window.add_tick_callback(move |window, _| {
                    if preview.upgrade().is_none_or(|panel| !panel.preview_ready()) {
                        return glib::ControlFlow::Continue;
                    }
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
            if let Some(updates) = safety_updates.clone() {
                let weak = Rc::downgrade(&panel);
                tasks_ui
                    .borrow_mut()
                    .push(follow(updates, stop_ui.clone(), move |state| {
                        if let Some(panel) = weak.upgrade() {
                            panel.safety_update(state);
                        }
                    }));
            }
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
                        panel.preparation_update();
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
                            let placed = if state.context { panel.tray_context(point) } else { panel.show_at(point) };
                            if !placed {
                                eprintln!(
                                    "rQuickShare: Cannot place the card on an available monitor"
                                );
                            }
                        }
                    }));
            }
        }
        *retained_ui.borrow_mut() = Some(panel);
    });
    let exit = application.run_with_args::<&str>(&[]);
    let mut reconnect = false;
    let mut mode = card_view::Drawer::Closed;
    if let Some(panel) = retained.borrow_mut().take() {
        reconnect = panel.reconnect_requested();
        mode = panel.drawer();
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
        if let Some(login) = login {
            login.close().await;
        }
        if let Some(guard) = guard {
            guard.close().await;
        }
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
    Ok((reconnect, mode))
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
        assert_eq!(options.mode, card_view::Drawer::Open);
        assert_eq!(options.output, Some(PathBuf::from("/tmp/panel.png")));
        assert!(!Options::parse([]).unwrap().preview);
    }
}
