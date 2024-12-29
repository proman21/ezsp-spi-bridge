use crate::ash::frame::Frame;
use crate::ash::Error;
use anyhow::{bail, Context, Result};
use bytes::BytesMut;
use futures::{Sink, SinkExt, Stream, StreamExt, TryStreamExt};
use std::pin::Pin;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot::{channel as oneshot_channel, Sender as OneshotSender};

pub struct AshStreamTaskHandles {
    read: Pin<Box<dyn Stream<Item = Result<Result<Frame, Error>, Error>>>>,
    write: Pin<Box<dyn Sink<Frame, Error = Error>>>,
    peeked: Option<Result<Result<Frame, Error>, Error>>,
    inbox: UnboundedReceiver<BytesMut>,
    outbox: UnboundedSender<BytesMut>,
    reset: Sender<OneshotSender<u8>>,
    error: Receiver<u8>,
}

impl AshStreamTaskHandles {
    pub(crate) fn new(
        reader: impl Stream<Item = Result<Result<Frame, Error>, Error>> + 'static,
        writer: impl Sink<Frame, Error = Error> + 'static,
        inbox: UnboundedReceiver<BytesMut>,
        outbox: UnboundedSender<BytesMut>,
        reset: Sender<OneshotSender<u8>>,
        error: Receiver<u8>,
    ) -> AshStreamTaskHandles {
        let read =
            Box::pin(reader) as Pin<Box<dyn Stream<Item = Result<Result<Frame, Error>, Error>>>>;
        let write = Box::pin(writer) as Pin<Box<dyn Sink<Frame, Error = Error>>>;
        AshStreamTaskHandles {
            read,
            write,
            peeked: None,
            inbox,
            outbox,
            reset,
            error,
        }
    }

    async fn get_next_frame(&mut self) -> Result<Option<Result<Frame, Error>>, Error> {
        if let Some(res) = self.peeked.take() {
            return Some(res).transpose();
        }
        self.read.try_next().await
    }

    pub(crate) async fn receive_frame(&mut self) -> Result<Result<Frame, Error>> {
        loop {
            match self.get_next_frame().await? {
                Some(res) => return Ok(res),
                None => bail!("Host has disconnected"),
            }
        }
    }

    async fn peek_frame(&mut self) -> Option<&Result<Result<Frame, Error>, Error>> {
        loop {
            if self.peeked.is_some() {
                return self.peeked.as_ref();
            } else if let Some(item) = self.read.next().await {
                self.peeked = Some(item);
            } else {
                return None;
            }
        }
    }

    pub(crate) async fn discard_extra_rst_frames(&mut self) -> Result<()> {
        while let Some(Ok(res)) = self.peek_frame().await {
            if matches!(res, Err(_) | Ok(Frame::Rst)) {
                let _ = self.get_next_frame().await;
            } else {
                break;
            }
        }
        Ok(())
    }

    pub(crate) async fn send_frame(&mut self, item: Frame) -> Result<()> {
        self.write.as_mut().send(item).await?;
        Ok(())
    }

    pub(crate) async fn reset_ncp(&mut self) -> Result<u8> {
        let (tx, rx) = oneshot_channel();
        self.reset
            .send(tx)
            .await
            .context("Failed to send reset signal to NCP")?;
        let reset_code = rx
            .await
            .context("Unable to receive reset response from NCP")?;
        Ok(reset_code)
    }

    pub(crate) fn send_data(&mut self, item: BytesMut) -> Result<()> {
        self.outbox.send(item)?;
        Ok(())
    }
}
