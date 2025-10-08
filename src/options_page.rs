use std::cell::RefCell;

use gtk::{glib, gio};
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::clone;

use crate::profile_object::ProfileObject;
use crate::check_object::CheckObject;
use crate::rsync_pane::RsyncPane;

//------------------------------------------------------------------------------
// MODULE: OptionsPage
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::OptionsPage)]
    #[template(resource = "/com/github/RsyncUI/ui/options_page.ui")]
    pub struct OptionsPage {
        #[template_child]
        pub(super) source_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) destination_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) check_mode_combo: TemplateChild<adw::ComboRow>,

        #[property(get)]
        #[template_child]
        pub(super) content_box: TemplateChild<gtk::Box>,
        #[property(get)]
        #[template_child]
        pub(super) rsync_pane: TemplateChild<RsyncPane>,

        #[property(get, set)]
        profile: RefCell<Option<ProfileObject>>,

        pub(super) bindings: RefCell<Option<Vec<glib::Binding>>>
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for OptionsPage {
        const NAME: &'static str = "OptionsPage";
        type Type = super::OptionsPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            CheckObject::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for OptionsPage {
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

    impl WidgetImpl for OptionsPage {}
    impl NavigationPageImpl for OptionsPage {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: OptionsPage
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct OptionsPage(ObjectSubclass<imp::OptionsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl OptionsPage {
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
                let bindings: Vec<glib::Binding> = vec![
                    // Bind profile property to widgets
                    pane.bind_widget(&profile, "source", &imp.source_row.get(), "subtitle"),
                    pane.bind_widget(&profile, "destination", &imp.destination_row.get(), "subtitle"),
                    pane.bind_widget(&profile, "check-mode", &imp.check_mode_combo.get(), "selected"),
                ];

                // Store bindings
                imp.bindings.replace(Some(bindings));

                // Set page title
                pane.set_title(&profile.name());
            }
        });

        // Source row activated signal
        imp.source_row.connect_activated(clone!(
            #[weak(rename_to = page)] self,
            move |row| {
                page.select_folder(row);
            }
        ));

        // Destination row activated signal
        imp.destination_row.connect_activated(clone!(
            #[weak(rename_to = page)] self,
            move |row| {
                page.select_folder(row);
            }
        ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Bind check mode combo selected item to subtitle
        imp.check_mode_combo.bind_property("selected-item", &imp.check_mode_combo.get(), "subtitle")
            .transform_to(|_, obj: Option<glib::Object>| {
                obj
                    .and_downcast::<CheckObject>()
                    .map(|obj| obj.subtitle())
            })
            .sync_create()
            .build();
    }

    //---------------------------------------
    // Public args function
    //---------------------------------------
    pub fn args(&self) -> Option<Vec<String>> {
        let imp = self.imp();

        let source = imp.source_row.subtitle().filter(|s| !s.is_empty());
        let destination = imp.destination_row.subtitle().filter(|s| !s.is_empty());

        source.zip(destination)
            .map(|(source, destination)| {
                let mut args: Vec<String> = [source, destination]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();

                if let Some(check_mode) = imp.check_mode_combo.selected_item()
                    .and_downcast::<CheckObject>()
                    .and_then(|obj| obj.switch())
                {
                    args.insert(0, check_mode);
                }

                args
            })
    }
}
