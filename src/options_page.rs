use std::cell::RefCell;
use std::io;
use std::fs;
use std::env;

use adw::{prelude::*, subclass::prelude::*};
use gtk::{gio, glib, gdk};
use glib::{clone, VariantTy};

use serde_json::{json, to_string_pretty, from_str, Map as JsonMap, Value as JsonValue};

use crate::{
    profile_object::{CheckMode, RecurseMode, ProfileObject},
    profile_dialog::ProfileDialog,
    rsync_page::RsyncPage
};

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
        pub(super) advanced_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) filter_row: TemplateChild<adw::ActionRow>,

        #[property(get, set, nullable)]
        profile: RefCell<Option<ProfileObject>>,

        #[property(get, set)]
        rsync_page: RefCell<RsyncPage>,

        pub(super) bindings: RefCell<Option<Vec<glib::Binding>>>,

        pub(super) config_json: RefCell<String>,
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
            Self::install_rsync_actions(klass);

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

                    page.profile_dialog("Duplicate", Some(&profile.name()), clone!(
                        #[weak] imp,
                        move |new_name| {
                            if let Some(pos) = imp.profile_model.find(&profile) {
                                let duplicate = profile.duplicate(new_name);

                                imp.profile_model.insert(pos + 1, &duplicate);

                                imp.profile_dropdown.set_selected(pos + 1);
                            }
                        }
                    ));
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
        // Install rsync actions
        //---------------------------------------
        fn install_rsync_actions(klass: &mut <Self as ObjectSubclass>::Class) {
            // Rsync start action
            klass.install_action_async("rsync.start", Some(VariantTy::BOOLEAN),
                async |page, _, param| {
                    if let Some(profile) = page.profile() {
                        // Get dry run
                        let dry_run = param
                            .and_then(|param| param.get::<bool>())
                            .expect("Could not get bool from variant");

                        // Show rsync page
                        let rsync_page = page.rsync_page();

                        rsync_page.set_can_pop(false);
                        rsync_page.set_profile(profile);

                        page.activate_action("navigation.push", Some(&"rsync".to_variant()))
                            .expect("Could not activate 'navigation.push' action");

                        // Start rsync
                        let _ = rsync_page.start_rsync(dry_run).await;

                        rsync_page.set_can_pop(true);
                    }
                }
            );

            // Rsync show cmdline action
            klass.install_action("rsync.show-cmdline", None, |page, _, _| {
                if let Some(profile) = page.profile() {
                    // Get profile options
                    let options = profile.options(true).into_iter()
                        .collect::<Vec<String>>()
                        .join(" ");

                    // Build command line dialog
                    let dialog = adw::AlertDialog::builder()
                        .width_request(450)
                        .heading("Rsync Command Line")
                        .body(
                            format!("rsync {} \"{}\" \"{}\"",
                                options,
                                profile.source(),
                                profile.destination()
                            )
                        )
                        .default_response("copy")
                        .close_response("close")
                        .build();

                    dialog.add_responses(&[("close", "C_lose"), ("copy", "_Copy")]);

                    dialog.connect_response(Some("copy"), |dialog, _| {
                        dialog.clipboard().set_text(&dialog.body());
                    });

                    dialog.present(Some(page));
                }
            });
        }

        //---------------------------------------
        // Bind shortcuts
        //---------------------------------------
        fn bind_shortcuts(klass: &mut <Self as ObjectSubclass>::Class) {
            // New profile key binding
            klass.add_binding_action(gdk::Key::N, gdk::ModifierType::CONTROL_MASK, "profile.new");

            // Rename profile key binding
            klass.add_binding_action(gdk::Key::M, gdk::ModifierType::CONTROL_MASK, "profile.rename");

            // Delete profile key binding
            klass.add_binding_action(gdk::Key::D, gdk::ModifierType::CONTROL_MASK, "profile.delete");

            // Duplicate profile key binding
            klass.add_binding_action(gdk::Key::P, gdk::ModifierType::CONTROL_MASK, "profile.duplicate");

            // Reset profile key binding
            klass.add_binding_action(gdk::Key::R, gdk::ModifierType::CONTROL_MASK, "profile.reset");

            // Delete all profiles key binding
            klass.add_binding_action(gdk::Key::D, gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK, "profile.delete-all");

            // Rsync show cmdline key binding
            klass.add_binding_action(gdk::Key::L, gdk::ModifierType::CONTROL_MASK, "rsync.show-cmdline");
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
                .or_else(|| env::var("HOME").ok().map(Into::into))
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
                // Set copy by name button initial state
                let source = profile.source();

                imp.copy_by_name_button.set_active(!source.is_empty() && !source.ends_with('/'));

                // Bind profile property to widgets
                let bindings: Vec<glib::Binding> = vec![
                    profile.bind_property("source", &imp.source_row.get(), "subtitle")
                        .bidirectional()
                        .sync_create()
                        .build(),

                    profile.bind_property("destination", &imp.destination_row.get(), "subtitle")
                        .bidirectional()
                        .sync_create()
                        .build(),

                    profile.bind_property("check-mode", &imp.check_mode_combo.get(), "selected")
                        .bidirectional()
                        .sync_create()
                        .build(),

                    profile.bind_property("check-mode", &imp.check_mode_combo.get(), "subtitle")
                        .transform_to(|_, mode: CheckMode| mode.desc())
                        .sync_create()
                        .build(),

                    profile.bind_property("recurse-mode", &imp.recurse_mode_combo.get(), "selected")
                        .bidirectional()
                        .sync_create()
                        .build(),

                    profile.bind_property("recurse-mode", &imp.recurse_mode_combo.get(), "subtitle")
                        .transform_to(|_, mode: RecurseMode| mode.desc())
                        .sync_create()
                        .build(),

                    profile.bind_property("adv-modified", &imp.advanced_row.get(), "subtitle")
                        .transform_to(|_, modified: bool| {
                            Some(
                                if modified {
                                    "User-defined options"
                                } else {
                                    "Default options"
                                }
                            )
                        })
                        .sync_create()
                        .build(),

                    profile.bind_property("filters", &imp.filter_row.get(), "subtitle")
                        .transform_to(|_, filters: Vec<String>| {
                            let n_filters = filters.len();

                            if n_filters == 0 {
                                Some("No active filter rules".into())
                            } else {
                                Some(format!("{n_filters} active filter rule{}", if n_filters == 1 { "" } else { "s" }))
                            }
                        })
                        .sync_create()
                        .build()
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
    // Profile dialog function
    //---------------------------------------
    fn profile_dialog<F>(&self, action: &str, name: Option<&str>, f: F)
    where F: Fn(&str) + 'static {
        let imp = self.imp();

        let profile_list: Vec<String> = imp.profile_model.iter::<ProfileObject>()
            .flatten()
            .map(|profile| profile.name())
            .collect();

        let dialog = ProfileDialog::new(action, name, profile_list);

        dialog.connect_response(Some("add"), move |dialog, _| {
            f(&dialog.profile_name());
        });

        dialog.present(Some(self));
    }

    //---------------------------------------
    // Load config function
    //---------------------------------------
    pub fn load_config(&self) -> io::Result<()> {
        let imp = self.imp();

        // Load config file
        let config_path = xdg::BaseDirectories::new()
            .find_config_file("Syncer/config.json")
            .ok_or_else(|| io::Error::other("Config file not found"))?;

        let json_str = fs::read_to_string(config_path)?;

        let json_map: JsonMap<String, JsonValue> = from_str(&json_str)?;

        // Store config
        imp.config_json.replace(json_str);

        // Get profile list
        let profile_map = json_map.get("profiles")
            .and_then(|value| value.as_object())
            .ok_or_else(|| io::Error::other("Could not load profiles from config file"))?;

        let profiles: Vec<ProfileObject> = profile_map.iter()
            .filter_map(|(name, value)| ProfileObject::from_json(name, value))
            .collect();

        // Add profiles to model
        imp.profile_model.splice(0, 0, &profiles);

        // Select current profile
        if let Some(pos) = json_map.get("current-profile")
            .and_then(|value| value.as_str().filter(|profile| !profile.is_empty()))
            .and_then(|current_profile| {
                imp.profile_model.iter::<ProfileObject>()
                    .flatten()
                    .position(|profile| profile.name() == current_profile)
            }) {
                imp.profile_dropdown.set_selected(pos as u32);
            }

        Ok(())
    }

    //---------------------------------------
    // Save config function
    //---------------------------------------
    pub fn save_config(&self) -> io::Result<()> {
        let imp = self.imp();

        let current_profile = imp.profile_dropdown.selected_item()
            .and_downcast::<ProfileObject>()
            .map_or_else(String::new, |profile| profile.name());

        let profiles_map: JsonMap<String, JsonValue> = imp.profile_model
            .iter::<ProfileObject>()
            .flatten()
            .map(|profile| profile.to_json())
            .collect();

        let mut json_map = JsonMap::new();
        json_map.insert(String::from("current-profile"), json!(current_profile));
        json_map.insert(String::from("profiles"), profiles_map.into());

        let json_str = to_string_pretty(&json_map)?;

        // Save config only if different from stored config
        if json_str == *imp.config_json.borrow() {
            Ok(())
        } else {
            let config_path = xdg::BaseDirectories::new()
                .place_config_file("Syncer/config.json")?;

            fs::write(config_path, json_str.as_bytes())
        }
    }
}
