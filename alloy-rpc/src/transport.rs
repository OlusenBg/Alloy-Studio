//! Async channel abstractions for sending and receiving [`RpcMessage`] values.
//!
//! The primary entry point is [`channel`], which produces a paired
//! [`RpcSender`] / [`RpcReceiver`].  [`RpcSender`] provides higher-level
//! helpers that construct the correct [`RpcMessage`] envelope before
//! forwarding to the underlying channel.  [`PendingRequests`] tracks
//! in-flight requests so that responses can be matched back to their
//! originating callers.

use serde::Serialize;
use tokio::sync::{mpsc, oneshot};

use crate::envelope::{RpcId, RpcMessage};
use crate::error::{Error, RpcError};

// ---------------------------------------------------------------------------
// PendingRequests
// ---------------------------------------------------------------------------

/// A map from numeric request id → response sender.
///
/// When a caller issues a request it inserts a [`oneshot::Sender`] here keyed
/// by the request id.  The dispatch loop looks up and removes the entry when
/// the matching response arrives, completing the future.
pub struct PendingRequests {
    inner: dashmap::DashMap<i64, oneshot::Sender<RpcMessage>>,
}

impl PendingRequests {
    /// Create a new, empty pending-request table.
    pub fn new() -> Self {
        Self {
            inner: dashmap::DashMap::new(),
        }
    }

    /// Register a pending request with `id` and return the corresponding
    /// receiver that will resolve when the response arrives.
    pub fn insert(&self, id: i64) -> oneshot::Receiver<RpcMessage> {
        let (tx, rx) = oneshot::channel();
        self.inner.insert(id, tx);
        rx
    }

    /// Complete a pending request by delivering `msg` to its waiter.
    ///
    /// Returns `true` if an entry was found and the message delivered,
    /// `false` if no pending request with `id` exists.
    pub fn complete(&self, id: i64, msg: RpcMessage) -> bool {
        if let Some((_, tx)) = self.inner.remove(&id) {
            // If the receiver was dropped we can ignore the error.
            let _ = tx.send(msg);
            true
        } else {
            false
        }
    }

    /// Remove and discard a pending request without delivering a response.
    pub fn cancel(&self, id: i64) {
        self.inner.remove(&id);
    }

    /// Return the number of in-flight requests.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Return `true` if there are no in-flight requests.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Default for PendingRequests {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// RpcSender
// ---------------------------------------------------------------------------

/// A cloneable handle for sending [`RpcMessage`] values over an mpsc channel.
///
/// All higher-level helpers serialise their arguments via `serde_json` and
/// wrap the result in the appropriate [`RpcMessage`] envelope.
#[derive(Clone)]
pub struct RpcSender {
    tx: mpsc::Sender<RpcMessage>,
}

impl RpcSender {
    fn new(tx: mpsc::Sender<RpcMessage>) -> Self {
        Self { tx }
    }

    // --- Core send -----------------------------------------------------------

    /// Send a raw [`RpcMessage`], waiting until capacity is available.
    pub async fn send(&self, msg: RpcMessage) -> Result<(), Error> {
        self.tx.send(msg).await.map_err(|_| Error::ChannelClosed)
    }

    /// Attempt to send a raw [`RpcMessage`] without blocking.
    ///
    /// Returns [`Error::ChannelClosed`] if the channel is closed, or
    /// [`Error::Io`] with `WouldBlock` if the channel is full.
    pub fn try_send(&self, msg: RpcMessage) -> Result<(), Error> {
        self.tx.try_send(msg).map_err(|e| match e {
            mpsc::error::TrySendError::Closed(_) => Error::ChannelClosed,
            mpsc::error::TrySendError::Full(_) => Error::Io(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "RPC send channel is full",
            )),
        })
    }

    // --- Typed helpers -------------------------------------------------------

    /// Send a JSON-RPC request with a numeric id.
    ///
    /// The `params` value is serialised to JSON; serialisation errors are
    /// returned as [`Error::Json`].
    pub async fn send_request(
        &self,
        id: i64,
        method: &str,
        params: impl Serialize,
    ) -> Result<(), Error> {
        let msg = RpcMessage::new_request(RpcId::Number(id), method, params);
        self.send(msg).await
    }

    /// Send a JSON-RPC notification (no id, no response expected).
    pub async fn send_notification(
        &self,
        method: &str,
        params: impl Serialize,
    ) -> Result<(), Error> {
        let msg = RpcMessage::new_notification(method, params);
        self.send(msg).await
    }

    /// Send a successful response to the request identified by `id`.
    pub async fn send_response(&self, id: i64, result: impl Serialize) -> Result<(), Error> {
        let msg = RpcMessage::new_ok(RpcId::Number(id), result);
        self.send(msg).await
    }

    /// Send an error response to the request identified by `id`.
    pub async fn send_error_response(&self, id: i64, error: RpcError) -> Result<(), Error> {
        let msg = RpcMessage::new_err(RpcId::Number(id), error);
        self.send(msg).await
    }

    // --- Capacity / state ----------------------------------------------------

    /// Return the number of messages that can still be buffered before the
    /// channel becomes full.
    pub fn capacity(&self) -> usize {
        self.tx.capacity()
    }

    /// Return `true` if the receiving end of the channel has been dropped.
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

// ---------------------------------------------------------------------------
// RpcReceiver
// ---------------------------------------------------------------------------

/// The receiving half of an mpsc RPC channel.
pub struct RpcReceiver {
    rx: mpsc::Receiver<RpcMessage>,
}

impl RpcReceiver {
    fn new(rx: mpsc::Receiver<RpcMessage>) -> Self {
        Self { rx }
    }

    /// Receive the next message, or `None` if all senders have been dropped.
    pub async fn recv(&mut self) -> Option<RpcMessage> {
        self.rx.recv().await
    }

    /// Try to receive a message without blocking.
    pub fn try_recv(&mut self) -> Result<RpcMessage, mpsc::error::TryRecvError> {
        self.rx.try_recv()
    }

    /// Close the receiver, preventing any further messages from being sent.
    pub fn close(&mut self) {
        self.rx.close();
    }
}

// ---------------------------------------------------------------------------
// channel constructor
// ---------------------------------------------------------------------------

/// Create a bounded mpsc channel with the given `capacity` and return a
/// paired ([`RpcSender`], [`RpcReceiver`]).
pub fn channel(capacity: usize) -> (RpcSender, RpcReceiver) {
    let (tx, rx) = mpsc::channel(capacity);
    (RpcSender::new(tx), RpcReceiver::new(rx))
}
