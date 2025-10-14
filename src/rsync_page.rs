use std::cell::Cell;
use std::sync::LazyLock;
use std::collections::HashMap;

use gtk::glib;
use adw::subclass::prelude::*;
use adw::prelude::*;

use regex::{Regex, Captures};

use crate::stats_table::{Stats, StatsRow, StatsTable};

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
        pub(super) stats_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) stats_table: TemplateChild<StatsTable>,
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
            .transform_to(|_, paused: bool| Some(if paused { "rsync-start-symbolic" } else { "rsync-pause-symbolic" }))
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

        imp.progress_label.set_label("0%");
        imp.progress_bar.set_fraction(0.0);

        imp.transferred_label.set_label("");
        imp.speed_label.set_label("0");

        imp.message_box.set_css_classes(&[]);
        imp.message_image.set_icon_name(Some("rsync-message-symbolic"));
        imp.message_label.set_label("");

        imp.stats_stack.set_visible_child_name("empty");
    }

    //---------------------------------------
    // Public set message function
    //---------------------------------------
    pub fn set_message(&self, message: &str) {
        let imp = self.imp();

        imp.message_label.set_label(message);
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

        if imp.stats_stack.visible_child_name() != Some("buttons".into()) {
            imp.stats_stack.set_visible_child_name("buttons");
        }
    }

    //---------------------------------------
    // Error function
    //---------------------------------------
    fn error(&self, code: i32, errors: &[String]) -> Option<String> {
        let error_map = HashMap::from([
            (1, "syntax or usage error"),
            (20, "terminated by user")
        ]);

        error_map.get(&code)
            .map(|s| s.to_string())
            .or_else(|| {
                static EXPR: LazyLock<Regex> = LazyLock::new(|| {
                    Regex::new(r"^rsync error:\s*(?P<err>.*?)\(.*")
                        .expect("Failed to compile Regex")
                });

                errors.into_iter()
                    .filter_map(|error| {
                        EXPR.captures(error).and_then(|caps| {
                            caps.name("err")
                                .map(|m| m.as_str().trim_end().to_owned())
                        })
                    })
                    .next()
            })
    }

    //---------------------------------------
    // Stats function
    //---------------------------------------
    fn stats(&self, stats: &[String]) -> Option<Stats> {
        let stats = stats.join("\n");

        static EXPR: LazyLock<Regex> = LazyLock::new(|| {
            let expr = [
                r"Number of files:\s*(?P<st>[\d,]+)\s*\(?(?:reg:\s*(?P<sf>[\d,]+))?,?\s*(?:dir:\s*(?P<sd>[\d,]+))?,?\s*(?:link:\s*(?P<sl>[\d,]+))?,?\s*(?:special:\s*(?P<ss>[\d,]+))?,?\s*\)?",
                r"Number of created files:\s*(?P<ct>[\d,]+)\s*\(?(?:reg:\s*(?P<cf>[\d,]+))?,?\s*(?:dir:\s*(?P<cd>[\d,]+))?,?\s*(?:link:\s*(?P<cl>[\d,]+))?,?\s*(?:special:\s*(?P<cs>[\d,]+))?,?\s*\)?",
                r"Number of deleted files:\s*(?P<dt>[\d,]+)\s*\(?(?:reg:\s*(?P<df>[\d,]+))?,?\s*(?:dir:\s*(?P<dd>[\d,]+))?,?\s*(?:link:\s*(?P<dl>[\d,]+))?,?\s*(?:special:\s*(?P<ds>[\d,]+))?,?\s*\)?",
                r"Number of regular files transferred: (?P<nt>[\d,]+)",
                r"Total file size: (?P<bs>.+) bytes",
                r"Total transferred file size: (?P<bt>.+) bytes"
            ]
            .join("\n");

            Regex::new(&expr)
                .expect("Failed to compile Regex")
        });

        EXPR.captures(&stats)
            .map(|caps| {
                let get_match = |caps: &Captures, m: &str| -> String {
                    let mut text = caps.name(m)
                        .map(|m| m.as_str().to_owned())
                        .unwrap_or_default();

                    if text.ends_with(",") {
                        text.pop();
                    }

                    text
                };

                Stats {
                    n_transfers: get_match(&caps, "nt"),
                    n_source: StatsRow {
                        total: get_match(&caps, "st"),
                        files: get_match(&caps, "sf"),
                        dirs: get_match(&caps, "sd"),
                        links: get_match(&caps, "sl"),
                        specials: get_match(&caps, "ss")
                    },
                    n_created: StatsRow {
                        total: get_match(&caps, "ct"),
                        files: get_match(&caps, "cf"),
                        dirs: get_match(&caps, "cd"),
                        links: get_match(&caps, "cl"),
                        specials: get_match(&caps, "cs")
                    },
                    n_deleted: StatsRow {
                        total: get_match(&caps, "dt"),
                        files: get_match(&caps, "df"),
                        dirs: get_match(&caps, "dd"),
                        links: get_match(&caps, "dl"),
                        specials: get_match(&caps, "ds")
                    },
                    source_bytes: get_match(&caps, "bs"),
                    transfer_bytes: get_match(&caps, "bt")
                }
            })
    }

    //---------------------------------------
    // Public set exit status function
    //---------------------------------------
    pub fn set_exit_status(&self, code: Option<i32>, stats: &[String], errors: &[String]) {
        let imp = self.imp();

        let stats = self.stats(stats);

        match (code, stats) {
            (Some(0), Some(stats)) => {
                imp.progress_label.set_label("100%");
                imp.progress_bar.set_fraction(1.0);

                imp.message_box.set_css_classes(&["success"]);
                imp.message_image.set_icon_name(Some("rsync-success-symbolic"));

                imp.message_label.set_label(&format!(
                    "Transfer successful: {} of {} files [{} of {}]",
                    stats.n_created.total,
                    stats.n_source.total,
                    stats.transfer_bytes,
                    stats.source_bytes
                ));

                imp.stats_table.fill(&stats);

                imp.stats_stack.set_visible_child_name("stats");
            },
            (Some(0), None) => {
                imp.message_box.set_css_classes(&["warning"]);
                imp.message_image.set_icon_name(Some("rsync-success-symbolic"));

                imp.message_label.set_label("Transfer successful: could not retrieve stats");

                imp.stats_stack.set_visible_child_name("empty");
            },
            (Some(code), _) => {
                imp.message_box.set_css_classes(&["error"]);
                imp.message_image.set_icon_name(Some("rsync-error-symbolic"));

                if let Some(error) = self.error(code, errors) {
                    imp.message_label.set_label(&format!("Transfer failed: {error} (code {code})"));

                } else {
                    imp.message_label.set_label(&format!("Transfer failed: unknown error (code {code})"));
                }

                imp.stats_stack.set_visible_child_name("empty");
            }
            _ => ()
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
