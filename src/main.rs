mod app;
mod window;
mod profile_object;
mod options_page;
mod filter_expander_row;
mod filter_row;
mod advanced_page;
mod adv_switchrow;
mod rsync_page;
mod stats_table;
mod output_window;
mod output_item;
mod output_header;
mod rsync_process;
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
