//! Run a BuildLoop for `shell.nix`, watching for input file changes.
//! Can be used together with `direnv`.
use crate::ops::{ok, OpResult};

use std::path::{Path, PathBuf};

use crate::communicate::client;
use crate::communicate::Ping;

/// See the documentation for lorri::cli::Command::Shell for more
/// details.
pub fn main(nix_file: PathBuf) -> OpResult {
    // TODO: set up socket path, make it settable by the user
    client::ping(None)
        // TODO
        .connect(Path::new("/tmp/lorri-socket"))
        .unwrap()
        .write(&Ping { nix_file: nix_file })
        .unwrap();
    ok()
}

// /// Handle the ping
// // the ReadWriter here has to be the inverse of the `Client.ping()`, which is `ReadWriter<!, Ping>`
// fn ping(rw: ReadWriter<Ping, NoMessage>) {
//     // tx.send(rw.read())
// }
