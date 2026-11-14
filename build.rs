use std::{env, path::PathBuf, process::Command};
fn main() {
    if !env::var("TARGET").unwrap_or_default().contains("windows") {
        return;
    }
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/aireluma.ico");
    println!("cargo:rerun-if-changed=assets/footer-screen.ico");
    println!("cargo:rerun-if-changed=assets/footer-wifi.ico");
    println!("cargo:rerun-if-changed=assets/footer-clock.ico");

    println!("cargo:rerun-if-changed=assets/footer-screen-light.ico");
    println!("cargo:rerun-if-changed=assets/footer-wifi-light.ico");
    println!("cargo:rerun-if-changed=assets/footer-clock-light.ico");
    let obj = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("app-res.o");
    let status = Command::new("windres")
        .args(["assets/app.rc", "-O", "coff", "-o"])
        .arg(&obj)
        .status()
        .expect("windres is required");
    assert!(status.success(), "failed to compile application icon");
    println!("cargo:rustc-link-arg={}", obj.display());
}
