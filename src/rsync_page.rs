use std::cell::Cell;
use std::sync::LazyLock;

use gtk::glib;
use adw::subclass::prelude::*;
use adw::prelude::*;

use regex::Regex;
use itertools::Itertools;

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
        pub(super) message_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) progress_bar: TemplateChild<gtk::ProgressBar>,

        #[template_child]
        pub(super) button_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) pause_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) pause_content: TemplateChild<adw::ButtonContent>,

        #[property(get, set)]
        paused: Cell<bool>,
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
            obj.setup_widgets();
        }
    }

    impl WidgetImpl for RsyncPage {}
    impl NavigationPageImpl for RsyncPage {}
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
        // Page hidden signal
        self.connect_hidden(|page| {
            page.reset();
        });
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Bind paused property to pause button
        self.bind_property("paused", &imp.pause_content.get(), "icon-name")
            .transform_to(|_, paused: bool| Some(if paused { "start-symbolic" } else { "pause-symbolic" }))
            .sync_create()
            .build();

        self.bind_property("paused", &imp.pause_content.get(), "label")
            .transform_to(|_, paused: bool| Some(if paused { "_Resume" } else { "_Pause" }))
            .sync_create()
            .build();

        self.bind_property("paused", &imp.pause_button.get(), "action-name")
            .transform_to(|_, paused: bool| Some(if paused { "rsync.resume" } else { "rsync.pause" }))
            .sync_create()
            .build();
    }

    //---------------------------------------
    // Reset function
    //---------------------------------------
    fn reset(&self) {
        let imp = self.imp();

        self.set_can_pop(false);

        imp.progress_label.set_label("");
        imp.progress_bar.set_fraction(0.0);

        imp.transferred_label.set_label("");
        imp.speed_label.set_label("");

        imp.message_label.set_css_classes(&[]);
        imp.message_label.set_label("");

        imp.button_stack.set_visible_child_name("running");
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

        imp.progress_label.set_label(&format!("{progress}%"));
        imp.progress_bar.set_fraction(progress/100.0);

        imp.transferred_label.set_label(size);
        imp.speed_label.set_label(speed);
    }

    //---------------------------------------
    // Public set exit status function
    //---------------------------------------
    pub fn set_exit_status(&self, code: Option<i32>, stats: &[String]) {
        let imp = self.imp();

        let stats = stats.join("\n");

        static EXPR: LazyLock<Regex> = LazyLock::new(|| {
            let expr = [
                r"Number of files:\s*([\d,]+)(?:\s*\([^)]*\))?",
                r"Number of created files:\s*([\d,]+)(?:\s*\([^)]*\))?",
                r"Number of deleted files:\s*([\d,]+)(?:\s*\([^)]*\))?",
                r".+",
                r"Total file size: (.+) bytes",
                "Total transferred file size: (.+) bytes"
            ]
            .join("\n");

            Regex::new(&expr)
                .expect("Failed to compile Regex")
        });

        let data = EXPR.captures(&stats)
            .and_then(|caps| {
                caps.iter()
                    .flatten()
                    .map(|m| m.as_str())
                    .collect_tuple()
            });

        match (code, data) {
            (Some(0), Some((_, files, created, _, size, transferred))) => {
                imp.progress_label.set_label("100%");
                imp.progress_bar.set_fraction(1.0);

                imp.message_label.set_css_classes(&["success"]);

                imp.message_label.set_label(&format!("Transfer successful: {created} of {files} files [{transferred} of {size}]"));
            },
            (Some(0), None) => {
                imp.message_label.set_css_classes(&["warning"]);

                imp.message_label.set_label("Transfer successful: could not retrieve stats");
            },
            (Some(code), _) => {
                imp.message_label.set_css_classes(&["error"]);

                imp.message_label.set_label(&format!("Transfer failed: error code {code}"));
            }
            _ => ()
        }

        imp.button_stack.set_visible_child_name("finished");

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
