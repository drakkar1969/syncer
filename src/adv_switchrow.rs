use std::cell::{Cell, RefCell};

use gtk::glib;
use adw::subclass::prelude::*;
use adw::prelude::*;

//------------------------------------------------------------------------------
// MODULE: AdvSwitchRow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::AdvSwitchRow)]
    #[template(resource = "/com/github/RsyncUI/ui/adv_switchrow.ui")]
    pub struct AdvSwitchRow {
        #[template_child]
        pub(super) switch: TemplateChild<gtk::Switch>,

        #[property(get, set)]
        active: Cell<bool>,

        #[property(get, set)]
        prop_name: RefCell<String>,
        #[property(get, set)]
        param: RefCell<String>,
        #[property(get, set, nullable)]
        alt_param: RefCell<Option<String>>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for AdvSwitchRow {
        const NAME: &'static str = "AdvSwitchRow";
        type Type = super::AdvSwitchRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AdvSwitchRow {
        //---------------------------------------
        // Constructor
        //---------------------------------------
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.setup_widgets();
        }
    }

    impl WidgetImpl for AdvSwitchRow {}
    impl ListBoxRowImpl for AdvSwitchRow {}
    impl PreferencesRowImpl for AdvSwitchRow {}
    impl ActionRowImpl for AdvSwitchRow {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: AdvSwitchRow
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct AdvSwitchRow(ObjectSubclass<imp::AdvSwitchRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl AdvSwitchRow {
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

impl Default for AdvSwitchRow {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
