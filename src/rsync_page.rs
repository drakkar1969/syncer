use std::cell::{Cell, RefCell};
use std::sync::{LazyLock, OnceLock};
use std::str::FromStr;
use std::time::Duration;
use std::io;
use std::process::Stdio;

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;
use glib::clone;

use strum::EnumString;
use async_channel::Sender;
use tokio::{
    runtime::Runtime,
    process::{Command, ChildStdout, ChildStderr},
    io::AsyncReadExt as _
};
use nix::{
    errno::Errno as NixErrno,
    sys::signal::{kill as nix_kill, Signal as NixSignal},
    unistd::Pid as NixPid
};
use regex::Regex;

use crate::{
    window::AppWindow,
    profile_object::ProfileObject,
    output_page::OutputPage,
    utils::{case, convert}
};

//------------------------------------------------------------------------------
// CONST Variables
//------------------------------------------------------------------------------
const BUFFER_SIZE: usize = 16384;
const ITEMIZE_TAG: &str = "[ITEMIZE]";

//------------------------------------------------------------------------------
// ENUM: RsyncState
//------------------------------------------------------------------------------
#[derive(Default, Debug, Eq, PartialEq, Clone, Copy, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "RsyncState")]
pub enum RsyncState {
    #[default]
    Stopped,
    Running,
    Paused
}

//------------------------------------------------------------------------------
// ENUM: RsyncSend
//------------------------------------------------------------------------------
#[derive(Debug, PartialEq)]
#[repr(u32)]
enum RsyncSend {
    Start(Option<i32>),
    RecurseBegin(String),
    Recurse(String),
    RecurseEnd(String),
    Progress(String, String, f64),
    Message(RsyncMsg, String),
    Stats(String),
    Error(String),
    Exit(Option<i32>)
}

//------------------------------------------------------------------------------
// ENUM: RsyncMsg
//------------------------------------------------------------------------------
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, glib::Enum, EnumString)]
#[repr(u32)]
#[enum_type(name = "RsyncMsg")]
pub enum RsyncMsg {
    Stat,
    Error,
    Info,
    #[strum(serialize = "f")]
    File,
    #[strum(serialize = "d")]
    Directory,
    #[strum(serialize = "L")]
    Link,
    #[strum(serialize = "D")]
    Device,
    #[strum(serialize = "S")]
    Special,
    #[default]
    None
}

//------------------------------------------------------------------------------
// STRUCT: RsyncOutput
//------------------------------------------------------------------------------
#[derive(Default, Debug, Clone)]
pub struct RsyncOutput {
    messages: Vec<(RsyncMsg, String)>,
    stats: Vec<String>,
    errors: Vec<String>
}

impl RsyncOutput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_message(&mut self, type_: RsyncMsg, msg: String) {
        self.messages.push((type_, msg));
    }

    pub fn push_stat(&mut self, msg: String) {
        self.stats.push(msg);
    }

    pub fn push_error(&mut self, msg: String) {
        self.errors.push(msg);
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty() && self.stats.is_empty() && self.errors.is_empty()
    }

    pub fn messages(&self) -> &[(RsyncMsg, String)] {
        &self.messages
    }

    pub fn stats(&self) -> &[String] {
        &self.stats
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }
}

//------------------------------------------------------------------------------
// STRUCT: RsyncStats
//------------------------------------------------------------------------------
#[derive(Default, Debug)]
pub struct RsyncStats {
    source_items: String,
    transferred_items: String,
    source_size: String,
    transferred_size: String,
    speed: String
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
    #[template(resource = "/com/github/Syncer/ui/rsync_page.ui")]
    pub struct RsyncPage {
        #[template_child]
        pub(super) status_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) status_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
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
        #[template_child]
        pub(super) source_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) filters_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub(super) button_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) pause_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) pause_content: TemplateChild<adw::ButtonContent>,
        #[template_child]
        pub(super) stop_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) output_button: TemplateChild<gtk::Button>,

        #[property(get, set)]
        output_page: RefCell<OutputPage>,

        #[property(get, set)]
        profile: RefCell<ProfileObject>,
        #[property(get, set, construct, builder(RsyncState::default()))]
        state: Cell<RsyncState>,

        pub(super) dry_run: Cell<bool>,
        pub(super) pid: Cell<Option<NixPid>>,
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
            klass.bind_template();

            Self::install_actions(klass);
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

    impl RsyncPage {
        //---------------------------------------
        // Install actions
        //---------------------------------------
        fn install_actions(klass: &mut <Self as ObjectSubclass>::Class) {
            // Pause rsync action
            klass.install_action("rsync.pause", None, |page, _, _| {
                if page.state() == RsyncState::Paused {
                    let result = page.rsync_resume();

                    if result.is_err() {
                        let window = page.root()
                            .and_downcast::<AppWindow>()
                            .expect("Could not get main window");

                        window.show_toast("Error: Failed to resume rsync");
                    }
                } else if page.state() == RsyncState::Running {
                    let result = page.rsync_pause();

                    if result.is_err() {
                        let window = page.root()
                            .and_downcast::<AppWindow>()
                            .expect("Could not get main window");

                        window.show_toast("Error: Failed to pause rsync");
                    }
                }
            });

            // Stop rsync action
            klass.install_action("rsync.stop", None, |page, _, _| {
                if page.rsync_terminate().is_err() {
                    let window = page.root()
                        .and_downcast::<AppWindow>()
                        .expect("Could not get main window");

                    window.show_toast("Error: Failed to terminate rsync");
                }
            });

            // Rsync output action
            klass.install_action("rsync.output", None, |page, _, _| {
                page.activate_action("navigation.push", Some(&"output".to_variant()))
                    .expect("Could not activate 'navigation.push' action");
            });
        }
    }
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
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        // Profile property notify signal
        self.connect_profile_notify(|page| {
            let imp = page.imp();

            let profile = page.profile();

            // Set page title
            page.set_title(&profile.name());

            // Set source folder
            imp.source_label.set_label(&profile.source());

            // Set filter rules
            let filters = if profile.filters().is_empty() {
                "None".into()
            } else {
                profile.filters().iter()
                    .filter_map(|filter| filter.split_once(' '))
                    .map(|(rule, pattern)| {
                        format!("{} {pattern}", case::capitalize_first(rule))
                    })
                    .collect::<Vec<String>>()
                    .join(" \u{2022} ")
            };

            imp.filters_label.set_label(&filters);
        });

        // State property notify signal
        self.connect_state_notify(|page| {
            let imp = page.imp();

            match page.state() {
                RsyncState::Stopped => {
                    imp.stop_button.set_sensitive(false);
                    imp.pause_button.set_sensitive(false);
                }

                RsyncState::Running => {
                    imp.stop_button.set_sensitive(true);
                    imp.pause_button.set_sensitive(true);

                    imp.pause_content.set_icon_name("media-playback-pause-symbolic");
                    imp.pause_content.set_label("_Pause");
                }

                RsyncState::Paused => {
                    imp.stop_button.set_sensitive(true);
                    imp.pause_button.set_sensitive(true);

                    imp.pause_content.set_icon_name("media-playback-start-symbolic");
                    imp.pause_content.set_label("_Resume");
                }
            }

        });
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        self.ui_reset();
    }

    //---------------------------------------
    // UI reset function
    //---------------------------------------
    pub fn ui_reset(&self) {
        let imp = self.imp();

        self.ui_status_format(&["heading"], "rsync-status-symbolic");
        self.ui_status("Waiting…");

        self.ui_message("---");

        self.ui_transferred("0");
        self.ui_speed("n/a");
        self.ui_bar_progress(0.0);

        imp.button_stack.set_visible_child_name("empty");

        self.output_page().clear();
    }

    //---------------------------------------
    // UI start function
    //---------------------------------------
    fn ui_start(&self) {
        // Show pause and terminate buttons
        glib::timeout_add_local_once(Duration::from_millis(150), clone!(
            #[weak(rename_to = page)] self,
            move || {
                if page.state() != RsyncState::Stopped {
                    page.imp().button_stack.set_visible_child_name("rsync");
                }
            }
        ));
    }

    //---------------------------------------
    // UI status function
    //---------------------------------------
    fn ui_status(&self, status: &str) {
        self.imp().status_label.set_label(status);
    }

    //---------------------------------------
    // UI status format function
    //---------------------------------------
    fn ui_status_format(&self, css_classes: &[&str], icon: &str) {
        let imp = self.imp();

        imp.status_box.set_css_classes(css_classes);
        imp.status_image.set_icon_name(Some(icon));
    }

    //---------------------------------------
    // UI message function
    //---------------------------------------
    fn ui_message(&self, message: &str) {
        self.imp().message_label.set_label(message);
    }

    //---------------------------------------
    // UI transferred function
    //---------------------------------------
    fn ui_transferred(&self, size: &str) {
        self.imp().transferred_label.set_label(&format!("{size}B"));
    }

    //---------------------------------------
    // UI speed function
    //---------------------------------------
    fn ui_speed(&self, speed: &str) {
        self.imp().speed_label.set_label(speed);
    }

    //---------------------------------------
    // UI bar progress function
    //---------------------------------------
    fn ui_bar_progress(&self, fraction: f64) {
        let imp = self.imp();

        imp.progress_label.set_label(&format!("{fraction}%"));
        imp.progress_bar.set_fraction(fraction/100.0);
    }

    //---------------------------------------
    // UI exit status function
    //---------------------------------------
    fn ui_exit_status(&self, code: Option<i32>, output: &RsyncOutput) {
        let imp = self.imp();

        let stats_table = Self::rsync_stats(output);

        // Show exit status
        match code {
            Some(0) => {
                // Ensure progress bar at 100% if success
                self.ui_bar_progress(100.0);

                if let Some(stats) = stats_table.as_ref() {
                    let status = format!("Success: {}B of {}B transferred",
                        stats.transferred_size,
                        stats.source_size
                    );

                    let msg = format!("{} of {} files transferred",
                        stats.transferred_items,
                        stats.source_items
                    );

                    self.ui_status_format(&["success", "heading"], "rsync-success-symbolic");
                    self.ui_status(&status);

                    self.ui_message(&msg);
                } else {
                    self.ui_status_format(&["warning", "heading"], "rsync-success-symbolic");
                    self.ui_status("Success: could not retrieve stats");

                    self.ui_message("Transfer information not available");
                }
            }

            Some(code) => {
                let (error, details) = Self::rsync_errors(code, &output.errors);

                self.ui_status_format(&["error", "heading"], "rsync-error-symbolic");
                self.ui_status(&error);

                self.ui_message(&details);
            }

            None => {
                self.ui_status_format(&["error", "heading"], "rsync-error-symbolic");
                self.ui_status("Unknown error");

                self.ui_message("Error details not available");
            }
        }

        // Show other stats
        if !imp.dry_run.get() && let Some(stats) = stats_table {
            self.ui_speed(&format!("{}B/s", stats.speed));
        }

        // Show details
        imp.button_stack.set_visible_child_name("output");

        if output.is_empty() {
            imp.output_button.set_visible(false);
        } else {
            imp.output_button.set_visible(true);
            imp.output_button.grab_focus();

            // Populate output window
            glib::idle_add_local_once(clone!(
                #[weak(rename_to = page)] self,
                #[strong] output,
                move || {
                    page.output_page().load(&output);
                }
            ));
        }
    }

    //---------------------------------------
    // Tokio runtime helper function
    //---------------------------------------
    fn runtime() -> &'static Runtime {
        static RUNTIME: OnceLock<Runtime> = OnceLock::new();
        RUNTIME.get_or_init(|| {
            Runtime::new().expect("Setting up tokio runtime needs to succeed.")
        })
    }

    //---------------------------------------
    // Parse progress async function
    //---------------------------------------
    async fn parse_progress(line: &str, sender: &Sender::<RsyncSend>) {
        for chunk in line.trim_start_matches('\r').split_terminator('\r') {
            let parts: Vec<&str> = chunk
                .split_whitespace()
                .collect();

            if parts.len() >= 3 && let (size, speed, Ok(progress)) = (
                parts[0],
                parts[2],
                parts[1].trim_end_matches('%').parse::<f64>()
            ) {
                sender
                    .send(RsyncSend::Progress(
                        size.into(),
                        speed.into(),
                        progress
                    ))
                    .await
                    .expect("Could not send through channel");
            }
        }
    }

    //---------------------------------------
    // Parse message async function
    //---------------------------------------
    async fn parse_message(line: &str, sender: &Sender::<RsyncSend>) {
        if line.starts_with(ITEMIZE_TAG) && let Some((flags, msg)) = line
            .trim_start_matches(ITEMIZE_TAG)
            .split_once(' ') {
                if flags.starts_with('*') {
                    let msg = format!("{} {}",
                        case::capitalize_first(flags.trim_start_matches('*')),
                        msg
                    );

                    sender.send(RsyncSend::Message(RsyncMsg::Info, msg))
                        .await
                        .expect("Could not send through channel");
                } else if let Some(type_) = flags.get(1..2)
                    .and_then(|type_| RsyncMsg::from_str(type_).ok()) {
                        sender.send(RsyncSend::Message(type_, msg.into()))
                            .await
                            .expect("Could not send through channel");
                }
            } else {
                let msg = case::capitalize_first(line);

                sender.send(RsyncSend::Message(RsyncMsg::Info, msg))
                    .await
                    .expect("Could not send through channel");
            }
    }

    //---------------------------------------
    // Parse stdout async function
    //---------------------------------------
    async fn parse_stdout(mut stdout: ChildStdout, sender: Sender::<RsyncSend>) {
        let mut buffer = [0u8; BUFFER_SIZE];
        let mut pending = vec![];

        let mut stats_mode = false;
        let mut recurse_mode = false;

        while let Ok(read) = stdout.read(&mut buffer).await {
            // Break if stdout is empty
            if read == 0 {
                break;
            }

            // Add buffer to pending
            pending.extend_from_slice(&buffer[..read]);

            // Continue if buffer is full
            if read == BUFFER_SIZE {
                continue;
            }

            // Drain pending and convert to string
            let bytes = std::mem::take(&mut pending);
            let text = String::from_utf8_lossy(&bytes);

            // Process stdout line by line
            for line in text.lines().filter(|&line| !line.is_empty()) {
                // Recursion start line
                if line.contains("building file list") {
                    recurse_mode = true;

                    for chunk in line.split_terminator('\r') {
                        if chunk.starts_with("building file list ...") {
                            sender.send(RsyncSend::RecurseBegin(case::capitalize_first(chunk)))
                                .await
                                .expect("Could not send through channel");
                        } else {
                            sender.send(RsyncSend::Recurse(chunk.into()))
                                .await
                                .expect("Could not send through channel");
                        }
                    }

                    continue;
                }

                // Recursion line
                if recurse_mode {
                    if line.ends_with("to consider") {
                        // Recursion end line
                        recurse_mode = false;

                        for chunk in line.split('\r') {
                            if chunk.ends_with("to consider") {
                                sender.send(RsyncSend::RecurseEnd(chunk.into()))
                                    .await
                                    .expect("Could not send through channel");
                            } else {
                                sender.send(RsyncSend::Recurse(chunk.into()))
                                    .await
                                    .expect("Could not send through channel");
                            }
                        }

                        continue;
                    } else if line.starts_with(' ') && line.contains("files...") {
                        for chunk in line.split_terminator('\r') {
                            sender.send(RsyncSend::Recurse(chunk.into()))
                                .await
                                .expect("Could not send through channel");
                        }

                        continue;
                    }
                }

                // Progress line
                if line.starts_with('\r') {
                    Self::parse_progress(line, &sender).await;

                    continue;
                }

                // Stats line
                if stats_mode || line.starts_with("Number of files:") {
                    stats_mode = true;

                    sender.send(RsyncSend::Stats(line.into()))
                        .await
                        .expect("Could not send through channel");

                    continue;
                }

                // Message line
                Self::parse_message(line, &sender).await;
            }
        }
    }

    //---------------------------------------
    // Parse stderr async function
    //---------------------------------------
    async fn parse_stderr(mut stderr: ChildStderr, sender: Sender::<RsyncSend>) {
        let mut buffer = [0u8; BUFFER_SIZE];

        while let Ok(read) = stderr.read(&mut buffer).await {
            // Break if stderr is empty
            if read == 0 {
                break;
            }

            // Read stderr and process line by line
            let error = String::from_utf8_lossy(&buffer[..read]);

            for line in error.lines().filter(|&line| !line.is_empty()) {
                sender.send(RsyncSend::Error(case::capitalize_first(line)))
                    .await
                    .expect("Could not send through channel");
            }
        }
    }

    //---------------------------------------
    // Start rsync function
    //---------------------------------------
    #[allow(clippy::future_not_send)]
    pub async fn start_rsync(&self, dry_run: bool) -> io::Result<()> {
        // Get args
        let profile = self.profile();

        let args: Vec<String> = profile.options(false)
            .into_iter()
            .chain(dry_run.then_some("--dry-run".into()))
            .chain(
                [
                    "--human-readable",
                    "--info=backup,copy,del,flist2,misc,name,progress2,skip2,symsafe,stats2",
                    "--debug=filter"
                ]
                .into_iter()
                .map(ToOwned::to_owned)
            )
            .chain([format!("--out-format={ITEMIZE_TAG}%i %n%L")])
            .chain([profile.source(), profile.destination()])
            .collect();

        self.imp().dry_run.set(dry_run);

        // Spawn tokio task to run rsync
        let (sender, receiver) = async_channel::bounded(1);

        let rsync_task = Self::runtime().spawn(
            async move {
                // Start rsync
                let mut rsync_process = Command::new("rsync")
                    .args(args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .kill_on_drop(true)
                    .spawn()?;

                // Get sdtout/stderr handles
                let stdout = rsync_process.stdout.take()
                    .ok_or_else(|| io::Error::other("Could not get stdout"))?;

                let stderr = rsync_process.stderr.take()
                    .ok_or_else(|| io::Error::other("Could not get stderr"))?;

                // Send rsync process id
                sender
                    .send(RsyncSend::Start(rsync_process.id().map(|id| id as i32)))
                    .await
                    .expect("Could not send through channel");

                // Spawn task to read stdout
                let sender_out = sender.clone();

                let stdout_task = tokio::spawn(Self::parse_stdout(stdout, sender_out));

                // Spawn task to read stderr
                let sender_err = sender.clone();

                let stderr_task = tokio::spawn(Self::parse_stderr(stderr, sender_err));

                // Wait for stdout, stderr and process
                let (_, _, status_res) = tokio::join!(
                    stdout_task,
                    stderr_task,
                    rsync_process.wait()
                );

                let code = status_res
                    .map_or_else(|_| None, |status| status.code());

                // Send rsync exit code
                sender
                    .send(RsyncSend::Exit(code))
                    .await
                    .expect("Could not send through channel");

                Ok::<(), io::Error>(())
            }
        );

        // Attach receiver for tokio task
        let mut output = RsyncOutput::new();
        let mut sync_shown = false;

        while let Ok(msg) = receiver.recv().await {
            let imp = self.imp();

            match msg {
                RsyncSend::Start(id) => {
                    imp.pid.set(id.map(NixPid::from_raw));

                    self.set_state(RsyncState::Running);

                    self.ui_start();
                }

                RsyncSend::RecurseBegin(msg) => {
                    self.ui_status(&msg);

                    output.push_message(RsyncMsg::Info, msg);
                }

                RsyncSend::Recurse(msg) => {
                    self.ui_message(&msg);
                }

                RsyncSend::RecurseEnd(msg) => {
                    self.ui_status(&format!("Syncing to {}", profile.destination()));
                    sync_shown = true;

                    self.ui_message(&msg);

                    output.push_message(RsyncMsg::Info, msg);
                }

                RsyncSend::Progress(size, speed, progress) => {
                    self.ui_transferred(&size);

                    if !self.imp().dry_run.get() {
                        self.ui_speed(&speed);
                    }

                    self.ui_bar_progress(progress);
                }

                RsyncSend::Message(type_, msg) => {
                    if !sync_shown {
                        self.ui_status(
                            &format!("Syncing to {}", profile.destination())
                        );
                        sync_shown = true;
                    }

                    self.ui_message(&msg);

                    output.push_message(type_, msg);
                }

                RsyncSend::Stats(stat) => {
                    output.push_stat(stat);
                }

                RsyncSend::Error(error) => {
                    output.push_error(error);
                }

                RsyncSend::Exit(code) => {
                    self.set_state(RsyncState::Stopped);

                    imp.pid.set(None);

                    self.ui_exit_status(code, &output);
                }
            }
        }

        rsync_task
            .await
            .expect("Failed to complete tokio task")
    }

    //---------------------------------------
    // Rsync terminate function
    //---------------------------------------
    pub fn rsync_terminate(&self) -> Result<(), NixErrno> {
        let imp = self.imp();

        let pid = imp.pid.get().ok_or(NixErrno::ESRCH)?;

        // Resume rsync if paused
        if self.state() == RsyncState::Paused {
            nix_kill(pid, NixSignal::SIGCONT)?;

            self.set_state(RsyncState::Running);
        }

        // Terminate rsync
        nix_kill(pid, NixSignal::SIGTERM)?;

        Ok(())
    }

    //---------------------------------------
    // Rsync pause function
    //---------------------------------------
    pub fn rsync_pause(&self) -> Result<(), NixErrno> {
        let imp = self.imp();

        let pid = imp.pid.get().ok_or(NixErrno::ESRCH)?;

        // Pause rsync if not paused
        if self.state() == RsyncState::Running {
            nix_kill(pid, NixSignal::SIGSTOP)?;

            self.set_state(RsyncState::Paused);
        }

        Ok(())
    }

    //---------------------------------------
    // Rsync resume function
    //---------------------------------------
    pub fn rsync_resume(&self) -> Result<(), NixErrno> {
        let imp = self.imp();

        let pid = imp.pid.get().ok_or(NixErrno::ESRCH)?;

        // Resume rsync if paused
        if self.state() == RsyncState::Paused {
            nix_kill(pid, NixSignal::SIGCONT)?;

            self.set_state(RsyncState::Running);
        }

        Ok(())
    }

    //---------------------------------------
    // Rsync stats function
    //---------------------------------------
    pub fn rsync_stats(output: &RsyncOutput) -> Option<RsyncStats> {
        static EXPR: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?x)
                Number\s*of\s*files:\s*(?P<st>[\d,.]+)\s*\(?(?:reg:\s*(?P<sfiles>[\d,.]+))?,?\s*(?:dir:\s*(?P<sd>[\d,.]+))?,?\s*(?:link:\s*(?P<slinks>[\d,.]+))?,?\s*(?:special:\s*(?P<sspecials>[\d,.]+))?,?\s*\)?\n
                Number\s*of\s*created\s*files:\s*(?P<ct>[\d,.]+)\s*\(?(?:reg:\s*(?P<cfiles>[\d,.]+))?,?\s*(?:dir:\s*(?P<cd>[\d,.]+))?,?\s*(?:link:\s*(?P<clinks>[\d,.]+))?,?\s*(?:special:\s*(?P<cspecials>[\d,.]+))?,?\s*\)?\n
                Number\s*of\s*deleted\s*files:\s*(?P<dt>[\d,.]+)\s*\(?(?:reg:\s*(?P<dfiles>[\d,.]+))?,?\s*(?:dir:\s*(?P<dd>[\d,.]+))?,?\s*(?:link:\s*(?P<dlinks>[\d,.]+))?,?\s*(?:special:\s*(?P<dspecials>[\d,.]+))?,?\s*\)?\n
                Number\s*of\s*regular\s*files\s*transferred:\s*(?P<tfiles>[\d,.]+)\n
                Total\s*file\s*size:\s*(?P<ssize>.+)\s*bytes\n
                Total\s*transferred\s*file\s*size:\s*(?P<tsize>.+)\s*bytes\n
                .*\n
                .*\n
                .*\n
                .*\n
                .*\n
                Total\s*bytes\s*sent:\s*(?P<sbytes>.*?)\n
                Total\s*bytes\s*received:\s*(?P<rbytes>.*?)\n
                sent\s*.*?\s*bytes\s*received\s*.*?\s*bytes(?P<speed>.*?)\s*bytes
            ")
            .expect("Failed to compile Regex")
        });

        EXPR.captures(&output.stats.join("\n"))
            .map(|caps| {
                // Helper closure to extract regex match
                let regex_match = |s: &str| -> String {
                    caps.name(s)
                        .map_or("0", |m| m.as_str().trim_end_matches(',').trim())
                        .to_owned()
                };

                let source_items = convert::num_to_string(
                    ["sfiles", "slinks", "sspecials"].into_iter()
                        .map(|s| convert::string_to_num::<i64>(&regex_match(s)))
                        .sum::<i64>()
                );

                let transferred_items = convert::num_to_string(
                    output.messages.iter()
                        .filter(|(type_, _)| {
                            [
                                RsyncMsg::File,
                                RsyncMsg::Link,
                                RsyncMsg::Device,
                                RsyncMsg::Special
                            ]
                            .contains(type_)
                        })
                        .count()
                );

                RsyncStats {
                    source_items,
                    transferred_items,
                    source_size: regex_match("ssize"),
                    transferred_size: regex_match("tsize"),
                    speed: regex_match("speed")
                }
            })
    }

    //---------------------------------------
    // Rsync errors function
    //---------------------------------------
    pub fn rsync_errors(code: i32, errors: &[String]) -> (String, String) {
        static EXPR: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(?P<err>[^(]*).*")
                .expect("Failed to compile Regex")
        });

        // Helper closure to extract errors
        let format_error = |msg: &str| -> Option<String> {
            EXPR.captures(msg)?
                .name("err")
                .map(|m| {
                    let s = m.as_str()
                        .trim_end_matches('.')
                        .replace("Rsync: ", "")
                        .replace("Rsync error: ", "")
                        .replace("Rsync warning: ", "")
                        .replace("[sender]", "");

                    case::capitalize_first(s.trim())
                })
        };

        // Get detailed and main (last) errors
        let Some((main, details)) = errors.split_last() else {
            return ("Unknown error".into(), "n/a".into());
        };

        // Get error string
        let main_error = format!("{} ({code})", match code {
            // Terminated by user
            20 => "Terminated by user".into(),

            // Other error
            _ => format_error(main).unwrap_or_else(|| "Unknown error".into())
        });

        let error_details = details.iter()
            .map(|err| format_error(err).unwrap_or_else(|| "n/a".into()))
            .collect::<Vec<String>>()
            .join(" | ");

        (main_error, error_details)
    }
}

impl Default for RsyncPage {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
