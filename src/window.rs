use std::cell::{Cell, RefCell};
use std::iter;
use std::time::Duration;

use gtk::{gio, glib, gdk};
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::{clone, closure_local, Variant, VariantTy};

use crate::Application;
use crate::sidebar::Sidebar;
use crate::profile_object::ProfileObject;
use crate::options_page::OptionsPage;
use crate::advanced_page::AdvancedPage;
use crate::rsync_page::RsyncPage;
use crate::rsync_process::{RsyncProcess, Stats};

//------------------------------------------------------------------------------
// MODULE: AppWindow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::AppWindow)]
    #[template(resource = "/com/github/RsyncUI/ui/window.ui")]
    pub struct AppWindow {
        #[template_child]
        pub(super) split_view: TemplateChild<adw::OverlaySplitView>,

        #[template_child]
        pub(super) sidebar: TemplateChild<Sidebar>,

        #[template_child]
        pub(super) content_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) content_navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) new_profile_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) options_page: TemplateChild<OptionsPage>,
        #[template_child]
        pub(super) advanced_page: TemplateChild<AdvancedPage>,
        #[template_child]
        pub(super) rsync_page: TemplateChild<RsyncPage>,

        #[property(get)]
        rsync_process: RefCell<RsyncProcess>,

        pub(super) close_request: Cell<bool>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for AppWindow {
        const NAME: &'static str = "AppWindow";
        type Type = super::AppWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            ProfileObject::ensure_type();

            klass.bind_template();

            //---------------------------------------
            // Content push options action
            //---------------------------------------
            klass.install_action("content.push-options", None, |window, _, _| {
                window.imp().content_navigation_view.push_by_tag("advanced");
            });

            //---------------------------------------
            // Rsync start action
            //---------------------------------------
            klass.install_action("rsync.start", Some(VariantTy::BOOLEAN), |window, _, parameter| {
                let imp = window.imp();

                // Get dry run
                let dry_run = parameter
                    .and_then(Variant::get::<bool>)
                    .expect("Could not get bool from variant");

                // Show rsync page
                imp.content_navigation_view.push_by_tag("rsync");

                // Get args
                let args = vec![
                        "--human-readable",
                        "--info=copy,del,flist0,misc,name,progress2,symsafe,stats2"
                    ]
                    .into_iter()
                    .chain(iter::once("--dry-run").filter(|_| dry_run))
                    .map(ToOwned::to_owned)
                    .chain(imp.options_page.profile().unwrap().args(false))
                    .collect();

                // Start rsync
                window.rsync_process().start(args);
            });

            //---------------------------------------
            // Rsync terminate action
            //---------------------------------------
            klass.install_action("rsync.terminate", None, |window, _, _| {
                window.rsync_process().terminate();
            });

            //---------------------------------------
            // Rsync pause action
            //---------------------------------------
            klass.install_action("rsync.pause", None, |window, _, _| {
                window.rsync_process().pause();
            });

            //---------------------------------------
            // Rsync resume action
            //---------------------------------------
            klass.install_action("rsync.resume", None, |window, _, _| {
                window.rsync_process().resume();
            });

            //---------------------------------------
            // Rsync show cmdline action
            //---------------------------------------
            klass.install_action("rsync.show-cmdline", None, |window, _, _| {
                let imp = window.imp();

                let builder = gtk::Builder::from_resource("/com/github/RsyncUI/ui/builder/rsync_cmdline_dialog.ui");

                let dialog: adw::AlertDialog = builder.object("dialog")
                    .expect("Could not get object from resource");

                let label: gtk::Label = builder.object("label")
                    .expect("Could not get object from resource");

                let args: Vec<String> = iter::once(String::from("rsync"))
                    .chain(imp.options_page.profile().unwrap().args(true))
                    .collect();

                label.set_label(&args.join(" "));

                let copy_button: gtk::Button = builder.object("copy_button")
                    .expect("Could not get object from resource");

                copy_button.connect_clicked(clone!(
                    #[weak] window,
                    move |_| {
                        window.clipboard().set_text(&label.label());
                    }
                ));

                dialog.present(Some(window));
            });

            //---------------------------------------
            // New profile key binding
            //---------------------------------------
            klass.add_binding(gdk::Key::N, gdk::ModifierType::CONTROL_MASK, |window| {
                window.imp().sidebar.activate_action("sidebar.new-profile", None)
                    .expect("Could not activate action 'sidebar.new-profile'");

                glib::Propagation::Stop
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AppWindow {
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

    impl WidgetImpl for AppWindow {}
    impl WindowImpl for AppWindow {
        //---------------------------------------
        // Close request function
        //---------------------------------------
        fn close_request(&self) -> glib::Propagation {
            let window = &*self.obj();

            if window.rsync_process().running() {
                gtk::prelude::WidgetExt::activate_action(window, "rsync.pause", None)
                    .expect("Could not activate action 'rsync.pause'");

                let dialog = adw::AlertDialog::builder()
                    .heading("Exit RsyncUI?")
                    .body("Terminate transfer process and exit.")
                    .default_response("exit")
                    .build();

                dialog.add_responses(&[("cancel", "_Cancel"), ("exit", "E_xit")]);
                dialog.set_response_appearance("exit", adw::ResponseAppearance::Destructive);

                dialog.connect_response(Some("exit"), clone!(
                    #[weak] window,
                    #[weak(rename_to = imp)] self,
                    move |_, _| {
                        imp.close_request.set(true);

                        gtk::prelude::WidgetExt::activate_action(&window, "rsync.terminate", None)
                            .expect("Could not activate action 'rsync.terminate'");
                    }
                ));

                dialog.present(Some(window));

                return glib::Propagation::Stop;
            }

            let _ = self.sidebar.save_config();

            glib::Propagation::Proceed
        }
    }
    impl ApplicationWindowImpl for AppWindow {}
    impl AdwApplicationWindowImpl for AppWindow {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: AppWindow
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct AppWindow(ObjectSubclass<imp::AppWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl AppWindow {
    //---------------------------------------
    // New function
    //---------------------------------------
    pub fn new(app: &Application) -> Self {
        glib::Object::builder()
            .property("application", app)
            .build()
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

        // New profile button clicked signal
        imp.new_profile_button.connect_clicked(clone!(
            #[weak] imp,
            move |_| {
                imp.sidebar.activate_action("sidebar.new-profile", None)
                    .expect("Could not activate action 'sidebar.new-profile'");
            }
        ));

        // Sidebar n_items property notify signal
        imp.sidebar.connect_n_items_notify(clone!(
            #[weak] imp,
            move |sidebar| {
                if sidebar.n_items() == 0 {
                    imp.content_navigation_view.pop();

                    imp.content_stack.set_visible_child_name("status");
                } else {
                    imp.content_stack.set_visible_child_name("profile");
                }
            }
        ));

        // Rsync page showing/hidden signals
        imp.rsync_page.connect_showing(clone!(
            #[weak] imp,
            move |_| {
                imp.sidebar.set_sensitive(false);

                imp.sidebar.action_set_enabled("sidebar.new-profile", false);
            }
        ));

        imp.rsync_page.connect_hidden(clone!(
            #[weak] imp,
            move |_| {
                imp.sidebar.set_sensitive(true);

                imp.sidebar.action_set_enabled("sidebar.new-profile", true);
            }
        ));

        // Rsync process paused property notify signal
        let rsync_process = self.rsync_process();

        rsync_process.connect_paused_notify(clone!(
            #[weak] imp,
            move |process| {
                imp.rsync_page.set_pause_button_state(process.paused());
            }
        ));

        // Rsync process status signals
        rsync_process.connect_closure("start", false, closure_local!(
            #[weak] imp,
            #[weak] rsync_process,
            move |_: RsyncProcess| {
                glib::timeout_add_local_once(Duration::from_millis(150), clone!(
                    #[weak] imp,
                    #[weak] rsync_process,
                    move || {
                        if rsync_process.running() {
                            imp.rsync_page.set_start();
                        }
                    }
                ));
            }
        ));

        rsync_process.connect_closure("message", false, closure_local!(
            #[weak] imp,
            move |_: RsyncProcess, message: String| {
                imp.rsync_page.set_message(&message);
            }
        ));

        rsync_process.connect_closure("progress", false, closure_local!(
            #[weak] imp,
            move |_: RsyncProcess, size: String, speed: String, progress: f64| {
                imp.rsync_page.set_status(&size, &speed, progress);
            }
        ));

        rsync_process.connect_closure("exit", false, closure_local!(
            #[weak(rename_to = window)] self,
            #[weak] imp,
            move |_: RsyncProcess, code: i32, stats: Option<Stats>, error: Option<String>, messages: Vec<String>, stats_msgs: Vec<String>| {
                if imp.close_request.get() {
                    window.close();
                } else {
                    imp.rsync_page.set_exit_status(code, stats.as_ref(), error.as_deref(), &messages, &stats_msgs);
                }
            }
        ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Bind sidebar visibility to options page sidebar button
        imp.split_view.bind_property("show-sidebar", &imp.options_page.sidebar_button(), "active")
            .bidirectional()
            .sync_create()
            .build();

        // Bind sidebar selected profile to options page
        imp.sidebar.bind_property("selected-profile", &imp.options_page.get(), "profile")
            .sync_create()
            .build();

        // Bind sidebar selected profile to advanced page
        imp.sidebar.bind_property("selected-profile", &imp.advanced_page.get(), "profile")
            .sync_create()
            .build();

        // Bind sidebar selected profile to rsyn page
        imp.sidebar.bind_property("selected-profile", &imp.rsync_page.get(), "profile")
            .sync_create()
            .build();

        // Load profiles from config file
        let _ = imp.sidebar.load_config();
    }
}
