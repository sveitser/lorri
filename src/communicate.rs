//! TODO

extern crate bincode;

use std::io::Write;
use std::marker::PhantomData;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Wrapper around a socket that can send and receive structured messages.
///
/// `timeout` arguments set the socket timeout before reading/writing.
/// `None` means no timeout.
pub struct ReadWriter<R, W> {
    // where R: serde::Deserialize {
    socket: UnixStream,
    phantom_r: PhantomData<R>,
    phantom_w: PhantomData<W>,
}

#[derive(Debug)]
pub enum ReadError {
    Deserialize(bincode::Error),
    Timeout,
}

// TODO: combine with ReadError?
#[derive(Debug)]
pub enum WriteError {
    Serialize(bincode::Error),
    Timeout,
}
#[derive(Debug)]
pub enum ReadWriteError {
    R(ReadError),
    W(WriteError),
}

/// A timeout. `None` means forever.
type Timeout = Option<Duration>;

fn into_bincode_io_error<T>(res: std::io::Result<T>) -> bincode::Result<T> {
    res.map_err(|e| Box::new(bincode::ErrorKind::Io(e)))
}

/// Small helper to flush the socket after a message has been written.
fn serialize_and_flush<T>(mut socket: &UnixStream, mes: &T) -> bincode::Result<()>
where
    T: serde::Serialize,
{
    bincode::serialize_into(socket, mes)?;
    into_bincode_io_error(socket.flush())
}

impl<R, W> ReadWriter<R, W> {
    /// Create from a unix socket.
    // TODO: &mut UnixStream
    pub fn new(socket: UnixStream) -> ReadWriter<R, W> {
        ReadWriter {
            socket: socket,
            phantom_r: PhantomData,
            phantom_w: PhantomData,
        }
    }

    // socket timeout must not be 0 (or it crashes)
    fn sanitize_timeout(timeout: Timeout) -> Timeout {
        timeout.filter(|d| d != &Duration::new(0, 0))
    }

    /// Send a message to the other side and wait for a reply.
    /// The timeout counts for the whole roundtrip.
    pub fn communicate(&self, timeout: Timeout, mes: &W) -> Result<R, ReadWriteError>
    where
        R: serde::de::DeserializeOwned,
        W: serde::Serialize,
    {
        let i = Instant::now();
        self.write(timeout, mes).map_err(ReadWriteError::W)?;
        // remove the passed time from timeout
        let new_timeout = timeout.and_then(|d| d.checked_sub(i.elapsed()));
        self.read(new_timeout).map_err(ReadWriteError::R)
    }

    /// Check if the underlying socket timed out when serializing/deserilizing.
    fn is_timed_out(e: &bincode::ErrorKind) -> bool {
        match e {
            bincode::ErrorKind::Io(io) => match io.kind() {
                std::io::ErrorKind::TimedOut => true,
                _ => false,
            },
            _ => false,
        }
    }

    /// Wait for a message to arrive.
    pub fn read(&self, timeout: Timeout) -> Result<R, ReadError>
    where
        R: serde::de::DeserializeOwned,
    {
        into_bincode_io_error(
            self.socket
                .set_read_timeout(Self::sanitize_timeout(timeout)),
        )
        .map_err(ReadError::Deserialize)?;

        // XXX: “If this returns an Error, `reader` may be in an invalid state”.
        // what the heck does that mean.
        bincode::deserialize_from(&self.socket).map_err(|e| {
            if Self::is_timed_out(&e) {
                ReadError::Timeout
            } else {
                ReadError::Deserialize(e)
            }
        })
    }

    /// Send a message to the other side.
    pub fn write(&self, timeout: Timeout, mes: &W) -> Result<(), WriteError>
    where
        W: serde::Serialize,
    {
        into_bincode_io_error(
            self.socket
                .set_write_timeout(Self::sanitize_timeout(timeout)),
        )
        .map_err(WriteError::Serialize)?;

        serialize_and_flush(&self.socket, mes).map_err(|e| {
            if Self::is_timed_out(&e) {
                WriteError::Timeout
            } else {
                WriteError::Serialize(e)
            }
        })
    }
}

/// Enum of all communication modes the lorri daemon supports.
#[derive(Serialize, Deserialize)]
pub enum CommunicationType {
    /// Ping the daemon from a project to tell it to watch & evaluate
    Ping,
    // /// Listen for events on the daemon
    // DaemonListener,
    // /// Get the daemon status
    // DaemonStatus,
}

#[derive(Serialize, Deserialize)]
pub struct Ping {
    pub nix_file: PathBuf,
}
pub enum NoMessage {}
// pub struct DaemonEvent {}
// pub struct DaemonStatus {}

pub mod daemon {
    use super::*;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::Path;
    use std::time::Duration;

    /// The server only ever answers if a connection attempt was accepted.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct ConnectionAccepted();

    /// Server-side part of a socket transmission,
    /// listening for incoming messages.
    pub struct Daemon {
        listener: UnixListener,
        timeout: Timeout,
    }

    #[derive(Debug)]
    pub enum AcceptError {
        // something went wrong in the `accept()` syscall
        Accept(std::io::Error),
        Message(ReadWriteError),
    }

    #[derive(Debug)]
    pub struct BindError(std::io::Error);

    impl Daemon {
        // TODO: remove socket file when done
        pub fn new(socket_path: &Path) -> Result<Daemon, BindError> {
            Ok(Daemon {
                listener: UnixListener::bind(socket_path).map_err(BindError)?,
                // TODO: set some timeout?
                timeout: None,
            })
        }

        /// Accept a new connection on the socket,
        /// read the communication type and then delegate to the
        /// corresponding handling subroutine.
        // TODO: this should probably be outside of this module
        // TODO: something needs to loop over accept
        pub fn accept<F: 'static>(
            &self,
            handler: F,
        ) -> Result<std::thread::JoinHandle<()>, AcceptError>
        where
            F: FnOnce(UnixStream, CommunicationType) -> (),
            F: std::marker::Send,
        {
            // - socket accept
            let (mut unix_stream, _) = self.listener.accept().map_err(AcceptError::Accept)?;
            // - read first message as a `CommunicationType`
            // TODO: move to this
            // let commType: CommunicationType = ReadWriter::<CommunicationType, ConnectionAccepted>::new(unixStream)
            //     .react(self.timeout, |commType| -> ConnectionAccepted())
            //     .map_err(AcceptError::Message)?;
            let comm_type: CommunicationType = bincode::deserialize_from(&unix_stream)
                .map_err(|e| AcceptError::Message(ReadWriteError::R(ReadError::Deserialize(e))))?;
            bincode::serialize_into(&unix_stream, &ConnectionAccepted())
                .map_err(|e| AcceptError::Message(ReadWriteError::W(WriteError::Serialize(e))))?;
            unix_stream.flush().map_err(AcceptError::Accept)?;
            // spawn a thread with the accept handler
            Ok(std::thread::spawn(move || handler(unix_stream, comm_type)))
        }
    }

    // pub fn daemonListener

}

/// Clients that can talk to the daemon.
///
/// `R` is the type of messages this client reads.
/// `W` is the type of messages this client writes.
pub mod client {
    use super::*;
    use std::path::Path;

    pub struct Client<R, W> {
        comm_type: CommunicationType,
        rw: Option<ReadWriter<R, W>>,
        timeout: Timeout,
    }

    #[derive(Debug)]
    pub enum Error {
        NotConnected,
        Message(ReadWriteError),
    }

    #[derive(Debug)]
    pub enum InitError {
        SocketConnect(std::io::Error),
        ServerHandshake(ReadWriteError),
    }

    // TODO: builder pattern for timeouts?

    impl<R, W> Client<R, W> {
        /// “Bake” a Client, aka set its communication type (and message type arguments).
        fn bake(timeout: Timeout, comm_type: CommunicationType) -> Client<R, W> {
            Client {
                comm_type: comm_type,
                rw: None,
                timeout: timeout,
            }
        }

        /// Connect to the daemon.
        pub fn connect(self, socket_path: &Path) -> Result<Client<R, W>, InitError> {
            // TODO: check if the file exists and is a socket
            // - connect to `socket_path`
            let mut socket = UnixStream::connect(socket_path).map_err(InitError::SocketConnect)?;
            // - send initial message with the CommunicationType
            // - wait for server to acknowledge connect
            // TODO: use this
            // let c: daemon::ConnectionAccepted = ReadWriter::new(socket)
            //     .communicate(self.timeout, &self.comm_type)
            //     .map_err(InitError::ServerHandshake)?;
            bincode::serialize_into(&socket, &self.comm_type);
            socket.flush().unwrap();
            let _: daemon::ConnectionAccepted =
                bincode::deserialize_from(&socket).expect("hurr durr");
            Ok(Client {
                comm_type: self.comm_type,
                rw: Some(ReadWriter::new(socket)),
                timeout: self.timeout,
            })
        }

        // mirror the readwriter functions here
        pub fn read(self) -> Result<R, Error>
        where
            R: serde::de::DeserializeOwned,
        {
            self.rw
                .ok_or(Error::NotConnected)?
                .read(self.timeout)
                .map_err(|e| Error::Message(ReadWriteError::R(e)))
        }

        pub fn write(self, mes: &W) -> Result<(), Error>
        where
            W: serde::Serialize,
        {
            self.rw
                .ok_or(Error::NotConnected)?
                .write(self.timeout, mes)
                .map_err(|e| Error::Message(ReadWriteError::W(e)))
        }
    }

    /// Client for the `Ping` communication type
    pub fn ping(timeout: Timeout) -> Client<NoMessage, Ping> {
        Client::bake(timeout, CommunicationType::Ping)
    }

    // /// Client for the `DaemonListener` communication type
    // pub fn daemon_listener(timeout: Timeout) -> Client<DaemonEvent, NoMessage> {
    //     Client::bake(timeout, CommunicationType::DaemonListener)
    // }

    // /// Client for the `DaemonStatus` communication type
    // pub fn daemon_status(timeout: Timeout) -> Client<DaemonStatus, NoMessage> {
    //     Client::bake(timeout, CommunicationType::DaemonStatus)
    // }

}
