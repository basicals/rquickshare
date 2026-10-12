#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(target_os = "linux")]
async fn run() -> Result<(), rqs_lib::service::process::ProcessError> {
    use rqs_lib::service::process::*;
    use tokio_util::sync::CancellationToken;

    let args = Arguments::parse(std::env::args_os().skip(1))?;
    if args.help {
        println!("rquickshare-daemon [--check] [--settings PATH]\n--check validates existing settings without starting transports or changing files.");
        return Ok(());
    }
    // Register both signals before any potentially waiting startup operation.
    let mut signals = ShutdownSignals::new()?;
    let settings = read_settings(&match args.settings {
        Some(path) => path,
        None => settings_path()?,
    })?;
    if settings.visibility_recovered {
        eprintln!("Saved visibility is invalid; receiving starts hidden. Choose a visibility setting in the application.");
    }
    if args.check {
        println!("Settings checked; no service started.");
        return Ok(());
    }
    let directory = runtime_directory()?;
    let _owner = LaunchGuard::acquire(&directory)?;
    let address = BusAddress::session(&directory)?;
    let stop = CancellationToken::new();
    let server = serve(settings, address, stop.clone());
    tokio::pin!(server);
    tokio::select! {
        result = &mut server => result,
        signal = signals.wait() => {
            stop.cancel();
            let stopped = server.await;
            signal.and(stopped)
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    eprintln!("The native daemon is available on Linux only.");
    std::process::ExitCode::FAILURE
}
