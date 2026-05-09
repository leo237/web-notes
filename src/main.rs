mod app;
mod config;
mod diagnostics;
mod domain;
mod notes;
mod paths;

fn main() -> anyhow::Result<()> {
    app::run()
}
