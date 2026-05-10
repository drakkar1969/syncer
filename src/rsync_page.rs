use std::cell::RefCell;
use std::time::Duration;

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;
use glib::{clone, closure_local};

use crate::{
    window::AppWindow,
    profile_object::ProfileObject,
    rsync::{Rsync, RsyncState, RsyncOutput},
    output_page::OutputPage,
    utils::{case::capitalize_first, size}
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
        pub(super) status_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) status_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) message_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) transferred_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) speed_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) recurse_spinner: TemplateChild<adw::Spinner>,
        #[template_child]
        pub(super) progress_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) source_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) filters_wrap: TemplateChild<adw::WrapBox>,

        #[template_child]
        pub(super) button_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) pause_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) pause_content: TemplateChild<adw::ButtonContent>,
        #[template_child]
        pub(super) stop_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) output_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) output_image: TemplateChild<gtk::Image>,

        #[property(get, set)]
        profile: RefCell<ProfileObject>,
        #[property(get, set)]
        rsync: RefCell<Rsync>,

        #[property(get, set)]
        output_page: RefCell<OutputPage>,
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

            Self::install_actions(klass);
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

    impl RsyncPage {
        //---------------------------------------
        // Install actions
        //---------------------------------------
        fn install_actions(klass: &mut <Self as ObjectSubclass>::Class) {
            // Pause rsync action
            klass.install_action("rsync.pause", None, |page, _, _| {
                let rsync = page.rsync();

                if rsync.state() == RsyncState::Paused {
                    let result = rsync.resume();

                    if result.is_err() {
                        let window = page.root()
                            .and_downcast::<AppWindow>()
                            .expect("Could not get main window");

                        window.show_toast("Error: Failed to resume rsync");
                    }
                } else if rsync.state() == RsyncState::Running {
                    let result = rsync.pause();

                    if result.is_err() {
                        let window = page.root()
                            .and_downcast::<AppWindow>()
                            .expect("Could not get main window");

                        window.show_toast("Error: Failed to pause rsync");
                    }
                }
            });

            // Stop rsync action
            klass.install_action("rsync.stop", None, |page, _, _| {
                if page.rsync().terminate().is_err() {
                    let window = page.root()
                        .and_downcast::<AppWindow>()
                        .expect("Could not get main window");

                    window.show_toast("Error: Failed to terminate rsync");
                }
            });

            // Rsync output action
            klass.install_action("rsync.output", None, |page, _, _| {
                page.activate_action("navigation.push", Some(&"output".to_variant()))
                    .expect("Could not activate 'navigation.push' action");
            });
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
        // Profile property notify signal
        self.connect_profile_notify(|page| {
            let imp = page.imp();

            let profile = page.profile();

            // Set page title
            page.set_title(&profile.name());

            // Set source folder
            imp.source_label.set_label(&profile.source());

            // Helper closure to add tag labels for filters
            let add_filter_tag = |title: &str, dimmed: bool| {
                let css_classes = if dimmed {
                    vec!["caption-heading", "tag", "dimmed"]
                } else {
                    vec!["caption-heading", "tag"]
                };

                let label = gtk::Label::builder()
                    .label(title)
                    .css_classes(css_classes)
                    .build();

                imp.filters_wrap.append(&label);
            };

            // Set filter rules
            let max = 6;
            let mut i = 0;

            imp.filters_wrap.remove_all();

            if profile.filters().is_empty() {
                add_filter_tag("None", true);
            } else {
                for filter in profile.filters() {
                    if i < max && let Some((rule, pattern)) = filter.split_once(' ') {
                        add_filter_tag(
                            &format!("{} {pattern}", capitalize_first(rule)), false
                        );
                    } else if i == max {
                        add_filter_tag(" ... ", false);

                        break;
                    }

                    i += 1;
                } 
            }
        });

        // State property notify signal
        let rsync = self.rsync();

        rsync.connect_state_notify(clone!(
            #[weak(rename_to = page)] self,
            move |rsync| {
                let imp = page.imp();

                match rsync.state() {
                    RsyncState::Stopped => {
                        imp.stop_button.set_sensitive(false);
                        imp.pause_button.set_sensitive(false);
                    }

                    RsyncState::Running => {
                        imp.stop_button.set_sensitive(true);
                        imp.pause_button.set_sensitive(true);

                        imp.pause_content.set_icon_name("media-playback-pause-symbolic");
                        imp.pause_content.set_label("_Pause");
                    }

                    RsyncState::Paused => {
                        imp.stop_button.set_sensitive(true);
                        imp.pause_button.set_sensitive(true);

                        imp.pause_content.set_icon_name("media-playback-start-symbolic");
                        imp.pause_content.set_label("_Resume");
                    }
                }
            }
        ));

        // Rsync start signal
        rsync.connect_closure("start", false, closure_local!(
            #[weak(rename_to = page)] self,
            move |_: Rsync| {
                page.ui_start();
            }
        ));

        // Rsync status signal
        rsync.connect_closure("status", false, closure_local!(
            #[weak(rename_to = page)] self,
            move |_: Rsync, msg: String| {
                page.ui_status(&msg);
            }
        ));

        // Rsync message signal
        rsync.connect_closure("message", false, closure_local!(
            #[weak(rename_to = page)] self,
            move |_: Rsync, msg: String| {
                page.ui_message(&msg);
            }
        ));

        // Rsync recurse complete signal
        rsync.connect_closure("recurse-complete", false, closure_local!(
            #[weak(rename_to = page)] self,
            move |_: Rsync| {
                page.ui_recursion(true);
            }
        ));

        // Rsync progress signal
        rsync.connect_closure("progress", false, closure_local!(
            #[weak(rename_to = page)] self,
            move |rsync: Rsync, size: String, speed: String, progress: f64| {
                page.ui_transferred(&size::format(&size));

                if !rsync.dry_run() {
                    page.ui_speed(&size::format(&speed));
                }

                page.ui_bar_progress(progress);
            }
        ));

        // Rsync exit signal
        rsync.connect_closure("exit", false, closure_local!(
            #[weak(rename_to = page)] self,
            move |_: Rsync, code_var: glib::Variant, obj: glib::BoxedAnyObject| {
                let code = code_var.get::<Option::<i32>>()
                    .expect("Could not extract value from variant");

                let output = obj.borrow::<RsyncOutput>();

                page.ui_exit_status(code, &output);
            }
        ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        self.ui_reset();
    }

    //---------------------------------------
    // UI reset function
    //---------------------------------------
    pub fn ui_reset(&self) {
        let imp = self.imp();

        self.ui_status_format(&["heading"], "rsync-status-symbolic");
        self.ui_status("Waiting…");

        self.ui_message("---");

        self.ui_transferred("0");
        self.ui_speed("n/a");
        self.ui_bar_progress(0.0);

        self.ui_recursion(false);

        imp.button_stack.set_visible_child_name("empty");
    }

    //---------------------------------------
    // UI start function
    //---------------------------------------
    fn ui_start(&self) {
        // Show pause and terminate buttons
        glib::timeout_add_local_once(Duration::from_millis(200), clone!(
            #[weak(rename_to = page)] self,
            move || {
                if page.rsync().state() != RsyncState::Stopped {
                    page.imp().button_stack.set_visible_child_name("rsync");
                }
            }
        ));
    }

    //---------------------------------------
    // UI status function
    //---------------------------------------
    fn ui_status(&self, status: &str) {
        self.imp().status_label.set_label(status);
    }

    //---------------------------------------
    // UI status format function
    //---------------------------------------
    fn ui_status_format(&self, css_classes: &[&str], icon: &str) {
        let imp = self.imp();

        imp.status_box.set_css_classes(css_classes);
        imp.status_image.set_icon_name(Some(icon));
    }

    //---------------------------------------
    // UI message function
    //---------------------------------------
    fn ui_message(&self, message: &str) {
        self.imp().message_label.set_label(message);
    }

    //---------------------------------------
    // UI transferred function
    //---------------------------------------
    fn ui_transferred(&self, size: &str) {
        self.imp().transferred_label.set_label(&format!("{size}B"));
    }

    //---------------------------------------
    // UI speed function
    //---------------------------------------
    fn ui_speed(&self, speed: &str) {
        self.imp().speed_label.set_label(speed);
    }

    //---------------------------------------
    // UI recursion function
    //---------------------------------------
    fn ui_recursion(&self, complete: bool) {
        let imp = self.imp();

        let visible = imp.recurse_spinner.is_visible();

        if visible == complete {
            self.imp().recurse_spinner.set_visible(!complete);
        }
    }

    //---------------------------------------
    // UI bar progress function
    //---------------------------------------
    fn ui_bar_progress(&self, fraction: f64) {
        let imp = self.imp();

        imp.progress_label.set_label(&format!("{fraction}%"));
        imp.progress_bar.set_fraction(fraction/100.0);
    }

    //---------------------------------------
    // UI exit status function
    //---------------------------------------
    fn ui_exit_status(&self, code: Option<i32>, output: &RsyncOutput) {
        let imp = self.imp();

        let stats_table = Rsync::stats(output);

        // Show exit status
        match code {
            Some(0) => {
                // Ensure progress bar at 100% if success
                self.ui_bar_progress(100.0);

                if let Some(stats) = stats_table.as_ref() {
                    let status = format!("Success: {}B of {}B transferred",
                        stats.transferred_size(),
                        stats.source_size()
                    );

                    let msg = format!("{} of {} files transferred",
                        stats.transferred_items(),
                        stats.source_items()
                    );

                    self.ui_status_format(&["success", "heading"], "rsync-success-symbolic");
                    self.ui_status(&status);

                    self.ui_message(&msg);
                } else {
                    self.ui_status_format(&["warning", "heading"], "rsync-success-symbolic");
                    self.ui_status("Success: could not retrieve stats");

                    self.ui_message("Transfer information not available");
                }
            }

            Some(code) => {
                let (error, details) = Rsync::errors(code, output.errors());

                self.ui_status_format(&["error", "heading"], "rsync-error-symbolic");
                self.ui_status(&error);

                self.ui_message(&details);
            }

            None => {
                self.ui_status_format(&["error", "heading"], "rsync-error-symbolic");
                self.ui_status("Unknown error");

                self.ui_message("Error details not available");
            }
        }

        // Show other stats
        if !self.rsync().dry_run() && let Some(stats) = stats_table {
            self.ui_speed(&format!("{}B/s", stats.speed()));
        }

        // Show details
        if !output.is_empty() {
            let spinner = adw::SpinnerPaintable::new(Some(&imp.output_image.get()));

            imp.output_button.set_sensitive(false);
            imp.output_image.set_paintable(Some(&spinner));
            imp.button_stack.set_visible_child_name("output");

            let output_owned = output.to_owned();

            // Populate output page
            glib::idle_add_local_once(clone!(
                #[weak(rename_to = page)] self,
                move || {
                    let imp = page.imp();

                    page.output_page().load(output_owned);

                    imp.output_image.set_icon_name(Some("go-next-symbolic"));
                    imp.output_button.set_sensitive(true);
                    imp.output_button.grab_focus();
                }
            ));
        }
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
