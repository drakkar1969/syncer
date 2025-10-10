use std::cell::Cell;

use gtk::glib;
use adw::subclass::prelude::*;
use gtk::prelude::*;

//------------------------------------------------------------------------------
// MODULE: RsyncPane
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::RsyncPane)]
    #[template(resource = "/com/github/RsyncUI/ui/rsync_pane.ui")]
    pub struct RsyncPane {
        #[template_child]
        pub(super) revealer: TemplateChild<gtk::Revealer>,

        #[template_child]
        pub(super) message_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) transferred_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) speed_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) progress_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) stop_button: TemplateChild<gtk::Button>,

        #[property(get, set)]
        reveal_child: Cell<bool>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for RsyncPane {
        const NAME: &'static str = "RsyncPane";
        type Type = super::RsyncPane;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for RsyncPane {
        //---------------------------------------
        // Constructor
        //---------------------------------------
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.setup_widgets();
        }
    }

    impl WidgetImpl for RsyncPane {}
    impl BinImpl for RsyncPane {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: RsyncPane
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct RsyncPane(ObjectSubclass<imp::RsyncPane>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RsyncPane {
    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        self.bind_property("reveal-child", &imp.revealer.get(), "reveal-child")
            .sync_create()
            .build();
    }

    //---------------------------------------
    // Public transition duration function
    //---------------------------------------
    pub fn transition_duration(&self) -> u32 {
        self.imp().revealer.transition_duration()
    }

    //---------------------------------------
    // Public reset function
    //---------------------------------------
    pub fn reset(&self) {
        let imp = self.imp();

        imp.message_label.set_label("");

        imp.transferred_label.set_label("");
        imp.speed_label.set_label("");
        imp.progress_label.set_label("");
        imp.progress_bar.set_fraction(0.0);
    }

    //---------------------------------------
    // Public set message function
    //---------------------------------------
    pub fn set_message(&self, message: &str) {
        self.imp().message_label.set_label(message);
    }

    //---------------------------------------
    // Public set status function
    //---------------------------------------
    pub fn set_status(&self, size: &str, speed: &str, progress: f64) {
        let imp = self.imp();

        imp.transferred_label.set_label(size);
        imp.speed_label.set_label(speed);
        imp.progress_label.set_label(&format!("{progress}%"));
        imp.progress_bar.set_fraction(progress/100.0);
    }

    //---------------------------------------
    // Public set progress function
    //---------------------------------------
    pub fn set_progress(&self, progress: f64) {
        let imp = self.imp();

        imp.progress_label.set_label(&format!("{progress}%"));
        imp.progress_bar.set_fraction(progress/100.0);
    }

    //---------------------------------------
    // Public set exit status function
    //---------------------------------------
    pub fn set_exit_status(&self, success: bool, message: &str) {
        let imp = self.imp();

        if success {
            imp.message_label.set_css_classes(&["success"]);
        } else {
            imp.message_label.set_css_classes(&["error"]);
        }

        imp.message_label.set_label(message);
    }
}

impl Default for RsyncPane {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
