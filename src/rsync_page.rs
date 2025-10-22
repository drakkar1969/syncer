use std::cell::RefCell;

use gtk::glib;
use adw::subclass::prelude::*;
use adw::prelude::*;

use crate::stats_table::StatsTable;
use crate::rsync::Stats;

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
        pub(super) stats_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) stats_table: TemplateChild<StatsTable>,

        #[template_child]
        pub(super) button_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) pause_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) pause_content: TemplateChild<adw::ButtonContent>,

        #[property(get, set)]
        source: RefCell<String>,
        #[property(get, set)]
        destination: RefCell<String>,
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
    impl ObjectImpl for RsyncPage {}

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

            let source = obj.source();
            let destination = obj.destination();

            self.source_box.set_visible(!source.is_empty());
            self.source_label.set_label(&source);

            self.destination_box.set_visible(!destination.is_empty());
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
    // Reset function
    //---------------------------------------
    fn reset(&self) {
        let imp = self.imp();

        self.set_can_pop(false);

        imp.progress_label.set_label("0%");
        imp.progress_bar.set_fraction(0.0);

        imp.transferred_label.set_label("");
        imp.speed_label.set_label("0");

        imp.message_box.set_css_classes(&[]);
        imp.message_image.set_icon_name(Some("rsync-message-symbolic"));
        imp.message_label.set_label("");

        imp.stats_stack.set_visible_child_name("empty");
        imp.button_stack.set_visible_child_name("empty");
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
    // Set message function
    //---------------------------------------
    pub fn set_message(&self, message: &str) {
        let imp = self.imp();

        imp.message_label.set_label(message);
    }

    //---------------------------------------
    // Set status function
    //---------------------------------------
    pub fn set_status(&self, size: &str, speed: &str, progress: f64, dry_run: bool) {
        let imp = self.imp();

        imp.progress_label.set_label(&format!("{progress}%"));
        imp.progress_bar.set_fraction(progress/100.0);

        imp.transferred_label.set_label(size);
        imp.speed_label.set_label(speed);

        if imp.button_stack.visible_child_name() == Some("empty".into()) && !dry_run {
            imp.button_stack.set_visible_child_name("buttons");
        }
    }

    //---------------------------------------
    // Set exit status function
    //---------------------------------------
    pub fn set_exit_status(&self, code: i32, stats: Option<&Stats>, error: Option<&str>) {
        let imp = self.imp();

        imp.button_stack.set_visible_child_name("empty");

        match (code, stats) {
            (-1, _) => {}

            (0, Some(stats)) => {
                imp.progress_label.set_label("100%");
                imp.progress_bar.set_fraction(1.0);

                imp.message_box.set_css_classes(&["success", "heading"]);
                imp.message_image.set_icon_name(Some("rsync-success-symbolic"));

                imp.message_label.set_label(&format!(
                    "Success: {} of {} transferred",
                    stats.bytes.transferred,
                    stats.bytes.source
                ));

                imp.speed_label.set_label(&stats.bytes.speed);

                imp.stats_table.fill(stats);

                imp.stats_stack.set_visible_child_name("stats");
            }

            (0, None) => {
                imp.message_box.set_css_classes(&["warning", "heading"]);
                imp.message_image.set_icon_name(Some("rsync-success-symbolic"));

                imp.message_label.set_label("Success: could not retrieve stats");
            }

            (code, _) => {
                imp.message_box.set_css_classes(&["error", "heading"]);
                imp.message_image.set_icon_name(Some("rsync-error-symbolic"));

                if let Some(error) = error {
                    imp.message_label.set_label(&format!("{error} (code {code})"));

                } else {
                    imp.message_label.set_label(&format!("Unknown error (code {code})"));
                }
            }
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
