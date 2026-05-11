use std::cell::Cell;

use adw::{prelude::*, subclass::prelude::*};
use gtk::{gio, glib};
use glib::clone;

use crate::{
    Application,
    start_page::StartPage,
    advanced_page::AdvancedPage,
    filters_page::FiltersPage,
    rsync_page::RsyncPage,
    rsync::RsyncState,
    output_page::OutputPage
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
        pub(super) new_profile_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) start_page: TemplateChild<StartPage>,
        #[template_child]
        pub(super) advanced_page: TemplateChild<AdvancedPage>,
        #[template_child]
        pub(super) filters_page: TemplateChild<FiltersPage>,
        #[template_child]
        pub(super) rsync_page: TemplateChild<RsyncPage>,
        #[template_child]
        pub(super) output_page: TemplateChild<OutputPage>,

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
            klass.bind_template();
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
        // Close request virtual method
        //---------------------------------------
        fn close_request(&self) -> glib::Propagation {
            let rsync = self.rsync_page.rsync();

            if rsync.state() != RsyncState::Stopped {
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

                        if rsync.terminate().is_err() {
                            imp.obj().show_toast("Error: Failed to terminate rsync");
                        }
                    }
                ));

                dialog.present(Some(&*self.obj()));

                return glib::Propagation::Stop;
            }

            if self.start_page.save_config().is_err() {
                self.obj().show_toast("Error: Failed to save config to file");
            }

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
    // Show toast function
    //---------------------------------------
    pub fn show_toast(&self, message: &str){
        let toast = adw::Toast::builder()
            .custom_title(
                &gtk::Label::builder()
                    .label(message)
                    .css_classes(["heading", "error"])
                    .build()
            )
            .build();

        self.imp().toast_overlay.add_toast(toast);
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
                imp.start_page.activate_action("profile.new", None)
                    .expect("Could not activate action 'new-profile'");
            }
        ));

        // Profile model items changed signal
        imp.start_page.profile_model().connect_items_changed(clone!(
            #[weak] imp,
            move |model, _, _, _| {
                if model.n_items() == 0 {
                    imp.navigation_view.pop_to_tag("start");

                    imp.status_stack.set_visible_child_name("status");
                } else {
                    imp.status_stack.set_visible_child_name("main");
                }
            }
        ));

        // Rsync page rsync state property notify signal
        imp.rsync_page.rsync().connect_state_notify(clone!(
            #[weak(rename_to = window)] self,
            move |page| {
                let imp = window.imp();

                if page.state() == RsyncState::Stopped && imp.close_request.get() {
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

        // Set page widget properties
        imp.start_page.set_rsync_page(imp.rsync_page.get());

        imp.rsync_page.set_navigation_view(imp.navigation_view.get());
        imp.rsync_page.set_output_page(imp.output_page.get());

        // Bind selected profile to start page
        let profile_dropdown = imp.start_page.profile_dropdown();

        profile_dropdown.bind_property("selected-item", &imp.start_page.get(), "profile")
            .sync_create()
            .build();

        // Bind selected profile to advanced page
        profile_dropdown.bind_property("selected-item", &imp.advanced_page.get(), "profile")
            .sync_create()
            .build();

        // Bind selected profile to filters page
        profile_dropdown.bind_property("selected-item", &imp.filters_page.get(), "profile")
            .sync_create()
            .build();

        // Load profiles from config file
        if imp.start_page.load_config().is_err() {
            self.show_toast("Error: Failed to load config from file");
        }
    }
}
