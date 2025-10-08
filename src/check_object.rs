use std::cell::RefCell;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::prelude::ObjectExt;

//------------------------------------------------------------------------------
// MODULE: CheckObject
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::CheckObject)]
    pub struct CheckObject {
        #[property(get, set)]
        title: RefCell<String>,
        #[property(get, set)]
        subtitle: RefCell<String>,
        #[property(get, set)]
        switch: RefCell<Option<String>>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for CheckObject {
        const NAME: &'static str = "CheckObject";
        type Type = super::CheckObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for CheckObject {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: CheckObject
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct CheckObject(ObjectSubclass<imp::CheckObject>);
}

impl Default for CheckObject {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
