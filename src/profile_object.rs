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
        #[property(get, set)]
        name: RefCell<String>,

        #[property(get, set)]
        source: RefCell<String>,
        #[property(get, set)]
        destination: RefCell<String>,

        #[property(get, set, default = 0, construct)]
        check_mode: Cell<u32>,
        #[property(get, set, default = true, construct)]
        recursive: Cell<bool>,
        #[property(get, set, default = true, construct)]
        preserve_time: Cell<bool>,
        #[property(get, set, default = false, construct)]
        preserve_permissions: Cell<bool>,
        #[property(get, set, default = false, construct)]
        preserve_owner: Cell<bool>,
        #[property(get, set, default = false, construct)]
        preserve_group: Cell<bool>,
        #[property(get, set, default = false, construct)]
        numeric_ids: Cell<bool>,
        #[property(get, set, default = false, construct)]
        preserve_symlinks: Cell<bool>,
        #[property(get, set, default = false, construct)]
        preserve_hardlinks: Cell<bool>,
        #[property(get, set, default = false, construct)]
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
        compress_data: Cell<bool>,
        #[property(get, set, default = false, construct)]
        backup: Cell<bool>,
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
}
