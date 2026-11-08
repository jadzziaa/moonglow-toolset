use std::cell::RefCell;
use std::collections::VecDeque;
use std::{fmt, mem};

use super::app_state::HandlePendingUserEvents;
use crate::event::Event;
use crate::event_loop::ActiveEventLoop as RootActiveEventLoop;

struct EventHandlerData {
    #[allow(clippy::type_complexity)]
    handler: Box<dyn FnMut(Event<HandlePendingUserEvents>, &RootActiveEventLoop) + 'static>,
}

impl fmt::Debug for EventHandlerData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventHandlerData").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) struct EventHandler {
    /// This can be in the following states:
    /// - Not registered by the event loop (None).
    /// - Present (Some(handler)).
    /// - Currently executing the handler / in use (RefCell borrowed).
    inner: RefCell<Option<EventHandlerData>>,
    /// (Moonglow's patch, rust-windowing/winit#3992.) Events that came
    /// while the handler was running, handled after it, in order.
    deferred: RefCell<VecDeque<Event<HandlePendingUserEvents>>>,
}

impl EventHandler {
    pub(crate) const fn new() -> Self {
        Self { inner: RefCell::new(None), deferred: RefCell::new(VecDeque::new()) }
    }

    /// Set the event loop handler for the duration of the given closure.
    ///
    /// This is similar to using the `scoped-tls` or `scoped-tls-hkt` crates
    /// to store the handler in a thread local, such that it can be accessed
    /// from within the closure.
    pub(crate) fn set<'handler, R>(
        &self,
        handler: impl FnMut(Event<HandlePendingUserEvents>, &RootActiveEventLoop) + 'handler,
        closure: impl FnOnce() -> R,
    ) -> R {
        // SAFETY: We extend the lifetime of the handler here so that we can
        // store it in `EventHandler`'s `RefCell`.
        //
        // This is sound, since we make sure to unset the handler again at the
        // end of this function, and as such the lifetime isn't actually
        // extended beyond `'handler`.
        let handler = unsafe {
            mem::transmute::<
                Box<dyn FnMut(Event<HandlePendingUserEvents>, &RootActiveEventLoop) + 'handler>,
                Box<dyn FnMut(Event<HandlePendingUserEvents>, &RootActiveEventLoop) + 'static>,
            >(Box::new(handler))
        };

        match self.inner.try_borrow_mut().as_deref_mut() {
            Ok(Some(_)) => {
                unreachable!("tried to set handler while another was already set");
            },
            Ok(data @ None) => {
                *data = Some(EventHandlerData { handler });
            },
            Err(_) => {
                unreachable!("tried to set handler that is currently in use");
            },
        }

        struct ClearOnDrop<'a>(&'a EventHandler);

        impl Drop for ClearOnDrop<'_> {
            fn drop(&mut self) {
                match self.0.inner.try_borrow_mut().as_deref_mut() {
                    Ok(data @ Some(_)) => {
                        *data = None;
                    },
                    Ok(None) => {
                        tracing::error!("tried to clear handler, but no handler was set");
                    },
                    Err(_) => {
                        // Note: This is not expected to ever happen, this
                        // module generally controls the `RefCell`, and
                        // prevents it from ever being borrowed outside of it.
                        //
                        // But if it _does_ happen, it is a serious error, and
                        // we must abort the process, it'd be unsound if we
                        // weren't able to unset the handler.
                        eprintln!("tried to clear handler that is currently in use");
                        std::process::abort();
                    },
                }
            }
        }

        let _clear_on_drop = ClearOnDrop(self);

        // Note: The RefCell should not be borrowed while executing the
        // closure, that'd defeat the whole point.
        closure()

        // `_clear_on_drop` will be dropped here, or when unwinding, ensuring
        // soundness.
    }

    pub(crate) fn in_use(&self) -> bool {
        self.inner.try_borrow().is_err()
    }

    pub(crate) fn ready(&self) -> bool {
        matches!(self.inner.try_borrow().as_deref(), Ok(Some(_)))
    }

    pub(crate) fn handle_event(
        &self,
        event: Event<HandlePendingUserEvents>,
        event_loop: &RootActiveEventLoop,
    ) {
        match self.inner.try_borrow_mut().as_deref_mut() {
            Ok(Some(EventHandlerData { handler })) => {
                // It is important that we keep the reference borrowed here,
                // so that `in_use` can properly detect that the handler is
                // still in use.
                //
                // If the handler unwinds, the `RefMut` will ensure that the
                // handler is no longer borrowed.
                (handler)(event, event_loop);
                // (Moonglow's patch.) Those that came meanwhile, now: the
                // handler still held, so that what they bring about waits
                // in turn rather than coming back in.
                loop {
                    let next = self.deferred.borrow_mut().pop_front();
                    let Some(event) = next else { break };
                    (handler)(event, event_loop);
                }
            },
            Ok(None) => {
                // `NSApplication`, our app delegate and this handler are all
                // global state and so it's not impossible that we could get
                // an event after the application has exited the `EventLoop`.
                tracing::error!("tried to run event handler, but no handler was set");
            },
            Err(_) => {
                // Prevent re-entrancy. (Moonglow's patch: winit panicked
                // here, in a callback that can't unwind, and the app
                // aborted. A nested run loop inside the handler brings
                // this about: a native file dialog's `runModal`, resized,
                // or a drag from Finder over the window; rust-windowing/
                // winit#3992. The event is kept and handled once the
                // handler returns. A `ScaleFactorChanged` kept so loses
                // the size its handler would write back, which the caller
                // reads as it returns: a display change while a dialog is
                // open, the window sized as before.)
                tracing::warn!(
                    "an event came while another was being handled: kept for after it \
                     (winit#3992)"
                );
                self.deferred.borrow_mut().push_back(event);
            },
        }
    }
}
