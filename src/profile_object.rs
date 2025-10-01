use std::cell::{Cell, RefCell};

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::prelude::ObjectExt;

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
        #[property(get, set, construct_only)]
        name: RefCell<String>,
        #[property(get, set, construct_only)]
        is_default: Cell<bool>,

        #[property(get, set)]
        source: RefCell<String>,
        #[property(get, set)]
        destination: RefCell<String>,

        #[property(get, set, default = true, construct)]
        preserve_time: Cell<bool>,
        #[property(get, set, default = false, construct)]
        preserve_permissions: Cell<bool>,
        #[property(get, set, default = false, construct)]
        preserve_owner: Cell<bool>,
        #[property(get, set, default = false, construct)]
        preserve_group: Cell<bool>,
        #[property(get, set, default = false, construct)]
        delete_destination: Cell<bool>,
        #[property(get, set, default = false, construct)]
        no_leave_filesystem: Cell<bool>,
        #[property(get, set, default = false, construct)]
        ignore_existing: Cell<bool>,
        #[property(get, set, default = false, construct)]
        skip_newer: Cell<bool>,
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
            .property("is-default", false)
            .build()
    }
}

impl Default for ProfileObject {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder()
            .property("name", "Default")
            .property("is-default", true)
            .build()
    }
}
