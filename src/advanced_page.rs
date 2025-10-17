use std::cell::RefCell;

use gtk::glib;
use adw::subclass::prelude::*;
use adw::prelude::*;

use crate::profile_object::ProfileObject;

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
    #[template(resource = "/com/github/RsyncUI/ui/advanced_page.ui")]
    pub struct AdvancedPage {
        #[template_child]
        pub(super) recursive_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) preserve_time_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) preserve_permissions_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) preserve_owner_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) preserve_group_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) numeric_ids_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) preserve_symlinks_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) preserve_hardlinks_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) preserve_devices_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) one_filesystem_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) delete_destination_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) existing_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) ignore_existing_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) skip_newer_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) compress_data_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) backup_switch: TemplateChild<adw::SwitchRow>,

        #[property(get, set, nullable)]
        profile: RefCell<Option<ProfileObject>>,

        pub(super) bindings: RefCell<Option<Vec<glib::Binding>>>
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
    // Bind widget helper function
    //---------------------------------------
    fn bind_widget(&self, profile: &ProfileObject, source: &str, widget: &impl IsA<gtk::Widget>, target: &str) -> glib::Binding{
        profile.bind_property(source, widget, target)
            .bidirectional()
            .sync_create()
            .build()
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        // Profile property notify signal
        self.connect_profile_notify(|page| {
            let imp = page.imp();

            if let Some(bindings) = imp.bindings.take() {
                for binding in bindings {
                    binding.unbind();
                }
            }

            if let Some(profile) = page.profile() {
                let bindings: Vec<glib::Binding> = vec![
                    // Bind profile property to widgets
                    page.bind_widget(&profile, "recursive", &imp.recursive_switch.get(), "active"),
                    page.bind_widget(&profile, "preserve-time", &imp.preserve_time_switch.get(), "active"),
                    page.bind_widget(&profile, "preserve-permissions", &imp.preserve_permissions_switch.get(), "active"),
                    page.bind_widget(&profile, "preserve-owner", &imp.preserve_owner_switch.get(), "active"),
                    page.bind_widget(&profile, "preserve-group", &imp.preserve_group_switch.get(), "active"),
                    page.bind_widget(&profile, "numeric-ids", &imp.numeric_ids_switch.get(), "active"),
                    page.bind_widget(&profile, "preserve-symlinks", &imp.preserve_symlinks_switch.get(), "active"),
                    page.bind_widget(&profile, "preserve-hardlinks", &imp.preserve_hardlinks_switch.get(), "active"),
                    page.bind_widget(&profile, "preserve-devices", &imp.preserve_devices_switch.get(), "active"),
                    page.bind_widget(&profile, "no-leave-filesystem", &imp.one_filesystem_switch.get(), "active"),
                    page.bind_widget(&profile, "delete-destination", &imp.delete_destination_switch.get(), "active"),
                    page.bind_widget(&profile, "existing", &imp.existing_switch.get(), "active"),
                    page.bind_widget(&profile, "ignore-existing", &imp.ignore_existing_switch.get(), "active"),
                    page.bind_widget(&profile, "skip-newer", &imp.skip_newer_switch.get(), "active"),
                    page.bind_widget(&profile, "compress-data", &imp.compress_data_switch.get(), "active"),
                    page.bind_widget(&profile, "backup", &imp.backup_switch.get(), "active")
                ];

                // Store bindings
                imp.bindings.replace(Some(bindings));

                // Set page title
                page.set_title(&profile.name());
            }
        });
    }

    //---------------------------------------
    // Public args function
    //---------------------------------------
    pub fn args(&self) -> Vec<&str> {
        let imp = self.imp();

        let mut args: Vec<&str> = vec![];

        if imp.recursive_switch.is_active() {
            args.push("-r");
        } else {
            args.push("-d");
        }

        if imp.preserve_time_switch.is_active() {
            args.push("-t");
        }

        if imp.preserve_permissions_switch.is_active() {
            args.push("-p");
        }

        if imp.preserve_owner_switch.is_active() {
            args.push("-o");
        }

        if imp.preserve_group_switch.is_active() {
            args.push("-g");
        }

        if imp.numeric_ids_switch.is_active() {
            args.push("--numeric-ids");
        }

        if imp.preserve_symlinks_switch.is_active() {
            args.push("-l");
        }

        if imp.preserve_hardlinks_switch.is_active() {
            args.push("-H");
        }

        if imp.preserve_devices_switch.is_active() {
            args.push("-D");
        }

        if imp.one_filesystem_switch.is_active() {
            args.push("-x");
        }

        if imp.delete_destination_switch.is_active() {
            args.push("--delete");
        }

        if imp.existing_switch.is_active() {
            args.push("--existing");
        }

        if imp.ignore_existing_switch.is_active() {
            args.push("---ignore-existing");
        }

        if imp.skip_newer_switch.is_active() {
            args.push("-u");
        }

        if imp.compress_data_switch.is_active() {
            args.push("-x");
        }

        if imp.backup_switch.is_active() {
            args.push("-b");
        }

        args
    }
}
