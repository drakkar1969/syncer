use std::cell::{Cell, RefCell};

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::prelude::{ObjectExt, ToValue};

use serde_json::{json, Map as JsonMap, Value as JsonValue};

//------------------------------------------------------------------------------
// MODULE: ProfileObject
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::ProfileObject)]
    pub struct ProfileObject {
        #[property(get, set)]
        name: RefCell<String>,

        #[property(get, set, default = "")]
        source: RefCell<String>,
        #[property(get, set, default = "")]
        destination: RefCell<String>,

        #[property(get, set, default = 1, construct)]
        check_mode: Cell<u32>,
        #[property(get, set, default = true, construct)]
        recursive: Cell<bool>,
        #[property(get, set, default = true, construct)]
        preserve_time: Cell<bool>,
        #[property(get, set, default = true, construct)]
        preserve_permissions: Cell<bool>,
        #[property(get, set, default = true, construct)]
        preserve_owner: Cell<bool>,
        #[property(get, set, default = true, construct)]
        preserve_group: Cell<bool>,
        #[property(get, set, default = false, construct)]
        numeric_ids: Cell<bool>,
        #[property(get, set, default = true, construct)]
        preserve_symlinks: Cell<bool>,
        #[property(get, set, default = false, construct)]
        preserve_hardlinks: Cell<bool>,
        #[property(get, set, default = true, construct)]
        preserve_devices: Cell<bool>,
        #[property(get, set, default = false, construct)]
        no_leave_filesystem: Cell<bool>,
        #[property(get, set, default = false, construct)]
        delete_destination: Cell<bool>,
        #[property(get, set, default = false, construct)]
        existing: Cell<bool>,
        #[property(get, set, default = false, construct)]
        ignore_existing: Cell<bool>,
        #[property(get, set, default = false, construct)]
        skip_newer: Cell<bool>,
        #[property(get, set, default = false, construct)]
        partial: Cell<bool>,
        #[property(get, set, default = false, construct)]
        compress_data: Cell<bool>,
        #[property(get, set, default = false, construct)]
        backup: Cell<bool>,
        #[property(get, set, default = false, construct)]
        secluded_args: Cell<bool>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for ProfileObject {
        const NAME: &'static str = "ProfileObject";
        type Type = super::ProfileObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProfileObject {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: ProfileObject
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct ProfileObject(ObjectSubclass<imp::ProfileObject>);
}

impl ProfileObject {
    //---------------------------------------
    // New function
    //---------------------------------------
    pub fn new(name: &str) -> Self {
        glib::Object::builder()
            .property("name", name)
            .build()
    }

    //---------------------------------------
    // From json function
    //---------------------------------------
    pub fn from_json(json_value: &JsonValue) -> Option<Self> {
        let obj: Self = glib::Object::builder().build();

        let json_map = json_value.as_object()?;

        for (key, value) in json_map {
            if let Some(prop) = obj.find_property(key) {
                match value {
                    JsonValue::String(s) => {
                        obj.set_property_from_value(key, &s.to_value())
                    },
                    JsonValue::Number(i) => {
                        let value = i.as_u64()
                            .map(|i| (i as u32).to_value())
                            .unwrap_or(prop.default_value().to_owned());

                        obj.set_property_from_value(key, &value);
                    },
                    JsonValue::Bool(b) => {
                        obj.set_property_from_value(key, &b.to_value());
                    }
                    _ => {}
                }
            }
        }

        Some(obj)
    }

    //---------------------------------------
    // Public duplicate function
    //---------------------------------------
    pub fn duplicate(&self, name: &str) -> Self {
        let dup_obj: Self = glib::Object::builder()
            .property("name", name)
            .build();

        for property in self.list_properties() {
            let nick = property.nick();

            if nick != "name" {
                dup_obj.set_property(nick, self.property_value(nick));
            }
        }

        dup_obj
    }

    //---------------------------------------
    // Public reset function
    //---------------------------------------
    pub fn reset(&self) {
        for property in self.list_properties() {
            let nick = property.nick();

            if nick != "name" {
                self.set_property_from_value(nick, property.default_value());
            }
        }
    }

    //---------------------------------------
    // Public to json function
    //---------------------------------------
    pub fn to_json(&self) -> JsonValue {
        let mut json_map = JsonMap::new();

        for prop in self.list_properties() {
            let value = self.property_value(prop.nick());

            let json_value = match prop.value_type() {
                glib::Type::STRING => json!(value.get::<String>().unwrap()),
                glib::Type::U32 => json!(value.get::<u32>().unwrap()),
                glib::Type::BOOL => json!(value.get::<bool>().unwrap()),
                _ => json!(null)
            };

            json_map.insert(prop.name().to_owned(), json_value);
        }

        JsonValue::Object(json_map)
    }
}
