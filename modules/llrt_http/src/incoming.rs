use std::{
    error::Error as StdError,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};

use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::{
    body::{Body, Frame, Incoming, SizeHint},
    HeaderMap,
};
use pin_project_lite::pin_project;
use tokio::sync::{broadcast, watch};

pub fn channel(incoming: Incoming) -> (IncomingSender, IncomingReceiver) {
    let (data_tx, data_rx) = broadcast::channel(16);
    let (want_tx, want_rx) = watch::channel(());

    let sender = IncomingSender {
        inner: incoming,
        want_rx,
        data_tx,
    };
    let receiver = IncomingReceiver {
        closed: false,
        recv_fut: None,
        want_tx,
        data_rx,
    };

    (sender, receiver)
}

// The hyper frame is not `Clone`, so we need our own.
// See: https://github.com/hyperium/hyper/discussions/3768
#[derive(Clone)]
enum ClonableFrame<T> {
    Data(T),
    Trailers(HeaderMap),
}

impl<T> From<Frame<T>> for ClonableFrame<T> {
    fn from(frame: Frame<T>) -> Self {
        let frame = match frame.into_data() {
            Ok(data) => return ClonableFrame::Data(data),
            Err(frame) => frame,
        };

        match frame.into_trailers() {
            Ok(trailers) => ClonableFrame::Trailers(trailers),
            Err(_) => unreachable!(),
        }
    }
}

impl<T> From<ClonableFrame<T>> for Frame<T> {
    fn from(frame: ClonableFrame<T>) -> Self {
        match frame {
            ClonableFrame::Data(data) => Frame::data(data),
            ClonableFrame::Trailers(trailers) => Frame::trailers(trailers),
        }
    }
}

type RecvOutput =
    Result<Result<ClonableFrame<Bytes>, Arc<hyper::Error>>, broadcast::error::RecvError>;

pub struct IncomingSender {
    inner: Incoming,
    want_rx: watch::Receiver<()>,
    data_tx: broadcast::Sender<Result<ClonableFrame<Bytes>, Arc<hyper::Error>>>,
}

impl IncomingSender {
    pub async fn process(mut self) {
        loop {
            // Wait for the receiver to be ready
            if self.want_rx.changed().await.is_err() {
                tracing::trace!("All receivers are dead, closing sender");
                return;
            }

            // Check if the receiver is closed
            if self.inner.is_end_stream() {
                return;
            }

            // Get the next frame
            let frame = match self.inner.frame().await {
                Some(Ok(frame)) => frame,
                Some(Err(err)) => {
                    self.data_tx.send(Err(Arc::new(err))).ok();
                    continue;
                },
                None => return,
            };

            // Send the frame
            let clonable_frame = ClonableFrame::from(frame);
            if self.data_tx.send(Ok(clonable_frame)).is_err() {
                tracing::trace!("All receivers are dead, closing sender");
                return;
            }
        }
    }
}

pin_project! {
    pub struct IncomingReceiver {
        closed: bool,
        #[pin]
        recv_fut: Option<Pin<Box<dyn Future<Output = RecvOutput>>>>,
        want_tx: watch::Sender<()>,
        data_rx: broadcast::Receiver<Result<ClonableFrame<Bytes>, Arc<hyper::Error>>>,
    }
}

impl Clone for IncomingReceiver {
    fn clone(&self) -> Self {
        Self {
            closed: self.closed,
            recv_fut: None,
            want_tx: self.want_tx.clone(),
            data_rx: self.data_rx.resubscribe(),
        }
    }
}

impl Body for IncomingReceiver {
    type Data = Bytes;
    type Error = Box<dyn StdError + Send + Sync>;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let mut this = self.project();

        // We already have a pending frame, poll it
        if let Some(recv_fut) = this.recv_fut.as_mut().as_pin_mut() {
            let recv_out = match ready!(recv_fut.poll(cx)) {
                Ok(Ok(frame)) => Some(Ok(Frame::from(frame))),
                Ok(Err(err)) => Some(Err(err.into())),
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    Some(Err("Lagged too much behind".into()))
                },
                Err(broadcast::error::RecvError::Closed) => {
                    *this.closed = true;
                    None
                },
            };
            *this.recv_fut = None;
            return Poll::Ready(recv_out);
        }

        // If the receiver is closed, we are done
        if *this.closed {
            return Poll::Ready(None);
        }

        // Check if there are frames available
        match this.data_rx.try_recv() {
            Ok(Ok(frame)) => return Poll::Ready(Some(Ok(Frame::from(frame)))),
            Ok(Err(err)) => return Poll::Ready(Some(Err(err.into()))),
            Err(broadcast::error::TryRecvError::Lagged(_)) => {
                return Poll::Ready(Some(Err("Lagged too much behind".into())))
            },
            Err(broadcast::error::TryRecvError::Empty) => (),
            Err(broadcast::error::TryRecvError::Closed) => {
                *this.closed = true;
                return Poll::Ready(None);
            },
        }

        // Signal the sender that we are ready to receive
        if this.want_tx.send(()).is_err() {
            *this.closed = true;
            return Poll::Ready(None);
        }

        // Wait for the next frame
        let recv_fut = Box::pin(this.data_rx.recv());
        let recv_fut_static = erase_lifetime(recv_fut);
        *this.recv_fut = Some(recv_fut_static);
        Poll::Pending
    }

    fn is_end_stream(&self) -> bool {
        self.closed
    }

    fn size_hint(&self) -> SizeHint {
        // Since use a broadcast and a reader can miss frames, we can't know the exact size
        SizeHint::default()
    }
}

fn erase_lifetime<'a, T>(
    fut: Pin<Box<dyn Future<Output = T> + Send + 'a>>,
) -> Pin<Box<dyn Future<Output = T> + Send + 'static>> {
    unsafe { std::mem::transmute(fut) }
}
