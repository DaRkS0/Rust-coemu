use async_trait::async_trait;
use bytes::Bytes;
use futures_util::FutureExt;
use network::{Actor, PacketDecode, PacketHandler, PacketProcess, Server};
use tracing::{debug, info, warn};

mod errors;
use errors::Error;

mod packets;
use async_ctrlc::CtrlC;
use packets::{MsgAccount, PacketType};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
struct AuthServer;

impl Server for AuthServer {}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
struct Handler;

#[async_trait]
impl PacketHandler for Handler {
    type Error = Error;

    async fn handle(
        &self,
        (id, bytes): (u16, Bytes),
        actor: &Actor,
    ) -> Result<(), Self::Error> {
        let id = id.into();
        match id {
            PacketType::MsgAccount => {
                let msg = MsgAccount::decode(&bytes)?;
                debug!("{:?}", msg);
                msg.process(actor).await?;
                actor.shutdown().await?;
            },
            _ => {
                warn!("{:?}", id);
                actor.shutdown().await?;
                return Ok(());
            },
        };
        Ok(())
    }
}

fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
    println!(
        r#"
 _____         _____                  
/  __ \       |  ___|                 
| /  \/  ___  | |__  _ __ ___   _   _ 
| |     / _ \ |  __|| '_ ` _ \ | | | |
| \__/\| (_) || |___| | | | | || |_| |
 \____/ \___/ \____/|_| |_| |_| \__,_|
                                      
                                       
Copyright 2020 Shady Khalifa (@shekohex)
     All Rights Reserved.
 "#
    );
    info!("Starting Auth Server");
    info!("Initializing server...");

    smol::block_on(async {
        let ctrlc = CtrlC::new()?.map(Ok);
        let server = AuthServer::run("0.0.0.0:9958", Handler::default());
        info!("Starting Server on 9958");
        smol::future::race(ctrlc, server).await?;
        Result::<(), Error>::Ok(())
    })?;
    Ok(())
}
