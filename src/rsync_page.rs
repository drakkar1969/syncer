use std::cell::RefCell;

use gtk::glib;
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::clone;

use crate::profile_object::ProfileObject;
use crate::stats_table::StatsTable;
use crate::details_window::DetailsWindow;
use crate::rsync_process::Stats;

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
    #[template(resource = "/com/github/RsyncUI/ui/rsync_page.ui")]
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
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) stats_table: TemplateChild<StatsTable>,
        #[template_child]
        pub(super) pause_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) pause_content: TemplateChild<adw::ButtonContent>,
        #[template_child]
        pub(super) details_button: TemplateChild<gtk::Button>,

        #[property(get, set, nullable)]
        profile: RefCell<Option<ProfileObject>>,

        pub(super) details: RefCell<Option<Vec<String>>>,
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

            let source = obj.profile().map(|profile| profile.source()).unwrap_or_default();
            let destination = obj.profile().map(|profile| profile.destination()).unwrap_or_default();

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

        // Details button clicked signal
        imp.details_button.connect_clicked(clone!(
            #[weak(rename_to = page)] self,
            move|_| {
                if let Some(details) = page.imp().details.borrow().as_deref() {
                    let parent = page.root()
                        .and_downcast::<gtk::Window>()
                        .expect("Could not downcast to 'GtkWindow'");

                    let window = DetailsWindow::new(&parent);

                    window.display(details);
                }
            }
        ));
    }

    //---------------------------------------
    // Reset function
    //---------------------------------------
    fn reset(&self) {
        let imp = self.imp();

        self.set_can_pop(false);

        imp.details.replace(None);

        imp.progress_label.set_label("0%");
        imp.progress_bar.set_fraction(0.0);

        imp.transferred_label.set_label("0B");
        imp.speed_label.set_label("0B/s");

        imp.message_box.set_css_classes(&[]);
        imp.message_image.set_icon_name(Some("rsync-message-symbolic"));
        imp.message_label.set_label("");

        imp.stack.set_visible_child_name("empty");
        imp.stats_table.reset();
    }

    //---------------------------------------
    // Set pause button state function
    //---------------------------------------
    pub fn set_pause_button_state(&self, paused: bool) {
        let imp = self.imp();

        if paused {
            imp.pause_content.set_icon_name("rsync-start-symbolic");
            imp.pause_content.set_label("_Resume");
            imp.pause_button.set_action_name(Some("rsync.resume"));
        } else {
            imp.pause_content.set_icon_name("rsync-pause-symbolic");
            imp.pause_content.set_label("_Pause");
            imp.pause_button.set_action_name(Some("rsync.pause"));
        }
    }

    //---------------------------------------
    // Set start function
    //---------------------------------------
    pub fn set_start(&self) {
        let imp = self.imp();

        if imp.stack.visible_child_name() == Some("empty".into()) {
            imp.stack.set_visible_child_name("buttons");
        }
    }

    //---------------------------------------
    // Set message function
    //---------------------------------------
    pub fn set_message(&self, message: &str) {
        let imp = self.imp();

        imp.message_label.set_label(message);
    }

    //---------------------------------------
    // Set status function
    //---------------------------------------
    pub fn set_status(&self, size: &str, speed: &str, progress: f64) {
        let imp = self.imp();

        imp.progress_label.set_label(&format!("{progress}%"));
        imp.progress_bar.set_fraction(progress/100.0);

        imp.transferred_label.set_label(&format!("{size}B"));
        imp.speed_label.set_label(speed);
    }

    //---------------------------------------
    // Set exit status function
    //---------------------------------------
    pub fn set_exit_status(&self, code: i32, stats: Option<&Stats>, error: Option<&str>, details: &[String]) {
        let imp = self.imp();

        // Store messages
        let has_details = !details.is_empty();

        imp.details.replace(has_details.then(|| details.to_vec()));

        imp.details_button.set_sensitive(has_details);

        // Ensure progress bar at 100% if success
        if code == 0 {
            imp.progress_label.set_label("100%");
            imp.progress_bar.set_fraction(1.0);
        }

        // Show exit status in message label
        match (code, stats) {
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

                let error = error.unwrap_or("Unknown error");

                imp.message_label.set_label(&format!("{error} (code {code})"));
            }
        }

        // Show stats
        if let Some(stats) = stats {
            imp.speed_label.set_label(&format!("{}B/s", stats.speed));

            imp.stats_table.fill(stats);

            imp.stack.set_visible_child_name("stats");
        } else {
            imp.stack.set_visible_child_name("empty");
        }

        self.set_can_pop(true);
    }
}

impl Default for RsyncPage {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
