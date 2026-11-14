#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;
fn main() {
    #[cfg(target_os = "windows")]
    windows::run();
    #[cfg(target_os = "linux")]
    linux::run();
}
