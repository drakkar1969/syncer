use std::cell::Cell;
use std::sync::{OnceLock, LazyLock};
use std::str::FromStr;
use std::io;
use std::process::Stdio;

use gtk::{prelude::ObjectExt, subclass::prelude::*, glib};
use glib::{subclass::Signal, types::StaticType, variant::ToVariant};

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
    profile_object::ProfileObject,
    utils::{case::capitalize_first, convert, size}
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
#[derive(Debug, Clone)]
#[repr(u32)]
pub enum RsyncSend {
    Start(Option<i32>),
    ListBegin(String),
    ListItem(String),
    ListEnd(String),
    RecurseComplete,
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

impl RsyncStats {
    pub fn source_items(&self) -> &str {
        &self.source_items
    }

    pub fn transferred_items(&self) -> &str {
        &self.transferred_items
    }

    pub fn source_size(&self) -> &str {
        &self.source_size
    }

    pub fn transferred_size(&self) -> &str {
        &self.transferred_size
    }

    pub fn speed(&self) -> &str {
        &self.speed
    }
}

//------------------------------------------------------------------------------
// MODULE: Rsync
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Rsync)]
    pub struct Rsync {
        #[property(get, set, construct, builder(RsyncState::default()))]
        state: Cell<RsyncState>,
        #[property(get, set)]
        dry_run: Cell<bool>,

        pub(super) pid: Cell<Option<NixPid>>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for Rsync {
        const NAME: &'static str = "Rsync";
        type Type = super::Rsync;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Rsync {
        //---------------------------------------
        // Signals
        //---------------------------------------
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("start")
                        .build(),
                    Signal::builder("status")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("message")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("recurse-complete")
                        .build(),
                    Signal::builder("progress")
                        .param_types([
                            String::static_type(),
                            String::static_type(),
                            f64::static_type()
                        ])
                        .build(),
                    Signal::builder("exit")
                        .param_types([
                            glib::Variant::static_type(),
                            glib::BoxedAnyObject::static_type()
                        ])
                        .build(),
                ]
            })
        }
    }
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: Rsync
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct Rsync(ObjectSubclass<imp::Rsync>);
}

impl Rsync {
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
    // Start function
    //---------------------------------------
    #[allow(clippy::future_not_send)]
    pub async fn start(&self, profile: &ProfileObject, dry_run: bool) -> io::Result<()> {
        // Get args
        let args: Vec<String> = profile.switches(false)
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

        self.set_dry_run(dry_run);

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

        while let Ok(rsync_send) = receiver.recv().await {
            let imp = self.imp();

            match rsync_send {
                RsyncSend::Start(id) => {
                    imp.pid.set(id.map(NixPid::from_raw));
                    self.set_state(RsyncState::Running);

                    self.emit_by_name::<()>("start", &[]);
                }

                RsyncSend::ListBegin(msg) => {
                    self.emit_by_name::<()>("status", &[&msg]);

                    output.push_message(RsyncMsg::Info, msg);
                }

                RsyncSend::ListItem(msg) => {
                    self.emit_by_name::<()>("message", &[&msg]);
                }

                RsyncSend::ListEnd(msg) => {
                    sync_shown = true;

                    self.emit_by_name::<()>("status",
                        &[&format!("Syncing to {}", profile.destination())]
                    );
                    self.emit_by_name::<()>("message", &[&msg]);
                    self.emit_by_name::<()>("recurse-complete", &[]);

                    output.push_message(RsyncMsg::Info, msg);
                }

                RsyncSend::RecurseComplete => {
                    self.emit_by_name::<()>("recurse-complete", &[]);
                }

                RsyncSend::Progress(size, speed, progress) => {
                    self.emit_by_name::<()>("progress", &[&size, &speed, &progress]);
                }

                RsyncSend::Message(type_, msg) => {
                    if !sync_shown {
                        self.emit_by_name::<()>("status",
                            &[&format!("Syncing to {}", profile.destination())]
                        );

                        sync_shown = true;
                    }

                    self.emit_by_name::<()>("message", &[&msg]);

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

                    let final_output = std::mem::take(&mut output);

                    self.emit_by_name::<()>("exit",
                        &[&code.to_variant(),
                        &glib::BoxedAnyObject::new(final_output)]
                    );

                    break;
                }
            }
        }

        rsync_task
            .await
            .expect("Failed to complete tokio task")
    }

    //---------------------------------------
    // Parse progress async function
    //---------------------------------------
    async fn parse_progress(line: &str, sender: &Sender::<RsyncSend>) {
        for chunk in line.trim_start_matches('\r').split_terminator('\r') {
            let parts: Vec<&str> = chunk
                .split_whitespace()
                .collect();

            if parts.len() < 6 {
                return;
            }

            if let (size, speed, Ok(progress)) = (
                parts[0].to_owned(),
                parts[2].to_owned(),
                parts[1].trim_end_matches('%').parse::<f64>()
            ) {
                sender
                    .send(RsyncSend::Progress(size, speed, progress))
                    .await
                    .expect("Could not send through channel");
            }

            if parts[5].contains("to-chk") {
                sender
                    .send(RsyncSend::RecurseComplete)
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
                        capitalize_first(flags.trim_start_matches('*')),
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
                let msg = capitalize_first(line);

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
                // File list start line
                if line.contains("building file list") {
                    recurse_mode = true;

                    for chunk in line.split_terminator('\r') {
                        if chunk.starts_with("building file list ...") {
                            sender.send(RsyncSend::ListBegin(capitalize_first(chunk)))
                                .await
                                .expect("Could not send through channel");
                        } else {
                            sender.send(RsyncSend::ListItem(chunk.into()))
                                .await
                                .expect("Could not send through channel");
                        }
                    }

                    continue;
                }

                // File list line
                if recurse_mode {
                    if line.ends_with("to consider") {
                        // File list end line
                        recurse_mode = false;

                        for chunk in line.split('\r') {
                            if chunk.ends_with("to consider") {
                                sender.send(RsyncSend::ListEnd(chunk.into()))
                                    .await
                                    .expect("Could not send through channel");
                            } else {
                                sender.send(RsyncSend::ListItem(chunk.into()))
                                    .await
                                    .expect("Could not send through channel");
                            }
                        }

                        continue;
                    } else if line.starts_with(' ') && line.contains("files...") {
                        for chunk in line.split_terminator('\r') {
                            sender.send(RsyncSend::ListItem(chunk.into()))
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
                sender.send(RsyncSend::Error(capitalize_first(line)))
                    .await
                    .expect("Could not send through channel");
            }
        }
    }

    //---------------------------------------
    // Terminate function
    //---------------------------------------
    pub fn terminate(&self) -> Result<(), NixErrno> {
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
    // Pause function
    //---------------------------------------
    pub fn pause(&self) -> Result<(), NixErrno> {
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
    // Resume function
    //---------------------------------------
    pub fn resume(&self) -> Result<(), NixErrno> {
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
    // Stats function
    //---------------------------------------
    pub fn stats(output: &RsyncOutput) -> Option<RsyncStats> {
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
                    source_size: size::format(&regex_match("ssize")).into_owned(),
                    transferred_size: size::format(&regex_match("tsize")).into_owned(),
                    speed: size::format(&regex_match("speed")).into_owned()
                }
            })
    }

    //---------------------------------------
    // Errors function
    //---------------------------------------
    pub fn errors(code: i32, errors: &[String]) -> (String, String) {
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

                    capitalize_first(s.trim())
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

impl Default for Rsync {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
