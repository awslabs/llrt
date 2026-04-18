use std::{
    pin::Pin,
    task::{Context, Poll, Waker},
};

use bytes::Bytes;
use http_body::Body;

pub trait BodyDrain: Body<Data = Bytes> + Unpin {
    fn drain_ready<F>(&mut self, mut f: F)
    where
        F: FnMut(Bytes),
    {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        while let Poll::Ready(Some(Ok(frame))) = Pin::new(&mut *self).poll_frame(&mut cx) {
            if let Ok(data) = frame.into_data() {
                f(data);
            }
        }
    }
}

// Ensure the implementation also carries the constraint
impl<T: Body<Data = Bytes> + Unpin + ?Sized> BodyDrain for T {}
