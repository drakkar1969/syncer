use std::cell::{Cell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

//------------------------------------------------------------------------------
// MODULE: AdvancedSwitch
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::AdvancedSwitch)]
    #[template(resource = "/com/github/Syncer/ui/advanced_switch.ui")]
    pub struct AdvancedSwitch {
        #[template_child]
        pub(super) switch: TemplateChild<gtk::Switch>,

        #[property(get, set)]
        active: Cell<bool>,

        #[property(get, set)]
        nick: RefCell<String>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for AdvancedSwitch {
        const NAME: &'static str = "AdvancedSwitch";
        type Type = super::AdvancedSwitch;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AdvancedSwitch {
        //---------------------------------------
        // Constructor
        //---------------------------------------
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.setup_widgets();
        }
    }

    impl WidgetImpl for AdvancedSwitch {}
    impl ListBoxRowImpl for AdvancedSwitch {}
    impl PreferencesRowImpl for AdvancedSwitch {}
    impl ActionRowImpl for AdvancedSwitch {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: AdvancedSwitch
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct AdvancedSwitch(ObjectSubclass<imp::AdvancedSwitch>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl AdvancedSwitch {
    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        self.bind_property("active", &imp.switch.get(), "active")
            .bidirectional()
            .sync_create()
            .build();
    }
}
