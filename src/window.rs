use std::cell::Cell;

use gtk::{gio, glib, gdk};
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::clone;

use nix::sys::signal as nix_signal;
use nix::unistd::Pid as NixPid;

use crate::Application;
use crate::sidebar::Sidebar;
use crate::profile_object::ProfileObject;
use crate::options_page::OptionsPage;
use crate::advanced_page::AdvancedPage;

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
        pub(super) sidebar: TemplateChild<Sidebar>,

        #[template_child]
        pub(super) content_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) content_navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) options_page: TemplateChild<OptionsPage>,
        #[template_child]
        pub(super) advanced_page: TemplateChild<AdvancedPage>,

        #[property(get, set)]
        rsync_running: Cell<bool>,

        pub(super) dry_run: Cell<bool>,
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

            // Content push options action
            klass.install_action("content.push-options", None, |window, _, _| {
                window.imp().content_navigation_view.push_by_tag("settings");
            });

            // Rsync start action
            klass.install_action("rsync.start", Some(glib::VariantTy::BOOLEAN), |window, _, parameter| {
                let dry_run = parameter
                    .and_then(|param| param.get::<bool>())
                    .expect("Could not get bool from variant");

                window.imp().dry_run.set(dry_run);

                window.set_rsync_running(true);
            });

            // Rsync stop action
            klass.install_action("rsync.stop", None, |window, _, _| {
                let imp = window.imp();

                if let Some(id) = imp.options_page.rsync_pane().rsync_id() {
                    let pid = NixPid::from_raw(id);

                    let _ = nix_signal::kill(pid, nix_signal::Signal::SIGTERM);
                }
            });

            // Rsync close action
            klass.install_action("rsync.close", None, |window, _, _| {
                window.set_rsync_running(false);
            });

            //---------------------------------------
            // New profile key binding
            //---------------------------------------
            klass.add_binding(gdk::Key::N, gdk::ModifierType::CONTROL_MASK, |window| {
                window.imp().sidebar.activate_action("sidebar.new-profile", None)
                    .unwrap();

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
    impl WindowImpl for AppWindow {}
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
    // Rsync args function
    //---------------------------------------
    fn rsync_args(&self) -> Vec<String> {
        let imp = self.imp();

        imp.options_page.args()
            .map(|mut options| {
                let mut args: Vec<String> = ["-s", "--human-readable", "--info=copy,del,flist0,misc,name,progress2,symsafe,stats2"]
                    .into_iter()
                    .map(|s| s.to_owned())
                    .collect();

                args.append(
                    &mut imp.advanced_page.args().into_iter().map(|s| s.to_owned()).collect()
                );

                args.append(&mut options);

                args
            })
            .unwrap_or_default()
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

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

        // Rsync running property notify signal
        // self.connect_rsync_running_notify(clone!(
        //     #[weak] imp,
        //     move |window| {
        //         let running = window.rsync_running();

        //         imp.sidebar_new_button.set_sensitive(!running);
        //         imp.sidebar_view.set_sensitive(!running);

        //         imp.options_page.content_box().set_sensitive(!running);

        //         let rsync_pane = imp.options_page.rsync_pane();

        //         rsync_pane.set_args(window.rsync_args());
        //         rsync_pane.set_running(running);
        //     }
        // ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Bind sidebar selected item to rsync page
        imp.sidebar.bind_property("selected-item", &imp.options_page.get(), "profile")
            .sync_create()
            .build();

        // Bind sidebar selected item to option page
        imp.sidebar.bind_property("selected-item", &imp.advanced_page.get(), "profile")
            .sync_create()
            .build();
    }
}
