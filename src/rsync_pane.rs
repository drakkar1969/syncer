use std::cell::Cell;
use std::sync::OnceLock;
use std::io;
use std::process::Stdio;
use std::os::unix::process::ExitStatusExt;

use gtk::glib;
use adw::subclass::prelude::*;
use gtk::prelude::*;
use glib::clone;

use tokio::runtime::Runtime as TkRuntime;
use tokio::process::Command as TkCommand;
use tokio::io::AsyncReadExt as _;

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
// MODULE: RsyncPane
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::RsyncPane)]
    #[template(resource = "/com/github/RsyncUI/ui/rsync_pane.ui")]
    pub struct RsyncPane {
        #[template_child]
        pub(super) revealer: TemplateChild<gtk::Revealer>,

        #[template_child]
        pub(super) message_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) transferred_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) speed_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) progress_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) progress_bar: TemplateChild<gtk::ProgressBar>,

        #[property(get, set)]
        running: Cell<bool>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for RsyncPane {
        const NAME: &'static str = "RsyncPane";
        type Type = super::RsyncPane;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for RsyncPane {
        //---------------------------------------
        // Constructor
        //---------------------------------------
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.setup_signals();
        }
    }

    impl WidgetImpl for RsyncPane {}
    impl BinImpl for RsyncPane {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: RsyncPane
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct RsyncPane(ObjectSubclass<imp::RsyncPane>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RsyncPane {
    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

        // Running property notify signal
        self.connect_running_notify(|pane| {
            pane.imp().revealer.set_reveal_child(pane.running());
        });

        // Revealer child revealed signal
        imp.revealer.connect_child_revealed_notify(clone!(
            #[weak(rename_to = pane)] self,
            move |revealer| {
                if revealer.reveals_child() {
                    pane.start_rsync();
                } else {
                    pane.reset();
                }
            }
        ));
    }

    //---------------------------------------
    // Reset function
    //---------------------------------------
    fn reset(&self) {
        let imp = self.imp();

        imp.message_label.set_label("");

        imp.transferred_label.set_label("");
        imp.speed_label.set_label("");
        imp.progress_label.set_label("");
        imp.progress_bar.set_fraction(0.0);
    }


    //---------------------------------------
    // Set message function
    //---------------------------------------
    fn set_message(&self, message: &str) {
        self.imp().message_label.set_label(message);
    }

    //---------------------------------------
    // set status function
    //---------------------------------------
    fn set_status(&self, size: &str, speed: &str, progress: f64) {
        let imp = self.imp();

        imp.transferred_label.set_label(size);
        imp.speed_label.set_label(speed);
        imp.progress_label.set_label(&format!("{progress}%"));
        imp.progress_bar.set_fraction(progress/100.0);
    }

    //---------------------------------------
    // Set progress function
    //---------------------------------------
    fn set_progress(&self, progress: f64) {
        let imp = self.imp();

        imp.progress_label.set_label(&format!("{progress}%"));
        imp.progress_bar.set_fraction(progress/100.0);
    }

    //---------------------------------------
    // Set exit status function
    //---------------------------------------
    fn set_exit_status(&self, success: bool, message: &str) {
        let imp = self.imp();

        if success {
            imp.message_label.set_css_classes(&["success"]);
        } else {
            imp.message_label.set_css_classes(&["error"]);
        }

        imp.message_label.set_label(message);
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
    // fn build_rsync_args(&self) -> Vec<String> {
    //     let imp = self.imp();

    //     let mut args: Vec<String> = vec!["--human-readable", "-s", "--info=flist0,name1,stats2,progress2"]
    //         .into_iter()
    //         .map(|s| s.to_owned())
    //         .collect();


    //     args
    // }

    //---------------------------------------
    // Start rsync function
    //---------------------------------------
    fn start_rsync(&self) {
        let args = ["-r", "-t", "-s", "-H", "--progress", "--human-readable", "--info=flist0,name1,stats2,progress2", "/home/drakkar/Downloads/Torrents/Alien: Earth (ELITE)/", "/home/drakkar/Scratch/RSYNC"];

        let (sender, receiver) = async_channel::bounded(1);

        RsyncPane::runtime().spawn(
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
            #[weak(rename_to = pane)] self,
            async move {
                let mut stats: Vec<String> = vec![];
                let mut errors: Vec<String> = vec![];

                while let Ok(msg) = receiver.recv().await {
                    match msg {
                        RsyncMsg::Message(message) => {
                            pane.set_message(&message);
                        },

                        RsyncMsg::Progress(size, speed, progress) => {
                            pane.set_status(&size, &speed, progress);
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
                                    pane.set_exit_status(true, "Transfer successfully completed");

                                    pane.set_progress(100.0);
                                },
                                (Some(exit), _) => {
                                    pane.set_exit_status(false, &format!("Transfer failed with error code {}", exit));
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

impl Default for RsyncPane {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
