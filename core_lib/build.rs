extern crate prost_build;

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Write;

fn main() {
    // Index generation must not invalidate its own next Cargo invocation.
    println!("cargo:rerun-if-changed=src/proto_src");
    println!("cargo:rerun-if-changed=bindings");
    println!("cargo:rerun-if-env-changed=PROTOC");
    println!("cargo:rerun-if-env-changed=PROTOC_INCLUDE");
    if let Some(include) = std::env::var_os("PROTOC_INCLUDE") {
        println!(
            "cargo:rerun-if-changed={}",
            std::path::Path::new(&include).display()
        );
    }
    prost_build::compile_protos(
        &[
            "src/proto_src/device_to_device_messages.proto",
            "src/proto_src/offline_wire_formats.proto",
            "src/proto_src/securegcm.proto",
            "src/proto_src/securemessage.proto",
            "src/proto_src/ukey.proto",
            "src/proto_src/wire_format.proto",
        ],
        &["src/proto_src"],
    )
    .unwrap();

    let mut exports: Vec<_> = fs::read_dir("./bindings")
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|p| {
            p.path()
                .file_stem()
                .and_then(OsStr::to_str)
                .map(str::to_owned)
        })
        .filter(|f| f != "index")
        .map(|f| format!("export * from \"./{}\"", f))
        .collect();
    // Sort it to avoid having the index.ts being different for no reason
    exports.sort();

    let index = exports.join("\n");
    if fs::read("./bindings/index.ts").ok().as_deref() != Some(index.as_bytes()) {
        let mut file = File::create("./bindings/index.ts").unwrap();
        file.write_all(index.as_bytes()).unwrap();
    }
}
