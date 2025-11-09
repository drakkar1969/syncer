use std::cell::{Cell, RefCell};

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::prelude::ObjectExt;
use glib::translate::IntoGlib;

use strum::{EnumProperty, FromRepr};
use indexmap::IndexMap;
use serde_json::{json, Map as JsonMap, Value as JsonValue};

//------------------------------------------------------------------------------
// ENUM: CheckMode
//------------------------------------------------------------------------------
#[derive(Default, Debug, Eq, PartialEq, Clone, Copy, glib::Enum, EnumProperty, FromRepr)]
#[repr(u32)]
#[enum_type(name = "CheckMode")]
pub enum CheckMode {
    #[strum(props(Desc="No check performed (all files updated)", Switch="--ignore-times"))]
    Ignore,
    #[default]
    #[strum(props(Desc="Check file size and modification time"))]
    Default,
    #[enum_value(name = "Size Only")]
    #[strum(props(Desc="Check file size only", Switch="--size-only"))]
    SizeOnly,
    #[strum(props(Desc="Compare checksum for files with matching size", Switch="--checksum"))]
    Checksum,
}

impl CheckMode {
    pub fn value(self) -> u32 {
        self.into_glib() as u32
    }

    pub fn desc<'a>(self) -> Option<&'a str> {
        self.get_str("Desc")
    }

    pub fn switch<'a>(self) -> Option<&'a str> {
        self.get_str("Switch")
    }
}

//------------------------------------------------------------------------------
// DATA: Advanced Switches
//------------------------------------------------------------------------------
const ADVANCED_ARGS: [(&str, (&str, Option<&str>)); 18] = [
    ("recursive", ("-r", Some("-d"))),
    ("incremental-recursion", ("--i-r", Some("--no-i-r"))),
    ("preserve-time", ("-t", None)),
    ("preserve-permissions", ("-p", None)),
    ("preserve-owner", ("-o", None)),
    ("preserve-group", ("-g", None)),
    ("numeric-ids", ("--numeric-ids", None)),
    ("preserve-symlinks", ("-l", None)),
    ("preserve-hardlinks", ("-H", None)),
    ("preserve-devices", ("-D", None)),
    ("one-filesystem", ("-x", None)),
    ("delete-destination", ("--delete", None)),
    ("existing", ("--existing", None)),
    ("ignore-existing", ("--ignore-existing", None)),
    ("skip-newer", ("-u", None)),
    ("partial", ("--partial", None)),
    ("backup", ("-b", None)),
    ("secluded-args", ("-s", None)),
];

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

        #[property(get, set, default = false, construct)]
        source_copy_by_name: Cell<bool>,
        #[property(get, set, default = "")]
        source: RefCell<String>,
        #[property(get, set, default = "")]
        destination: RefCell<String>,

        #[property(get, set, default = CheckMode::default(), construct, builder(CheckMode::default()))]
        check_mode: Cell<CheckMode>,
        #[property(get, set, default = "", construct)]
        extra_options: RefCell<String>,

        #[property(get, set, default = true, construct)]
        recursive: Cell<bool>,
        #[property(get, set, default = false, construct)]
        incremental_recursion: Cell<bool>,
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
        one_filesystem: Cell<bool>,
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
            if obj.has_property(key) {
                match value {
                    JsonValue::String(s) => {
                        obj.set_property(key, s);
                    },
                    JsonValue::Number(i) => {
                        let mode = i.as_u64()
                            .and_then(|i| CheckMode::from_repr(i as u32))
                            .unwrap_or_default();

                        obj.set_property(key, mode);
                    },
                    JsonValue::Bool(b) => {
                        obj.set_property(key, b);
                    }
                    _ => {}
                }
            }
        }

        Some(obj)
    }

    //---------------------------------------
    // Duplicate function
    //---------------------------------------
    pub fn duplicate(&self, name: &str) -> Self {
        let dup_obj: Self = glib::Object::builder()
            .property("name", name)
            .build();

        for property in self.list_properties() {
            let nick = property.nick();

            if nick != "name" {
                dup_obj.set_property_from_value(nick, &self.property_value(nick));
            }
        }

        dup_obj
    }

    //---------------------------------------
    // Reset function
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
    // To json function
    //---------------------------------------
    pub fn to_json(&self) -> JsonValue {
        let mut json_map = JsonMap::new();

        for prop in self.list_properties() {
            let value = self.property_value(prop.nick());

            let json_value = if let Ok(s) = value.get::<String>() {
                json!(s)
            } else if let Ok(b) = value.get::<bool>() {
                json!(b)
            } else if let Ok(mode) = value.get::<CheckMode>() {
                json!(mode.value())
            } else {
                json!(null)
            };

            json_map.insert(prop.name().to_owned(), json_value);
        }

        JsonValue::Object(json_map)
    }

    //---------------------------------------
    // To args function
    //---------------------------------------
    pub fn to_args(&self, quoted: bool) -> Vec<String> {
        let adv_args_map = IndexMap::from(ADVANCED_ARGS);

        // Advanced options
        let mut args: Vec<String> = adv_args_map.iter()
            .filter_map(|(&nick, &(arg, off_arg))| {
                let value = self.property_value(nick)
                    .get::<bool>()
                    .ok()?;

                value.then_some(arg)
                    .or(off_arg)
                    .map(ToOwned::to_owned)
            })
            .collect();

        // Check mode
        if let Some(mode) = self.check_mode().switch() {
            args.push(mode.to_owned());
        }

        // Extra options
        let replace = if quoted { "\"" } else { "" };

        if !self.extra_options().is_empty() {
            let mut extra_options = self.extra_options()
                .replace(['\'', '"'], replace)
                .split(' ')
                .map(ToOwned::to_owned)
                .collect::<Vec<String>>();

            args.append(&mut extra_options);
        }

        // Source and destination
        if quoted {
            args.push(format!("\"{}\"", self.source()));
            args.push(format!("\"{}\"", self.destination()));
        } else {
            args.push(self.source());
            args.push(self.destination());
        }

        args
    }
}
