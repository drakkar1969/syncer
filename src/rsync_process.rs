use std::cell::Cell;
use std::sync::{OnceLock, LazyLock};
use std::io;
use std::process::Stdio;
use std::str::FromStr;

use gtk::subclass::prelude::*;
use gtk::prelude::{ObjectExt, StaticType};
use gtk::glib;
use glib::subclass::Signal;

use strum::EnumString;
use async_channel::Sender;
use tokio::{
    runtime::Runtime,
    process::{Command, ChildStdout, ChildStderr},
    io::AsyncReadExt as _
};
use nix::{
    errno::Errno,
    sys::signal::{kill as nix_kill, Signal as NixSignal},
    unistd::Pid
};
use regex::Regex;

use crate::utils::{convert, case};

//------------------------------------------------------------------------------
// CONST Variables
//------------------------------------------------------------------------------
const BUFFER_SIZE: usize = 16384;
pub const ITEMIZE_TAG: &str = "[ITEMIZE]";

//------------------------------------------------------------------------------
// ENUM: RsyncSend
//------------------------------------------------------------------------------
#[derive(Debug, PartialEq)]
#[repr(u32)]
enum RsyncSend {
    Start(Option<i32>),
    Message(RsyncMsgType, String),
    Recurse(String),
    Progress(String, String, f64),
    Stats(String),
    Error(String),
    Exit(i32)
}

//------------------------------------------------------------------------------
// ENUM: RsyncMsgType
//------------------------------------------------------------------------------
#[allow(non_camel_case_types)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, glib::Enum, EnumString)]
#[repr(u32)]
#[enum_type(name = "RsyncMsgType")]
pub enum RsyncMsgType {
    Stat,
    Error,
    Info,
    f,
    d,
    L,
    D,
    S,
    #[default]
    None
}

//------------------------------------------------------------------------------
// STRUCT: RsyncMessages
//------------------------------------------------------------------------------
#[derive(Default, Debug, Clone, glib::Boxed)]
#[boxed_type(name = "RsyncMessages")]
pub struct RsyncMessages {
    messages: Vec<(RsyncMsgType, String)>,
    stats: Vec<String>,
    errors: Vec<String>
}

impl RsyncMessages {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn messages(&self) -> &[(RsyncMsgType, String)] {
        &self.messages
    }

    pub fn stats(&self) -> &[String] {
        &self.stats
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    pub fn push_message(&mut self, flag: RsyncMsgType, msg: String) {
        self.messages.push((flag, msg));
    }

    pub fn push_stat(&mut self, msg: String) {
        self.stats.push(msg);
    }

    pub fn push_error(&mut self, msg: String) {
        self.errors.push(msg);
    }
}

//------------------------------------------------------------------------------
// STRUCT: RsyncStats
//------------------------------------------------------------------------------
#[derive(Default, Debug, Clone, glib::Boxed)]
#[boxed_type(name = "RsyncStats", nullable)]
pub struct RsyncStats {
    pub source_total: String,
    pub source_files: String,
    pub source_dirs: String,
    pub source_links: String,
    pub source_specials: String,
    pub destination_total: String,
    pub destination_files: String,
    pub destination_dirs: String,
    pub destination_links: String,
    pub destination_specials: String,
    pub destination_deleted: String,
    pub bytes_source: String,
    pub bytes_transferred: String,
    pub speed: String
}

//------------------------------------------------------------------------------
// MODULE: RsyncProcess
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::RsyncProcess)]
    pub struct RsyncProcess {
        #[property(get, set)]
        running: Cell<bool>,
        #[property(get, set)]
        paused: Cell<bool>,

        pub(super) pid: Cell<Option<Pid>>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for RsyncProcess {
        const NAME: &'static str = "RsyncProcess";
        type Type = super::RsyncProcess;
    }

    #[glib::derived_properties]
    impl ObjectImpl for RsyncProcess {
        //---------------------------------------
        // Signals
        //---------------------------------------
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("start")
                        .build(),
                    Signal::builder("message")
                        .param_types([String::static_type()])
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
                            i32::static_type(),
                            RsyncMessages::static_type(),
                        ])
                        .build()
                ]
            })
        }
    }
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: RsyncProcess
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct RsyncProcess(ObjectSubclass<imp::RsyncProcess>);
}

impl RsyncProcess {
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
    // Handle progress async function
    //---------------------------------------
    async fn handle_progress(line: &str, sender: &Sender::<RsyncSend>) -> io::Result<()> {
        for chunk in line.split_terminator('\r') {
            let parts: Vec<&str> = chunk
                .split_whitespace()
                .collect();

            if let (Some(&size), Some(&speed), Some(progress)) = (
                parts.first(),
                parts.get(2),
                parts.get(1).and_then(|s| {
                    s.trim_end_matches('%').parse::<f64>().ok()
                })
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

        Ok(())
    }

    //---------------------------------------
    // Handle message async function
    //---------------------------------------
    async fn handle_message(line: &str, sender: &Sender::<RsyncSend>) -> io::Result<()> {
        let (tag, msg) = if line.starts_with(ITEMIZE_TAG) {
            let (changes, msg) = line
                .trim_start_matches(ITEMIZE_TAG)
                .split_once(' ')
                .unwrap_or(("", line));

            if changes.starts_with('*') {
                (
                    RsyncMsgType::Info,
                    format!("{} {}",
                        case::capitalize_first(changes.trim_start_matches('*')),
                        msg
                    )
                )
            } else {
                (
                    changes.get(1..2)
                        .and_then(|c| RsyncMsgType::from_str(c).ok())
                        .unwrap_or_default(),
                    msg.to_owned()
                )
            }
        } else {
            (RsyncMsgType::Info, case::capitalize_first(line))
        };

        sender.send(RsyncSend::Message(tag, msg))
            .await
            .expect("Could not send through channel");

        Ok(())
    }

    //---------------------------------------
    // Parse stdout async function
    //---------------------------------------
    async fn parse_stdout(mut stdout: ChildStdout, sender: Sender::<RsyncSend>) -> io::Result<()> {
        let mut buffer = [0u8; BUFFER_SIZE];
        let mut overflow = String::with_capacity(4 * BUFFER_SIZE);

        let mut stats_mode = false;
        let mut recurse_mode = false;

        while let Ok(bytes) = stdout.read(&mut buffer).await {
            // Break if stdout is empty
            if bytes == 0 {
                break;
            }

            // If buffer overflow, save stdout and continue
            if bytes >= BUFFER_SIZE {
                overflow.push_str(&String::from_utf8_lossy(&buffer[..bytes]));
                continue;
            }

            // Read stdout
            let mut text = String::from_utf8_lossy(&buffer[..bytes])
                .into_owned();

            if !overflow.is_empty() {
                text.insert_str(0, &overflow);

                overflow.clear();
            }

            // Process stdout line by line
            for line in text.split_terminator('\n') {
                if line.is_empty() {
                    continue;
                }

                // Progress line
                if line.starts_with('\r') {
                    Self::handle_progress(line, &sender).await?;
                    continue;
                }

                // Recursion line
                if recurse_mode && line.ends_with('\r') {
                    for chunk in line.split_terminator('\r') {
                        sender.send(RsyncSend::Recurse(chunk.into()))
                            .await
                            .expect("Could not send through channel");
                    }

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

                // Recursion toggle lines
                if line.contains("building file list ...") {
                    recurse_mode = true;
                    continue;
                }

                if line.ends_with("to consider") {
                    recurse_mode = false;
                    continue;
                }

                // Message line
                Self::handle_message(line, &sender).await?;
            }
        }

        Ok(())
    }

    //---------------------------------------
    // Parse stderr async function
    //---------------------------------------
    async fn parse_stderr(mut stderr: ChildStderr, sender: Sender::<RsyncSend>) -> io::Result<()> {
        let mut buffer = [0u8; BUFFER_SIZE];

        while let Ok(bytes) = stderr.read(&mut buffer).await {
            // Break if stderr is empty
            if bytes == 0 {
                break;
            }

            // Read stderr and process line by line
            let error = String::from_utf8_lossy(&buffer[..bytes]);

            for line in error.split_terminator('\n') {
                if !line.is_empty() {
                    sender.send(RsyncSend::Error(case::capitalize_first(line)))
                        .await
                        .expect("Could not send through channel");
                }
            }
        }

        Ok(())
    }

    //---------------------------------------
    // Start function
    //---------------------------------------
    pub async fn start(&self, args: Vec<String>) -> io::Result<()> {
        // Spawn tokio task to run rsync
        let (sender, receiver) = async_channel::bounded(1);

        let rsync_task = Self::runtime().spawn(
            async move {
                // Start rsync
                let mut rsync_process = Command::new("rsync")
                    .args(args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;

                // Send rsync process id
                sender
                    .send(RsyncSend::Start(rsync_process.id().map(|id| id as i32)))
                    .await
                    .expect("Could not send through channel");

                // Spawn task to read stdout
                let stdout = rsync_process.stdout.take()
                    .ok_or_else(|| io::Error::other("Could not get stdout"))?;

                let sender_out = sender.clone();

                let stdout_task = tokio::spawn(Self::parse_stdout(stdout, sender_out));

                // Spawn task to read stderr
                let stderr = rsync_process.stderr.take()
                    .ok_or_else(|| io::Error::other("Could not get stderr"))?;

                let sender_err = sender.clone();

                let stderr_task = tokio::spawn(Self::parse_stderr(stderr, sender_err));

                // Wait for stdout, stderr and process
                let (stdout_res, stderr_res, status_res) = tokio::join!(
                    stdout_task,
                    stderr_task,
                    rsync_process.wait()
                );

                let (_, _, status) = (stdout_res?, stderr_res?, status_res?);

                // Send rsync exit code
                sender
                    .send(RsyncSend::Exit(status.code().unwrap_or(1)))
                    .await
                    .expect("Could not send through channel");

                Ok::<(), io::Error>(())
            }
        );

        // Attach receiver for tokio task
        let imp = self.imp();

        let mut messages = RsyncMessages::new();

        while let Ok(msg) = receiver.recv().await {
            match msg {
                RsyncSend::Start(id) => {
                    imp.pid.set(id.map(Pid::from_raw));
                    self.set_running(true);

                    self.emit_by_name::<()>("start", &[]);
                }

                RsyncSend::Message(flag, msg) => {
                    self.emit_by_name::<()>("message", &[&msg]);

                    messages.push_message(flag, msg);
                }

                RsyncSend::Recurse(message) => {
                    self.emit_by_name::<()>("message", &[&message]);
                }

                RsyncSend::Progress(size, speed, progress) => {
                    self.emit_by_name::<()>("progress", &[
                        &size,
                        &speed,
                        &progress
                    ]);
                }

                RsyncSend::Stats(stat) => {
                    messages.push_stat(stat);
                }

                RsyncSend::Error(error) => {
                    messages.push_error(error);
                }

                RsyncSend::Exit(code) => {
                    self.set_running(false);
                    self.set_paused(false);
                    imp.pid.set(None);

                    self.emit_by_name::<()>("exit", &[
                        &code,
                        &messages
                    ]);
                }
            }
        }

        rsync_task.await?
    }

    //---------------------------------------
    // Terminate function
    //---------------------------------------
    pub fn terminate(&self) -> Result<(), Errno> {
        let imp = self.imp();

        if let Some(pid) = imp.pid.get() {
            // Resume rsync if paused
            if self.paused() {
                nix_kill(pid, NixSignal::SIGCONT)?;

                self.set_paused(false);
            }

            // Terminate rsync
            nix_kill(pid, NixSignal::SIGTERM)?;
        }

        Ok(())
    }

    //---------------------------------------
    // Pause function
    //---------------------------------------
    pub fn pause(&self) -> Result<(), Errno> {
        let imp = self.imp();

        // Pause rsync if not paused
        if !self.paused() && let Some(pid) = imp.pid.get() {
            nix_kill(pid, NixSignal::SIGSTOP)?;

            self.set_paused(true);
        }

        Ok(())
    }

    //---------------------------------------
    // Resume function
    //---------------------------------------
    pub fn resume(&self) -> Result<(), Errno> {
        let imp = self.imp();

        // Resume rsync if paused
        if self.paused() && let Some(pid) = imp.pid.get() {
            nix_kill(pid, NixSignal::SIGCONT)?;

            self.set_paused(false);
        }

        Ok(())
    }

    //---------------------------------------
    // Stats function
    //---------------------------------------
    pub fn stats(stats: &[String]) -> Option<RsyncStats> {
        static EXPR: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?x)
                Number\s*of\s*files:\s*(?P<st>[\d,.]+)\s*\(?(?:reg:\s*(?P<sf>[\d,.]+))?,?\s*(?:dir:\s*(?P<sd>[\d,.]+))?,?\s*(?:link:\s*(?P<sl>[\d,.]+))?,?\s*(?:special:\s*(?P<ss>[\d,.]+))?,?\s*\)?\n
                Number\s*of\s*created\s*files:\s*(?P<dt>[\d,.]+)\s*\(?(?:reg:\s*(?P<df>[\d,.]+))?,?\s*(?:dir:\s*(?P<dd>[\d,.]+))?,?\s*(?:link:\s*(?P<dl>[\d,.]+))?,?\s*(?:special:\s*(?P<ds>[\d,.]+))?,?\s*\)?\n
                Number\s*of\s*deleted\s*files:\s*(?P<dr>[\d,.]+)\s*\(?(?:reg:\s*(?P<nf>[\d,.]+))?,?\s*(?:dir:\s*(?P<nd>[\d,.]+))?,?\s*(?:link:\s*(?P<nl>[\d,.]+))?,?\s*(?:special:\s*(?P<ns>[\d,.]+))?,?\s*\)?\n
                Number\s*of\s*regular\s*files\s*transferred:\s*(?P<tt>[\d,.]+)\n
                Total\s*file\s*size:\s*(?P<bs>.+)\s*bytes\n
                Total\s*transferred\s*file\s*size:\s*(?P<bt>.+)\s*bytes\n
                .*\n
                .*\n
                .*\n
                .*\n
                .*\n
                .*\n
                .*\n
                sent\s*.*?\s*bytes\s*received\s*.*?\s*bytes(?P<ts>.*?)\s*bytes
            ")
            .expect("Failed to compile Regex")
        });

        EXPR.captures(&stats.join("\n"))
            .map(|caps| {
                let regex_match = |m: &str| -> String {
                    caps.name(m)
                        .map_or("0", |m| m.as_str().trim_end_matches(',').trim())
                        .to_owned()
                };

                let d_total = regex_match("dt");
                let d_files = regex_match("df");
                let d_transf = regex_match("tt");

                let dest_total = convert::max_str::<u32>(&d_total, &d_transf);
                let dest_files = convert::max_str::<u32>(&d_files, &d_transf);

                RsyncStats {
                    source_total: regex_match("st"),
                    source_files: regex_match("sf"),
                    source_dirs: regex_match("sd"),
                    source_links: regex_match("sl"),
                    source_specials: regex_match("ss"),
                    destination_total: dest_total,
                    destination_files: dest_files,
                    destination_dirs: regex_match("dd"),
                    destination_links: regex_match("dl"),
                    destination_specials: regex_match("ds"),
                    destination_deleted: regex_match("dr"),
                    bytes_source: regex_match("bs"),
                    bytes_transferred: regex_match("bt"),
                    speed: regex_match("ts")
                }
            })
    }

    //---------------------------------------
    // Error function
    //---------------------------------------
    pub fn error(code: i32, errors: &[String]) -> Option<String> {
        static EXPR: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(?P<err>[^(]*).*")
                .expect("Failed to compile Regex")
        });

        // Get first (detailed) and last (main) errors
        let (err_detail, err_main) = (errors.first()?, errors.last()?);

        // Helper closure to extract error
        let extract_error = |msg: &str| -> Option<String> {
            EXPR.captures(msg)?
                .name("err")
                .map(|m| {
                    let s = m.as_str().trim()
                        .trim_end_matches('.')
                        .replace("Rsync error: ", "")
                        .replace("Rsync warning: ", "");

                    case::capitalize_first(&s)
                })
        };

        // Get error string
        match code {
            // Terminated by user
            20 => Some(String::from("Terminated by user")),

            // Usage error
            1 => extract_error(err_detail)
                .or_else(|| extract_error(err_main)),

            // Other error
            _ => extract_error(err_main)
        }
    }
}

impl Default for RsyncProcess {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
