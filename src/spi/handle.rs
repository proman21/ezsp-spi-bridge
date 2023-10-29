use std::{result, sync::Arc};

use super::{
    device::SpiDevice,
    error::{Error, Result},
    ncp::NCP,
};
use bytes::Bytes;
use tokio::{
    sync::{
        mpsc::{channel, error::TryRecvError, Receiver, Sender},
        oneshot::{channel as oneshot_channel, Sender as OneshotSender},
        Notify,
    },
    task::{spawn_blocking, JoinError, JoinHandle},
};

type MessageResponseSender<T> = OneshotSender<Result<T>>;

enum SpiActorMessage {
    SendFrame {
        frame: Bytes,
        ret: MessageResponseSender<Bytes>,
    },
    Reset {
        to_bootloader: bool,
        ret: MessageResponseSender<()>,
    },
    Wakeup {
        ret: MessageResponseSender<()>,
    },
}

fn spi_device_actor<D: SpiDevice + Send>(
    device: D,
    mut mailbox: Receiver<SpiActorMessage>,
    interrupt: Arc<Notify>,
) -> impl FnOnce() -> D + Send {
    move || {
        let mut ncp = NCP::new(device);
        loop {
            match mailbox.try_recv() {
                Ok(SpiActorMessage::SendFrame { frame, ret }) => {
                    let _ = ret.send(ncp.send(frame));
                }
                Ok(SpiActorMessage::Reset { to_bootloader, ret }) => {
                    let _ = ret.send(ncp.reset(to_bootloader));
                }
                Ok(SpiActorMessage::Wakeup { ret }) => {
                    let _ = ret.send(ncp.wakeup());
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    break;
                }
            }
            match ncp.has_callback() {
                Ok(true) => interrupt.notify_one(),
                _ => {}
            }
        }
        ncp.into_inner()
    }
}

pub struct SpiDeviceHandle<D: SpiDevice + Send + 'static> {
    actor: JoinHandle<D>,
    mailbox: Sender<SpiActorMessage>,
    interrupt: Arc<Notify>,
}

impl<D: SpiDevice + Send + 'static> SpiDeviceHandle<D> {
    pub fn new(device: D) -> SpiDeviceHandle<D> {
        let (mailbox, recv) = channel(1);
        let interrupt = Arc::new(Notify::new());
        let actor = spawn_blocking(spi_device_actor(device, recv, interrupt.clone()));

        SpiDeviceHandle {
            actor,
            mailbox,
            interrupt,
        }
    }

    async fn send_message(&self, msg: SpiActorMessage) -> Result<()> {
        self.mailbox
            .send(msg)
            .await
            .map_err(|_| Error::InternalError)
    }

    pub async fn send_frame(&self, frame: Bytes) -> Result<Bytes> {
        let (ret, res) = oneshot_channel();
        let msg = SpiActorMessage::SendFrame { frame, ret };

        self.send_message(msg).await?;

        res.await.map_err(|_| Error::InternalError)?
    }

    pub async fn reset(&self, to_bootloader: bool) -> Result<()> {
        let (ret, res) = oneshot_channel();
        let msg = SpiActorMessage::Reset { to_bootloader, ret };

        self.send_message(msg).await?;

        res.await.map_err(|_| Error::InternalError)?
    }

    pub async fn wake(&self) -> Result<()> {
        let (ret, res) = oneshot_channel();
        let msg = SpiActorMessage::Wakeup { ret };

        self.send_message(msg).await?;

        res.await.map_err(|_| Error::InternalError)?
    }

    pub async fn has_callback(&self) {
        self.interrupt.notified().await
    }

    pub async fn shutdown(self) -> result::Result<D, JoinError> {
        drop(self.mailbox);
        self.actor.await
    }
}
