use std::cell::Cell;
use std::sync::{OnceLock, LazyLock};
use std::io;
use std::process::Stdio;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::prelude::{ObjectExt, StaticType};
use glib::clone;
use glib::subclass::Signal;

use tokio::runtime::Runtime as TkRuntime;
use tokio::process::Command as TkCommand;
use tokio::io::AsyncReadExt as _;
use nix::sys::signal as nix_signal;
use nix::unistd::Pid as NixPid;
use regex::{Regex, Captures};
use itertools::{Itertools, izip};

//------------------------------------------------------------------------------
// ENUM: Msg
//------------------------------------------------------------------------------
#[derive(Debug, Default, PartialEq)]
#[repr(u32)]
enum Msg {
    #[default]
    None,
    Start(Option<i32>),
    Message(String),
    Progress(String, String, f64),
    Stats(String),
    Error(String),
    Exit(i32)
}

//------------------------------------------------------------------------------
// STRUCT: StatsRow
//------------------------------------------------------------------------------
#[derive(Default, Debug, Clone)]
pub struct StatsRow {
    pub total: String,
    pub files: String,
    pub dirs: String,
    pub links: String,
    pub specials: String
}

//------------------------------------------------------------------------------
// STRUCT: StatsBytes
//------------------------------------------------------------------------------
#[derive(Default, Debug, Clone)]
pub struct StatsBytes {
    pub source: String,
    pub transferred: String,
    pub speed: String
}

//------------------------------------------------------------------------------
// STRUCT: Stats
//------------------------------------------------------------------------------
#[derive(Default, Debug, Clone, glib::Boxed)]
#[boxed_type(name = "Stats", nullable)]
pub struct Stats {
    pub transferred: String,
    pub source: StatsRow,
    pub created: StatsRow,
    pub deleted: StatsRow,
    pub bytes: StatsBytes
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
                            Option::<Stats>::static_type(),
                            Option::<String>::static_type(),
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
    // Stats function
    //---------------------------------------
    fn stats(&self, stats: &[String]) -> Option<Stats> {
        static EXPR: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?x)
                Number\s*of\s*files:\s*(?P<st>[\d,.]+)\s*\(?(?:reg:\s*(?P<sf>[\d,.]+))?,?\s*(?:dir:\s*(?P<sd>[\d,.]+))?,?\s*(?:link:\s*(?P<sl>[\d,.]+))?,?\s*(?:special:\s*(?P<ss>[\d,.]+))?,?\s*\)?\n
                Number\s*of\s*created\s*files:\s*(?P<ct>[\d,.]+)\s*\(?(?:reg:\s*(?P<cf>[\d,.]+))?,?\s*(?:dir:\s*(?P<cd>[\d,.]+))?,?\s*(?:link:\s*(?P<cl>[\d,.]+))?,?\s*(?:special:\s*(?P<cs>[\d,.]+))?,?\s*\)?\n
                Number\s*of\s*deleted\s*files:\s*(?P<dt>[\d,.]+)\s*\(?(?:reg:\s*(?P<df>[\d,.]+))?,?\s*(?:dir:\s*(?P<dd>[\d,.]+))?,?\s*(?:link:\s*(?P<dl>[\d,.]+))?,?\s*(?:special:\s*(?P<ds>[\d,.]+))?,?\s*\)?\n
                Number\s*of\s*regular\s*files\s*transferred:\s*(?P<tn>[\d,.]+)\n
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
                        .map(|m| m.as_str().trim_end_matches(',').trim().to_owned())
                        .unwrap_or_default()
                };

                Stats {
                    transferred: get_match(&caps, "tn"),
                    source: StatsRow {
                        total: get_match(&caps, "st"),
                        files: get_match(&caps, "sf"),
                        dirs: get_match(&caps, "sd"),
                        links: get_match(&caps, "sl"),
                        specials: get_match(&caps, "ss")
                    },
                    created: StatsRow {
                        total: get_match(&caps, "ct"),
                        files: get_match(&caps, "cf"),
                        dirs: get_match(&caps, "cd"),
                        links: get_match(&caps, "cl"),
                        specials: get_match(&caps, "cs")
                    },
                    deleted: StatsRow {
                        total: get_match(&caps, "dt"),
                        files: get_match(&caps, "df"),
                        dirs: get_match(&caps, "dd"),
                        links: get_match(&caps, "dl"),
                        specials: get_match(&caps, "ds")
                    },
                    bytes: StatsBytes {
                        source: get_match(&caps, "bs"),
                        transferred: get_match(&caps, "bt"),
                        speed:format!("{}B/s", get_match(&caps, "ts"))
                    }
                }
            })
    }

    //---------------------------------------
    // Error function
    //---------------------------------------
    fn error(&self, code: i32, errors: &[String]) -> Option<String> {
        static EXPR: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(?P<err>[^(]*).*")
                .expect("Failed to compile Regex")
        });

        // Return none if more that two error strings
        let (err_detail, err_main) = errors.iter().collect_tuple()?;

        // Get error string
        match code {
            // Terminated by user
            20 => { Some(String::from("Terminated by user")) }

            // Usage error | partial transfer due to error
            1 | 23 => {
                EXPR.captures(err_detail)?
                    .name("err")
                    .map(|m| {
                        let mut chars = m.as_str().trim().trim_end_matches('.').chars();

                        chars.next()
                            .map_or_else(
                                String::new,
                                |first| first.to_uppercase().collect::<String>() + chars.as_str()
                            )
                    })
                    .or_else(|| {
                        EXPR.captures(err_main)?
                            .name("err")
                            .map(|m| m.as_str().trim().replace("rsync error: ", ""))
                    })
            }

            // Other error
            _ => {
                EXPR.captures(err_main)?
                    .name("err")
                    .map(|m| m.as_str().trim().replace("rsync error: ", ""))
            }
        }
    }

    //---------------------------------------
    // Start function
    //---------------------------------------
    pub fn start(&self, args: Vec<String>) {
        const BUFFER_SIZE: usize = 16384;

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
                    .send(Msg::Start(rsync_process.id().map(|id| id as i32)))
                    .await
                    .expect("Could not send through channel");

                // Get handles to read rsync stdout and stderr
                let mut stdout = rsync_process.stdout.take()
                    .ok_or_else(|| io::Error::other("Could not get stdout"))?;

                let mut stderr = rsync_process.stderr.take()
                    .ok_or_else(|| io::Error::other("Could not get stderr"))?;

                // Create buffers to read stdout and stderr
                let mut buffer_stdout = [0u8; BUFFER_SIZE];
                let mut buffer_stderr = [0u8; BUFFER_SIZE];

                let mut overflow = String::with_capacity(BUFFER_SIZE);

                let mut stats = false;

                'read: loop {
                    tokio::select! {
                        // Read stdout when available
                        result = stdout.read(&mut buffer_stdout) => {
                            let bytes = result?;

                            if bytes == 0 {
                                continue 'read;
                            }

                            if bytes >= BUFFER_SIZE {
                                overflow = String::from_utf8_lossy(&buffer_stdout[..bytes])
                                    .into_owned();

                                continue 'read;
                            }

                            let mut text = String::from_utf8_lossy(&buffer_stdout[..bytes])
                                .into_owned();

                            if !overflow.is_empty() {
                                text.insert_str(0, &overflow);

                                overflow.clear();
                            }

                            for line in text.split_terminator('\n') {
                                if line.is_empty() {
                                    continue;
                                }

                                if line.starts_with('\r') {
                                    for chunk in line.split_terminator('\r') {
                                        let vec: Vec<&str> = chunk
                                            .split_whitespace()
                                            .collect();

                                        let values = izip!(
                                            vec.first().map(|&s| s.to_owned()),
                                            vec.get(2).map(|&s| s.to_owned()),
                                            vec.get(1).and_then(|s| {
                                                s.trim_end_matches('%').parse::<f64>().ok()
                                            })
                                        ).next();

                                        if let Some((size, speed, progress)) = values {
                                            sender
                                                .send(Msg::Progress(size, speed, progress))
                                                .await
                                                .expect("Could not send through channel");
                                        }
                                    }
                                } else if line.starts_with("Number of files:") || stats {
                                    stats = true;

                                    sender
                                        .send(Msg::Stats(line.to_owned()))
                                        .await
                                        .expect("Could not send through channel");
                                } else {
                                    sender
                                        .send(Msg::Message(line.to_owned()))
                                        .await
                                        .expect("Could not send through channel");
                                }
                            }
                        }

                        // Read stderr when available
                        result = stderr.read(&mut buffer_stderr) => {
                            let bytes = result?;

                            if bytes != 0 {
                                let error = String::from_utf8_lossy(&buffer_stderr[..bytes]);

                                for chunk in error.split_terminator('\n') {
                                    if !chunk.is_empty() {
                                        sender
                                            .send(Msg::Error(chunk.to_owned()))
                                            .await
                                            .expect("Could not send through channel");
                                    }
                                }
                            }
                        }

                        // Process exit
                        result = rsync_process.wait() => {
                            let status = result?;

                            sender
                                .send(Msg::Exit(status.code().unwrap_or(1)))
                                .await
                                .expect("Could not send through channel");

                            break 'read;
                        }
                    }
                }

                Ok::<(), io::Error>(())
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
                        Msg::Start(id) => {
                            imp.id.set(id);
                            process.set_running(true);

                            process.emit_by_name::<()>("start", &[]);
                        }

                        Msg::Message(message) => {
                            process.emit_by_name::<()>("message", &[&message]);

                            messages.push(message);
                        }

                        Msg::Progress(size, speed, progress) => {
                            process.emit_by_name::<()>("progress", &[
                                &size,
                                &speed,
                                &progress
                            ]);
                        }

                        Msg::Stats(stat) => {
                            stats.push(stat);
                        }

                        Msg::Error(error) => {
                            errors.push(error);
                        }

                        Msg::Exit(code) => {
                            process.set_running(false);
                            process.set_paused(false);
                            imp.id.set(None);

                            process.emit_by_name::<()>("exit", &[
                                &code,
                                &process.stats(&stats),
                                &process.error(code, &errors),
                                &messages
                            ]);
                        }

                        Msg::None => {}
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
}

impl Default for RsyncProcess {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
