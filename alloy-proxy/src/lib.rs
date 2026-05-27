//! `alloy-proxy` — Remote filesystem proxy: IPC server, file ops, file watcher, digest cache.

pub mod digest;
pub mod error;
pub mod fs_ops;
pub mod handler;
pub mod server;
pub mod watcher;

pub use digest::DigestCache;
pub use error::ProxyError;
pub use handler::RequestHandler;
pub use server::ProxyServer;
pub use watcher::FileWatcher;
