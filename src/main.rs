mod app;
mod window;
mod profile_object;
mod profile_dialog;
mod options_page;
mod advanced_page;
mod advanced_switch;
mod filters_page;
mod filter_row;
mod filter_dialog;
mod rsync_page;
mod output_window;
mod output_item;
mod output_header;
mod utils;

use gtk::prelude::*;
use gtk::{gio, glib};

use app::Application;

const APP_ID: &str = "com.github.Syncer";

fn main() -> glib::ExitCode {
    // Register and include resources
    gio::resources_register_include!("resources.gresource")
        .expect("Failed to register resources");

    // Run app
    let app = Application::new(APP_ID, gio::ApplicationFlags::default());

    app.run()
}
