use futures::Stream;
use futures::task::Poll;
use pin_project_lite::pin_project;
use std::pin::Pin;
// use tokio::macros::support::Poll;

// borrowed with gratitude from https://stackoverflow.com/a/70623592/8059394

impl<T: Stream> TupleWindowsExt for T {}
pub trait TupleWindowsExt: Stream {
    fn tuple_windows(self) -> TupleWindows<Self>
    where
        Self: Sized,
    {
        TupleWindows::new(self)
    }
}

pin_project! {
    #[derive(Debug)]
    pub struct TupleWindows<S: Stream> {
        #[pin]
        stream: S,
        previous: Option<S::Item>,
    }
}

impl<S: Stream> TupleWindows<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            previous: None,
        }
    }
}

impl<S: Stream> Stream for TupleWindows<S>
where
    S::Item: Clone,
{
    type Item = (S::Item, S::Item);

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        let current = match futures::ready!(this.stream.as_mut().poll_next(cx)) {
            Some(next) => next,
            None => return Poll::Ready(None),
        };

        if let Some(previous) = this.previous {
            let res = (previous.clone(), current.clone());
            *this.previous = Some(current);
            Poll::Ready(Some(res))
        } else {
            let next = match this.stream.poll_next(cx) {
                Poll::Ready(next) => next,
                Poll::Pending => {
                    *this.previous = Some(current);
                    return Poll::Pending;
                }
            };
            *this.previous = next.clone();
            Poll::Ready(next.map(|next| (current, next)))
        }
    }
}
