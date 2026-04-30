use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::str::FromStr;

use gtk::{prelude::ObjectExt, subclass::prelude::*, glib};

use strum::{EnumProperty, FromRepr, AsRefStr, EnumString};
use indexmap::IndexMap;
use serde_json::{json, Map as JsonMap, Value as JsonValue};

use crate::filter_row::FilterRule;

//------------------------------------------------------------------------------
// ENUM: CheckMode
//------------------------------------------------------------------------------
#[derive(Default, Debug, Eq, PartialEq, Clone, Copy, glib::Enum, EnumProperty, FromRepr, AsRefStr, EnumString)]
#[repr(u32)]
#[enum_type(name = "CheckMode")]
#[strum(serialize_all = "kebab-case")]
pub enum CheckMode {
    #[default]
    #[strum(props(Desc="Check file size and modification time"))]
    Default,
    #[strum(props(Desc="No check performed (all files updated)", Switch="--ignore-times"))]
    Ignore,
    #[enum_value(name = "Size Only")]
    #[strum(props(Desc="Check file size only", Switch="--size-only"))]
    SizeOnly,
    #[strum(props(Desc="Compare checksum for files with matching size", Switch="--checksum"))]
    Checksum,
}

impl CheckMode {
    pub fn desc<'a>(self) -> Option<&'a str> {
        self.get_str("Desc")
    }

    pub fn switch<'a>(self) -> Option<&'a str> {
        self.get_str("Switch")
    }
}

//------------------------------------------------------------------------------
// ENUM: RecurseMode
//------------------------------------------------------------------------------
#[derive(Default, Debug, Eq, PartialEq, Clone, Copy, glib::Enum, EnumProperty, FromRepr, AsRefStr, EnumString)]
#[repr(u32)]
#[enum_type(name = "RecurseMode")]
#[strum(serialize_all = "kebab-case")]
pub enum RecurseMode {
    #[default]
    #[strum(props(Desc="Recurse into directories incrementally", Switches="-r"))]
    Incremental,
    #[enum_value(name = "Non-Incremental")]
    #[strum(props(Desc="Recurse into directories (non-incremental)", Switches="-r --no-i-r"))]
    NonIncremental,
    #[enum_value(name = "No Recursion")]
    #[strum(props(Desc="Don't recurse into directories", Switches="-d"))]
    NoRecursion,
}

impl RecurseMode {
    pub fn desc<'a>(self) -> Option<&'a str> {
        self.get_str("Desc")
    }

    pub fn switches<'a>(self) -> Option<&'a str> {
        self.get_str("Switches")
    }
}

//------------------------------------------------------------------------------
// DATA: Advanced Options
//------------------------------------------------------------------------------
const ADVANCED_OPTIONS: [(&str, &str); 15] = [
    ("preserve-time", "-t"),
    ("preserve-permissions", "-p"),
    ("preserve-owner", "-o"),
    ("preserve-group", "-g"),
    ("numeric-ids", "--numeric-ids"),
    ("preserve-symlinks", "-l"),
    ("preserve-hardlinks", "-H"),
    ("preserve-devices", "-D"),
    ("one-filesystem", "-x"),
    ("delete-destination", "--delete"),
    ("existing", "--existing"),
    ("ignore-existing", "--ignore-existing"),
    ("skip-newer", "-u"),
    ("partial", "--partial"),
    ("backup", "-b"),
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

        #[property(get, set, default = Self::default_source().as_ref(), construct)]
        source: RefCell<String>,
        #[property(get, set, default = "", construct)]
        destination: RefCell<String>,

        #[property(get, set, construct, builder(CheckMode::default()))]
        check_mode: Cell<CheckMode>,
        #[property(get, set, construct, builder(RecurseMode::default()))]
        recurse_mode: Cell<RecurseMode>,
        #[property(get, set, construct)]
        filters: RefCell<Vec<String>>,

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

        #[property(get = Self::adv_modified)]
        adv_modified: PhantomData<bool>,
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

    impl ProfileObject {
        //---------------------------------------
        // Property default value
        //---------------------------------------
        fn default_source() -> String {
            let mut home_path = glib::home_dir().to_string_lossy().into_owned();
            home_path.push('/');

            home_path
        }

        //---------------------------------------
        // Property getter
        //---------------------------------------
        fn adv_modified(&self) -> bool {
            let obj = self.obj();

            let mut modified = false;

            for (nick, _) in ADVANCED_OPTIONS {
                let default = obj.find_property(nick)
                    .map(|property| property.default_value().get::<bool>());

                let value = obj.property_value(nick).get::<bool>();

                if default != Some(value) {
                    modified = true;
                    break;
                }
            }

            modified
        }
    }
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
        let profile: Self = glib::Object::builder()
            .property("name", name)
            .build();

        for (nick, _) in ADVANCED_OPTIONS {
            profile.connect_notify(Some(nick), |obj, _| {
                obj.notify_adv_modified();
            });
        }

        profile
    }

    //---------------------------------------
    // From json function
    //---------------------------------------
    pub fn from_json(name: &str, json_value: &JsonValue) -> Option<Self> {
        let obj = Self::new(name);

        let advanced_map = IndexMap::from(ADVANCED_OPTIONS);

        let json_map = json_value.as_object()?;

        for (key, value) in json_map {
            if obj.has_property(key) {
                match value {
                    JsonValue::Array(v) => {
                        let vec: Vec<String> = v.iter()
                            .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                            .collect();

                        obj.set_property(key, vec);
                    }
                    JsonValue::String(s) if key == "check-mode" => {
                        let mode = CheckMode::from_str(s)
                            .unwrap_or_default();

                        obj.set_property(key, mode);
                    },
                    JsonValue::String(s) if key == "recurse-mode" => {
                        let mode = RecurseMode::from_str(s)
                            .unwrap_or_default();

                        obj.set_property(key, mode);
                    },
                    JsonValue::String(s) => {
                        obj.set_property(key, s);
                    },
                    JsonValue::Bool(b) if advanced_map.contains_key(key.as_str()) => {
                        obj.set_property(key, b);
                    }
                    _ => {}
                }
            }
        }

        Some(obj)
    }

    //---------------------------------------
    // To json function
    //---------------------------------------
    pub fn to_json(&self) -> (String, JsonValue) {
        let mut json_map: JsonMap<String, JsonValue> = self.list_properties()
            .iter()
            .filter(|&prop| !["name", "adv-modified"].contains(&prop.nick()))
            .map(|prop| {
                let value = self.property_value(prop.nick());

                let json_value = if let Ok(v) = value.get::<Vec<String>>() {
                    json!(v)
                } else if let Ok(mode) = value.get::<CheckMode>() {
                    json!(mode.as_ref())
                } else if let Ok(mode) = value.get::<RecurseMode>() {
                    json!(mode.as_ref())
                } else if let Ok(s) = value.get::<String>() {
                    json!(s)
                } else if let Ok(b) = value.get::<bool>() {
                    json!(b)
                } else {
                    json!(null)
                };

                (prop.name().to_owned(), json_value)
            })
            .collect();

        json_map.sort_keys();

        (self.name(), JsonValue::Object(json_map))
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

            if !["name", "adv-modified"].contains(&nick) {
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

            if !["name", "adv-modified"].contains(&nick) {
                self.set_property_from_value(nick, property.default_value());
            }
        }
    }

    //---------------------------------------
    // Reset advanced function
    //---------------------------------------
    pub fn reset_advanced(&self) {
        let advanced_map = IndexMap::from(ADVANCED_OPTIONS);

        for property in self.list_properties() {
            let nick = property.nick();

            if advanced_map.contains_key(nick) {
                self.set_property_from_value(nick, property.default_value());
            }
        }
    }

    //---------------------------------------
    // Options function
    //---------------------------------------
    pub fn options(&self, quoted: bool) -> Vec<String> {
        // Check mode
        let mut options: Vec<String> = self.check_mode().switch()
            .map_or_else(Vec::new, |mode| vec![mode.to_owned()]);

        // Recurse mode
        if let Some(mode) = self.recurse_mode().switches() {
            options.extend(
                mode.split(' ').map(ToOwned::to_owned)
            );
        }

        // Advanced options
        options.extend(
            IndexMap::from(ADVANCED_OPTIONS).iter()
                .filter_map(|(&nick, &arg)| {
                    let value = self.property_value(nick)
                        .get::<bool>()
                        .ok()?;

                    value.then_some(arg).map(ToOwned::to_owned)
                })
        );

        // Filters
        let quote_char = if quoted { "'" } else { "" };

        options.extend(
            self.filters().into_iter()
                .filter_map(|filter| {
                    filter
                        .split_once(' ')
                        .and_then(|(s, pattern)| {
                            FilterRule::from_str(s).ok()
                                .and_then(|rule| rule.get_str("Rule"))
                                .map(|rule| {
                                    format!("-f{quote_char}{rule} {pattern}{quote_char}")
                                })
                        })
                })
        );

        options
    }
}

impl Default for ProfileObject {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
