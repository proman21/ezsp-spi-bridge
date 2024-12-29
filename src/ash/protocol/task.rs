use super::handles::AshStreamTaskHandles;
use super::state::State;
use super::stream::AshStream;
use crate::ash::frame::Frame;
use crate::ash::Error;
use anyhow::Result;
use bytes::BytesMut;
use futures::{Sink, Stream};
use tokio::sync::mpsc::{
    channel, unbounded_channel, Receiver, Sender, UnboundedReceiver, UnboundedSender,
};
use tokio::sync::oneshot::Sender as OneshotSender;

pub struct AshStreamTask {
    state: State,
    handles: AshStreamTaskHandles,
}

impl AshStreamTask {
    fn new(
        reader: impl Stream<Item = Result<Result<Frame, Error>, Error>> + 'static,
        writer: impl Sink<Frame, Error = Error> + 'static,
        inbox: UnboundedReceiver<BytesMut>,
        outbox: UnboundedSender<BytesMut>,
        reset: Sender<OneshotSender<u8>>,
        error: Receiver<u8>,
    ) -> AshStreamTask {
        let handles = AshStreamTaskHandles::new(reader, writer, inbox, outbox, reset, error);
        AshStreamTask {
            state: State::initial(),
            handles,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub async fn step(&mut self) -> Result<()> {
        self.state.process(&mut self.handles).await
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            self.step().await?;
        }
    }
}

pub fn create_ash_stream_task(
    reader: impl Stream<Item = Result<Result<Frame, Error>, Error>> + 'static,
    writer: impl Sink<Frame, Error = Error> + 'static,
) -> (AshStreamTask, AshStream) {
    let (write, inbox) = unbounded_channel();
    let (outbox, read) = unbounded_channel();
    let (reset_sender, reset) = channel(1);
    let (error, error_receiver) = channel(1);
    let task = AshStreamTask::new(reader, writer, inbox, outbox, reset_sender, error_receiver);
    let stream = AshStream::new(read, reset, write, error);
    (task, stream)
}
