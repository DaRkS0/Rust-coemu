use crate::actor::Message;
use crate::{Actor, ActorState, Error, PacketHandler};
use async_trait::async_trait;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::ops::Deref;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::sync::mpsc;
use tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tokio_stream::StreamExt;
use tq_codec::{TQCodec, TQEncoder};
use tq_crypto::Cipher;

#[async_trait]
pub trait Server: Sized + Send + Sync {
    type Cipher: Cipher;
    type ActorState: ActorState;
    type PacketHandler: PacketHandler<ActorState = Self::ActorState>;

    /// Get Called once a Stream Got Connected, Returing Error here will stop
    /// the stream task and disconnect them from the server.
    #[tracing::instrument(skip(state))]
    async fn on_connected(
        state: &<Self::PacketHandler as PacketHandler>::State,
        addr: SocketAddr,
    ) -> Result<(), Error> {
        let _ = addr;
        let _ = state;
        Ok(())
    }

    /// Get Called right before ending the connection with that client.
    /// good chance to clean up anything related to that actor.
    #[tracing::instrument(skip(state, actor))]
    async fn on_disconnected(
        state: &<Self::PacketHandler as PacketHandler>::State,
        actor: Actor<Self::ActorState>,
    ) -> Result<(), Error> {
        let _ = state;
        ActorState::dispose(actor.deref(), actor.handle()).await?;
        Ok(())
    }

    /// Runs the server and listen on the configured Address for new
    /// Connections.
    #[tracing::instrument(skip(state))]
    async fn run<A>(
        addr: A,
        state: <Self::PacketHandler as PacketHandler>::State,
    ) -> Result<(), Error>
    where
        A: Debug + ToSocketAddrs + Send + Sync,
    {
        let listener = TcpListener::bind(addr).await?;
        let mut incoming = TcpListenerStream::new(listener);
        tracing::trace!("Starting Server main loop");
        tracing::info!("Server is Ready for New Connections.");
        while let Some(stream) = incoming.next().await {
            let state = state.clone();
            let stream = match stream {
                Ok(s) => {
                    tracing::debug!("Got Connection from {}", s.peer_addr()?);
                    s.set_nodelay(true)?;
                    s.set_linger(None)?;
                    s.set_ttl(5)?;
                    s
                },
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        "Error while accepting new connection, dropping it."
                    );
                    continue;
                },
            };
            tokio::spawn(async move {
                tracing::trace!("Calling on_connected lifetime hook");
                Self::on_connected(&state, stream.peer_addr()?).await?;
                if let Err(e) = handle_stream::<Self>(stream, state).await {
                    tracing::error!("{}", e);
                }
                tracing::debug!("Task Ended.");
                Result::<_, Error>::Ok(())
            });
        }
        Ok(())
    }
}

#[tracing::instrument(skip(stream, state))]
async fn handle_stream<S: Server>(
    stream: TcpStream,
    state: <S::PacketHandler as PacketHandler>::State,
) -> Result<(), Error> {
    let (tx, rx) = mpsc::channel(50);
    let actor = Actor::new(tx);
    let cipher = S::Cipher::default();
    let (encoder, mut decoder) = TQCodec::new(stream, cipher.clone()).split();
    // Start MsgHandler in a seprate task.
    let message_task = tokio::spawn(handle_msg(rx, encoder, cipher));

    while let Some(packet) = decoder.next().await {
        let (id, bytes) = packet?;
        if let Err(err) =
            S::PacketHandler::handle((id, bytes), &state, &actor).await
        {
            let result = actor
                .send(err)
                .await
                .map_err(|e| Error::Other(e.to_string()));
            if let Err(e) = result {
                match e {
                    Error::SendError => {
                        tracing::error!("Actor is dead, stopping task.");
                        break;
                    },
                    _ => {
                        tracing::error!("{e:?}");
                    },
                }
            }
        }
    }
    tracing::trace!("Calling on_disconnected lifetime hook");
    message_task.abort();
    S::on_disconnected(&state, actor).await?;
    tracing::debug!("Socket Closed, stopping task.");
    Ok(())
}

#[tracing::instrument(skip(rx, encoder, cipher))]
async fn handle_msg<C: Cipher>(
    rx: mpsc::Receiver<Message>,
    mut encoder: TQEncoder<TcpStream, C>,
    cipher: C,
) -> Result<(), Error> {
    use Message::*;
    let mut rx_stream = ReceiverStream::new(rx);
    while let Some(msg) = rx_stream.next().await {
        match msg {
            GenerateKeys(seed) => {
                cipher.generate_keys(seed);
            },
            Packet(id, bytes) => {
                encoder.send((id, bytes)).await?;
            },
            Shutdown => {
                encoder.close().await?;
                break;
            },
        };
    }
    tracing::debug!("Socket Closed, stopping handle message.");
    encoder.close().await?;
    Ok(())
}
