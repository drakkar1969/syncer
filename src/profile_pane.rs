use std::cell::{Cell, RefCell};
use std::os::unix::process::ExitStatusExt;
use std::sync::OnceLock;
use std::process::Stdio;
use std::io;

use gtk::{glib, gio};
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::clone;

use tokio::runtime::Runtime as TkRuntime;
use tokio::process::Command as TkCommand;
use tokio::io::AsyncReadExt as _;

use crate::profile_object::ProfileObject;
use crate::progress_pane::ProgressPane;

//------------------------------------------------------------------------------
// ENUM: CheckMode
//------------------------------------------------------------------------------
#[derive(Default, Debug, Eq, PartialEq, Clone, Copy, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "CheckMode")]
pub enum CheckMode {
    #[default]
    Default,
    #[enum_value(name = "Size Only")]
    SizeOnly,
    Checksum,
}

//------------------------------------------------------------------------------
// ENUM: RsyncMsg
//------------------------------------------------------------------------------
#[derive(Debug, Default, PartialEq)]
#[repr(u32)]
pub enum RsyncMsg {
    #[default]
    None,
    Message(String),
    Progress(String, String, f64),
    Stats(String),
    Error(String),
    Exit(Option<i32>, Option<i32>)
}

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
        pub(super) nav_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) nav_page_profile: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) nav_page_settings: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) profile_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) new_profile_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub(super) source_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) destination_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) check_mode_combo: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) settings_row: TemplateChild<adw::ActionRow>,

        #[template_child]
        pub(super) rsync_button: TemplateChild<gtk::Button>,

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

        #[template_child]
        pub(super) progress_pane: TemplateChild<ProgressPane>,

        #[property(get, set)]
        profile: RefCell<Option<ProfileObject>>,
        #[property(get, set)]
        rsync_running: Cell<bool>,

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
            CheckMode::ensure_type();

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
                let bindings: Vec<glib::Binding> = vec![
                    // Bind profile property to pane title
                    profile.bind_property("name", &imp.nav_page_profile.get(), "title")
                        .sync_create()
                        .build(),
                    profile.bind_property("name", &imp.nav_page_settings.get(), "title")
                        .sync_create()
                        .build(),

                    // Bind profile property to widgets
                    pane.bind_widget(&profile, "source", &imp.source_row.get(), "subtitle"),
                    pane.bind_widget(&profile, "destination", &imp.destination_row.get(), "subtitle"),
                    pane.bind_widget(&profile, "check-mode", &imp.check_mode_combo.get(), "selected"),

                    pane.bind_widget(&profile, "recursive", &imp.recursive_switch.get(), "active"),
                    pane.bind_widget(&profile, "preserve-time", &imp.preserve_time_switch.get(), "active"),
                    pane.bind_widget(&profile, "preserve-permissions", &imp.preserve_permissions_switch.get(), "active"),
                    pane.bind_widget(&profile, "preserve-owner", &imp.preserve_owner_switch.get(), "active"),
                    pane.bind_widget(&profile, "preserve-group", &imp.preserve_group_switch.get(), "active"),
                    pane.bind_widget(&profile, "numeric-ids", &imp.numeric_ids_switch.get(), "active"),
                    pane.bind_widget(&profile, "preserve-symlinks", &imp.preserve_symlinks_switch.get(), "active"),
                    pane.bind_widget(&profile, "preserve-hardlinks", &imp.preserve_hardlinks_switch.get(), "active"),
                    pane.bind_widget(&profile, "preserve-devices", &imp.preserve_devices_switch.get(), "active"),
                    pane.bind_widget(&profile, "no-leave-filesystem", &imp.one_filesystem_switch.get(), "active"),
                    pane.bind_widget(&profile, "delete-destination", &imp.delete_destination_switch.get(), "active"),
                    pane.bind_widget(&profile, "existing", &imp.existing_switch.get(), "active"),
                    pane.bind_widget(&profile, "ignore-existing", &imp.ignore_existing_switch.get(), "active"),
                    pane.bind_widget(&profile, "skip-newer", &imp.skip_newer_switch.get(), "active"),
                    pane.bind_widget(&profile, "compress-data", &imp.compress_data_switch.get(), "active"),
                    pane.bind_widget(&profile, "backup", &imp.backup_switch.get(), "active")
                ];

                // Store bindings
                imp.bindings.replace(Some(bindings));

                // Show profile page
                imp.profile_stack.set_visible_child_name("profile");
            } else {
                // set pane title
                imp.nav_page_profile.set_title(" ");
                imp.nav_page_settings.set_title(" ");

                // Show status page
                imp.profile_stack.set_visible_child_name("status");
            }
        });

        // Rsync running property notify signal
        self.connect_rsync_running_notify(|pane| {
            let imp = pane.imp();

            let enabled = !pane.rsync_running();

            imp.source_row.set_sensitive(enabled);
            imp.destination_row.set_sensitive(enabled);
            imp.check_mode_combo.set_sensitive(enabled);
            imp.settings_row.set_sensitive(enabled);

            imp.progress_pane.set_reveal(pane.rsync_running());
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

        // Check mode combo selected property notify
        imp.check_mode_combo.connect_selected_notify(|row| {
            let i = row.selected();

            let subtitle = match i {
                0 => "Check file size and modification time",
                1 => "Check file size only",
                2 => "Compare 128-bit checksum for files with matching size",
                _ => unreachable!()
            };

            row.set_subtitle(subtitle);
        });

        // Settings row activated signal
        imp.settings_row.connect_activated(clone!(
            #[weak] imp,
            move |_| {
                imp.nav_view.push_by_tag("settings");
            }
        ));

        // Rsync button clicked signal
        imp.rsync_button.connect_clicked(clone!(
            #[weak(rename_to = pane)] self,
            move |_| {
                pane.set_rsync_running(true);
            }
        ));

        // Progress pane revealed property notify
        imp.progress_pane.connect_revealed_notify(clone!(
            #[weak(rename_to = pane)] self,
            move |progress_pane| {
                if progress_pane.revealed() {
                    pane.start_rsync();
                }
            }
        ));
    }

    //---------------------------------------
    // Tokio runtime helper function
    //---------------------------------------
    fn runtime() -> &'static TkRuntime {
        static RUNTIME: OnceLock<TkRuntime> = OnceLock::new();
        RUNTIME.get_or_init(|| {
            TkRuntime::new().expect("Setting up tokio runtime needs to succeed.")
        })
    }

    //---------------------------------------
    // Build rsync args function
    //---------------------------------------
    fn build_rsync_args(&self) -> Vec<String> {
        let imp = self.imp();

        let mut args: Vec<String> = vec!["--human-readable", "-s", "--info=flist0,name1,stats2,progress2"]
            .into_iter()
            .map(|s| s.to_owned())
            .collect();

        match imp.check_mode_combo.selected() {
            1 => { args.push(String::from("--size-only")) },
            2 => { args.push(String::from("--checksum")) },
            _ => {}
        }

        if imp.recursive_switch.is_active() {
            args.push(String::from("-r"));
        } else {
            args.push(String::from("-d"));
        }

        if imp.preserve_time_switch.is_active() {
            args.push(String::from("-t"));
        }

        if imp.preserve_permissions_switch.is_active() {
            args.push(String::from("-p"));
        }

        if imp.preserve_owner_switch.is_active() {
            args.push(String::from("-o"));
        }

        if imp.preserve_group_switch.is_active() {
            args.push(String::from("-g"));
        }

        if imp.numeric_ids_switch.is_active() {
            args.push(String::from("--numeric-ids"));
        }

        if imp.preserve_symlinks_switch.is_active() {
            args.push(String::from("-l"));
        }

        if imp.preserve_hardlinks_switch.is_active() {
            args.push(String::from("-H"));
        }

        if imp.preserve_devices_switch.is_active() {
            args.push(String::from("-D"));
        }

        if imp.one_filesystem_switch.is_active() {
            args.push(String::from("-x"));
        }

        if imp.delete_destination_switch.is_active() {
            args.push(String::from("--delete"));
        }

        if imp.existing_switch.is_active() {
            args.push(String::from("--existing"));
        }

        if imp.ignore_existing_switch.is_active() {
            args.push(String::from("---ignore-existing"));
        }

        if imp.skip_newer_switch.is_active() {
            args.push(String::from("-u"));
        }

        if imp.compress_data_switch.is_active() {
            args.push(String::from("-x"));
        }

        if imp.backup_switch.is_active() {
            args.push(String::from("-b"));
        }

        args.push(imp.source_row.subtitle().unwrap_or_default().to_string());
        args.push(imp.destination_row.subtitle().unwrap_or_default().to_string());

        args
    }

    //---------------------------------------
    // Start rsync function
    //---------------------------------------
    fn start_rsync(&self) {
        let imp = self.imp();

        let args = ["-r", "-t", "-s", "-H", "--progress", "--human-readable", "--info=flist0,name1,stats2,progress2", "/home/drakkar/Downloads/Torrents/Alien: Earth (ELITE)/", "/home/drakkar/Scratch/RSYNC"];

        let (sender, receiver) = async_channel::bounded(1);

        ProfilePane::runtime().spawn(
            async move {
                // Start rsync
                let mut rsync_process = TkCommand::new("rsync")
                    .args(args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;

                // Get handles to read rsync stdout and stderr
                let mut stdout = rsync_process.stdout.take()
                    .ok_or_else(|| io::Error::other("Could not get stdout"))?;

                let mut stderr = rsync_process.stderr.take()
                    .ok_or_else(|| io::Error::other("Could not get stderr"))?;

                // Create buffers to read stdout and stderr
                const BUFFER_SIZE: usize = 32768;

                let mut buffer_stdout = [0u8; BUFFER_SIZE];
                let mut buffer_stderr = [0u8; BUFFER_SIZE];

                let mut overflow = String::with_capacity(BUFFER_SIZE);

                let mut stats = false;

                loop {
                    tokio::select! {
                        // Read stdout when available
                        result = stdout.read(&mut buffer_stdout) => {
                            let bytes = result?;

                            if bytes >= BUFFER_SIZE {
                                overflow = String::from_utf8(buffer_stdout[..bytes].to_vec())
                                    .unwrap_or_default();
                            } else if bytes != 0 {
                                let mut text = String::from_utf8(buffer_stdout[..bytes].to_vec())
                                    .unwrap_or_default();

                                if !overflow.is_empty() {
                                    text.insert_str(0, &overflow);

                                    overflow.clear();
                                }

                                for chunk in text.split_terminator("\n") {
                                    if chunk.is_empty() {
                                        continue;
                                    }

                                    if chunk.starts_with("\r") {
                                        for line in chunk.split_terminator("\r") {
                                            let vec: Vec<&str> = line.split_whitespace().collect();

                                            let values = vec.first()
                                                .map(|s| s.to_string())
                                                .zip(vec.get(2).map(|s| s.to_string()))
                                                .zip(
                                                    vec.get(1)
                                                        .map(|s| s.to_string().replace("%", ""))
                                                        .and_then(|s| s.parse().ok())
                                                );

                                            if let Some(((size, speed), progress)) = values {
                                                sender
                                                    .send(RsyncMsg::Progress(size, speed, progress))
                                                    .await
                                                    .expect("Could not send through channel");
                                            }
                                        }
                                    } else if chunk.starts_with("Number of files:") || stats {
                                        stats = true;

                                        sender
                                            .send(RsyncMsg::Stats(chunk.to_owned()))
                                            .await
                                            .expect("Could not send through channel");
                                    } else {
                                        sender
                                            .send(RsyncMsg::Message(chunk.to_owned()))
                                            .await
                                            .expect("Could not send through channel");
                                    }
                                }
                            }
                        }

                        // Read stderr when available
                        result = stderr.read(&mut buffer_stderr) => {
                            let bytes = result?;

                            if bytes != 0 {
                                let error = String::from_utf8(buffer_stderr[..bytes].to_vec())
                                    .unwrap_or_default();

                                for chunk in error.split_terminator("\n") {
                                    if chunk.is_empty() {
                                        continue;
                                    }

                                    sender
                                        .send(RsyncMsg::Error(chunk.to_owned()))
                                        .await
                                        .expect("Could not send through channel");
                                }
                            }
                        }

                        // Process exit
                        result = rsync_process.wait() => {
                            let status = result?;

                            let code = status.code();

                            let signal = code.map_or_else(|| status.signal(), |_| None);

                            sender
                                .send(RsyncMsg::Exit(code, signal))
                                .await
                                .expect("Could not send through channel");

                            break;
                        }
                    }
                }

                Ok::<(), io::Error>(())
            }
        );

        glib::spawn_future_local(clone!(
            #[weak] imp,
            async move {
                let mut stats: Vec<String> = vec![];
                let mut errors: Vec<String> = vec![];

                while let Ok(msg) = receiver.recv().await {
                    match msg {
                        RsyncMsg::Message(message) => {
                            imp.progress_pane.set_message(&message);
                        },

                        RsyncMsg::Progress(size, speed, progress) => {
                            imp.progress_pane.set_status(&size, &speed, progress);
                        },

                        RsyncMsg::Stats(stat) => {
                            stats.push(stat);
                        },

                        RsyncMsg::Error(error) => {
                            errors.push(error);
                        },

                        RsyncMsg::Exit(code, signal) => {
                            println!("Exit Code = {:?}", code);
                            println!("Signal = {:?}", signal);

                            match (code, signal) {
                                (Some(0), _) => {
                                    imp.progress_pane.set_exit_status(true, "Transfer successfully completed");

                                    imp.progress_pane.set_progress(100.0);
                                },
                                (Some(exit), _) => {
                                    imp.progress_pane.set_exit_status(false, &format!("Transfer failed with error code {}", exit));
                                },
                                _ => {}
                            }
                        }

                        _ => {}
                    }
                }
            }
        ));
    }
}
