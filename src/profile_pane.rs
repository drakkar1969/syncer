use std::cell::RefCell;

use gtk::{glib, gio};
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::clone;

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
        #[template_child]
        pub(super) source_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) destination_row: TemplateChild<adw::ActionRow>,

        #[template_child]
        pub(super) preserve_time_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) preserve_permissions_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) preserve_owner_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) preserve_group_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) delete_destination_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) no_leave_filesystem_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) ignore_existing_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) skip_newer_switch: TemplateChild<adw::SwitchRow>,

        #[property(get, set)]
        profile: RefCell<Option<ProfileObject>>,

        pub(super) bindings: RefCell<Option<Vec<glib::Binding>>>
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for ProfilePane {
        const NAME: &'static str = "ProfilePane";
        type Type = super::ProfilePane;
        type ParentType = adw::NavigationPage;

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

            obj.setup_signals();
        }
    }

    impl WidgetImpl for ProfilePane {}
    impl NavigationPageImpl for ProfilePane {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: ProfilePane
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct ProfilePane(ObjectSubclass<imp::ProfilePane>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ProfilePane {
    //---------------------------------------
    // Select folder helper function
    //---------------------------------------
    fn select_folder(&self, row: &adw::ActionRow) {
        let dialog = gtk::FileDialog::builder()
            .title(format!("Select {}", row.title().replace('_', "")))
            .modal(true)
            .build();

        dialog.set_initial_folder(
            row.subtitle()
                .filter(|subtitle| !subtitle.is_empty())
                .map(gio::File::for_path)
                .as_ref()
        );

        let root = row.root()
            .and_downcast::<gtk::Window>();

        dialog.select_folder(root.as_ref(), None::<&gio::Cancellable>, clone!(
            #[weak] row,
            move |result| {
                if let Some(path) = result.ok().and_then(|file| file.path()) {
                    row.set_subtitle(&path.display().to_string());
                }
            }
        ));
    }

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
        let imp = self.imp();

        // Profile property notify signal
        self.connect_profile_notify(|pane| {
            let imp = pane.imp();

            if let Some(bindings) = imp.bindings.take() {
                for binding in bindings {
                    binding.unbind();
                }
            }

            if let Some(profile) = pane.profile() {
                let mut bindings: Vec<glib::Binding> = vec![];

                // Bind profile property to pane title
                bindings.push(profile.bind_property("name", pane, "title")
                    .sync_create()
                    .build());

                // Bind profile property to widgets
                bindings.push(pane.bind_widget(&profile, "preserve-time", &imp.preserve_time_switch.get(), "active"));
                bindings.push(pane.bind_widget(&profile, "preserve-permissions", &imp.preserve_permissions_switch.get(), "active"));
                bindings.push(pane.bind_widget(&profile, "preserve-owner", &imp.preserve_owner_switch.get(), "active"));
                bindings.push(pane.bind_widget(&profile, "preserve-group", &imp.preserve_group_switch.get(), "active"));
                bindings.push(pane.bind_widget(&profile, "delete-destination", &imp.delete_destination_switch.get(), "active"));
                bindings.push(pane.bind_widget(&profile, "no-leave-filesystem", &imp.no_leave_filesystem_switch.get(), "active"));
                bindings.push(pane.bind_widget(&profile, "ignore-existing", &imp.ignore_existing_switch.get(), "active"));
                bindings.push(pane.bind_widget(&profile, "skip-newer", &imp.skip_newer_switch.get(), "active"));

                // Store bindings
                imp.bindings.replace(Some(bindings));
            }
        });

        // Source row activated signal
        imp.source_row.connect_activated(clone!(
            #[weak(rename_to = pane)] self,
            move |row| {
                pane.select_folder(row);
            }
        ));

        // Destination row activated signal
        imp.destination_row.connect_activated(clone!(
            #[weak(rename_to = pane)] self,
            move |row| {
                pane.select_folder(row);
            }
        ));
    }
}
