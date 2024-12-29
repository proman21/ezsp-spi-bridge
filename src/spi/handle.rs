use super::{
    device::SpiDevice,
    error::{Error, Result},
    ncp::NCP,
};
use bytes::Bytes;
use std::{result, sync::Arc};
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

fn spi_device_actor<D>(
    device: D,
    mut mailbox: Receiver<SpiActorMessage>,
    interrupt: Arc<Notify>,
) -> impl FnOnce() -> D + Send
where
    D: SpiDevice + Send,
{
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

pub struct SpiDeviceActor<D> {
    handle: JoinHandle<D>,
}

impl<D> SpiDeviceActor<D>
where
    D: SpiDevice + Send + 'static,
{
    fn new(
        device: D,
        mailbox: Receiver<SpiActorMessage>,
        interrupt: Arc<Notify>,
    ) -> SpiDeviceActor<D> {
        let handle = spawn_blocking(spi_device_actor(device, mailbox, interrupt));

        SpiDeviceActor { handle }
    }

    pub async fn into_inner(self) -> result::Result<D, JoinError> {
        self.handle.await
    }
}

#[derive(Clone)]
pub struct SpiDeviceHandle {
    mailbox: Sender<SpiActorMessage>,
    interrupt: Arc<Notify>,
}

impl SpiDeviceHandle {
    fn new(mailbox: Sender<SpiActorMessage>, interrupt: Arc<Notify>) -> SpiDeviceHandle {
        SpiDeviceHandle { mailbox, interrupt }
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
}

pub fn spi_device_handle<D>(device: D) -> (SpiDeviceActor<D>, SpiDeviceHandle)
where
    D: SpiDevice + Send + 'static,
{
    let (tx, rx) = channel(1);
    let interrupt = Arc::new(Notify::new());
    let actor = SpiDeviceActor::new(device, rx, interrupt.clone());
    let handle = SpiDeviceHandle::new(tx, interrupt);
    (actor, handle)
}
