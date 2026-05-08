use std::cell::RefCell;

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;
use glib::clone;

//------------------------------------------------------------------------------
// MODULE: ProfileDialog
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::ProfileDialog)]
    #[template(resource = "/com/github/Syncer/ui/profile_dialog.ui")]
    pub struct ProfileDialog {
        #[template_child]
        pub(super) profile_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) error_label: TemplateChild<gtk::Label>,

        #[property(get, set)]
        profile_name: RefCell<String>,

        pub(super) profile_list: RefCell<Vec<String>>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for ProfileDialog {
        const NAME: &'static str = "ProfileDialog";
        type Type = super::ProfileDialog;
        type ParentType = adw::AlertDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProfileDialog {
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

    impl WidgetImpl for ProfileDialog {}
    impl AdwDialogImpl for ProfileDialog {}
    impl AdwAlertDialogImpl for ProfileDialog {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: ProfileDialog
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct ProfileDialog(ObjectSubclass<imp::ProfileDialog>)
        @extends adw::AlertDialog, adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl ProfileDialog {
    //---------------------------------------
    // New function
    //---------------------------------------
    pub fn new(action: &str, name: Option<&str>, profile_list: Vec<String>) -> Self {
        let profile_name = name
            .map_or_else(String::new, |name| format!("{name}-1"));

        let dialog: Self = glib::Object::builder()
            .property("heading", format!("{action} Profile"))
            .property("profile-name", profile_name)
            .build();

        dialog.set_response_label("add", action);

        dialog.imp().profile_list.replace(profile_list);

        dialog
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        self.imp().profile_entry.connect_changed(clone!(
            #[weak(rename_to = dialog)] self,
            move |entry| {
                let imp = dialog.imp();

                let profile_name = entry.text();

                let profile_list = imp.profile_list.borrow();

                let existing_profile = profile_list
                    .iter()
                    .find(|&name| name == &profile_name);

                if let Some(profile) = existing_profile {
                    imp.error_label
                        .set_label(&format!("Profile “{profile}” already exists"));
                    imp.error_label.set_visible(true);
                } else {
                    imp.error_label.set_visible(false);
                }

                dialog.set_response_enabled("add",
                    existing_profile.is_none() && !profile_name.is_empty()
                );
            }
        ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Bind properties to widget
        self.bind_property("profile-name", &imp.profile_entry.get(), "text")
            .sync_create()
            .bidirectional()
            .build();
    }
}
