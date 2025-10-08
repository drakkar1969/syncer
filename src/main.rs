mod app;
mod window;
mod sidebar_row;
mod profile_object;
mod rsync_page;
mod check_object;
mod options_page;
mod progress_pane;

use gtk::{glib, gio};
use gtk::prelude::*;

use app::Application;

const APP_ID: &str = "com.github.RsyncUI";

fn main() -> glib::ExitCode {
    // Register and include resources
    gio::resources_register_include!("resources.gresource")
        .expect("Failed to register resources");

    // Run app
    let app = Application::new(APP_ID, gio::ApplicationFlags::default());

    app.run()
}
