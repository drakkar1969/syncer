use gtk::{gio, glib};
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::clone;

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
        pub(super) sidebar_selection: TemplateChild<gtk::SingleSelection>,
        #[template_child]
        pub(super) sidebar_model: TemplateChild<gio::ListStore>,
        #[template_child]
        pub(super) sidebar_factory: TemplateChild<gtk::SignalListItemFactory>,

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
            ProfileObject::ensure_type();

            klass.bind_template();

            //---------------------------------------
            // Add class actions
            //---------------------------------------
            // New profile action
            klass.install_action("sidebar.new-profile", None, |window, _, _| {
                window.profile_name_dialog("Add New Profile");
            });

            // Delete profile action
            klass.install_action("sidebar.delete-profile", Some(glib::VariantTy::STRING), |window, _, parameter| {
                let imp = window.imp();

                let name = parameter
                    .and_then(|param| param.get::<String>())
                    .expect("Could not get string from variant");

                if let Some(pos) = imp.sidebar_model.iter::<ProfileObject>().flatten()
                    .position(|obj| obj.name() == name)
                {
                    imp.sidebar_model.remove(pos as u32);
                }
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
    // Profile name dialog helper function
    //---------------------------------------
    fn profile_name_dialog(&self, heading: &str) {
        let imp = self.imp();

        let builder = gtk::Builder::from_resource("/com/github/RsyncUI/ui/builder/profile_name_dialog.ui");

        let dialog: adw::AlertDialog = builder.object("dialog")
            .expect("Could not get object from resource");

        dialog.set_heading(Some(heading));

        let entry: adw::EntryRow = builder.object("entry")
            .expect("Could not get object from resource");

        entry.connect_changed(clone!(
            #[weak] dialog,
            move |entry| {
                dialog.set_response_enabled("add", !entry.text().is_empty());
            }
        ));

        dialog.connect_response(Some("add"), clone!(
            #[weak] imp,
            move |_, _| {
                imp.sidebar_model.append(&ProfileObject::new(&entry.text()));
            }
        ));

        dialog.present(Some(self));
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

        // Sidebar factory setup signal
        imp.sidebar_factory.connect_setup(clone!(
            move |_, item| {
                let builder = gtk::Builder::from_resource("/com/github/RsyncUI/ui/builder/sidebar_item.ui");

                let child: gtk::Box = builder.object("box")
                    .expect("Could not get object from resource");

                item
                    .downcast_ref::<gtk::ListItem>()
                    .expect("Could not downcast to 'GtkListItem'")
                    .set_child(Some(&child));
            }
        ));

        // Sidebar factory bind signal
        imp.sidebar_factory.connect_bind(clone!(
            move |_, item| {
                let item = item
                    .downcast_ref::<gtk::ListItem>()
                    .expect("Could not downcast to 'GtkListItem'");

                let obj = item
                    .item()
                    .and_downcast::<ProfileObject>()
                    .expect("Could not downcast to 'ProfileObject'");

                let child = item
                    .child()
                    .and_downcast::<gtk::Box>()
                    .expect("Could not downcast to 'GtkBox'");

                let label = child
                    .first_child()
                    .and_downcast::<gtk::Label>()
                    .expect("Could not downcast to 'GtkLabel'");

                label.set_label(&obj.name());

                let menu_button = child
                    .last_child()
                    .and_downcast::<gtk::MenuButton>()
                    .expect("Could not downcast to 'GtkMenuButton'");

                let menu_model = gio::Menu::new();

                let menu_item = gio::MenuItem::new(Some("Delete"), Some("sidebar.delete-profile"));
                menu_item.set_attribute_value("target", Some(&obj.name().to_variant()));
                menu_model.append_item(&menu_item);

                menu_button.set_menu_model(Some(&menu_model));
            }
        ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Add default profile to sidebar
        imp.sidebar_model.append(&ProfileObject::default());

        // Bind sidebar selected item to profile pane
        imp.sidebar_selection.bind_property("selected-item", &imp.profile_pane.get(), "profile")
            .sync_create()
            .build();
    }
}
