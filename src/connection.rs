use std::{io, net::SocketAddr, sync::Arc};

use async_bincode::tokio::AsyncBincodeWriter;
use bincode::Options;
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use quinn::{Incoming, NewConnection, TransportConfig};
use rcgen::RcgenError;
use tokio::io::AsyncReadExt;

use crate::oneshot_map::OneshotMap;

struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum ConnectionError {
    CertGenerationError(RcgenError),
    CertSerializationError(RcgenError),
    InvalidLocalCert(rustls::Error),
    BindError(io::Error),
    InvalidClientConfig(quinn::ConnectError),
    FailedToConnect(quinn::ConnectionError),
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum StreamError {
    FailedToOpen(quinn::ConnectionError),
    FailedToSendID(bincode::ErrorKind),
}

pub struct Connection {
    listen_addr: SocketAddr,
    id: Vec<u32>,
    num_children: u32,
    num_streams: u32,
    state: Arc<ConnectionState>,
    recv_mapper: Arc<OneshotMap<Vec<u32>, quinn::RecvStream>>,
}

struct ConnectionState {
    connection: quinn::Connection,
}

impl Connection {
    pub async fn new(
        listen_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> Result<Self, ConnectionError> {
        let id = Vec::new();

        let mut transport_config = TransportConfig::default();
        transport_config.max_idle_timeout(None); // TODO: Can we get low gear to work with idle timeout?
        transport_config.max_concurrent_uni_streams(1024u32.into());
        let transport_config = Arc::new(transport_config);

        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .map_err(ConnectionError::CertGenerationError)?;
        let key = rustls::PrivateKey(cert.serialize_private_key_der());
        let cert = vec![rustls::Certificate(
            cert.serialize_der()
                .map_err(ConnectionError::CertSerializationError)?,
        )];
        let server_crypto = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert, key)
            .map_err(ConnectionError::InvalidLocalCert)?;
        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
        server_config.transport = Arc::clone(&transport_config);
        let (_endpoint, incoming) = quinn::Endpoint::server(server_config, listen_addr)
            .map_err(ConnectionError::BindError)?;
        let client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_custom_certificate_verifier(SkipServerVerification::new()) // TODO: Verify server cert
            .with_no_client_auth();
        let mut client_config = quinn::ClientConfig::new(Arc::new(client_crypto));
        client_config.transport = transport_config;
        let client_bind_addr = match remote_addr {
            SocketAddr::V4(_) => "0.0.0.0:0".parse().unwrap(),
            SocketAddr::V6(_) => "[::]:0".parse().unwrap(),
        };
        let client_connecting = quinn::Endpoint::client(client_bind_addr)
            .map_err(ConnectionError::BindError)?
            .connect_with(client_config, remote_addr, "localhost")
            .map_err(ConnectionError::InvalidClientConfig)?;
        let NewConnection { connection, .. } = client_connecting
            .await
            .map_err(ConnectionError::FailedToConnect)?;
        let recv_mapper = Arc::new(OneshotMap::default());
        tokio::task::spawn(handle_incoming(
            listen_addr,
            incoming,
            Arc::clone(&recv_mapper),
        ));

        Ok(Self {
            listen_addr,
            id,
            num_children: 0,
            num_streams: 0,
            state: Arc::new(ConnectionState { connection }),
            recv_mapper,
        })
    }

    pub async fn open_bi(
        &mut self,
        name: &str,
    ) -> Result<(quinn::SendStream, quinn::RecvStream), StreamError> {
        let mut id = self.id.clone();
        id.push(self.num_streams);

        let mut send = self
            .state
            .connection
            .open_uni()
            .await
            .map_err(StreamError::FailedToOpen)?;
        info!(
            "{} {:?} {}: Opened outgoing stream",
            self.listen_addr, id, name
        );
        AsyncBincodeWriter::from(&mut send)
            .for_async()
            .send(&id)
            .await
            .map_err(|b| StreamError::FailedToSendID(*b))?;

        // `unwrap()` cannot fail, because we never reuse IDs.
        let recv = self.recv_mapper.recv(id.clone()).await.unwrap();
        info!(
            "{} {:?} {}: Handling incoming stream",
            self.listen_addr, id, name
        );

        self.num_streams += 1;
        Ok((send, recv))
    }

    pub fn fork(&mut self) -> Self {
        let mut id = self.id.clone();
        id.push(self.num_children);
        self.num_children += 1;
        Self {
            listen_addr: self.listen_addr,
            id,
            num_children: 0,
            num_streams: 0,
            state: Arc::clone(&self.state),
            recv_mapper: Arc::clone(&self.recv_mapper),
        }
    }

    pub fn listen_addr(&self) -> &SocketAddr {
        &self.listen_addr
    }
}

impl Drop for ConnectionState {
    fn drop(&mut self) {
        self.connection.close(0u32.into(), b"done");
    }
}

async fn handle_incoming(
    listen_addr: SocketAddr,
    mut incoming: Incoming,
    recv_mapper: Arc<OneshotMap<Vec<u32>, quinn::RecvStream>>,
) {
    // TODO: Support multiple remote parties connecting on the same port.
    let connecting = match incoming.next().await {
        None => {
            error!(
                "{}: Did not receive any incoming QUIC connection",
                listen_addr
            );
            return;
        }
        Some(connecting) => connecting,
    };

    let mut new_conn = match connecting.await {
        Err(e) => {
            error!(
                "{}: Incoming QUIC connection failed to establish: {}",
                listen_addr, e
            );
            return;
        }
        Ok(new_conn) => new_conn,
    };

    while let Some(recv) = new_conn.uni_streams.next().await {
        let mut recv = match recv {
            Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                // This is normal.
                return;
            }
            Err(e) => {
                error!("{}: QUIC connection failed: {}", listen_addr, e);
                return;
            }
            Ok(recv) => recv,
        };

        let id_len = match recv.read_u32().await {
            Err(e) => {
                error!(
                    "{}: Ignoring incoming stream due to failure to receive length of ID: {}",
                    listen_addr, e
                );
                continue;
            }
            Ok(id_len) => id_len,
        };

        if id_len > 1024 {
            error!(
                "{}: Ignoring incoming stream due to ID too long",
                listen_addr
            );
            continue;
        }

        let mut id_buffer = vec![0; id_len as usize];
        if let Err(e) = recv.read_exact(&mut id_buffer).await {
            error!(
                "{}: Ignoring incoming stream due to failure to receive ID: {}",
                listen_addr, e
            );
            continue;
        }

        let id: Vec<u32> = match bincode::options().deserialize(&id_buffer) {
            Err(e) => {
                error!(
                    "{}: Ignoring incoming stream due to failure to deserialize ID: {}",
                    listen_addr, e
                );
                continue;
            }
            Ok(id) => id,
        };

        if let Err(_) = recv_mapper.send(id.clone(), recv).await {
            error!(
                "{}, ID {:?}: Incoming stream with duplicate ID",
                listen_addr, id
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use async_bincode::tokio::{AsyncBincodeReader, AsyncBincodeWriter};
    use futures_util::{SinkExt, StreamExt};

    use super::Connection;

    #[tokio::test]
    async fn connection() {
        const P0_ADDR: &str = "[::1]:50051";
        const P1_ADDR: &str = "[::1]:50052";

        tokio::try_join!(
            tokio::task::spawn(async move {
                run_party(P0_ADDR, P1_ADDR).await.unwrap();
            }),
            tokio::task::spawn(async move {
                run_party(P1_ADDR, P0_ADDR).await.unwrap();
            }),
        )
        .unwrap();
    }

    async fn run_party(local: &str, remote: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let local_addr = local.parse().unwrap();
        let remote_addr = remote.parse().unwrap();

        let mut conn1 = Connection::new(local_addr, remote_addr).await?;
        let mut conn2 = conn1.fork();
        let mut conn3 = conn1.fork();
        let mut conn4 = conn2.fork();

        tokio::try_join!(
            open_bi_and_exchange_i32(&mut conn1, 1),
            open_bi_and_exchange_i32(&mut conn2, 2),
            open_bi_and_exchange_i32(&mut conn3, 3),
            open_bi_and_exchange_i32(&mut conn4, 4),
        )?;

        Ok(())
    }

    async fn open_bi_and_exchange_i32(
        conn: &mut Connection,
        payload: i32,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (mut tx, mut rx) = conn.open_bi("test:open_bi_and_exchange_i32").await?;
        AsyncBincodeWriter::from(&mut tx)
            .for_async()
            .send(payload)
            .await?;
        let received: i32 = AsyncBincodeReader::from(&mut rx).next().await.unwrap()?;
        assert_eq!(payload, received);
        let _ = tx.finish().await;
        Ok(())
    }
}
