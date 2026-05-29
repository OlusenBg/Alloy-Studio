//! TCP server that accepts robot connections and feeds packets into `TelemetryStream`.

use std::net::SocketAddr;
use std::sync::Arc;

use futures::SinkExt;
use futures::StreamExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;
use tracing::{debug, info, warn};

use crate::protocol::{Packet, PacketCodec, PacketKind};
use crate::stream::TelemetryStream;

/// A TCP server that listens for robot telemetry connections.
pub struct TelemetryServer {
    listener: TcpListener,
    stream: Arc<TelemetryStream>,
}

impl TelemetryServer {
    /// Bind to `0.0.0.0:{port}`.
    ///
    /// Returns `(server, shared_stream)`. Consumers should subscribe to `shared_stream`
    /// before calling `run()` to ensure they receive all packets.
    pub async fn bind(port: u16) -> anyhow::Result<(Self, Arc<TelemetryStream>)> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        let stream = Arc::new(TelemetryStream::new());
        let addr = listener.local_addr()?;
        info!("TelemetryServer listening on {}", addr);
        Ok((
            Self {
                listener,
                stream: Arc::clone(&stream),
            },
            stream,
        ))
    }

    /// Bind to `0.0.0.0:{port}` and use an existing `TelemetryStream`.
    ///
    /// This allows callers to share the same stream between the server and other
    /// components (such as the UI bridge) by providing the stream up front.
    pub async fn bind_with_stream(port: u16, stream: Arc<TelemetryStream>) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        let addr = listener.local_addr()?;
        info!("TelemetryServer listening on {}", addr);
        Ok(Self { listener, stream })
    }

    /// Return the local address this server is bound to.
    pub fn local_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    /// Accept loop: spawns a task per connection.
    ///
    /// Runs until the listener errors (e.g. is dropped externally).
    pub async fn run(self) -> anyhow::Result<()> {
        let telemetry = self.stream;
        loop {
            match self.listener.accept().await {
                Ok((socket, addr)) => {
                    info!("Accepted telemetry connection from {}", addr);
                    let telem = Arc::clone(&telemetry);
                    tokio::spawn(async move {
                        Self::handle_connection(socket, addr, telem).await;
                    });
                }
                Err(e) => {
                    warn!("Accept error: {}", e);
                    return Err(e.into());
                }
            }
        }
    }

    /// Drive a single robot connection: decode packets, respond to pings, publish the rest.
    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        telemetry: Arc<TelemetryStream>,
    ) {
        let mut framed = Framed::new(stream, PacketCodec);

        info!("Connection opened: {}", addr);

        while let Some(result) = framed.next().await {
            match result {
                Ok(packet) => {
                    debug!("Received packet from {}: {:?}", addr, packet.kind);

                    // Respond to pings
                    if let PacketKind::Ping { seq } = packet.kind {
                        let pong = Packet::now(PacketKind::Pong { seq });
                        if let Err(e) = framed.send(pong).await {
                            warn!("Failed to send Pong to {}: {}", addr, e);
                            break;
                        }
                        continue;
                    }

                    telemetry.publish(packet);
                }
                Err(e) => {
                    warn!("Decode error from {}: {}", addr, e);
                    break;
                }
            }
        }

        info!("Connection closed: {}", addr);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Packet, PacketCodec};
    use futures::SinkExt;
    use tokio::net::TcpStream;
    use tokio_util::codec::Framed;

    /// Bind the server on port 0 (OS-assigned), connect, and verify a packet is received.
    #[tokio::test]
    async fn server_receives_battery_packet() {
        let (server, stream) = TelemetryServer::bind(0).await.unwrap();
        let addr = server.local_addr().unwrap();

        // Run the server in a background task
        tokio::spawn(server.run());

        // Connect a client
        let tcp = TcpStream::connect(addr).await.unwrap();
        let mut client = Framed::new(tcp, PacketCodec);

        // Send a battery packet
        client.send(Packet::battery(13.2)).await.unwrap();

        // Give the server a moment to process
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let records = stream.history().get("battery_volts");
        assert!(!records.is_empty());
        assert!((records[0].value - 13.2).abs() < 1e-9);
    }

    #[tokio::test]
    async fn server_responds_to_ping() {
        let (server, _stream) = TelemetryServer::bind(0).await.unwrap();
        let addr = server.local_addr().unwrap();

        tokio::spawn(server.run());

        let tcp = TcpStream::connect(addr).await.unwrap();
        let mut client = Framed::new(tcp, PacketCodec);

        // Send a ping
        client.send(Packet::ping(99)).await.unwrap();

        // Expect a pong back
        let response = tokio::time::timeout(tokio::time::Duration::from_millis(200), client.next())
            .await
            .expect("timed out waiting for pong")
            .expect("stream ended")
            .expect("decode error");

        match response.kind {
            PacketKind::Pong { seq } => assert_eq!(seq, 99),
            other => panic!("expected Pong, got {:?}", other),
        }
    }
}
