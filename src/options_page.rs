use std::cell::RefCell;

use gtk::{glib, gio};
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::clone;
use strum::EnumProperty;

use crate::profile_object::{CheckMode, ProfileObject};

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
        pub(super) copy_by_name_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) swap_paths_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) source_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) destination_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) check_mode_combo: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) extra_options_row: TemplateChild<adw::EntryRow>,

        #[property(get)]
        #[template_child]
        pub(super) sidebar_button: TemplateChild<gtk::ToggleButton>,

        #[property(get, set, nullable)]
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
    fn select_folder(&self, row: &adw::ActionRow, add_trailing: bool) {
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
                    let mut subtitle = path.display().to_string();

                    if add_trailing {
                        subtitle.push('/');
                    }

                    row.set_subtitle(&subtitle);
                }
            }
        ));
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

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
                    profile.bind_property("source-copy-by-name", &imp.copy_by_name_button.get(), "active")
                        .bidirectional()
                        .sync_create()
                        .build(),

                    profile.bind_property("source", &imp.source_row.get(), "subtitle")
                        .bidirectional()
                        .sync_create()
                        .build(),

                    profile.bind_property("destination", &imp.destination_row.get(), "subtitle")
                        .bidirectional()
                        .sync_create()
                        .build(),

                    profile.bind_property("check-mode", &imp.check_mode_combo.get(), "selected")
                        .transform_to(|_, mode: CheckMode| Some(mode.value()))
                        .transform_from(|_, index: u32| CheckMode::from_repr(index))
                        .bidirectional()
                        .sync_create()
                        .build(),

                    profile.bind_property("extra-options", &imp.extra_options_row.get(), "text")
                        .bidirectional()
                        .sync_create()
                        .build(),
                ];

                // Store bindings
                imp.bindings.replace(Some(bindings));
            }
        });

        // Swap paths button clicked signal
        imp.swap_paths_button.connect_clicked(clone!(
            #[weak] imp,
            move |_| {
                let source = imp.source_row.subtitle().unwrap_or_default();
                let destination = imp.destination_row.subtitle().unwrap_or_default();

                if imp.copy_by_name_button.is_active() {
                    imp.source_row.set_subtitle(destination.trim_end_matches('/'));
                } else if !destination.is_empty() && !destination.ends_with('/') {
                    imp.source_row.set_subtitle(&format!("{destination}/"));
                } else {
                    imp.source_row.set_subtitle(&destination);
                }

                imp.destination_row.set_subtitle(source.trim_end_matches('/'));
            }
        ));

        // Copy by name button toggled signal
        imp.copy_by_name_button.connect_toggled(clone!(
            #[weak] imp,
            move |button| {
                let source = imp.source_row.subtitle().unwrap_or_default();

                if !source.is_empty() {
                    if button.is_active() {
                        imp.source_row.set_subtitle(source.trim_end_matches('/'));
                    } else if !source.ends_with('/') {
                        imp.source_row.set_subtitle(&format!("{source}/"));
                    }
                }
            }
        ));

        // Source row activated signal
        imp.source_row.connect_activated(clone!(
            #[weak(rename_to = page)] self,
            move |row| {
                let add_trailing = !page.imp().copy_by_name_button.is_active();

                page.select_folder(row, add_trailing);
            }
        ));

        // Destination row activated signal
        imp.destination_row.connect_activated(clone!(
            #[weak(rename_to = page)] self,
            move |row| {
                page.select_folder(row, false);
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
                let mode = obj
                    .and_downcast::<adw::EnumListItem>()
                    .and_then(|item| CheckMode::from_repr(item.value() as u32))?;

                mode.get_str("Desc")
            })
            .sync_create()
            .build();
    }
}
