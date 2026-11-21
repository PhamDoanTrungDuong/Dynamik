//! Gives notch.exe its icon (the first choice in icon-options/, the tray's default) so Explorer and
//! the Start menu shortcut show it. Skipped with a warning when windres isn't on PATH.
use std::{env, path::PathBuf, process::Command};

fn main() {
    let ico = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../icon-options/icon-dog-c-violet.ico");
    println!("cargo:rerun-if-changed={}", ico.display());
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let rc = out.join("icon.rc");
    std::fs::write(&rc, format!("1 ICON \"{}\"\n", ico.display().to_string().replace('\\', "/"))).unwrap();
    let obj = out.join("icon.o");
    let built = Command::new("windres").arg(&rc).args(["-O", "coff", "-o"]).arg(&obj).status().is_ok_and(|s| s.success());
    if built {
        println!("cargo:rustc-link-arg-bins={}", obj.display());
    } else {
        println!("cargo:warning=windres not found; notch.exe is built without its icon");
    }
}
