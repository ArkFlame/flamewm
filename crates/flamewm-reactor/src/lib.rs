//! Small calloop-backed reactor for FlameWM's single desktop event loop.
//!
//! Registered I/O objects are transferred to calloop and are released with the registration; this
//! crate never duplicates a descriptor. Tokio is deliberately not part of this integration boundary.

use std::os::fd::{AsFd, AsRawFd};
use std::time::Duration;

use calloop::generic::Generic;
use calloop::timer::{TimeoutAction, Timer};
use calloop::{EventLoop, Interest, Mode, PostAction, Readiness, RegistrationToken};

/// Readiness flags exposed by the reactor without leaking calloop's source types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FdReadiness {
    pub readable: bool,
    pub writable: bool,
    pub error: bool,
    pub hangup: bool,
}

impl From<Readiness> for FdReadiness {
    fn from(readiness: Readiness) -> Self {
        Self {
            readable: readiness.readable,
            writable: readiness.writable,
            error: readiness.error,
            // calloop reports terminal/error readiness but does not distinguish HUP from ERR.
            hangup: readiness.error,
        }
    }
}

/// The process-local event loop owner.
pub struct Reactor {
    event_loop: EventLoop<'static, ()>,
}

impl Reactor {
    /// Creates an empty reactor.
    pub fn new() -> calloop::Result<Self> {
        Ok(Self {
            event_loop: EventLoop::try_new()?,
        })
    }

    /// Registers an `AsFd` source. The source is owned by calloop until removal or shutdown.
    pub fn register_fd<F, C>(
        &mut self,
        source: F,
        interest: Interest,
        callback: C,
    ) -> calloop::Result<RegistrationToken>
    where
        F: AsFd + AsRawFd + 'static,
        C: FnMut(i32, FdReadiness) + 'static,
    {
        let fd = source.as_raw_fd();
        let mut callback = callback;
        self.event_loop
            .handle()
            .insert_source(
                Generic::new(source, interest, Mode::Level),
                move |readiness, _, _| {
                    callback(fd, readiness.into());
                    Ok(PostAction::Continue)
                },
            )
            .map_err(Into::into)
    }

    /// Registers a one-shot or repeating timer.
    pub fn register_timer<C>(
        &mut self,
        delay: Duration,
        repeat: bool,
        callback: C,
    ) -> calloop::Result<RegistrationToken>
    where
        C: FnMut() + 'static,
    {
        let mut callback = callback;
        self.event_loop
            .handle()
            .insert_source(Timer::from_duration(delay), move |_, _, _| {
                callback();
                if repeat {
                    TimeoutAction::ToDuration(delay)
                } else {
                    TimeoutAction::Drop
                }
            })
            .map_err(Into::into)
    }

    /// Removes a previously registered source.
    pub fn remove(&mut self, token: RegistrationToken) -> calloop::Result<()> {
        self.event_loop.handle().remove(token);
        Ok(())
    }

    /// Dispatches pending events, waiting no longer than `timeout` for the next event.
    pub fn dispatch(&mut self, timeout: Option<Duration>) -> calloop::Result<()> {
        self.event_loop.dispatch(timeout, &mut ())
    }
}
