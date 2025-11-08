use std::cell::{Cell, RefCell};
use std::iter;
use std::time::Duration;
use std::io::{self, Write};
use std::fs;

use gtk::{gio, glib, gdk};
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::{clone, closure_local, Variant, VariantTy};

use itertools::Itertools;
use serde_json::{to_string_pretty, from_reader, Value as JsonValue};

use crate::Application;
use crate::profile_object::ProfileObject;
use crate::options_page::OptionsPage;
use crate::advanced_page::AdvancedPage;
use crate::rsync_page::RsyncPage;
use crate::rsync_process::{RsyncProcess, Stats};

//------------------------------------------------------------------------------
// MODULE: AppWindow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::AppWindow)]
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

        #[property(get)]
        rsync_process: RefCell<RsyncProcess>,

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

            //---------------------------------------
            // New profile action
            //---------------------------------------
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

            //---------------------------------------
            // Rename profile action
            //---------------------------------------
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

            //---------------------------------------
            // Delete profile action
            //---------------------------------------
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

            //---------------------------------------
            // Duplicate profile action
            //---------------------------------------
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

            //---------------------------------------
            // Reset profile action
            //---------------------------------------
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

            //---------------------------------------
            // Navigation pop action
            //---------------------------------------
            klass.install_action("navigation.pop", None, |window, _, _| {
                window.imp().navigation_view.pop();
            });

            //---------------------------------------
            // Navigation push advanced action
            //---------------------------------------
            klass.install_action("navigation.push-advanced", None, |window, _, _| {
                let imp = window.imp();

                imp.navigation_view.push_by_tag("advanced");
                imp.back_button.set_visible(true);
            });

            //---------------------------------------
            // Rsync start action
            //---------------------------------------
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
                let args = vec![
                        "--human-readable",
                        "--info=copy,del,flist0,misc,name,progress2,symsafe,stats2"
                    ]
                    .into_iter()
                    .chain(iter::once("--dry-run").filter(|_| dry_run))
                    .map(ToOwned::to_owned)
                    .chain(profile.args(false))
                    .collect();

                // Start rsync
                window.rsync_process().start(args);
            });

            //---------------------------------------
            // Rsync terminate action
            //---------------------------------------
            klass.install_action("rsync.terminate", None, |window, _, _| {
                window.rsync_process().terminate();
            });

            //---------------------------------------
            // Rsync pause action
            //---------------------------------------
            klass.install_action("rsync.pause", None, |window, _, _| {
                window.rsync_process().pause();
            });

            //---------------------------------------
            // Rsync resume action
            //---------------------------------------
            klass.install_action("rsync.resume", None, |window, _, _| {
                window.rsync_process().resume();
            });

            //---------------------------------------
            // Rsync show cmdline action
            //---------------------------------------
            klass.install_action("rsync.show-cmdline", None, |window, _, _| {
                let imp = window.imp();

                // Build command line dialog
                let builder = gtk::Builder::from_resource("/com/github/Syncer/ui/builder/rsync_cmdline_dialog.ui");

                let dialog: adw::AlertDialog = builder.object("dialog")
                    .expect("Could not get object from resource");

                let label: gtk::Label = builder.object("label")
                    .expect("Could not get object from resource");

                // Get profile
                let profile = imp.profile_dropdown.selected_item()
                    .and_downcast::<ProfileObject>()
                    .expect("Could not downcast to 'ProfileObject'");

                // Get args
                let args: Vec<String> = iter::once(String::from("rsync"))
                    .chain(profile.args(true))
                    .collect();

                // Init command line dialog
                label.set_label(&args.join(" "));

                let copy_button: gtk::Button = builder.object("copy_button")
                    .expect("Could not get object from resource");

                copy_button.connect_clicked(clone!(
                    #[weak] window,
                    move |_| {
                        window.clipboard().set_text(&label.label());
                    }
                ));

                dialog.present(Some(window));
            });

            //---------------------------------------
            // New profile key binding
            //---------------------------------------
            klass.add_binding_action(gdk::Key::N, gdk::ModifierType::CONTROL_MASK, "profile.new");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
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

            if window.rsync_process().running() {
                gtk::prelude::WidgetExt::activate_action(window, "rsync.pause", None)
                    .expect("Could not activate action 'rsync.pause'");

                let dialog = adw::AlertDialog::builder()
                    .heading("Exit Syncer?")
                    .body("Terminate transfer process and exit.")
                    .default_response("exit")
                    .build();

                dialog.add_responses(&[("cancel", "_Cancel"), ("exit", "E_xit")]);
                dialog.set_response_appearance("exit", adw::ResponseAppearance::Destructive);

                dialog.connect_response(Some("exit"), clone!(
                    #[weak] window,
                    #[weak(rename_to = imp)] self,
                    move |_, _| {
                        imp.close_request.set(true);

                        gtk::prelude::WidgetExt::activate_action(&window, "rsync.terminate", None)
                            .expect("Could not activate action 'rsync.terminate'");
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

        // Profile model n_items property notify signal
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

        // Navigation view popped signal
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

                imp.profile_dropdown.set_sensitive(false);
            }
        ));

        imp.rsync_page.connect_hidden(clone!(
            #[weak(rename_to = window)] self,
            #[weak] imp,
            move |_| {
                window.action_set_enabled("profile.new", true);

                imp.profile_dropdown.set_sensitive(true);
            }
        ));

        // Rsync process running property notify signal
        let rsync_process = self.rsync_process();

        rsync_process.connect_running_notify(clone!(
            #[weak] imp,
            move |process| {
                if !process.running() {
                    if imp.navigation_view.visible_page_tag()
                        .is_some_and(|tag| tag == "rsync")
                    {
                        imp.back_button.set_visible(true);
                    }
                }
            }
        ));

        // Rsync process paused property notify signal
        rsync_process.connect_paused_notify(clone!(
            #[weak] imp,
            move |process| {
                imp.rsync_page.set_pause_button_state(process.paused());
            }
        ));

        // Rsync process status signals
        rsync_process.connect_closure("start", false, closure_local!(
            #[weak] imp,
            #[weak] rsync_process,
            move |_: RsyncProcess| {
                glib::timeout_add_local_once(Duration::from_millis(150), clone!(
                    #[weak] imp,
                    #[weak] rsync_process,
                    move || {
                        if rsync_process.running() {
                            imp.rsync_page.set_start();
                        }
                    }
                ));
            }
        ));

        rsync_process.connect_closure("message", false, closure_local!(
            #[weak] imp,
            move |_: RsyncProcess, message: String| {
                imp.rsync_page.set_message(&message);
            }
        ));

        rsync_process.connect_closure("progress", false, closure_local!(
            #[weak] imp,
            move |_: RsyncProcess, size: String, speed: String, progress: f64| {
                imp.rsync_page.set_status(&size, &speed, progress);
            }
        ));

        rsync_process.connect_closure("exit", false, closure_local!(
            #[weak(rename_to = window)] self,
            #[weak] imp,
            move |_: RsyncProcess, code: i32, stats: Option<Stats>, error: Option<String>, messages: Vec<String>, stats_msgs: Vec<String>| {
                if imp.close_request.get() {
                    window.close();
                } else {
                    imp.rsync_page.set_exit_status(code, stats.as_ref(), error.as_deref(), &messages, &stats_msgs);
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

        let file = fs::File::open(config_path)?;

        let reader = io::BufReader::new(file);

        let json: Vec<JsonValue> = from_reader(reader)?;

        let profiles: Vec<ProfileObject> = json.iter()
            .filter_map(ProfileObject::from_json)
            .collect();

        // Add profiles to model
        imp.profile_model.splice(0, 0, &profiles);

        Ok(())
    }

    //---------------------------------------
    // Save config function
    //---------------------------------------
    pub fn save_config(&self) -> io::Result<()> {
        let profiles: Vec<JsonValue> = self.imp().profile_model.iter::<ProfileObject>()
            .flatten()
            .map(|obj| obj.to_json())
            .collect();

        let json = to_string_pretty(&profiles)
            .expect("Could not pretty print JSON string");

        let config_path = xdg::BaseDirectories::new()
            .place_config_file("Syncer/config.json")?;

        let mut file = fs::File::create(config_path)?;

        file.write_all(json.as_bytes())
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
