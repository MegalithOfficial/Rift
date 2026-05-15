mod app;
mod config;
mod model;
mod theme;

use adw::prelude::*;

fn main() -> glib::ExitCode {
    adw::init().expect("failed to initialize libadwaita");

    let app = app::build();
    app.run()
}
