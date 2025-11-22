use std::cell::RefCell;
use std::io;
use std::fs;

use adw::subclass::prelude::*;
use adw::prelude::*;
use gtk::{gio, glib, gdk};
use glib::clone;

use serde_json::{to_string_pretty, from_str, Map as JsonMap, Value as JsonValue};

use crate::profile_object::{CheckMode, RecurseMode, ProfileObject};

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
    #[template(resource = "/com/github/Syncer/ui/options_page.ui")]
    pub struct OptionsPage {
        #[property(get)]
        #[template_child]
        pub(super) profile_dropdown: TemplateChild<gtk::DropDown>,
        #[property(get)]
        #[template_child]
        pub(super) profile_model: TemplateChild<gio::ListStore>,

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
        pub(super) recurse_mode_combo: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) extra_options_row: TemplateChild<adw::EntryRow>,

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

            Self::install_profile_actions(klass);

            Self::bind_shortcuts(klass);
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

    impl OptionsPage {
        //---------------------------------------
        // Install profile actions
        //---------------------------------------
        fn install_profile_actions(klass: &mut <Self as ObjectSubclass>::Class) {
            // New profile action
            klass.install_action("profile.new", None, |page, _, _| {
                let imp = page.imp();

                page.profile_dialog("New", None, clone!(
                    #[weak] imp,
                    move |name| {
                        imp.profile_model.append(&ProfileObject::new(name));

                        imp.profile_dropdown.set_selected(imp.profile_model.n_items() - 1);
                    }
                ));
            });

            // Rename profile action
            klass.install_action("profile.rename", None, |page, _, _| {
                if let Some(profile) = page.profile() {
                    page.profile_dialog("Rename", Some(&profile.name()), move |new_name| {
                        profile.set_name(new_name);
                    });
                }
            });

            // Delete profile action
            klass.install_action("profile.delete", None, |page, _, _| {
                if let Some(profile) = page.profile() {
                    let imp = page.imp();

                    let dialog = adw::AlertDialog::builder()
                        .heading("Delete Profile?")
                        .body(format!("Permamenently delete the \"{}\" profile.",
                            profile.name()))
                        .default_response("delete")
                        .build();

                    dialog.add_responses(&[("cancel", "_Cancel"), ("delete", "_Delete")]);
                    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

                    dialog.connect_response(Some("delete"), clone!(
                        #[weak] imp,
                        move |_, _| {
                            if let Some(pos) = imp.profile_model.find(&profile) {
                                imp.profile_model.remove(pos);
                            }
                        })
                    );

                    dialog.present(Some(page));
                }
            });

            // Duplicate profile action
            klass.install_action("profile.duplicate", None, |page, _, _| {
                if let Some(profile) = page.profile() {
                    let imp = page.imp();

                    if let Some(pos) = imp.profile_model.find(&profile) {
                        page.profile_dialog("Duplicate", Some(&profile.name()), clone!(
                            #[weak] imp,
                            move |new_name| {
                                let duplicate = profile.duplicate(new_name);

                                imp.profile_model.insert(pos + 1, &duplicate);

                                imp.profile_dropdown.set_selected(pos + 1);
                            }
                        ));
                    }
                }
            });

            // Reset profile action
            klass.install_action("profile.reset", None, |page, _, _| {
                if let Some(profile) = page.profile() {
                    let dialog = adw::AlertDialog::builder()
                        .heading("Reset Profile?")
                        .body(format!("Reset the \"{}\" profile to default values.",
                            profile.name()))
                        .default_response("reset")
                        .build();

                    dialog.add_responses(&[("cancel", "_Cancel"), ("reset", "_Reset")]);
                    dialog.set_response_appearance("reset", adw::ResponseAppearance::Destructive);

                    dialog.connect_response(Some("reset"), move |_, _| {
                        profile.reset();
                    });

                    dialog.present(Some(page));
                }
            });

            // Delete all profiles action
            klass.install_action("profile.delete-all", None, |page, _, _| {
                let imp = page.imp();

                let dialog = adw::AlertDialog::builder()
                    .heading("Delete All Profiles?")
                    .body("Permamenently delete all profiles.")
                    .default_response("delete")
                    .build();

                dialog.add_responses(&[("cancel", "_Cancel"), ("delete", "_Delete")]);
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

                dialog.connect_response(Some("delete"), clone!(
                    #[weak] imp,
                    move |_, _| {
                        imp.profile_model.remove_all();
                    })
                );

                dialog.present(Some(page));
            });
        }

        //---------------------------------------
        // Bind shortcuts
        //---------------------------------------
        fn bind_shortcuts(klass: &mut <Self as ObjectSubclass>::Class) {
            // New profile key binding
            klass.add_binding_action(gdk::Key::N, gdk::ModifierType::CONTROL_MASK, "profile.new");
        }
    }
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
    fn select_folder(row: &adw::ActionRow, add_trailing: bool) {
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
                        .transform_from(|_, index: u32| {
                            Some(CheckMode::from_repr(index).unwrap_or_default())
                        })
                        .bidirectional()
                        .sync_create()
                        .build(),

                    profile.bind_property("recurse-mode", &imp.recurse_mode_combo.get(), "selected")
                        .transform_to(|_, mode: RecurseMode| Some(mode.value()))
                        .transform_from(|_, index: u32| {
                            Some(RecurseMode::from_repr(index).unwrap_or_default())
                        })
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
            #[weak] imp,
            move |row| {
                let add_trailing = !imp.copy_by_name_button.is_active();

                Self::select_folder(row, add_trailing);
            }
        ));

        // Destination row activated signal
        imp.destination_row.connect_activated(|row| {
            Self::select_folder(row, false);
        });
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

                mode.desc()
            })
            .sync_create()
            .build();

        // Bind recurse mode combo selected item to subtitle
        imp.recurse_mode_combo.bind_property("selected-item", &imp.recurse_mode_combo.get(), "subtitle")
            .transform_to(|_, obj: Option<glib::Object>| {
                let mode = obj
                    .and_downcast::<adw::EnumListItem>()
                    .and_then(|item| RecurseMode::from_repr(item.value() as u32))?;

                mode.desc()
            })
            .sync_create()
            .build();
    }

    //---------------------------------------
    // Profile dialog function
    //---------------------------------------
    fn profile_dialog<F>(&self, response: &str, default: Option<&str>, f: F)
    where F: Fn(&str) + 'static {
        let imp = self.imp();

        let builder = gtk::Builder::from_resource("/com/github/Syncer/ui/builder/profile_dialog.ui");

        let dialog: adw::AlertDialog = builder.object("dialog")
            .expect("Could not get object from resource");

        dialog.set_heading(Some(&format!("{response} Profile")));
        dialog.set_response_label("add", response);

        let profile_entry: adw::EntryRow = builder.object("profile_entry")
            .expect("Could not get object from resource");

        let error_label: gtk::Label = builder.object("error_label")
            .expect("Could not get object from resource");

        profile_entry.connect_changed(clone!(
            #[weak] imp,
            #[weak] dialog,
            move |entry| {
                let profile_name = entry.text();

                let existing_profile = imp.profile_model.iter::<ProfileObject>()
                    .flatten()
                    .map(|profile| profile.name())
                    .find(|name| name.eq_ignore_ascii_case(&profile_name))
                    .unwrap_or_default();

                if existing_profile.is_empty() {
                    error_label.set_label("");
                } else {
                    error_label.set_label(&format!("Profile {existing_profile} already exists"));
                }

                dialog.set_response_enabled("add",
                    existing_profile.is_empty() && !profile_name.is_empty()
                );
            }
        ));

        if let Some(text) = default {
            profile_entry.set_text(&format!("{text}-1"));
        }

        dialog.connect_response(Some("add"), move |_, _| {
            f(&profile_entry.text());
        });

        dialog.present(Some(self));
    }

    //---------------------------------------
    // Load config function
    //---------------------------------------
    pub fn load_config(&self) -> io::Result<()> {
        let imp = self.imp();

        // Load profiles from config file
        let config_path = xdg::BaseDirectories::new()
            .find_config_file("Syncer/config.json")
            .ok_or_else(|| io::Error::other("Config file not found"))?;

        let json_str = fs::read_to_string(config_path)?;

        let json_object: JsonMap<String, JsonValue> = from_str(&json_str)?;

        let profiles: Vec<ProfileObject> = json_object.iter()
            .filter_map(|(name, value)| ProfileObject::from_json(name, value))
            .collect();

        // Add profiles to model
        imp.profile_model.splice(0, 0, &profiles);

        Ok(())
    }

    //---------------------------------------
    // Save config function
    //---------------------------------------
    pub fn save_config(&self) -> io::Result<()> {
        let json_object: JsonMap<String, JsonValue> = self.imp().profile_model
            .iter::<ProfileObject>()
            .flatten()
            .map(|profile| profile.to_json())
            .collect();

        let config_path = xdg::BaseDirectories::new()
            .place_config_file("Syncer/config.json")?;

        let json_str = to_string_pretty(&json_object)?;

        fs::write(config_path, json_str.as_bytes())
    }
}
