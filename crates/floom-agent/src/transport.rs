//! Transports (Task 25.1, steps 3–4).
//!
//! Two, sharing the session:
//!
//! * [`local_channel`] — an in-memory message pair. A test drives the server
//!   loop itself, so the fixture never leaves the test thread.
//! * [`spawn_loopback`] — a TCP server bound to `127.0.0.1` **only**. The
//!   bind guard refuses a non-loopback host outright, a connection from a
//!   non-loopback peer is dropped, and a dev token gates every request. A
//!   debugging surface that listened on `0.0.0.0` unauthenticated would be a
//!   remote code-execution prompt, so the guard is the transport, not a
//!   convention.
//!
//! Framing is one length-prefixed JSON message each way; a connection serves
//! any number of requests until the client closes.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};

use floom_testkit::TestApp;

use crate::protocol::{
    AgentError, AgentRequest, AgentResponse, Command, PROTOCOL_VERSION, Payload,
};
use crate::session::AgentSession;

/// Largest single message we will read. Requests are small; the cap exists so
/// a hostile length prefix cannot allocate for an attacker.
const MAX_MESSAGE: u32 = 16 * 1024 * 1024;

/// An in-memory request/response pair over which the protocol runs unmodified.
pub fn local_channel() -> (LocalClient, LocalServer) {
    let (requests_tx, requests_rx) = mpsc::channel::<AgentRequest>();
    let (responses_tx, responses_rx) = mpsc::channel::<AgentResponse>();
    (
        LocalClient {
            requests: requests_tx,
            responses: responses_rx,
            next_id: AtomicU64::new(1),
        },
        LocalServer {
            requests: requests_rx,
            responses: responses_tx,
        },
    )
}

/// The client half. Monotonic ids pair sends with responses.
pub struct LocalClient {
    requests: Sender<AgentRequest>,
    responses: Receiver<AgentResponse>,
    next_id: AtomicU64,
}

impl LocalClient {
    /// Send a command without a token (a tokenless server accepts it).
    pub fn send(&self, command: Command) -> u64 {
        self.send_with_token(command, None)
    }

    /// Send a command with a token, for servers that require one.
    pub fn send_with_token(&self, command: Command, token: Option<&str>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.requests
            .send(AgentRequest {
                version: PROTOCOL_VERSION,
                id,
                token: token.map(str::to_string),
                command,
            })
            .expect("the server half of the channel lives as long as this client");
        id
    }

    /// The next response, if the server has answered one.
    pub fn receive(&self) -> Option<AgentResponse> {
        self.responses.try_recv().ok()
    }
}

/// The server half: drain requests, answer each through a session.
pub struct LocalServer {
    requests: Receiver<AgentRequest>,
    responses: Sender<AgentResponse>,
}

impl LocalServer {
    /// The next pending request, if any.
    pub fn next_request(&self) -> Option<AgentRequest> {
        self.requests.try_recv().ok()
    }

    pub fn respond(&self, response: AgentResponse) {
        self.responses
            .send(response)
            .expect("the client half of the channel lives as long as this server");
    }
}

/// Bind a listener for the agent transport.
///
/// The guard is the function, not a convention: a non-loopback host is an
/// error, because a headless inspection surface that listened on a routable
/// address would hand the tree — and through the control commands, the app's
/// input — to the network.
pub fn bind(host: &str, port: u16) -> std::io::Result<TcpListener> {
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    if !addr.ip().is_loopback() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("the agent transport only accepts a loopback bind, got {addr}"),
        ));
    }
    TcpListener::bind(addr)
}

/// Whether a peer may talk to the server. Defence in depth behind [`bind`]:
/// even if the listener were somehow on a routable address, a non-loopback
/// peer is dropped before its first request is read.
pub fn is_loopback_peer(addr: SocketAddr) -> bool {
    addr.ip().is_loopback()
}

/// Run the loopback server on a fresh thread.
///
/// Takes an app *factory*, not an app: the fixture is built inside the server
/// thread, so nothing headless has to be `Send`. Returns the address the
/// server actually bound (port 0 means "choose one").
pub fn spawn_loopback<F>(
    port: u16,
    token: Option<String>,
    make_app: F,
) -> std::io::Result<SocketAddr>
where
    F: FnOnce() -> TestApp + Send + 'static,
{
    let listener = bind("127.0.0.1", port)?;
    let addr = listener.local_addr()?;
    std::thread::spawn(move || {
        let mut app = make_app();
        let mut session = AgentSession::new(&mut app);
        if let Some(token) = token {
            session = session.with_token(token);
        }
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let admitted = stream.peer_addr().map(is_loopback_peer).unwrap_or(false);
            if !admitted {
                continue;
            }
            let _ = serve_connection(&mut stream, &mut session);
        }
    });
    Ok(addr)
}

fn serve_connection(stream: &mut TcpStream, session: &mut AgentSession<'_>) -> std::io::Result<()> {
    while let Some(bytes) = read_message(stream)? {
        let response = match serde_json::from_slice::<AgentRequest>(&bytes) {
            Ok(request) => session.handle(request),
            Err(e) => AgentResponse {
                id: 0,
                result: Err(AgentError::Unsupported(format!("unparsable request: {e}"))),
            },
        };
        write_message(stream, &serde_json::to_vec(&response)?)?;
    }
    Ok(())
}

/// One length-prefixed message, or `None` on a clean EOF before any byte.
fn read_message(stream: &mut TcpStream) -> std::io::Result<Option<Vec<u8>>> {
    let mut prefix = [0u8; 4];
    match stream.read_exact(&mut prefix) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_be_bytes(prefix);
    if len > MAX_MESSAGE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("message of {len} bytes exceeds the {MAX_MESSAGE}-byte cap"),
        ));
    }
    let mut bytes = vec![0u8; len as usize];
    stream.read_exact(&mut bytes)?;
    Ok(Some(bytes))
}

fn write_message(stream: &mut TcpStream, bytes: &[u8]) -> std::io::Result<()> {
    let len = u32::try_from(bytes.len())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "message too long"))?;
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(bytes)?;
    stream.flush()
}

/// A client for the loopback transport. One connection per call: an agent
/// issues a handful of commands per decision, and per-call connections keep
/// the client stateless.
pub struct AgentClient {
    addr: SocketAddr,
    token: Option<String>,
    next_id: AtomicU64,
}

impl AgentClient {
    pub fn connect(addr: SocketAddr, token: Option<String>) -> Self {
        Self {
            addr,
            token,
            next_id: AtomicU64::new(1),
        }
    }

    /// Send one command, wait for its response, return the payload.
    pub fn call(&self, command: Command) -> Result<Payload, AgentError> {
        let mut stream =
            TcpStream::connect(self.addr).map_err(|e| AgentError::Io(e.to_string()))?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = AgentRequest {
            version: PROTOCOL_VERSION,
            id,
            token: self.token.clone(),
            command,
        };
        let bytes = serde_json::to_vec(&request).expect("requests are serialisable");
        write_message(&mut stream, &bytes).map_err(|e| AgentError::Io(e.to_string()))?;
        let bytes = read_message(&mut stream)
            .map_err(|e| AgentError::Io(e.to_string()))?
            .ok_or_else(|| AgentError::Io("connection closed without a response".into()))?;
        let response: AgentResponse = serde_json::from_slice(&bytes)
            .map_err(|e| AgentError::Unsupported(format!("unparsable response: {e}")))?;
        response.result
    }
}
