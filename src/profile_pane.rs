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
            obj.setup_signals();
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
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Bind profile property to widgets
        let profile = self.profile();

        profile.bind_property("preserve-time", &imp.preserve_time_switch.get(), "active")
            .bidirectional()
            .sync_create()
            .build();
    }

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
                .map(|subtitle| gio::File::for_path(subtitle))
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
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

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
