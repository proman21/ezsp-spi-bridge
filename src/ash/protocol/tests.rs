use crate::{
    ash::{
        constants::{ASH_VERSION_2, RESET_POWERON},
        frame::Frame,
        protocol::{state::State, task::create_ash_stream_task}, Error,
    },
    test::MockTestSink,
};
use anyhow::{anyhow, Context};
use bytes::BytesMut;
use futures::{stream::iter, TryStreamExt};
use tokio_util::either::Either;
use std::{
    sync::{Arc, Mutex},
    task::Poll,
};
use tokio::{spawn, sync::mpsc::unbounded_channel};

#[tokio::test]
async fn it_responds_to_non_rst_frames_with_error_before_reset() {
    let read_buf = [Ok(Ok(Frame::data(
        0.try_into().unwrap(),
        false,
        0.try_into().unwrap(),
        BytesMut::new(),
    )))];
    let reader = iter(read_buf);

    let (tx, rx) = unbounded_channel();
    let mut writer = MockTestSink::default();
    writer
        .expect_poll_ready()
        .returning(|_| Poll::Ready(Ok(())));
    writer.expect_start_send().returning(move |item| {
        tx.send(item)?;
        Ok(())
    });
    writer
        .expect_poll_flush()
        .returning(|_| Poll::Ready(Ok(())));

    let (mut task, _handles) = create_ash_stream_task(reader, writer);

    let res = task.step().await;

    assert!(res.is_ok());
    let frame = rx.recv().await.expect("Mutex was poisoned");
    assert!(matches!(frame, Frame::Error { code, .. } if code == RESET_POWERON));
}

#[tokio::test]
async fn it_responds_to_rst_frame_with_rst_ack() {
    let read_buf = [Ok(Ok(Frame::Rst))];
    let reader = iter(read_buf);

    let buffer = Arc::new(Mutex::new(Vec::new()));
    let writer_buffer = buffer.clone();
    let mut writer = MockTestSink::default();
    writer
        .expect_poll_ready()
        .returning(|_| Poll::Ready(Ok(())));
    writer.expect_start_send().returning(move |item| {
        writer_buffer
            .lock()
            .map_err(|_| anyhow!("Mutex was poisoned"))?
            .push(item);
        Ok(())
    });
    writer
        .expect_poll_flush()
        .returning(|_| Poll::Ready(Ok(())));

    let (mut stream, mut handles) = create_ash_stream_task(reader, writer);

    let task = spawn(async move { stream.step().await.map(|_| stream) });

    let res = handles.receive().await
        .expect("Expected to receive reset signal");
    let rst_ret = match res {
        Either::Right(v) => v,
        _ => unreachable!()
    };
    rst_ret
        .send(RESET_POWERON)
        .expect("Expected to successfully send reset result");

    let stream = task
        .await
        .expect("Expected to successfully join stream task")
        .expect("Expected task execution to succeed");

    assert!(matches!(stream.state(), State::Connected(_)));
    let lock = buffer.lock().expect("Mutex was poisoned");
    let frame = lock.first().expect("Expected frame to be sent.");
    assert!(
        matches!(frame, Frame::RstAck{ version , code } if *version == ASH_VERSION_2 && *code == RESET_POWERON)
    );
}
