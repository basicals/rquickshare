//! Owned non-activating broker for external desktop contract fixtures.
use rqs_lib::service::process::BusAddress;
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
};
use zbus::Connection;

pub(crate) struct Broker {
    _child: OwnedChild,
    _directory: tempfile::TempDir,
    pub(crate) address: BusAddress,
}
impl Broker {
    pub(crate) fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("bus.conf");
        std::fs::write(
            &config,
            include_str!("../../docs/prototypes/preview-bus.conf"),
        )
        .unwrap();
        let child = Command::new("dbus-daemon")
            .args(["--nofork", "--print-address=1"])
            .arg(format!("--config-file={}", config.display()))
            .arg(format!(
                "--address=unix:path={}",
                directory.path().join("bus").display()
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut child = OwnedChild(child);
        let mut line = String::new();
        BufReader::new(child.0.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        Self {
            _child: child,
            _directory: directory,
            address: BusAddress::parse(line.trim()).unwrap(),
        }
    }
    pub(crate) async fn connection(&self) -> Connection {
        zbus::connection::Builder::unix_stream(self.address.connect().await.unwrap())
            .build()
            .await
            .unwrap()
    }
}
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
