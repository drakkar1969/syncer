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
        pub(super) listbox: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) model: TemplateChild<gio::ListStore>,

        #[property(get, set)]
        n_items: Cell<u32>,
        #[property(get, set, nullable)]
        selected_profile: RefCell<Option<ProfileObject>>,
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
                sidebar.profile_name_dialog("New", clone!(
                    #[weak] sidebar,
                    move |name| {
                        let imp = sidebar.imp();

                        imp.model.append(&ProfileObject::new(name));

                        sidebar.set_selected_index(imp.model.n_items() as i32 - 1);
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

                            sidebar.notify_selected_profile();
                        }
                    ));
                }
            });

            //---------------------------------------
            // Delete profile action
            //---------------------------------------
            klass.install_action("sidebar.delete-profile", Some(glib::VariantTy::STRING), |sidebar, _, parameter| {
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
                    #[weak] sidebar,
                    move |_, _| {
                        let imp = sidebar.imp();

                        if let Some(pos) = imp.model.iter::<ProfileObject>().flatten()
                            .position(|obj| obj.name() == name)
                        {
                            imp.model.remove(pos as u32);

                            sidebar.set_selected_index(pos as i32 - 1);
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
                        #[weak] sidebar,
                        move |new_name| {
                            let imp = sidebar.imp();

                            let dup_obj = obj.duplicate(new_name);

                            imp.model.insert(pos as u32 + 1, &dup_obj);

                            sidebar.set_selected_index(pos as i32 + 1);
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

                    sidebar.notify_selected_profile();
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

        // Listbox row selected signal
        imp.listbox.connect_row_selected(clone!(
            #[weak(rename_to = sidebar)] self,
            move |_, row| {
                sidebar.set_selected_profile(
                    row
                        .and_then(|row| row.downcast_ref::<SidebarRow>())
                        .and_then(|row| row.profile())
                );
            }
        ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Bind listbox to model
        imp.listbox.bind_model(Some(&imp.model.get()), |obj| {
            let profile = obj
                .downcast_ref::<ProfileObject>()
                .expect("Could not downcast to 'ProfileObject'");

            SidebarRow::new(profile).into()
        });

        // Bind model to properties
        imp.model.bind_property("n-items", self, "n-items")
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
    // Set selected index function
    //---------------------------------------
    fn set_selected_index(&self, index: i32) {
        let imp = self.imp();

        let row = imp.listbox.row_at_index(index)
            .or_else(|| imp.listbox.row_at_index(0));

        imp.listbox.select_row(row.as_ref());

        if let Some(row) = row {
            row.grab_focus();
        }
    }

    //---------------------------------------
    // Load config function
    //---------------------------------------
    pub fn load_config(&self) -> io::Result<()> {
        let imp = self.imp();

        // Load profiles from config file
        let config_path = xdg::BaseDirectories::new()
            .find_config_file("rsyncui/config.json")
            .ok_or(io::Error::other("Config file not found"))?;

        let file = fs::File::open(config_path)?;

        let reader = io::BufReader::new(file);

        let json: Vec<JsonValue> = from_reader(reader)?;

        let profiles: Vec<ProfileObject> = json.iter()
            .filter_map(ProfileObject::from_json)
            .collect();

        // Add profiles to model
        imp.model.splice(0, 0, &profiles);

        // Select first profile
        imp.listbox.select_row(imp.listbox.row_at_index(0).as_ref());

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

        let json = to_string_pretty(&profiles)
            .expect("Could not pretty print JSON string");

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
