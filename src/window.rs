use gtk::{gio, glib};
use adw::subclass::prelude::*;
use adw::prelude::*;

use crate::Application;
use crate::profile_object::ProfileObject;
use crate::profile_pane::ProfilePane;

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
        pub(super) sidebar_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) sidebar_selection: TemplateChild<gtk::SingleSelection>,
        #[template_child]
        pub(super) sidebar_model: TemplateChild<gio::ListStore>,
        #[template_child]
        pub(super) profile_nav_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) profile_pane: TemplateChild<ProfilePane>,
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
            ProfileObject::ensure_type();;

            klass.bind_template();
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
    // Setup widgets
    //---------------------------------------
    pub fn setup_widgets(&self) {
        let imp = self.imp();

        // Add default profile to sidebar
        imp.sidebar_model.append(&ProfileObject::default());

        // Bind sidebar selection to profile pane title
        imp.sidebar_selection.bind_property("selected-item", &imp.profile_nav_page.get(), "title")
            .transform_to(|_, obj: Option<glib::Object>| {
                let name = obj
                    .and_downcast::<ProfileObject>()
                    .expect("Could not downcast to 'ProfileObject'")
                    .name();

                Some(name)
            })
            .sync_create()
            .build();
    }
}
