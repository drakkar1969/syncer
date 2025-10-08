use std::cell::RefCell;
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
use crate::check_object::CheckObject;
use crate::progress_pane::ProgressPane;

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
// MODULE: RsyncPage
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::RsyncPage)]
    #[template(resource = "/com/github/RsyncUI/ui/rsync_page.ui")]
    pub struct RsyncPage {
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
        pub(super) progress_pane: TemplateChild<ProgressPane>,

        #[property(get, set)]
        profile: RefCell<Option<ProfileObject>>,

        pub(super) bindings: RefCell<Option<Vec<glib::Binding>>>
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for RsyncPage {
        const NAME: &'static str = "RsyncPage";
        type Type = super::RsyncPage;
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
    impl ObjectImpl for RsyncPage {
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

    impl WidgetImpl for RsyncPage {}
    impl NavigationPageImpl for RsyncPage {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: RsyncPage
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct RsyncPage(ObjectSubclass<imp::RsyncPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RsyncPage {
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

        // Progress pane revealed property notify
        // imp.progress_pane.connect_revealed_notify(clone!(
        //     #[weak(rename_to = pane)] self,
        //     move |progress_pane| {
        //         if progress_pane.revealed() {
        //             pane.start_rsync();
        //         }
        //     }
        // ));
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
    // Tokio runtime helper function
    //---------------------------------------
    // fn runtime() -> &'static TkRuntime {
    //     static RUNTIME: OnceLock<TkRuntime> = OnceLock::new();
    //     RUNTIME.get_or_init(|| {
    //         TkRuntime::new().expect("Setting up tokio runtime needs to succeed.")
    //     })
    // }

    //---------------------------------------
    // Build rsync args function
    //---------------------------------------
    // fn build_rsync_args(&self) -> Vec<String> {
    //     let imp = self.imp();

    //     let mut args: Vec<String> = vec!["--human-readable", "-s", "--info=flist0,name1,stats2,progress2"]
    //         .into_iter()
    //         .map(|s| s.to_owned())
    //         .collect();

    //     match imp.check_mode_combo.selected() {
    //         2 => { args.push(String::from("--checksum")) },
    //         _ => {}
    //     }

    //     if imp.recursive_switch.is_active() {
    //         args.push(String::from("-r"));
    //     } else {
    //         args.push(String::from("-d"));
    //     }

    //     if imp.preserve_time_switch.is_active() {
    //         args.push(String::from("-t"));
    //     }

    //     if imp.preserve_permissions_switch.is_active() {
    //         args.push(String::from("-p"));
    //     }

    //     if imp.preserve_owner_switch.is_active() {
    //         args.push(String::from("-o"));
    //     }

    //     if imp.preserve_group_switch.is_active() {
    //         args.push(String::from("-g"));
    //     }

    //     if imp.numeric_ids_switch.is_active() {
    //         args.push(String::from("--numeric-ids"));
    //     }

    //     if imp.preserve_symlinks_switch.is_active() {
    //         args.push(String::from("-l"));
    //     }

    //     if imp.preserve_hardlinks_switch.is_active() {
    //         args.push(String::from("-H"));
    //     }

    //     if imp.preserve_devices_switch.is_active() {
    //         args.push(String::from("-D"));
    //     }

    //     if imp.one_filesystem_switch.is_active() {
    //         args.push(String::from("-x"));
    //     }

    //     if imp.delete_destination_switch.is_active() {
    //         args.push(String::from("--delete"));
    //     }

    //     if imp.existing_switch.is_active() {
    //         args.push(String::from("--existing"));
    //     }

    //     if imp.ignore_existing_switch.is_active() {
    //         args.push(String::from("---ignore-existing"));
    //     }

    //     if imp.skip_newer_switch.is_active() {
    //         args.push(String::from("-u"));
    //     }

    //     if imp.compress_data_switch.is_active() {
    //         args.push(String::from("-x"));
    //     }

    //     if imp.backup_switch.is_active() {
    //         args.push(String::from("-b"));
    //     }

    //     args.push(imp.source_row.subtitle().unwrap_or_default().to_string());
    //     args.push(imp.destination_row.subtitle().unwrap_or_default().to_string());

    //     args
    // }

    //---------------------------------------
    // Start rsync function
    //---------------------------------------
    // fn start_rsync(&self) {
    //     let imp = self.imp();

    //     let args = ["-r", "-t", "-s", "-H", "--progress", "--human-readable", "--info=flist0,name1,stats2,progress2", "/home/drakkar/Downloads/Torrents/Alien: Earth (ELITE)/", "/home/drakkar/Scratch/RSYNC"];

    //     let (sender, receiver) = async_channel::bounded(1);

    //     RsyncPage::runtime().spawn(
    //         async move {
    //             // Start rsync
    //             let mut rsync_process = TkCommand::new("rsync")
    //                 .args(args)
    //                 .stdout(Stdio::piped())
    //                 .stderr(Stdio::piped())
    //                 .spawn()?;

    //             // Get handles to read rsync stdout and stderr
    //             let mut stdout = rsync_process.stdout.take()
    //                 .ok_or_else(|| io::Error::other("Could not get stdout"))?;

    //             let mut stderr = rsync_process.stderr.take()
    //                 .ok_or_else(|| io::Error::other("Could not get stderr"))?;

    //             // Create buffers to read stdout and stderr
    //             const BUFFER_SIZE: usize = 32768;

    //             let mut buffer_stdout = [0u8; BUFFER_SIZE];
    //             let mut buffer_stderr = [0u8; BUFFER_SIZE];

    //             let mut overflow = String::with_capacity(BUFFER_SIZE);

    //             let mut stats = false;

    //             loop {
    //                 tokio::select! {
    //                     // Read stdout when available
    //                     result = stdout.read(&mut buffer_stdout) => {
    //                         let bytes = result?;

    //                         if bytes >= BUFFER_SIZE {
    //                             overflow = String::from_utf8(buffer_stdout[..bytes].to_vec())
    //                                 .unwrap_or_default();
    //                         } else if bytes != 0 {
    //                             let mut text = String::from_utf8(buffer_stdout[..bytes].to_vec())
    //                                 .unwrap_or_default();

    //                             if !overflow.is_empty() {
    //                                 text.insert_str(0, &overflow);

    //                                 overflow.clear();
    //                             }

    //                             for chunk in text.split_terminator("\n") {
    //                                 if chunk.is_empty() {
    //                                     continue;
    //                                 }

    //                                 if chunk.starts_with("\r") {
    //                                     for line in chunk.split_terminator("\r") {
    //                                         let vec: Vec<&str> = line.split_whitespace().collect();

    //                                         let values = vec.first()
    //                                             .map(|s| s.to_string())
    //                                             .zip(vec.get(2).map(|s| s.to_string()))
    //                                             .zip(
    //                                                 vec.get(1)
    //                                                     .map(|s| s.to_string().replace("%", ""))
    //                                                     .and_then(|s| s.parse().ok())
    //                                             );

    //                                         if let Some(((size, speed), progress)) = values {
    //                                             sender
    //                                                 .send(RsyncMsg::Progress(size, speed, progress))
    //                                                 .await
    //                                                 .expect("Could not send through channel");
    //                                         }
    //                                     }
    //                                 } else if chunk.starts_with("Number of files:") || stats {
    //                                     stats = true;

    //                                     sender
    //                                         .send(RsyncMsg::Stats(chunk.to_owned()))
    //                                         .await
    //                                         .expect("Could not send through channel");
    //                                 } else {
    //                                     sender
    //                                         .send(RsyncMsg::Message(chunk.to_owned()))
    //                                         .await
    //                                         .expect("Could not send through channel");
    //                                 }
    //                             }
    //                         }
    //                     }

    //                     // Read stderr when available
    //                     result = stderr.read(&mut buffer_stderr) => {
    //                         let bytes = result?;

    //                         if bytes != 0 {
    //                             let error = String::from_utf8(buffer_stderr[..bytes].to_vec())
    //                                 .unwrap_or_default();

    //                             for chunk in error.split_terminator("\n") {
    //                                 if chunk.is_empty() {
    //                                     continue;
    //                                 }

    //                                 sender
    //                                     .send(RsyncMsg::Error(chunk.to_owned()))
    //                                     .await
    //                                     .expect("Could not send through channel");
    //                             }
    //                         }
    //                     }

    //                     // Process exit
    //                     result = rsync_process.wait() => {
    //                         let status = result?;

    //                         let code = status.code();

    //                         let signal = code.map_or_else(|| status.signal(), |_| None);

    //                         sender
    //                             .send(RsyncMsg::Exit(code, signal))
    //                             .await
    //                             .expect("Could not send through channel");

    //                         break;
    //                     }
    //                 }
    //             }

    //             Ok::<(), io::Error>(())
    //         }
    //     );

    //     glib::spawn_future_local(clone!(
    //         #[weak] imp,
    //         async move {
    //             let mut stats: Vec<String> = vec![];
    //             let mut errors: Vec<String> = vec![];

    //             while let Ok(msg) = receiver.recv().await {
    //                 match msg {
    //                     RsyncMsg::Message(message) => {
    //                         imp.progress_pane.set_message(&message);
    //                     },

    //                     RsyncMsg::Progress(size, speed, progress) => {
    //                         imp.progress_pane.set_status(&size, &speed, progress);
    //                     },

    //                     RsyncMsg::Stats(stat) => {
    //                         stats.push(stat);
    //                     },

    //                     RsyncMsg::Error(error) => {
    //                         errors.push(error);
    //                     },

    //                     RsyncMsg::Exit(code, signal) => {
    //                         println!("Exit Code = {:?}", code);
    //                         println!("Signal = {:?}", signal);

    //                         match (code, signal) {
    //                             (Some(0), _) => {
    //                                 imp.progress_pane.set_exit_status(true, "Transfer successfully completed");

    //                                 imp.progress_pane.set_progress(100.0);
    //                             },
    //                             (Some(exit), _) => {
    //                                 imp.progress_pane.set_exit_status(false, &format!("Transfer failed with error code {}", exit));
    //                             },
    //                             _ => {}
    //                         }
    //                     }

    //                     _ => {}
    //                 }
    //             }
    //         }
    //     ));
    // }
}
