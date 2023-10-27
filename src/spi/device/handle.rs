use bytes::{Bytes, BytesMut};
use spidev::{Spidev, SpidevTransfer};
use std::io::{Error, ErrorKind, Result};
use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot::{
            channel as oneshot_channel, Receiver as OneshotReceiver, Sender as OneshotSender,
        },
    },
    task::{spawn_blocking, JoinHandle},
};

type IoActorReturnSender<T> = OneshotSender<Result<T>>;
type IoActorResponse<T> = OneshotReceiver<Result<T>>;

enum IoActorMessage {
    DropUntil {
        ret: IoActorReturnSender<u8>,
    },
    Read {
        buf: BytesMut,
        ret: IoActorReturnSender<()>,
    },
    Write {
        buf: Bytes,
        ret: IoActorReturnSender<()>,
    },
}

impl IoActorMessage {
    fn drop_until() -> (IoActorMessage, IoActorResponse<u8>) {
        let (ret, recv) = oneshot_channel();
        let msg = IoActorMessage::DropUntil { ret };
        (msg, recv)
    }

    fn read(buf: BytesMut) -> (IoActorMessage, IoActorResponse<()>) {
        let (ret, recv) = oneshot_channel();
        let msg = IoActorMessage::Read { buf, ret };
        (msg, recv)
    }

    fn write(buf: Bytes) -> (IoActorMessage, IoActorResponse<()>) {
        let (ret, recv) = oneshot_channel();
        let msg = IoActorMessage::Write { buf, ret };
        (msg, recv)
    }
}

fn spi_read(spi: &Spidev, buf: &mut [u8]) -> Result<()> {
    let mut transfer = SpidevTransfer::read(buf);
    transfer.cs_change = 0;
    spi.transfer(&mut transfer)
}

fn spi_write(spi: &Spidev, buf: &[u8]) -> Result<()> {
    let mut transfer = SpidevTransfer::write(buf);
    transfer.cs_change = 0;
    spi.transfer(&mut transfer)
}

fn io_actor(spi: Spidev, mut recv: Receiver<IoActorMessage>) -> impl FnOnce() -> Spidev + Send {
    move || {
        while let Some(msg) = recv.blocking_recv() {
            match msg {
                IoActorMessage::DropUntil { ret } => {
                    let mut buf = [0xFF];
                    let res = loop {
                        if let Err(e) = spi_read(&spi, &mut buf) {
                            break Err(e);
                        };
                        if buf[0] != 0xFF {
                            break Ok(buf[0]);
                        }
                    };
                    let _ = ret.send(res);
                }
                IoActorMessage::Read { mut buf, ret } => {
                    let _ = ret.send(spi_read(&spi, &mut buf));
                }
                IoActorMessage::Write { buf, ret } => {
                    let _ = ret.send(spi_write(&spi, &buf));
                }
            }
        }
        return spi;
    }
}

#[derive(Debug)]
pub struct DeviceIoHandle {
    actor: JoinHandle<Spidev>,
    mailbox: Sender<IoActorMessage>,
}

impl DeviceIoHandle {
    pub fn new(spi: Spidev) -> DeviceIoHandle {
        let (mailbox, recv) = channel(1);
        let task = spawn_blocking(io_actor(spi, recv));

        DeviceIoHandle {
            actor: task,
            mailbox,
        }
    }

    async fn send_message(&self, msg: IoActorMessage) -> Result<()> {
        self.mailbox
            .send(msg)
            .await
            .map_err(|_| Error::new(ErrorKind::Other, "Actor Failed."))
    }

    pub async fn drop_until_byte(&self) -> Result<u8> {
        let (msg, res) = IoActorMessage::drop_until();

        self.send_message(msg).await?;

        res.await
            .map_err(|_| Error::new(ErrorKind::Other, "Actor failed."))?
    }

    pub async fn read_bytes(&self, buf: BytesMut) -> Result<()> {
        let (msg, res) = IoActorMessage::read(buf);

        self.send_message(msg).await?;

        res.await
            .map_err(|_| Error::new(ErrorKind::Other, "Actor failed."))?
    }

    pub async fn write_bytes(&self, buf: Bytes) -> Result<()> {
        let (msg, res) = IoActorMessage::write(buf);

        self.send_message(msg).await?;

        res.await
            .map_err(|_| Error::new(ErrorKind::Other, "Actor failed."))?
    }

    pub async fn shutdown(self) -> Result<Spidev> {
        let DeviceIoHandle { actor, mailbox } = self;
        drop(mailbox);
        actor
            .await
            .map_err(|_| Error::new(ErrorKind::Other, "Actor failed."))
    }
}
