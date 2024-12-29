use futures::Sink;
use mockall::automock;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::ash::Error;

#[automock(type Error = Error;)]
pub trait TestSink<Item: 'static> {
    type Error;

    fn poll_ready<'a>(self: Pin<&mut Self>, cx: &mut Context<'a>) -> Poll<Result<(), Self::Error>>;
    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error>;
    fn poll_flush<'a>(self: Pin<&mut Self>, cx: &mut Context<'a>) -> Poll<Result<(), Self::Error>>;
    fn poll_close<'a>(self: Pin<&mut Self>, cx: &mut Context<'a>) -> Poll<Result<(), Self::Error>>;
}

impl<Item: 'static> Sink<Item> for MockTestSink<Item> {
    type Error = <MockTestSink<Item> as TestSink<Item>>::Error;

    fn poll_ready<'a>(self: Pin<&mut Self>, cx: &mut Context<'a>) -> Poll<Result<(), Self::Error>> {
        TestSink::poll_ready(self, cx)
    }
    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        TestSink::start_send(self, item)
    }
    fn poll_flush<'a>(self: Pin<&mut Self>, cx: &mut Context<'a>) -> Poll<Result<(), Self::Error>> {
        TestSink::poll_flush(self, cx)
    }
    fn poll_close<'a>(self: Pin<&mut Self>, cx: &mut Context<'a>) -> Poll<Result<(), Self::Error>> {
        TestSink::poll_close(self, cx)
    }
}
