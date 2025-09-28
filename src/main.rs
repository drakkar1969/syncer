mod app;
mod window;

use gtk::{glib, gio};
use gtk::prelude::*;

use app::Application;

const APP_ID: &str = "com.github.RsyncGUI";

fn main() -> glib::ExitCode {
    // Register and include resources
    gio::resources_register_include!("resources.gresource")
        .expect("Failed to register resources");

    // Run app
    let app = Application::new(APP_ID, gio::ApplicationFlags::default());

    app.run()
}
