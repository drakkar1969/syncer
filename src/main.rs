mod app;
mod window;
mod sidebar;
mod sidebar_row;
mod profile_object;
mod options_page;
mod advanced_page;
mod adv_switchrow;
mod rsync_page;
mod stats_table;
mod rsync;

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
