//! Private stdin -> bounded RGBA helper. No GUI, path or session authority.
use rquickshare_desktop::preview::{self, Error, Pixels};
use std::io::{Read, Write};
use std::time::Duration;

fn limits() -> Result<(), Error> {
    use rustix::process::{setrlimit, Resource, Rlimit};
    for (resource, maximum) in [
        (Resource::As, preview::MEMORY_BYTES),
        (Resource::Cpu, preview::CPU_SECONDS),
        (Resource::Nofile, preview::DESCRIPTORS),
        (Resource::Core, 0),
    ] {
        setrlimit(
            resource,
            Rlimit {
                current: Some(maximum),
                maximum: Some(maximum),
            },
        )
        .map_err(|_| Error::Unavailable)?;
    }
    Ok(())
}
async fn decode(bytes: Vec<u8>) -> Result<Pixels, Error> {
    let pool = glycin::Pool::new(
        glycin::PoolConfig::new()
            .retention_time(Duration::ZERO)
            .max_parallel_operations(1),
    );
    let mut loader = glycin::Loader::new_vec(bytes);
    loader
        .sandbox_selector(glycin::SandboxSelector::Bwrap)
        .accepted_memory_formats(glycin::MemoryFormatSelection::R8g8b8a8)
        .main_context_selector(glycin::MainContextSelector::Managed)
        .use_expose_base_dir(false)
        .pool(pool)
        .limits(
            glycin::Limits::default()
                .timeout(Duration::from_secs(5))
                .max_dimensions((preview::MAX_DIMENSION, preview::MAX_DIMENSION)),
        );
    let mut image = loader.load().await.map_err(|_| Error::Unavailable)?;
    let mime = image.mime_type().to_string();
    if !["image/png", "image/jpeg"].contains(&mime.as_str()) {
        return Err(Error::Rejected);
    }
    preview::dimensions(image.details().width(), image.details().height())?;
    let frame = image.next_frame().await.map_err(|_| Error::Unavailable)?;
    if frame.memory_format() != glycin::MemoryFormat::R8g8b8a8 {
        return Err(Error::Rejected);
    }
    Pixels::from_rgba(
        frame.width(),
        frame.height(),
        frame.stride() as usize,
        frame.buf_slice(),
    )
}
fn run() -> Result<(), Error> {
    // Single-threaded process setup, before the runtime or Glycin starts threads.
    // Glycin's format converter otherwise sizes its Rayon pool from host CPUs.
    std::env::set_var("RAYON_NUM_THREADS", "1");
    limits()?;
    if std::env::args_os()
        .collect::<Vec<_>>()
        .get(1)
        .is_some_and(|arg| arg == "--help")
        && std::env::args_os().len() == 2
    {
        println!("rquickshare-preview-helper\nPrivate bounded stdin image to RGBA decoder. Requires Bubblewrap sandbox and Glycin loaders.");
        return Ok(());
    }
    if std::env::args_os().len() != 1 {
        return Err(Error::Rejected);
    }
    let mut bytes = Vec::new();
    std::io::stdin()
        .take((preview::MAX_ENCODED + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    preview::admit(&bytes)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| Error::Unavailable)?;
    let pixels = runtime.block_on(decode(bytes))?;
    let output = pixels.encode()?;
    std::io::stdout()
        .write_all(&output)
        .map_err(|_| Error::Unavailable)?;
    Ok(())
}
fn main() {
    let status = match run() {
        Ok(()) => 0,
        Err(Error::Rejected) => 2,
        Err(_) => 3,
    };
    std::process::exit(status);
}
