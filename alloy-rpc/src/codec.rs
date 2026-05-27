//! Tokio-util codec that frames JSON-RPC messages as newline-delimited JSON.
//!
//! Each message is serialised to a single JSON line (no embedded newlines) and
//! terminated with a `\n`.  On decode the codec reads one line at a time and
//! deserialises it as an [`RpcMessage`].

use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder, LinesCodec, LinesCodecError};

use crate::envelope::RpcMessage;
use crate::error::Error;

// ---------------------------------------------------------------------------
// AlloyCodec
// ---------------------------------------------------------------------------

/// A newline-delimited JSON codec for [`RpcMessage`] values.
///
/// Wraps [`LinesCodec`] to handle the framing; serialisation/deserialisation
/// is done with `serde_json`.
pub struct AlloyCodec(LinesCodec);

impl AlloyCodec {
    /// Create a codec with no maximum line length.
    pub fn new() -> Self {
        Self(LinesCodec::new())
    }

    /// Create a codec that returns an error when a line exceeds `max` bytes.
    pub fn new_with_max_length(max: usize) -> Self {
        Self(LinesCodec::new_with_max_length(max))
    }
}

impl Default for AlloyCodec {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Encoder
// ---------------------------------------------------------------------------

impl Encoder<RpcMessage> for AlloyCodec {
    type Error = Error;

    fn encode(&mut self, item: RpcMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let json = serde_json::to_string(&item)?;
        // Delegate to LinesCodec which appends the newline and reserves capacity.
        self.0
            .encode(json, dst)
            .map_err(lines_codec_err_to_error)
    }
}

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

impl Decoder for AlloyCodec {
    type Item = RpcMessage;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.0.decode(src).map_err(lines_codec_err_to_error)? {
            None => Ok(None),
            Some(line) => {
                if line.trim().is_empty() {
                    // Skip blank lines rather than erroring.
                    return Ok(None);
                }
                let msg: RpcMessage = serde_json::from_str(&line)?;
                Ok(Some(msg))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AlloyFramed convenience alias
// ---------------------------------------------------------------------------

/// A [`tokio_util::codec::Framed`] transport wrapping any `AsyncRead + AsyncWrite`
/// with the [`AlloyCodec`].
pub type AlloyFramed<T> = tokio_util::codec::Framed<T, AlloyCodec>;

/// Construct an [`AlloyFramed`] around an I/O object using the default codec.
pub fn framed<T>(io: T) -> AlloyFramed<T>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite,
{
    tokio_util::codec::Framed::new(io, AlloyCodec::new())
}

/// Construct an [`AlloyFramed`] with a maximum frame length.
pub fn framed_with_max_length<T>(io: T, max: usize) -> AlloyFramed<T>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite,
{
    tokio_util::codec::Framed::new(io, AlloyCodec::new_with_max_length(max))
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn lines_codec_err_to_error(e: LinesCodecError) -> Error {
    match e {
        LinesCodecError::MaxLineLengthExceeded => {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "RPC frame exceeded maximum line length",
            ))
        }
        LinesCodecError::Io(io_err) => Error::Io(io_err),
    }
}
