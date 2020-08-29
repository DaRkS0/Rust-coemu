#![feature(try_trait, backtrace)]
pub use async_trait::async_trait;
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};
use std::error::Error as StdError;

pub use crypto::{Cipher, NopCipher, TQCipher};
pub use derive_packethandler::PacketHandler;
pub use derive_packetid::PacketID;
pub use derive_server::Server;
pub use tq_codec::TQCodec;

mod errors;
pub use errors::Error;

mod actor;
pub use actor::{Actor, Message};

mod server;
pub use server::Server;
pub trait PacketID {
    /// Get the ID of that packet.
    fn id() -> u16;
}

#[async_trait]
pub trait PacketProcess {
    type Error: StdError;
    /// Process can be invoked by a packet after decode has been called to
    /// structure packet fields and properties. For the server
    /// implementations, this is called in the packet handler after the
    /// message has been dequeued from the server's PacketProcessor
    async fn process(&self, actor: &Actor) -> Result<(), Self::Error>;
}

pub trait PacketEncode {
    /// The Packet that we will encode.
    type Packet: Serialize + PacketID;
    /// Encodes the packet structure defined by this message struct into a byte
    /// packet that can be sent to the client. Invoked automatically by the
    /// client's send method. Encodes using byte ordering rules
    /// interoperable with the game client.
    fn encode(&self) -> Result<(u16, Bytes), Error>;
}

pub trait PacketDecode {
    /// The Packet that we will Decode into.
    type Packet: DeserializeOwned;
    /// Decodes a byte packet into the packet structure defined by this message
    /// struct. Should be invoked to structure data from the client for
    /// processing. Decoding follows TQ Digital's byte ordering rules for an
    /// all-binary protocol.
    fn decode(bytes: &Bytes) -> Result<Self::Packet, Error>;
}

#[async_trait]
pub trait PacketHandler {
    type Error: StdError;
    async fn handle(
        packet: (u16, Bytes),
        actor: &Actor,
    ) -> Result<(), Self::Error>;
}

impl<T> PacketEncode for T
where
    T: Serialize + PacketID,
{
    type Packet = T;

    fn encode(&self) -> Result<(u16, Bytes), Error> {
        let id = Self::id();
        let bytes = tq_serde::to_bytes(&self)?;
        Ok((id, bytes.freeze()))
    }
}

impl<T> PacketDecode for T
where
    T: DeserializeOwned,
{
    type Packet = T;

    fn decode(bytes: &Bytes) -> Result<T, Error> {
        Ok(tq_serde::from_bytes(bytes)?)
    }
}
