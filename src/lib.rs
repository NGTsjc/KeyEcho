pub mod app;
pub mod audio;
pub mod config;
pub mod language;
pub mod platform;

pub fn run() -> anyhow::Result<()> {
    app::run()
}
