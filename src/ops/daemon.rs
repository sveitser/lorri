//! Run a BuildLoop for `shell.nix`, watching for input file changes.
//! Can be used together with `direnv`.
use crate::build_loop::{self, BuildLoop};
use crate::ops::{ok, OpResult};
use crate::project::Project;
use crate::roots::Roots;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{mpsc, Mutex};
use std::thread;

use std::path::Path;

use crate::communicate::daemon;
use crate::communicate::{CommunicationType, NoMessage, Ping, ReadError, ReadWriter};

/// Instructs the daemon to start a build
struct StartBuild {
    nix_file: PathBuf,
}

/// See the documentation for lorri::cli::Command::Shell for more
/// details.
pub fn main() -> OpResult {
    let listener = daemon::Daemon::new(Path::new("/tmp/lorri-socket"))
        // TODO
        .unwrap();
    // TODO: set up socket path, make it settable by the user
    let daemon = Daemon::new();

    let (tx, rx) = mpsc::channel();

    let _accept_loop_handle = thread::spawn(move || loop {
        let tx2 = tx.clone();
        let _handle = listener
            .accept(|unix_stream, comm_type| match comm_type {
                CommunicationType::Ping => ping(ReadWriter::new(unix_stream), tx2),
            })
            // TODO
            .unwrap();
    });

    let _start_build_loop_handle = thread::spawn(|| {
        for start_build in rx {
            daemon.start_build(start_build.nix_file)
        }
    });

    for msg in daemon.build_events_rx {
        println!("{:#?}", msg);
    }

    ok()

    // TODO: join all accept handles & accept_loop_handle
    // handle.join().unwrap();
}

struct Daemon {
    // TODO: PathBuf is a nix file
    pub handlers: HashMap<PathBuf, std::thread::JoinHandle<()>>,
    pub build_events_tx: mpsc::Sender<build_loop::Event>,
    pub build_events_rx: mpsc::Receiver<build_loop::Event>,
}

impl Daemon {
    pub fn new() -> Daemon {
        let (tx, rx) = mpsc::channel();
        Daemon {
            handlers: HashMap::new(),
            build_events_rx: rx,
            build_events_tx: tx,
        }
    }

    /// Start a build in a new thread
    pub fn start_build(&mut self, nix_file: PathBuf) {
        // TODO: refactor Project/Roots stuff, a little bit too complicated
        // TODO: all these clones are not needed
        let project = Project::load(nix_file.clone(), Project::default_gc_root_dir()).unwrap();
        // TODO
        let roots = Roots::from_project(&project).unwrap();
        let mut build_loop = BuildLoop::new(nix_file.clone(), roots);

        let build_thread = {
            let tx = self.build_events_tx.clone();
            thread::spawn(move || {
                build_loop.forever(tx);
            })
        };
        let _ = self.handlers.insert(nix_file, build_thread);
    }
}

/// Handle the ping
// the ReadWriter here has to be the inverse of the `Client.ping()`, which is `ReadWriter<!, Ping>`
fn ping(rw: ReadWriter<Ping, NoMessage>, build_chan: mpsc::Sender<StartBuild>) {
    // TODO: read timeout
    let ping: Result<Ping, ReadError> = rw.read(None);
    match ping {
        Err(e) => eprintln!("didn’t receive a ping!! {:?}", e),
        Ok(p) => {
            eprintln!("pinged with {}", p.nix_file.display());
            build_chan
                .send(StartBuild {
                    nix_file: p.nix_file,
                })
                .expect("StartBuild channel closed")
        }
    }
}
