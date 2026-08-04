#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

fn main() {
    if let Err(error) = keyecho::run() {
        eprintln!("KeyEcho failed: {error:#}");
        std::process::exit(1);
    }
}
