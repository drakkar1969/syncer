use std::cell::Cell;
use std::io::{self, Write};
use std::fs;

use gtk::{gio, glib, gdk};
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::{clone, Variant, VariantTy};

use itertools::Itertools;
use serde_json::{to_string_pretty, from_str, Map as JsonMap, Value as JsonValue};

use crate::Application;
use crate::profile_object::ProfileObject;
use crate::options_page::OptionsPage;
use crate::advanced_page::AdvancedPage;
use crate::rsync_page::RsyncPage;
use crate::rsync_process::ITEMIZE_TAG;

//------------------------------------------------------------------------------
// MODULE: AppWindow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/Syncer/ui/window.ui")]
    pub struct AppWindow {
        #[template_child]
        pub(super) status_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) status_new_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) profile_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) profile_model: TemplateChild<gio::ListStore>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) options_page: TemplateChild<OptionsPage>,
        #[template_child]
        pub(super) advanced_page: TemplateChild<AdvancedPage>,
        #[template_child]
        pub(super) rsync_page: TemplateChild<RsyncPage>,

        pub(super) close_request: Cell<bool>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for AppWindow {
        const NAME: &'static str = "AppWindow";
        type Type = super::AppWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            ProfileObject::ensure_type();

            klass.bind_template();

            Self::install_profile_actions(klass);
            Self::install_rsync_actions(klass);

            Self::bind_shortcuts(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AppWindow {
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

    impl WidgetImpl for AppWindow {}
    impl WindowImpl for AppWindow {
        //---------------------------------------
        // Close request function
        //---------------------------------------
        fn close_request(&self) -> glib::Propagation {
            let window = &*self.obj();

            let rsync_process = self.rsync_page.rsync_process();

            if rsync_process.running() {
                rsync_process.pause();

                let dialog = adw::AlertDialog::builder()
                    .heading("Exit Syncer?")
                    .body("Terminate transfer process and exit.")
                    .default_response("exit")
                    .build();

                dialog.add_responses(&[("cancel", "_Cancel"), ("exit", "E_xit")]);
                dialog.set_response_appearance("exit", adw::ResponseAppearance::Destructive);

                dialog.connect_response(Some("exit"), clone!(
                    #[weak(rename_to = imp)] self,
                    move |_, _| {
                        imp.close_request.set(true);

                        rsync_process.terminate();
                    }
                ));

                dialog.present(Some(window));

                return glib::Propagation::Stop;
            }

            let _ = window.save_config();

            glib::Propagation::Proceed
        }
    }
    impl ApplicationWindowImpl for AppWindow {}
    impl AdwApplicationWindowImpl for AppWindow {}

    impl AppWindow {
        //---------------------------------------
        // Install profile actions
        //---------------------------------------
        fn install_profile_actions(klass: &mut <Self as ObjectSubclass>::Class) {
            // New profile action
            klass.install_action("profile.new", None, |window, _, _| {
                let imp = window.imp();

                window.profile_name_dialog("New", None, clone!(
                    #[weak] imp,
                    move |name| {
                        imp.profile_model.append(&ProfileObject::new(name));

                        imp.profile_dropdown.set_selected(imp.profile_model.n_items() - 1);
                    }
                ));
            });

            // Rename profile action
            klass.install_action("profile.rename", None, |window, _, _| {
                let imp = window.imp();

                let name = imp.profile_dropdown.selected_item()
                    .and_downcast::<ProfileObject>()
                    .expect("Could not downcast to 'ProfileObject'")
                    .name();

                if let Some(obj) = imp.profile_model.iter::<ProfileObject>().flatten()
                    .find(|obj| obj.name() == name)
                {
                    window.profile_name_dialog("Rename", Some(&name), move |new_name| {
                        obj.set_name(new_name);
                    });
                }
            });

            // Delete profile action
            klass.install_action("profile.delete", None, |window, _, _| {
                let imp = window.imp();

                let name = imp.profile_dropdown.selected_item()
                    .and_downcast::<ProfileObject>()
                    .expect("Could not downcast to 'ProfileObject'")
                    .name();

                let dialog = adw::AlertDialog::builder()
                    .heading("Delete Profile?")
                    .body(format!("Permamenently delete the \"{name}\" profile."))
                    .default_response("delete")
                    .build();

                dialog.add_responses(&[("cancel", "_Cancel"), ("delete", "_Delete")]);
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

                dialog.connect_response(Some("delete"), clone!(
                    #[weak] imp,
                    move |_, _| {
                        if let Some(pos) = imp.profile_model.iter::<ProfileObject>().flatten()
                            .position(|obj| obj.name() == name)
                        {
                            imp.profile_model.remove(pos as u32);
                        }
                    }
                ));

                dialog.present(Some(window));
            });

            // Duplicate profile action
            klass.install_action("profile.duplicate", None, |window, _, _| {
                let imp = window.imp();

                let name = imp.profile_dropdown.selected_item()
                    .and_downcast::<ProfileObject>()
                    .expect("Could not downcast to 'ProfileObject'")
                    .name();

                if let Some((pos, obj)) = imp.profile_model.iter::<ProfileObject>().flatten()
                    .find_position(|obj| obj.name() == name)
                {
                    window.profile_name_dialog("Duplicate", Some(&name), clone!(
                        #[weak] imp,
                        move |new_name| {
                            let dup_obj = obj.duplicate(new_name);

                            imp.profile_model.insert(pos as u32 + 1, &dup_obj);

                            imp.profile_dropdown.set_selected(pos as u32 + 1);
                        }
                    ));
                }
            });

            // Reset profile action
            klass.install_action("profile.reset", None, |window, _, _| {
                let imp = window.imp();

                let name = imp.profile_dropdown.selected_item()
                    .and_downcast::<ProfileObject>()
                    .expect("Could not downcast to 'ProfileObject'")
                    .name();

                let dialog = adw::AlertDialog::builder()
                    .heading("Reset Profile?")
                    .body(format!("Reset the \"{name}\" profile to default values."))
                    .default_response("delete")
                    .build();

                dialog.add_responses(&[("cancel", "_Cancel"), ("reset", "_Reset")]);
                dialog.set_response_appearance("reset", adw::ResponseAppearance::Destructive);

                dialog.connect_response(Some("reset"), clone!(
                    #[weak] imp,
                    move |_, _| {
                        if let Some(obj) = imp.profile_model.iter::<ProfileObject>().flatten()
                            .find(|obj| obj.name() == name)
                        {
                            obj.reset();
                        }
                    }
                ));

                dialog.present(Some(window));
            });
        }

        //---------------------------------------
        // Install rsync actions
        //---------------------------------------
        fn install_rsync_actions(klass: &mut <Self as ObjectSubclass>::Class) {
            // Rsync start action
            klass.install_action("rsync.start", Some(VariantTy::BOOLEAN), |window, _, parameter| {
                let imp = window.imp();

                // Get dry run
                let dry_run = parameter
                    .and_then(Variant::get::<bool>)
                    .expect("Could not get bool from variant");

                // Show rsync page
                imp.navigation_view.push_by_tag("rsync");

                // Get profile
                let profile = imp.profile_dropdown.selected_item()
                    .and_downcast::<ProfileObject>()
                    .expect("Could not downcast to 'ProfileObject'");

                // Get args
                let args = [
                        "--human-readable",
                        &format!("--out-format={ITEMIZE_TAG}%i %n%L"),
                        "--info=copy,del,flist2,misc,name,progress2,symsafe,stats2"
                    ]
                    .into_iter()
                    .chain(dry_run.then_some("--dry-run"))
                    .map(ToOwned::to_owned)
                    .chain(profile.to_args(false))
                    .collect();

                // Start rsync
                imp.rsync_page.rsync_process().start(args);
            });

            // Rsync show cmdline action
            klass.install_action("rsync.show-cmdline", None, |window, _, _| {
                let imp = window.imp();

                // Build command line dialog
                let builder = gtk::Builder::from_resource("/com/github/Syncer/ui/builder/rsync_cmdline_dialog.ui");

                let dialog: adw::AlertDialog = builder.object("dialog")
                    .expect("Could not get object from resource");

                let label: gtk::Label = builder.object("label")
                    .expect("Could not get object from resource");

                let copy_button: gtk::Button = builder.object("copy_button")
                    .expect("Could not get object from resource");

                // Get profile
                let profile = imp.profile_dropdown.selected_item()
                    .and_downcast::<ProfileObject>()
                    .expect("Could not downcast to 'ProfileObject'");

                // Init command line dialog
                label.set_label(&format!("rsync {}", profile.to_args(true).join(" ")));

                copy_button.connect_clicked(clone!(
                    #[weak] window,
                    move |_| {
                        window.clipboard().set_text(&label.label());
                    }
                ));

                dialog.present(Some(window));
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
// IMPLEMENTATION: AppWindow
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct AppWindow(ObjectSubclass<imp::AppWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl AppWindow {
    //---------------------------------------
    // New function
    //---------------------------------------
    pub fn new(app: &Application) -> Self {
        glib::Object::builder()
            .property("application", app)
            .build()
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

        // Status new button clicked signal
        imp.status_new_button.connect_clicked(clone!(
            #[weak(rename_to = window)] self,
            move |_| {
                gtk::prelude::WidgetExt::activate_action(&window, "profile.new", None)
                    .expect("Could not activate action 'new-profile'");
            }
        ));

        // Back button clicked signal
        imp.back_button.connect_clicked(clone!(
            #[weak] imp,
            move |_| {
                imp.navigation_view.pop();
            }
        ));

        // Profile model changed signal
        imp.profile_model.connect_items_changed(clone!(
            #[weak] imp,
            move |model, _, _, _| {
                if model.n_items() == 0 {
                    imp.navigation_view.pop();

                    imp.status_stack.set_visible_child_name("status");
                } else {
                    imp.status_stack.set_visible_child_name("main");
                }
            }
        ));

        // Navigation view pushed/popped signals
        imp.navigation_view.connect_pushed(clone!(
            #[weak] imp,
            move |view| {
                if view.visible_page().is_some_and(|page| page.can_pop()) {
                    imp.back_button.set_visible(true);
                }
            }
        ));

        imp.navigation_view.connect_popped(clone!(
            #[weak] imp,
            move |view, _| {
                if view.visible_page_tag() == Some("options".into()) {
                    imp.back_button.set_visible(false);
                }
            }
        ));

        // Rsync page showing/hidden signals
        imp.rsync_page.connect_showing(clone!(
            #[weak(rename_to = window)] self,
            #[weak] imp,
            move |_| {
                window.action_set_enabled("profile.new", false);
                window.action_set_enabled("profile.rename", false);
                window.action_set_enabled("profile.delete", false);
                window.action_set_enabled("profile.duplicate", false);
                window.action_set_enabled("profile.reset", false);

                imp.profile_dropdown.set_sensitive(false);
            }
        ));

        imp.rsync_page.connect_hidden(clone!(
            #[weak(rename_to = window)] self,
            #[weak] imp,
            move |_| {
                window.action_set_enabled("profile.new", true);
                window.action_set_enabled("profile.rename", true);
                window.action_set_enabled("profile.delete", true);
                window.action_set_enabled("profile.duplicate", true);
                window.action_set_enabled("profile.reset", true);

                imp.profile_dropdown.set_sensitive(true);
            }
        ));

        // Rsync process running property notify signal
        imp.rsync_page.rsync_process().connect_running_notify(clone!(
            #[weak(rename_to = window)] self,
            move |process| {
                if !process.running() {
                    let imp = window.imp();

                    if imp.close_request.get() {
                        window.close();
                    }

                    if imp.navigation_view.visible_page_tag()
                        .is_some_and(|tag| tag == "rsync")
                    {
                        imp.back_button.set_visible(true);
                    }
                }
            }
        ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Bind selected profile to options page
        imp.profile_dropdown.bind_property("selected-item", &imp.options_page.get(), "profile")
            .sync_create()
            .build();

        // Bind selected profile to advanced page
        imp.profile_dropdown.bind_property("selected-item", &imp.advanced_page.get(), "profile")
            .sync_create()
            .build();

        // Bind selected profile to rsync page
        imp.profile_dropdown.bind_property("selected-item", &imp.rsync_page.get(), "profile")
            .sync_create()
            .build();

        // Load profiles from config file
        let _ = self.load_config();
    }

    //---------------------------------------
    // Load config function
    //---------------------------------------
    fn load_config(&self) -> io::Result<()> {
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

        let mut file = fs::File::create(config_path)?;

        file.write_all(json_str.as_bytes())
    }

    //---------------------------------------
    // Profile name dialog function
    //---------------------------------------
    fn profile_name_dialog<F>(&self, response: &str, default: Option<&str>, f: F)
    where F: Fn(&str) + 'static {
        let builder = gtk::Builder::from_resource("/com/github/Syncer/ui/builder/profile_name_dialog.ui");

        let dialog: adw::AlertDialog = builder.object("dialog")
            .expect("Could not get object from resource");

        dialog.set_heading(Some(&format!("{response} Profile")));
        dialog.set_response_label("add", response);

        let entry: adw::EntryRow = builder.object("entry")
            .expect("Could not get object from resource");

        entry.connect_changed(clone!(
            #[weak] dialog,
            move |entry| {
                dialog.set_response_enabled("add", !entry.text().is_empty());
            }
        ));

        if let Some(text) = default {
            entry.set_text(text);
        }

        dialog.connect_response(Some("add"), move |_, _| {
            f(&entry.text());
        });

        dialog.present(Some(self));
    }
}
