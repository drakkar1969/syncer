use std::cell::RefCell;

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;
use glib::clone;

use crate::{
    profile_object::ProfileObject,
    advanced_switch::AdvancedSwitch
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
        pub(super) reset_button: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub(super) switches_box: TemplateChild<gtk::Box>,

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
            AdvancedSwitch::ensure_type();

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

            obj.setup_defaults();
            obj.setup_signals();
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
    fn switches(&self) -> Vec<AdvancedSwitch> {
        let imp = self.imp();

        let mut switches = vec![];

        let mut child = imp.switches_box.first_child();

        while let Some(group) = child.and_downcast_ref::<adw::PreferencesGroup>() {
            let mut i = 0;

            while let Some(switch) = group.row(i).and_downcast_ref::<AdvancedSwitch>() {
                switches.push(switch.clone());

                i += 1;
            }

            child = group.next_sibling();
        }

        switches
    }

    //---------------------------------------
    // Setup defaults
    //---------------------------------------
    fn setup_defaults(&self) {
        let profile = ProfileObject::default();

        // Set default property for switches
        for switch in self.switches() {
            let default = profile.find_property(&switch.nick())
                .and_then(|prop| prop.default_value().get::<bool>().ok())
                .expect("Could not get default value for property");

            switch.set_default(default);
        }
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

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
                // Bind profile property to switches
                let mut bindings: Vec<glib::Binding> = page.switches().iter()
                    .map(|switch| {
                        profile.bind_property(&switch.nick(), switch, "active")
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

        // Reset button activated signal
        imp.reset_button.connect_activated(clone!(
            #[weak(rename_to = page)] self,
            move |_| {
                let dialog = adw::AlertDialog::builder()
                    .heading("Reset Advanced Options?")
                    .body("Reset all options to their default values.")
                    .default_response("reset")
                    .build();

                dialog.add_responses(&[("cancel", "_Cancel"), ("reset", "_Reset")]);
                dialog.set_response_appearance("reset", adw::ResponseAppearance::Destructive);

                dialog.connect_response(Some("reset"), clone!(
                    #[weak] page,
                    move |_, _| {
                        if let Some(profile) = page.profile() {
                            profile.reset_advanced();
                        }
                    })
                );

                dialog.present(Some(&page));
            }
        ));
    }
}
