use std::cell::Cell;

use adw::subclass::prelude::*;
use adw::prelude::*;
use gtk::{gio, gdk, glib};
use glib::{ clone, Variant, VariantTy};

use crate::{
    Application,
    profile_object::ProfileObject,
    options_page::OptionsPage,
    advanced_page::AdvancedPage,
    rsync_page::RsyncPage,
    rsync_process::ITEMIZE_TAG
};

//------------------------------------------------------------------------------
// MODULE: AppWindow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/Syncer/ui/window.ui")]
    pub struct AppWindow {
        #[template_child]
        pub(super) status_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) status_new_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) options_page: TemplateChild<OptionsPage>,
        #[template_child]
        pub(super) advanced_page: TemplateChild<AdvancedPage>,
        #[template_child]
        pub(super) rsync_page: TemplateChild<RsyncPage>,

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

            Self::install_rsync_actions(klass);

            Self::bind_shortcuts(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

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
            let rsync_process = self.rsync_page.rsync_process();

            if rsync_process.running() {
                rsync_process.pause();

                let dialog = adw::AlertDialog::builder()
                    .heading("Exit Syncer?")
                    .body("Terminate transfer process and exit.")
                    .default_response("exit")
                    .build();

                dialog.add_responses(&[("cancel", "_Cancel"), ("exit", "E_xit")]);
                dialog.set_response_appearance("exit", adw::ResponseAppearance::Destructive);

                dialog.connect_response(Some("exit"), clone!(
                    #[weak(rename_to = imp)] self,
                    move |_, _| {
                        imp.close_request.set(true);

                        rsync_process.terminate();
                    }
                ));

                dialog.present(Some(&*self.obj()));

                return glib::Propagation::Stop;
            }

            let _ = self.options_page.save_config();

            glib::Propagation::Proceed
        }
    }
    impl ApplicationWindowImpl for AppWindow {}
    impl AdwApplicationWindowImpl for AppWindow {}

    impl AppWindow {
        //---------------------------------------
        // Install rsync actions
        //---------------------------------------
        fn install_rsync_actions(klass: &mut <Self as ObjectSubclass>::Class) {
            // Rsync start action
            klass.install_action("rsync.start", Some(VariantTy::BOOLEAN), |window, _, parameter| {
                let imp = window.imp();

                // Get dry run
                let dry_run = parameter
                    .and_then(Variant::get::<bool>)
                    .expect("Could not get bool from variant");

                // Show rsync page
                imp.navigation_view.push_by_tag("rsync");

                // Get profile
                let profile = imp.options_page.profile_dropdown().selected_item()
                    .and_downcast::<ProfileObject>()
                    .expect("Could not downcast to 'ProfileObject'");

                // Get args
                let args = [
                        "--human-readable",
                        &format!("--out-format={ITEMIZE_TAG}%i %n%L"),
                        "--info=copy,del,flist2,misc,name,progress2,symsafe,stats2"
                    ]
                    .into_iter()
                    .chain(dry_run.then_some("--dry-run"))
                    .map(ToOwned::to_owned)
                    .chain(profile.to_args(false))
                    .collect();

                // Start rsync
                imp.rsync_page.rsync_process().start(args);
            });

            // Rsync show cmdline action
            klass.install_action("rsync.show-cmdline", None, |window, _, _| {
                let imp = window.imp();

                // Build command line dialog
                let builder = gtk::Builder::from_resource("/com/github/Syncer/ui/builder/rsync_cmdline_dialog.ui");

                let dialog: adw::AlertDialog = builder.object("dialog")
                    .expect("Could not get object from resource");

                let label: gtk::Label = builder.object("label")
                    .expect("Could not get object from resource");

                // Get profile
                let profile = imp.options_page.profile_dropdown().selected_item()
                    .and_downcast::<ProfileObject>()
                    .expect("Could not downcast to 'ProfileObject'");

                // Init command line dialog
                label.set_label(&format!("rsync {}", profile.to_args(true).join(" ")));

                dialog.present(Some(window));
            });
        }

        //---------------------------------------
        // Bind shortcuts
        //---------------------------------------
        fn bind_shortcuts(klass: &mut <Self as ObjectSubclass>::Class) {
            // New profile key binding
            klass.add_binding(gdk::Key::N, gdk::ModifierType::CONTROL_MASK, |window| {
                let imp = window.imp();

                if imp.status_stack.visible_child_name() == Some("status".into()) {
                    imp.options_page.activate_action("profile.new", None)
                        .expect("Could not activate action 'new-profile'");
                }

                glib::Propagation::Stop
            });

            // Rsync show cmdline key binding
            klass.add_binding_action(gdk::Key::R, gdk::ModifierType::CONTROL_MASK, "rsync.show-cmdline");
        }
    }
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

        // Status new button clicked signal
        imp.status_new_button.connect_clicked(clone!(
            #[weak] imp,
            move |_| {
                imp.options_page.activate_action("profile.new", None)
                    .expect("Could not activate action 'new-profile'");
            }
        ));

        // Profile model items changed signal
        imp.options_page.profile_model().connect_items_changed(clone!(
            #[weak] imp,
            move |model, _, _, _| {
                if model.n_items() == 0 {
                    imp.navigation_view.pop_to_tag("options");

                    imp.status_stack.set_visible_child_name("status");
                } else {
                    imp.status_stack.set_visible_child_name("main");
                }
            }
        ));

        // Rsync process running property notify signal
        imp.rsync_page.rsync_process().connect_running_notify(clone!(
            #[weak(rename_to = window)] self,
            move |process| {
                if !process.running() && window.imp().close_request.get() {
                    window.close();
                }
            }
        ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        let profile_dropdown = imp.options_page.profile_dropdown();

        // Bind selected profile to options page
        profile_dropdown.bind_property("selected-item", &imp.options_page.get(), "profile")
            .sync_create()
            .build();

        // Bind selected profile to advanced page
        profile_dropdown.bind_property("selected-item", &imp.advanced_page.get(), "profile")
            .sync_create()
            .build();

        // Bind selected profile to rsync page
        profile_dropdown.bind_property("selected-item", &imp.rsync_page.get(), "profile")
            .sync_create()
            .build();

        // Load profiles from config file
        let _ = imp.options_page.load_config();
    }
}
