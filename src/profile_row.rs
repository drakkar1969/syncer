use std::cell::RefCell;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::prelude::*;

//------------------------------------------------------------------------------
// MODULE: ProfileRow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::ProfileRow)]
    #[template(resource = "/com/github/RsyncUI/ui/profile_row.ui")]
    pub struct ProfileRow {
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,

        #[property(get, set)]
        name: RefCell<String>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for ProfileRow {
        const NAME: &'static str = "ProfileRow";
        type Type = super::ProfileRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProfileRow {}

    impl WidgetImpl for ProfileRow {}
    impl ListBoxRowImpl for ProfileRow {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: ProfileRow
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct ProfileRow(ObjectSubclass<imp::ProfileRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl ProfileRow {
    //---------------------------------------
    // New function
    //---------------------------------------
    pub fn new(name: &str) -> Self {
        let obj: Self = glib::Object::builder()
            .property("name", name)
            .build();

        obj.imp().label.set_label(name);

        obj
    }
}
