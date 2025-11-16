use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::prelude::*;
use gtk::glib;

use crate::{
    profile_object::ProfileObject,
    adv_switchrow::AdvSwitchRow
};

//------------------------------------------------------------------------------
// MODULE: AdvancedPage
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::AdvancedPage)]
    #[template(resource = "/com/github/Syncer/ui/advanced_page.ui")]
    pub struct AdvancedPage {
        #[template_child]
        pub(super) switches_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) recursive_switch: TemplateChild<AdvSwitchRow>,
        #[template_child]
        pub(super) incremental_switch: TemplateChild<AdvSwitchRow>,

        #[property(get, set, nullable)]
        profile: RefCell<Option<ProfileObject>>,

        pub(super) bindings: RefCell<Option<Vec<glib::Binding>>>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for AdvancedPage {
        const NAME: &'static str = "AdvancedPage";
        type Type = super::AdvancedPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            AdvSwitchRow::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AdvancedPage {
        //---------------------------------------
        // Constructor
        //---------------------------------------
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.setup_signals();
            obj.setup_widgets();
        }
    }

    impl WidgetImpl for AdvancedPage {}
    impl NavigationPageImpl for AdvancedPage {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: AdvancedPage
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct AdvancedPage(ObjectSubclass<imp::AdvancedPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl AdvancedPage {
    //---------------------------------------
    // Switches helper function
    //---------------------------------------
    fn switches(&self) -> Vec<AdvSwitchRow> {
        let imp = self.imp();

        let mut switches = vec![];

        let mut child = imp.switches_box.first_child();

        while let Some(group) = child.and_downcast_ref::<adw::PreferencesGroup>() {
            let mut i = 0;

            while let Some(switch) = group.row(i).and_downcast_ref::<AdvSwitchRow>() {
                switches.push(switch.clone());

                i += 1;
            }

            child = group.next_sibling();
        }

        switches
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        // Profile property notify signal
        self.connect_profile_notify(|page| {
            let imp = page.imp();

            // Unbind stored bindings
            if let Some(bindings) = imp.bindings.take() {
                for binding in bindings {
                    binding.unbind();
                }
            }

            if let Some(profile) = page.profile() {
                // Bind profile property to widgets
                let mut bindings: Vec<glib::Binding> = page.switches().iter()
                    .map(|switch| {
                        profile.bind_property(&switch.prop_name(), switch, "active")
                            .bidirectional()
                            .sync_create()
                            .build()
                    })
                    .collect();

                // Bind profile property to page title
                bindings.push(
                    profile.bind_property("name", page, "title")
                        .sync_create()
                        .build()
                );

                // Store bindings
                imp.bindings.replace(Some(bindings));
            }
        });
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Bind recursive switch state to incremental recursion switch sensitive state
        imp.recursive_switch.bind_property("active", &imp.incremental_switch.get(), "sensitive")
            .sync_create()
            .build();
    }
}
