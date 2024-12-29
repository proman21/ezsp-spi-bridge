use anyhow::{bail, Result};
use bytes::BytesMut;
use tokio::select;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot::Sender as OneshotSender;
use tokio_util::either::Either;

pub struct AshStream {
    read: UnboundedReceiver<BytesMut>,
    reset: Receiver<OneshotSender<u8>>,
    write: UnboundedSender<BytesMut>,
    error: Sender<u8>,
}

impl AshStream {
    pub(crate) fn new(
        read: UnboundedReceiver<BytesMut>,
        reset: Receiver<OneshotSender<u8>>,
        write: UnboundedSender<BytesMut>,
        error: Sender<u8>,
    ) -> AshStream {
        AshStream {
            read,
            reset,
            write,
            error,
        }
    }

    pub async fn receive(&mut self) -> Result<Either<BytesMut, OneshotSender<u8>>> {
        select! {
            biased;
            Some(reset) = self.reset.recv() => Ok(Either::Right(reset)),
            Some(frame) = self.read.recv() => Ok(Either::Left(frame)),
            else => bail!("Stream has been closed")
        }
    }

    pub fn send(&mut self, message: Either<BytesMut, u8>) -> Result<()> {
        match message {
            Either::Left(frame) => {
                if let Err(_) = self.write.send(frame) {
                    bail!("Stream has been closed")
                }
            }
            Either::Right(code) => {
                if let Err(TrySendError::Closed(_)) = self.error.try_send(code) {
                    bail!("Stream has been closed")
                }
            }
        };
        Ok(())
    }
}
