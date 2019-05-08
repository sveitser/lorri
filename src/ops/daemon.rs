//! Run a BuildLoop for `shell.nix`, watching for input file changes.
//! Can be used together with `direnv`.
use crate::ops::{ok, OpResult};

use std::path::Path;

use crate::communicate::daemon::{AcceptError, BindError, Daemon};
use crate::communicate::{CommunicationType, NoMessage, Ping, ReadError, ReadWriter};

/// See the documentation for lorri::cli::Command::Shell for more
/// details.
pub fn main() -> OpResult {
    let daemon = Daemon::new(Path::new("/tmp/lorri-socket"))
        .unwrap();
    // TODO: set up socket path, make it settable by the user
    loop {
        let _handle =
            daemon.accept(|unix_stream, comm_type| match comm_type {
                CommunicationType::Ping => ping(ReadWriter::new(unix_stream)),
            })
            .unwrap();
    }

    // TODO: collect all handles and join at the end
    // handle.join().unwrap();

}

/// Handle the ping
// the ReadWriter here has to be the inverse of the `Client.ping()`, which is `ReadWriter<!, Ping>`
fn ping(rw: ReadWriter<Ping, NoMessage>) {
    let ping: Result<Ping, ReadError> = rw.read(None);
    match ping {
        Err(e) => eprintln!("didn’t receive a ping!! {:?}", e),
        Ok(p) => eprintln!("pinged with {}", p.nix_file.display()),
    }
}
