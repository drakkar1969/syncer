use std::cell::Cell;
use std::sync::OnceLock;
use std::io;
use std::process::Stdio;

use gtk::{gio, glib, gdk};
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::clone;

use tokio::runtime::Runtime as TkRuntime;
use tokio::process::Command as TkCommand;
use tokio::io::AsyncReadExt as _;
use nix::sys::signal as nix_signal;
use nix::unistd::Pid as NixPid;

use crate::Application;
use crate::sidebar::Sidebar;
use crate::profile_object::ProfileObject;
use crate::options_page::OptionsPage;
use crate::advanced_page::AdvancedPage;
use crate::rsync_page::RsyncPage;

//------------------------------------------------------------------------------
// ENUM: RsyncMsg
//------------------------------------------------------------------------------
#[derive(Debug, Default, PartialEq)]
#[repr(u32)]
pub enum RsyncMsg {
    #[default]
    None,
    Start(Option<i32>),
    Message(String),
    Progress(String, String, f64),
    Stats(String),
    Error(String),
    Exit(Option<i32>)
}

//------------------------------------------------------------------------------
// MODULE: AppWindow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/RsyncUI/ui/window.ui")]
    pub struct AppWindow {
        #[template_child]
        pub(super) sidebar: TemplateChild<Sidebar>,

        #[template_child]
        pub(super) content_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) content_navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) new_profile_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) options_page: TemplateChild<OptionsPage>,
        #[template_child]
        pub(super) advanced_page: TemplateChild<AdvancedPage>,
        #[template_child]
        pub(super) rsync_page: TemplateChild<RsyncPage>,

        pub(super) dry_run: Cell<bool>,
        pub(super) rsync_id: Cell<Option<i32>>,
        pub(super) rsync_running: Cell<bool>,
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
            // Content push options action
            //---------------------------------------
            klass.install_action("content.push-options", None, |window, _, _| {
                window.imp().content_navigation_view.push_by_tag("advanced");
            });

            //---------------------------------------
            // Rsync start action
            //---------------------------------------
            klass.install_action("rsync.start", Some(glib::VariantTy::BOOLEAN), |window, _, parameter| {
                let imp = window.imp();

                // Check if dry run
                let dry_run = parameter
                    .and_then(|param| param.get::<bool>())
                    .expect("Could not get bool from variant");

                window.imp().dry_run.set(dry_run);

                // Show rsync page
                imp.content_navigation_view.push_by_tag("rsync");

                // Start rsync
                window.start_rsync();
            });

            //---------------------------------------
            // Rsync terminate action
            //---------------------------------------
            klass.install_action("rsync.terminate", None, |window, _, _| {
                let imp = window.imp();

                if let Some(id) = imp.rsync_id.get() {
                    let pid = NixPid::from_raw(id);

                    // Resume if paused
                    if imp.rsync_page.paused() {
                        let _ = nix_signal::kill(pid, nix_signal::Signal::SIGCONT);

                        imp.rsync_page.set_paused(false);
                    }

                    // Terminate rsync
                    let _ = nix_signal::kill(pid, nix_signal::Signal::SIGTERM);
                }
            });

            //---------------------------------------
            // Rsync pause action
            //---------------------------------------
            klass.install_action("rsync.pause", None, |window, _, _| {
                let imp = window.imp();

                // Pause rsync if not paused
                if !imp.rsync_page.paused() && let Some(id) = imp.rsync_id.get() {
                    let pid = NixPid::from_raw(id);

                    let _ = nix_signal::kill(pid, nix_signal::Signal::SIGSTOP);

                    imp.rsync_page.set_paused(true);
                }
            });

            //---------------------------------------
            // Rsync resume action
            //---------------------------------------
            klass.install_action("rsync.resume", None, |window, _, _| {
                let imp = window.imp();

                // Resume rsync if paused
                if imp.rsync_page.paused() && let Some(id) = imp.rsync_id.get() {
                    let pid = NixPid::from_raw(id);

                    let _ = nix_signal::kill(pid, nix_signal::Signal::SIGCONT);

                    imp.rsync_page.set_paused(false);
                }
            });

            //---------------------------------------
            // New profile key binding
            //---------------------------------------
            klass.add_binding(gdk::Key::N, gdk::ModifierType::CONTROL_MASK, |window| {
                window.imp().sidebar.activate_action("sidebar.new-profile", None)
                    .expect("Could not activate action 'sidebar.new-profile'");

                glib::Propagation::Stop
            });
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
    impl WindowImpl for AppWindow {}
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

        // Window close request signal
        self.connect_close_request(|window| {
            let imp = window.imp();

            if imp.rsync_running.get() {
                if !imp.rsync_page.paused() {
                    gtk::prelude::WidgetExt::activate_action(window, "rsync.pause", None)
                        .expect("Could not activate action 'rsync-pause'");
                }

                let dialog = adw::AlertDialog::builder()
                    .heading("Exit RsyncUI?")
                    .body("Terminate transfer process and exit.")
                    .default_response("exit")
                    .build();

                dialog.add_responses(&[("cancel", "_Cancel"), ("exit", "E_xit")]);
                dialog.set_response_appearance("exit", adw::ResponseAppearance::Destructive);

                dialog.connect_response(Some("exit"), clone!(
                    #[weak] window,
                    #[weak] imp,
                    move |_, _| {
                        imp.close_request.set(true);

                        gtk::prelude::WidgetExt::activate_action(&window, "rsync.terminate", None)
                            .expect("Could not activate action 'rsync.terminate'");
                    }
                ));

                dialog.present(Some(window));

                return glib::Propagation::Stop;
            } else {
                let _ = imp.sidebar.save_config();
            }

            glib::Propagation::Proceed
        });

        // New profile button clicked signal
        imp.new_profile_button.connect_clicked(clone!(
            #[weak] imp,
            move |_| {
                imp.sidebar.activate_action("sidebar.new-profile", None)
                    .expect("Could not activate action 'sidebar.new-profile'");
            }
        ));

        // Sidebar n_items property notify signal
        imp.sidebar.connect_n_items_notify(clone!(
            #[weak] imp,
            move |sidebar| {
                if sidebar.n_items() == 0 {
                    imp.content_navigation_view.pop();

                    imp.content_stack.set_visible_child_name("status");
                } else {
                    imp.content_stack.set_visible_child_name("profile");
                }
            }
        ));

        // Rsync page shown/hidden signals
        imp.rsync_page.connect_shown(clone!(
            #[weak] imp,
            move |_| {
                imp.sidebar.set_sensitive(false);

                imp.sidebar.action_set_enabled("sidebar.new-profile", false);
            }
        ));

        imp.rsync_page.connect_hidden(clone!(
            #[weak] imp,
            move |_| {
                imp.sidebar.set_sensitive(true);

                imp.sidebar.action_set_enabled("sidebar.new-profile", true);
            }
        ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Bind sidebar selected item to options page
        imp.sidebar.bind_property("selected-item", &imp.options_page.get(), "profile")
            .sync_create()
            .build();

        // Bind sidebar selected item to advanced page
        imp.sidebar.bind_property("selected-item", &imp.advanced_page.get(), "profile")
            .sync_create()
            .build();

        // Load profiles from config file
        let _ = imp.sidebar.load_config();
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
    // Rsync args function
    //---------------------------------------
    fn rsync_args(&self) -> Vec<String> {
        let imp = self.imp();

        imp.options_page.args()
            .map(|mut options| {
                let mut args: Vec<String> = ["-s", "--human-readable", "--info=copy,del,flist0,misc,name,progress2,symsafe,stats2"]
                    .into_iter()
                    .map(|s| s.to_owned())
                    .collect();

                if imp.dry_run.get() {
                    args.push(String::from("--dry-run"));
                }

                args.append(&mut imp.advanced_page.args());
                args.append(&mut options);

                args
            })
            .unwrap_or_default()
    }

    //---------------------------------------
    // Start rsync function
    //---------------------------------------
    fn start_rsync(&self) {
        let args = self.rsync_args();

        let (sender, receiver) = async_channel::bounded(1);

        // Spawn tokio task to run rsync
        AppWindow::runtime().spawn(
            async move {
                // Start rsync
                let mut rsync_process = TkCommand::new("rsync")
                    .args(args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;

                // Send rsync process id
                sender
                    .send(RsyncMsg::Start(rsync_process.id().map(|id| id as i32)))
                    .await
                    .expect("Could not send through channel");

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

                            sender
                                .send(RsyncMsg::Exit(status.code()))
                                .await
                                .expect("Could not send through channel");

                            break;
                        }
                    }
                }

                Ok::<(), io::Error>(())
            }
        );

        // Attach receiver for tokio task
        glib::spawn_future_local(clone!(
            #[weak(rename_to = window)] self,
            async move {
                let imp = window.imp();

                let mut stats: Vec<String> = vec![];
                let mut errors: Vec<String> = vec![];

                while let Ok(msg) = receiver.recv().await {
                    match msg {
                        RsyncMsg::Start(id) => {
                            imp.rsync_id.set(id);
                            imp.rsync_running.set(true);
                        },

                        RsyncMsg::Message(message) => {
                            imp.rsync_page.set_message(&message);
                        },

                        RsyncMsg::Progress(size, speed, progress) => {
                            imp.rsync_page.set_status(&size, &speed, progress);
                        },

                        RsyncMsg::Stats(stat) => {
                            stats.push(stat);
                        },

                        RsyncMsg::Error(error) => {
                            errors.push(error);
                        },

                        RsyncMsg::Exit(code) => {
                            imp.rsync_running.set(false);
                            imp.rsync_id.set(None);

                            if imp.close_request.get() {
                                window.close();
                            } else {
                                imp.rsync_page.set_exit_status(code, &stats, &errors);
                            }
                        }

                        _ => {}
                    }
                }
            }
        ));
    }
}
