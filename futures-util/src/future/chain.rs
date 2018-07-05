use core::mem::PinMut;
use futures_core::Future;
use futures_core::task::{Context, Poll};

#[must_use = "futures do nothing unless polled"]
#[derive(Debug)]
crate enum Chain<Fut1, Fut2, Data> {
    First(Option<Fut1>, Option<Data>),
    Second(Fut2),
    Empty,
}

impl<Fut1, Fut2, Data> Chain<Fut1, Fut2, Data>
    where Fut1: Future,
          Fut2: Future,
{
    pub fn new(fut1: Fut1, data: Data) -> Chain<Fut1, Fut2, Data> {
        Chain::First(Some(fut1), Some(data))
    }

    pub fn poll<F>(
        self: PinMut<Self>,
        cx: &mut Context,
        async_op: F,
    ) -> Poll<Fut2::Output>
        where F: FnOnce(Fut1::Output, Data) -> Fut2,
    {
        let mut async_op = Some(async_op);

        // Safe to call `get_mut_unchecked` because we won't move the futures.
        let this = unsafe { PinMut::get_mut_unchecked(self) };

        loop {
            let (output, data) = match this {
                Chain::First(fut1, data) => {
                    match unsafe { PinMut::new_unchecked(fut1.as_mut().unwrap()) }.poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(output) => (output, data.take().unwrap()),
                    }
                }
                Chain::Second(fut2) => {
                    return unsafe { PinMut::new_unchecked(fut2) }.poll(cx);
                }
                Chain::Empty => unreachable!()
            };

            *this = Chain::Empty; // Drop fut1
            let fut2 = (async_op.take().unwrap())(output, data);
            *this = Chain::Second(fut2)
        }
    }
}
