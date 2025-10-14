use std::cell::{Cell, RefCell};
use std::io::{self, Write};
use std::fs;

use gtk::{glib, gio};
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::clone;

use itertools::Itertools;
use serde_json::{to_string_pretty, from_reader, Value as JsonValue};

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

                sidebar.profile_name_dialog("New", clone!(
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

                if let Some(obj) = imp.model.iter::<ProfileObject>().flatten()
                    .find(|obj| obj.name() == name)
                {
                    sidebar.profile_name_dialog("Rename", clone!(
                        #[weak] sidebar,
                        move |new_name| {
                            obj.set_name(new_name);

                            sidebar.notify_selected_item();
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
                    .body(format!("Permamenently delete the \"{}\" profile.", name))
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

            //---------------------------------------
            // Duplicate profile action
            //---------------------------------------
            klass.install_action("sidebar.duplicate-profile", Some(glib::VariantTy::STRING), |sidebar, _, parameter| {
                let imp = sidebar.imp();

                let name = parameter
                    .and_then(|param| param.get::<String>())
                    .expect("Could not get string from variant");

                if let Some((pos, obj)) = imp.model.iter::<ProfileObject>().flatten()
                    .find_position(|obj| obj.name() == name)
                {
                    sidebar.profile_name_dialog("Duplicate", clone!(
                        #[weak] imp,
                        move |new_name| {
                            let dup_obj = obj.duplicate(new_name);

                            imp.model.insert(pos as u32 + 1, &dup_obj);

                            imp.view.scroll_to(
                                pos as u32 + 1,
                                gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT,
                                None
                            );

                            imp.view.grab_focus();
                        }
                    ));
                }
            });

            //---------------------------------------
            // Reset profile action
            //---------------------------------------
            klass.install_action("sidebar.reset-profile", Some(glib::VariantTy::STRING), |sidebar, _, parameter| {
                let imp = sidebar.imp();

                let name = parameter
                    .and_then(|param| param.get::<String>())
                    .expect("Could not get string from variant");

                if let Some(obj) = imp.model.iter::<ProfileObject>().flatten()
                    .find(|obj| obj.name() == name)
                {
                    obj.reset();

                    sidebar.notify_selected_item();
                }
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
    }

    //---------------------------------------
    // Profile name dialog function
    //---------------------------------------
    fn profile_name_dialog<F>(&self, response: &str, f: F)
    where F: Fn(&str) + 'static {
        let builder = gtk::Builder::from_resource("/com/github/RsyncUI/ui/builder/profile_name_dialog.ui");

        let dialog: adw::AlertDialog = builder.object("dialog")
            .expect("Could not get object from resource");

        dialog.set_heading(Some(&format!("{response} Profile")));
        dialog.set_response_label("add", response);

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

    //---------------------------------------
    // Load config function
    //---------------------------------------
    pub fn load_config(&self) -> io::Result<()> {
        let config_path = xdg::BaseDirectories::new()
            .find_config_file("rsyncui/config.json")
            .ok_or(io::Error::other("Config file not found"))?;

        let file = fs::File::open(config_path)?;

        let reader = io::BufReader::new(file);

        let json: Vec<JsonValue> = from_reader(reader)?;

        let profiles: Vec<ProfileObject> = json.iter()
            .filter_map(ProfileObject::from_json)
            .collect();

        self.imp().model.splice(0, 0, &profiles);

        Ok(())
    }

    //---------------------------------------
    // Save config function
    //---------------------------------------
    pub fn save_config(&self) -> io::Result<()> {
        let profiles: Vec<JsonValue> = self.imp().model.iter::<ProfileObject>()
            .flatten()
            .map(|obj| obj.to_json())
            .collect();

        let json = to_string_pretty(&profiles).unwrap();

        let config_path = xdg::BaseDirectories::new()
            .place_config_file("rsyncui/config.json")?;

        let mut file = fs::File::create(config_path)?;

        file.write_all(json.as_bytes())
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
