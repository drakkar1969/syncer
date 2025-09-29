use std::cell::RefCell;

use gtk::glib;
use adw::subclass::prelude::*;
use gtk::prelude::*;

use crate::profile_object::ProfileObject;

//------------------------------------------------------------------------------
// MODULE: ProfilePane
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::ProfilePane)]
    #[template(resource = "/com/github/RsyncUI/ui/profile_pane.ui")]
    pub struct ProfilePane {
        #[property(get, set)]
        profile: RefCell<ProfileObject>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for ProfilePane {
        const NAME: &'static str = "ProfilePane";
        type Type = super::ProfilePane;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProfilePane {
        //---------------------------------------
        // Constructor
        //---------------------------------------
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.setup_widgets();
        }
    }

    impl WidgetImpl for ProfilePane {}
    impl BinImpl for ProfilePane {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: ProfilePane
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct ProfilePane(ObjectSubclass<imp::ProfilePane>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ProfilePane {
    //---------------------------------------
    // Constructor
    //---------------------------------------
    fn setup_widgets(&self) {

    }
}
