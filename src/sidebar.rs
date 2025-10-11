use std::cell::{Cell, RefCell};

use gtk::{glib, gio};
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::clone;

use itertools::Itertools;

use crate::sidebar_row::SidebarRow;
use crate::profile_object::ProfileObject;

//------------------------------------------------------------------------------
// MODULE: Sidebar
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::Sidebar)]
    #[template(resource = "/com/github/RsyncUI/ui/sidebar.ui")]
    pub struct Sidebar {
        #[template_child]
        pub(super) new_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub(super) view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) selection: TemplateChild<gtk::SingleSelection>,
        #[template_child]
        pub(super) model: TemplateChild<gio::ListStore>,
        #[template_child]
        pub(super) factory: TemplateChild<gtk::SignalListItemFactory>,

        #[property(get, set)]
        n_items: Cell<u32>,
        #[property(get, set)]
        selected_item: RefCell<Option<ProfileObject>>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for Sidebar {
        const NAME: &'static str = "Sidebar";
        type Type = super::Sidebar;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            //---------------------------------------
            // New profile action
            //---------------------------------------
            klass.install_action("sidebar.new-profile", None, |sidebar, _, _| {
                let imp = sidebar.imp();

                sidebar.profile_name_dialog("Create New Profile", "Create", clone!(
                    #[weak] imp,
                    move |name| {
                        imp.model.append(&ProfileObject::new(name));

                        imp.view.scroll_to(
                            imp.model.n_items() - 1,
                            gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT,
                            None
                        );

                        imp.view.grab_focus();
                    }
                ));
            });

            //---------------------------------------
            // Rename profile action
            //---------------------------------------
            klass.install_action("sidebar.rename-profile", Some(glib::VariantTy::STRING), |sidebar, _, parameter| {
                let imp = sidebar.imp();

                let name = parameter
                    .and_then(|param| param.get::<String>())
                    .expect("Could not get string from variant");

                if let Some((pos, obj)) = imp.model.iter::<ProfileObject>().flatten()
                    .find_position(|obj| obj.name() == name)
                {
                    sidebar.profile_name_dialog("Rename Profile", "Rename", clone!(
                        #[weak] imp,
                        move |name| {
                            imp.model.remove(pos as u32);

                            obj.set_name(name);

                            imp.model.insert(pos as u32, &obj);

                            imp.view.scroll_to(
                                pos as u32,
                                gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT,
                                None
                            );
                        }
                    ));
                }
            });

            //---------------------------------------
            // Delete profile action
            //---------------------------------------
            klass.install_action("sidebar.delete-profile", Some(glib::VariantTy::STRING), |sidebar, _, parameter| {
                let imp = sidebar.imp();

                let name = parameter
                    .and_then(|param| param.get::<String>())
                    .expect("Could not get string from variant");

                let dialog = adw::AlertDialog::builder()
                    .heading("Delete Profile?")
                    .body("This wil permamenently delete the profile.")
                    .default_response("delete")
                    .build();

                dialog.add_responses(&[("cancel", "_Cancel"), ("delete", "_Delete")]);
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

                dialog.connect_response(Some("delete"), clone!(
                    #[weak] imp,
                    move |_, _| {
                        if let Some(pos) = imp.model.iter::<ProfileObject>().flatten()
                            .position(|obj| obj.name() == name)
                        {
                            imp.model.remove(pos as u32);
                        }
                    }
                ));

                dialog.present(Some(sidebar));
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Sidebar {
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

    impl WidgetImpl for Sidebar {}
    impl BinImpl for Sidebar {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: Sidebar
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct Sidebar(ObjectSubclass<imp::Sidebar>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Sidebar {
    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

        // Factory setup signal
        imp.factory.connect_setup(|_, item| {
            item
                .downcast_ref::<gtk::ListItem>()
                .expect("Could not downcast to 'GtkListItem'")
                .set_child(Some(&SidebarRow::default()));
        });

        // Factory bind signal
        imp.factory.connect_bind(|_, item| {
            let obj = item
                .downcast_ref::<gtk::ListItem>()
                .and_then(|item| item.item())
                .and_downcast::<ProfileObject>()
                .expect("Could not downcast to 'ProfileObject'");

            let row = item
                .downcast_ref::<gtk::ListItem>()
                .and_then(|item| item.child())
                .and_downcast::<SidebarRow>()
                .expect("Could not downcast to 'SidebarRow'");

            row.bind(&obj);
        });

        // Factory unbind signal
        imp.factory.connect_unbind(|_, item| {
            let row = item
                .downcast_ref::<gtk::ListItem>()
                .and_then(|item| item.child())
                .and_downcast::<SidebarRow>()
                .expect("Could not downcast to 'SidebarRow'");

            row.unbind();
        });
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Bind selection to properties
        imp.selection.bind_property("n-items", self, "n-items")
            .sync_create()
            .build();

        imp.selection.bind_property("selected-item", self, "selected_item")
            .sync_create()
            .build();

        let profile = ProfileObject::new("TEST");
        profile.set_source("/home/drakkar/.cache");
        profile.set_destination("/home/drakkar/Scratch/RSYNC");

        imp.model.append(&profile);
    }

    //---------------------------------------
    // Profile name dialog function
    //---------------------------------------
    fn profile_name_dialog<F>(&self, heading: &str, label: &str, f: F)
    where F: Fn(&str) + 'static {
        let builder = gtk::Builder::from_resource("/com/github/RsyncUI/ui/builder/profile_name_dialog.ui");

        let dialog: adw::AlertDialog = builder.object("dialog")
            .expect("Could not get object from resource");

        dialog.set_heading(Some(heading));
        dialog.set_response_label("add", label);

        let entry: adw::EntryRow = builder.object("entry")
            .expect("Could not get object from resource");

        entry.connect_changed(clone!(
            #[weak] dialog,
            move |entry| {
                dialog.set_response_enabled("add", !entry.text().is_empty());
            }
        ));

        dialog.connect_response(Some("add"), move |_, _| {
            f(&entry.text());
        });

        dialog.present(Some(self));
    }
}

impl Default for Sidebar {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
