use super::{MsgTalk, TalkChannel};
use async_trait::async_trait;
use network::{Actor, PacketID, PacketProcess};
use num_enum::FromPrimitive;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, FromPrimitive)]
#[repr(u16)]
enum ActionType {
    #[num_enum(default)]
    Unknown = 0,
    SetLocation = 74,
    SetInventory = 75,
    SetAssociates = 76,
    SetProficiencies = 77,
    SetMagicSpells = 78,
    SetDirection = 79,
    SetAction = 80,
    SetMapARGB = 104,
    SetLoginComplete = 130,
}

/// Message containing a general action being performed by the client. Commonly
/// used as a request-response protocol for question and answer like exchanges.
/// For example, walk requests are responded to with an answer as to if the step
/// is legal or not.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PacketID)]
#[packet(id = 1010)]
pub struct MsgAction {
    client_timestamp: u32,
    character_id: u32,
    param0: u32,
    param1: u16,
    param2: u16,
    param3: u16,
    action_type: u16,
}

#[async_trait]
impl PacketProcess for MsgAction {
    type Error = crate::Error;

    async fn process(&self, actor: &Actor) -> Result<(), Self::Error> {
        let ty = self.action_type.into();
        match ty {
            ActionType::SetLocation => {
                let mut res = self.clone();
                res.param0 = 1002;
                res.param1 = 430;
                res.param2 = 380;
                actor.send(res).await?;
            },
            ActionType::SetMapARGB => {
                let mut res = self.clone();
                res.param0 = 0x00FF_FFFF;
                res.param1 = 430;
                res.param2 = 380;
                actor.send(res).await?;
            },
            _ => {
                let p = MsgTalk::from_system(
                    self.character_id,
                    TalkChannel::Talk,
                    format!("Missing Action Type {:?}", self.action_type),
                );
                actor.send(p).await?;
                let res = self.clone();
                actor.send(res).await?;
                warn!("Missing Action Type {:?}", self.action_type);
            },
        };
        Ok(())
    }
}
