#![recursion_limit = "256"]

mod binder;
mod parser;
mod util;

use crate::util::to_module_name;
use std::env;
use std::ffi::OsStr;
use std::fs::{read_dir, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::Command;

use ts_rs::TS;

pub fn main() {
    eprintln!("BATATA");
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Update and init submodule
    if let Err(error) = Command::new("git")
        .arg("submodule")
        .arg("update")
        .arg("--init")
        .current_dir(src_dir)
        .status()
    {
        eprintln!("{error}");
    }

    // find & apply patches to XML definitions to avoid crashes
    let mut patch_dir = src_dir.to_path_buf();
    patch_dir.push("build/patches");
    let mut mavlink_dir = src_dir.to_path_buf();
    mavlink_dir.push("mavlink");

    if let Ok(dir) = read_dir(patch_dir) {
        for entry in dir.flatten() {
            if let Err(error) = Command::new("git")
                .arg("apply")
                .arg(entry.path().as_os_str())
                .current_dir(&mavlink_dir)
                .status()
            {
                eprintln!("{error}");
            }
        }
    }

    let mut definitions_dir = src_dir.to_path_buf();
    definitions_dir.push("mavlink/message_definitions/v1.0");

    let out_dir = env::var("OUT_DIR").unwrap();

    let mut modules = vec![];

    for entry in read_dir(&definitions_dir).expect("could not read definitions directory") {
        let entry = entry.expect("could not read directory entry");

        let definition_file = entry.file_name();
        let module_name = to_module_name(&definition_file);

        let mut definition_rs = PathBuf::from(&module_name);
        definition_rs.set_extension("rs");

        modules.push(module_name.clone());

        let dest_path = Path::new(&out_dir).join(definition_rs);
        let mut outf = BufWriter::new(File::create(&dest_path).unwrap());

        // generate code
        parser::generate(
            &module_name,
            &definitions_dir,
            &definition_file.into_string().unwrap(),
            &mut outf,
        );
        dbg_format_code(&out_dir, &dest_path);

        // Re-run build if definition file changes
        println!("cargo:rerun-if-changed={}", entry.path().to_string_lossy());
    }

    // output mod.rs
    {
        let dest_path = Path::new(&out_dir).join("mod.rs");
        let mut outf = File::create(&dest_path).unwrap();

        // generate code
        binder::generate(modules, &mut outf);
        dbg_format_code(out_dir, dest_path);
    }

    /*
    println!("cargo:rerun-if-changed=src/generated/ardupilotmega.rs");
    // Generate all typescript bindings and join them into a single String
    let bindings = [
        Message::export_to_string().unwrap(),
        Answer::export_to_string().unwrap(),
        Question::export_to_string().unwrap(),
        Negotiation::export_to_string().unwrap(),
        BindOffer::export_to_string().unwrap(),
        BindAnswer::export_to_string().unwrap(),
        PeerIdAnswer::export_to_string().unwrap(),
        Stream::export_to_string().unwrap(),
        IceNegotiation::export_to_string().unwrap(),
        MediaNegotiation::export_to_string().unwrap(),
        EndSessionQuestion::export_to_string().unwrap(),
    ]
    .join("\n\n");
    // Remove all typescript "import type" because all types are going to live in the same typescritp file
    let re = Regex::new(r"(?m)^import type .*\n").unwrap();
    let bindings = re.replace_all(bindings.as_str(), "").to_string();
    // Replace all notices by a custom one
    let re = Regex::new(r"(?m)^// This file was generated by .*\n\n").unwrap();
    let mut bindings = re.replace_all(bindings.as_str(), "").to_string();
    let custom_notice_str = "// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs) during `cargo build` step. Do not edit this file manually.\n\n";
    bindings.insert_str(0, custom_notice_str);
    // Export to file
    let output_dir = Path::new("./src/stream/webrtc/frontend/bindings/");
    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir).unwrap();
    }
    let bindings_file_path = output_dir.join(Path::new("signalling_protocol.d.ts"));
    let mut bindings_file = fs::File::create(bindings_file_path).unwrap();
    bindings_file.write_all(bindings.as_bytes()).unwrap();
    */
}

#[cfg(feature = "format-generated-code")]
fn dbg_format_code(cwd: impl AsRef<Path>, path: impl AsRef<OsStr>) {
    if let Err(error) = Command::new("rustfmt").arg(path).current_dir(cwd).status() {
        eprintln!("{error}");
    }
}

// Does nothing
#[cfg(not(feature = "format-generated-code"))]
fn dbg_format_code(_: impl AsRef<Path>, _: impl AsRef<OsStr>) {}
