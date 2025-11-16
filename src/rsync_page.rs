use std::cell::RefCell;
use std::time::Duration;

use adw::subclass::prelude::*;
use adw::prelude::*;
use gtk::glib;
use glib::{clone, closure_local};

use crate::{
    profile_object::ProfileObject,
    stats_table::StatsTable,
    log_window::LogWindow,
    rsync_process::{RsyncProcess, RsyncMessages}
};

//------------------------------------------------------------------------------
// MODULE: RsyncPage
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::RsyncPage)]
    #[template(resource = "/com/github/Syncer/ui/rsync_page.ui")]
    pub struct RsyncPage {
        #[template_child]
        pub(super) progress_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) transferred_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) speed_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) message_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) message_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) message_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) source_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) source_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) destination_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) destination_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub(super) stats_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) stats_table: TemplateChild<StatsTable>,
        #[template_child]
        pub(super) button_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) pause_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) pause_content: TemplateChild<adw::ButtonContent>,
        #[template_child]
        pub(super) stop_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) log_button: TemplateChild<gtk::Button>,

        #[property(get, set, nullable)]
        profile: RefCell<Option<ProfileObject>>,
        #[property(get)]
        rsync_process: RefCell<RsyncProcess>,

        pub(super) log_window: RefCell<LogWindow>
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for RsyncPage {
        const NAME: &'static str = "RsyncPage";
        type Type = super::RsyncPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for RsyncPage {
        //---------------------------------------
        // Constructor
        //---------------------------------------
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.setup_signals();
        }
    }

    impl WidgetImpl for RsyncPage {}
    impl NavigationPageImpl for RsyncPage {
        //---------------------------------------
        // Hidden function
        //---------------------------------------
        fn hidden(&self) {
            self.obj().reset();
        }

        //---------------------------------------
        // Showing function
        //---------------------------------------
        fn showing(&self) {
            let obj = self.obj();

            let source = obj.profile()
                .map(|profile| profile.source())
                .unwrap_or_default();

            let destination = obj.profile()
                .map(|profile| profile.destination())
                .unwrap_or_default();

            self.source_box.set_visible(!source.is_empty() && !destination.is_empty());
            self.source_label.set_label(&source);

            self.destination_box.set_visible(!source.is_empty() && !destination.is_empty());
            self.destination_label.set_label(&destination);
        }
    }
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: RsyncPage
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct RsyncPage(ObjectSubclass<imp::RsyncPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RsyncPage {
    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

        // Profile property notify signal
        self.connect_profile_notify(|page| {
            if let Some(profile) = page.profile() {
                // Set page title
                page.set_title(&profile.name());
            }
        });

        // Rsync process paused property notify signal
        let rsync_process = self.rsync_process();

        rsync_process.connect_paused_notify(clone!(
            #[weak] imp,
            move |process| {
                if process.paused() {
                    imp.pause_content.set_icon_name("rsync-start-symbolic");
                    imp.pause_content.set_label("_Resume");
                } else {
                    imp.pause_content.set_icon_name("rsync-pause-symbolic");
                    imp.pause_content.set_label("_Pause");
                }
            }
        ));

        // Rsync process start signal
        rsync_process.connect_closure("start", false, closure_local!(
            #[weak] imp,
            move |process: RsyncProcess| {
                glib::timeout_add_local_once(Duration::from_millis(150), clone!(
                    #[weak] imp,
                    move || {
                        if process.running() {
                            imp.button_stack.set_visible_child_name("rsync");
                        }
                    }
                ));
            }
        ));

        // Rsync process message signal
        rsync_process.connect_closure("message", false, closure_local!(
            #[weak] imp,
            move |_: RsyncProcess, message: String| {
                imp.message_label.set_label(&message);
            }
        ));

        // Rsync process progress signal
        rsync_process.connect_closure("progress", false, closure_local!(
            #[weak] imp,
            move |_: RsyncProcess, size: String, speed: String, progress: f64| {
                imp.transferred_label.set_label(&format!("{size}B"));
                imp.speed_label.set_label(&speed);

                imp.progress_label.set_label(&format!("{progress}%"));
                imp.progress_bar.set_fraction(progress/100.0);
            }
        ));

        // Rsync process exit signal
        rsync_process.connect_closure("exit", false, closure_local!(
            #[weak(rename_to = page)] self,
            move |_: RsyncProcess, code: i32, messages: RsyncMessages| {
                page.set_exit_status(code, &messages);
            }
        ));

        // Pause button clicked signal
        imp.pause_button.connect_clicked(clone!(
            #[weak(rename_to = page)] self,
            move|_| {
                let process = page.rsync_process();

                if process.paused() {
                    process.resume();
                } else {
                    process.pause();
                }
            }
        ));

        // Stop button clicked signal
        imp.stop_button.connect_clicked(clone!(
            #[weak(rename_to = page)] self,
            move|_| {
                page.rsync_process().terminate();
            }
        ));

        // Log button clicked signal
        imp.log_button.connect_clicked(clone!(
            #[weak(rename_to = page)] self,
            move|_| {
                let parent = page.root()
                    .and_downcast::<gtk::Window>()
                    .expect("Could not downcast to 'GtkWindow'");

                page.imp().log_window.borrow().display(&parent);
            }
        ));
    }

    //---------------------------------------
    // Reset function
    //---------------------------------------
    fn reset(&self) {
        let imp = self.imp();

        self.set_can_pop(false);

        imp.progress_label.set_label("0%");
        imp.progress_bar.set_fraction(0.0);

        imp.transferred_label.set_label("0B");
        imp.speed_label.set_label("0B/s");

        imp.message_box.set_css_classes(&[]);
        imp.message_image.set_icon_name(Some("rsync-message-symbolic"));
        imp.message_label.set_label("");

        imp.stats_stack.set_visible_child_name("empty");
        imp.button_stack.set_visible_child_name("empty");

        imp.log_window.borrow().clear_messages();
    }

    //---------------------------------------
    // Set exit status function
    //---------------------------------------
    pub fn set_exit_status(&self, code: i32, messages: &RsyncMessages) {
        let imp = self.imp();

        // Ensure progress bar at 100% if success
        if code == 0 {
            imp.progress_label.set_label("100%");
            imp.progress_bar.set_fraction(1.0);
        }

        // Show exit status in message label
        let stats = RsyncProcess::stats(messages.stats());

        match (code, &stats) {
            (0, Some(stats)) => {
                imp.message_box.set_css_classes(&["success", "heading"]);
                imp.message_image.set_icon_name(Some("rsync-success-symbolic"));

                imp.message_label.set_label(&format!(
                    "Success: {}B of {}B transferred",
                    stats.bytes_transferred,
                    stats.bytes_source
                ));
            }

            (0, None) => {
                imp.message_box.set_css_classes(&["warning", "heading"]);
                imp.message_image.set_icon_name(Some("rsync-success-symbolic"));

                imp.message_label.set_label("Success: could not retrieve stats");
            }

            (code, _) => {
                imp.message_box.set_css_classes(&["error", "heading"]);
                imp.message_image.set_icon_name(Some("rsync-error-symbolic"));

                let error = RsyncProcess::error(code, messages.errors())
                    .unwrap_or_else(|| String::from("Unknown error"));

                imp.message_label.set_label(&format!("{error} (code {code})"));
            }
        }

        // Show stats
        if let Some(stats) = stats {
            imp.speed_label.set_label(&format!("{}B/s", stats.speed));

            imp.stats_table.fill(&stats);

            imp.stats_stack.set_visible_child_name("stats");
        } else {
            imp.stats_stack.set_visible_child_name("empty");
        }

        // Show details
        if messages.messages().is_empty() && messages.stats().is_empty() {
            imp.button_stack.set_visible_child_name("empty");
        } else {
            imp.button_stack.set_visible_child_name("log");

            // Populate log window
            imp.log_window.borrow().load_messages(messages);
        }

        self.set_can_pop(true);
    }
}
