use std::cell::Cell;
use std::sync::{OnceLock, LazyLock};
use std::io;
use std::process::Stdio;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::prelude::{ObjectExt, StaticType};
use glib::clone;
use glib::subclass::Signal;

use async_channel::Sender;
use tokio::runtime::Runtime as TkRuntime;
use tokio::process::{Command as TkCommand, Child as TkChild};
use tokio::io::AsyncReadExt as _;
use nix::sys::signal as nix_signal;
use nix::unistd::Pid as NixPid;
use regex::{Regex, Captures};

use crate::utils::convert;

//------------------------------------------------------------------------------
// ENUM: RsyncMsg
//------------------------------------------------------------------------------
#[derive(Debug, Default, PartialEq)]
#[repr(u32)]
enum RsyncMsg {
    #[default]
    None,
    Start(Option<i32>),
    Message(String),
    Recurse(String),
    Progress(String, String, f64),
    Stats(String),
    Error(String),
    Exit(i32)
}

//------------------------------------------------------------------------------
// STRUCT: Stats
//------------------------------------------------------------------------------
#[derive(Default, Debug, Clone, glib::Boxed)]
#[boxed_type(name = "Stats", nullable)]
pub struct Stats {
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

        pub(super) id: Cell<Option<i32>>,
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
                            Vec::<String>::static_type(),
                            Vec::<String>::static_type(),
                            Vec::<String>::static_type()
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
    fn runtime() -> &'static TkRuntime {
        static RUNTIME: OnceLock<TkRuntime> = OnceLock::new();
        RUNTIME.get_or_init(|| {
            TkRuntime::new().expect("Setting up tokio runtime needs to succeed.")
        })
    }

    //---------------------------------------
    // Parse output async function
    //---------------------------------------
    async fn parse_output(process: &mut TkChild, sender: &Sender::<RsyncMsg>) -> Result<(), io::Error> {
        const BUFFER_SIZE: usize = 16384;

        // Get handles to read rsync stdout and stderr
        let mut stdout = process.stdout.take()
            .ok_or_else(|| io::Error::other("Could not get stdout"))?;

        let mut stderr = process.stderr.take()
            .ok_or_else(|| io::Error::other("Could not get stderr"))?;

        // Create buffers to read stdout and stderr
        let mut buffer_stdout = [0u8; BUFFER_SIZE];
        let mut buffer_stderr = [0u8; BUFFER_SIZE];

        let mut overflow = String::with_capacity(4 * BUFFER_SIZE);

        let mut stats_mode = false;
        let mut recurse_mode = false;

        'read: loop {
            tokio::select! {
                // Read stdout when available
                result = stdout.read(&mut buffer_stdout) => {
                    let bytes = result?;

                    // Continue if stdout is empty
                    if bytes == 0 {
                        continue 'read;
                    }

                    // If buffer overflow, save stdout and continue
                    if bytes >= BUFFER_SIZE {
                        overflow.push_str(&String::from_utf8_lossy(&buffer_stdout[..bytes]));

                        continue 'read;
                    }

                    // Read stdout
                    let mut text = String::from_utf8_lossy(&buffer_stdout[..bytes])
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

                        if line.starts_with('\r') {
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
                                        .send(RsyncMsg::Progress(size.into(), speed.into(), progress))
                                        .await
                                        .expect("Could not send through channel");
                                }
                            }
                        } else if recurse_mode && line.ends_with('\r') {
                            for chunk in line.split_terminator('\r') {
                                sender
                                    .send(RsyncMsg::Recurse(chunk.into()))
                                    .await
                                    .expect("Could not send through channel");
                            }
                        } else if line.starts_with("Number of files:") || stats_mode {
                            stats_mode = true;

                            sender
                                .send(RsyncMsg::Stats(line.into()))
                                .await
                                .expect("Could not send through channel");
                        } else if line.contains("building file list ...") {
                            recurse_mode = true;
                        } else if line.ends_with("to consider") {
                            recurse_mode = false;
                        } else {
                            sender
                                .send(RsyncMsg::Message(line.into()))
                                .await
                                .expect("Could not send through channel");
                        }
                    }
                }

                // Read stderr when available
                result = stderr.read(&mut buffer_stderr) => {
                    let bytes = result?;

                    // If stderr is not empty, process stderr line by line
                    if bytes != 0 {
                        let error = String::from_utf8_lossy(&buffer_stderr[..bytes]);

                        for line in error.split_terminator('\n') {
                            if !line.is_empty() {
                                sender
                                    .send(RsyncMsg::Error(line.to_owned()))
                                    .await
                                    .expect("Could not send through channel");
                            }
                        }
                    }
                }

                // Process exit
                result = process.wait() => {
                    let status = result?;

                    sender
                        .send(RsyncMsg::Exit(status.code().unwrap_or(1)))
                        .await
                        .expect("Could not send through channel");

                    break 'read;
                }
            }
        }

        Ok(())
    }

    //---------------------------------------
    // Start function
    //---------------------------------------
    pub fn start(&self, args: Vec<String>) {
        // Spawn tokio task to run rsync
        let (sender, receiver) = async_channel::bounded(1);

        Self::runtime().spawn(
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

                // Parse rsync output
                Box::pin(Self::parse_output(&mut rsync_process, &sender)).await
            }
        );

        // Attach receiver for tokio task
        glib::spawn_future_local(clone!(
            #[weak(rename_to = process)] self,
            async move {
                let imp = process.imp();

                let mut messages: Vec<String> = vec![];
                let mut stats: Vec<String> = vec![];
                let mut errors: Vec<String> = vec![];

                while let Ok(msg) = receiver.recv().await {
                    match msg {
                        RsyncMsg::Start(id) => {
                            imp.id.set(id);
                            process.set_running(true);

                            process.emit_by_name::<()>("start", &[]);
                        }

                        RsyncMsg::Message(message) => {
                            process.emit_by_name::<()>("message", &[&message]);

                            messages.push(message);
                        }

                        RsyncMsg::Recurse(message) => {
                            process.emit_by_name::<()>("message", &[&message]);
                        }

                        RsyncMsg::Progress(size, speed, progress) => {
                            process.emit_by_name::<()>("progress", &[
                                &size,
                                &speed,
                                &progress
                            ]);
                        }

                        RsyncMsg::Stats(stat) => {
                            stats.push(stat);
                        }

                        RsyncMsg::Error(error) => {
                            errors.push(error);
                        }

                        RsyncMsg::Exit(code) => {
                            process.set_running(false);
                            process.set_paused(false);
                            imp.id.set(None);

                            process.emit_by_name::<()>("exit", &[
                                &code,
                                &messages,
                                &stats,
                                &errors
                            ]);
                        }

                        RsyncMsg::None => {}
                    }
                }
            }
        ));
    }

    //---------------------------------------
    // Terminate function
    //---------------------------------------
    pub fn terminate(&self) {
        let imp = self.imp();

        if let Some(id) = imp.id.get() {
            let pid = NixPid::from_raw(id);

            // Resume if paused
            if self.paused() {
                let _ = nix_signal::kill(pid, nix_signal::Signal::SIGCONT);

                self.set_paused(false);
            }

            // Terminate rsync
            let _ = nix_signal::kill(pid, nix_signal::Signal::SIGTERM);
        }
    }

    //---------------------------------------
    // Pause function
    //---------------------------------------
    pub fn pause(&self) {
        let imp = self.imp();

        // Pause rsync if not paused
        if !self.paused() && let Some(id) = imp.id.get() {
            let pid = NixPid::from_raw(id);

            let _ = nix_signal::kill(pid, nix_signal::Signal::SIGSTOP);

            self.set_paused(true);
        }
    }

    //---------------------------------------
    // Resume function
    //---------------------------------------
    pub fn resume(&self) {
        let imp = self.imp();

        // Resume rsync if paused
        if self.paused() && let Some(id) = imp.id.get() {
            let pid = NixPid::from_raw(id);

            let _ = nix_signal::kill(pid, nix_signal::Signal::SIGCONT);

            self.set_paused(false);
        }
    }

    //---------------------------------------
    // Stats function
    //---------------------------------------
    pub fn stats(stats: &[String]) -> Option<Stats> {
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
                let get_match = |caps: &Captures, m: &str| -> String {
                    caps.name(m)
                        .map_or("0", |m| m.as_str().trim_end_matches(',').trim())
                        .to_owned()
                };

                let d_total = get_match(&caps, "dt");
                let d_files = get_match(&caps, "df");
                let d_transf = get_match(&caps, "tt");

                let dest_total = convert::max_str::<u32>(&d_total, &d_transf);
                let dest_files = convert::max_str::<u32>(&d_files, &d_transf);

                Stats {
                    source_total: get_match(&caps, "st"),
                    source_files: get_match(&caps, "sf"),
                    source_dirs: get_match(&caps, "sd"),
                    source_links: get_match(&caps, "sl"),
                    source_specials: get_match(&caps, "ss"),
                    destination_total: dest_total,
                    destination_files: dest_files,
                    destination_dirs: get_match(&caps, "dd"),
                    destination_links: get_match(&caps, "dl"),
                    destination_specials: get_match(&caps, "ds"),
                    destination_deleted: get_match(&caps, "dr"),
                    bytes_source: get_match(&caps, "bs"),
                    bytes_transferred: get_match(&caps, "bt"),
                    speed: get_match(&caps, "ts")
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
        let extract_error = |s: &str| -> Option<String> {
            EXPR.captures(s)?
                .name("err")
                .map(|m| {
                    m.as_str().trim()
                        .trim_end_matches('.')
                        .replace("rsync error: ", "")
                        .replace("rsync warning: ", "")
                })
                .map(|mut s| {
                    if let Some(first) = s.get_mut(0..1) {
                        first.make_ascii_uppercase();
                    }

                    s
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
