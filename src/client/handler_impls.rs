use crate::{Client, Control, Frames, NotificationHandler, ProcessHandler, ProcessScope};

/// A trivial handler that does nothing.
impl NotificationHandler for () {}

/// A trivial handler that does nothing.
impl ProcessHandler for () {
    /// Return `Control::Continue` so that the client stays activated.
    fn process(&mut self, _: &Client, _: &ProcessScope) -> Control {
        Control::Continue
    }
}

/// Wrap a closure that can handle the `process` callback. This is called every time data from ports
/// is available from JACK.
///
/// # Example
/// ```
/// let state = ();
/// let handler = jack::ClosureProcessHandler::new(state)
///     .with_buffer_fn(move |_: &mut (), _: &Client, _: Frames| jack::Control::Continue)
///     .with_process_fn(move |_: &mut (), _: &Client, _: &ProcessScope| jack::Control::Continue);
/// ```
pub struct ClosureProcessHandler<T, B, P> {
    inner: T,
    buffer_fn: B,
    process_fn: P,
}

impl<T> ClosureProcessHandler<T, fn(&mut T, &Client, Frames), fn(&mut T, &Client, &ProcessScope)> {
    pub fn new(
        inner: T,
    ) -> ClosureProcessHandler<
        T,
        fn(&mut T, &Client, Frames) -> Control,
        fn(&mut T, &Client, &ProcessScope) -> Control,
    > where {
        ClosureProcessHandler {
            inner,
            buffer_fn: default_buffer_fn,
            process_fn: default_process_fn,
        }
    }
}

impl<T, B, P> ClosureProcessHandler<T, B, P> {
    pub fn with_buffer_fn<F>(self, f: F) -> ClosureProcessHandler<T, F, P>
    where
        F: 'static + Send + FnMut(&mut T, &Client, Frames) -> Control,
    {
        ClosureProcessHandler {
            inner: self.inner,
            buffer_fn: f,
            process_fn: self.process_fn,
        }
    }

    pub fn with_process_fn<F>(self, f: F) -> ClosureProcessHandler<T, B, F>
    where
        F: 'static + Send + FnMut(&mut T, &Client, &ProcessScope) -> Control,
    {
        ClosureProcessHandler {
            inner: self.inner,
            buffer_fn: self.buffer_fn,
            process_fn: f,
        }
    }
}

impl<
        T: Send,
        B: 'static + Send + FnMut(&mut T, &Client, Frames) -> Control,
        P: 'static + Send + FnMut(&mut T, &Client, &ProcessScope) -> Control,
    > ProcessHandler for ClosureProcessHandler<T, B, P>
{
    fn process(&mut self, c: &Client, ps: &ProcessScope) -> Control {
        (self.process_fn)(&mut self.inner, c, ps)
    }

    fn buffer_size(&mut self, c: &Client, size: Frames) -> Control {
        (self.buffer_fn)(&mut self.inner, c, size)
    }
}

fn default_buffer_fn<T>(_: &mut T, _: &Client, _: Frames) -> Control {
    Control::Continue
}

fn default_process_fn<T>(_: &mut T, _: &Client, _: &ProcessScope) -> Control {
    Control::Continue
}
