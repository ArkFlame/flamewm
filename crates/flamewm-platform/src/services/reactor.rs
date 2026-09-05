use std::collections::BTreeSet;

use flamewm_api::ports::{
    FdCallback, FdEvents, FdHandle, MainLoopPort, TimerCallback, TimerHandle,
};
use flamewm_api::{ErrorCode, FlameError, FlameResult};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReactorService {
    fds: BTreeSet<FdHandle>,
    timers: BTreeSet<TimerHandle>,
}

impl ReactorService {
    pub fn add_fd<P: MainLoopPort>(
        &mut self,
        port: &mut P,
        fd: i32,
        events: FdEvents,
        callback: FdCallback,
    ) -> FlameResult<FdHandle> {
        if fd < 0 {
            return Err(FlameError::new(
                ErrorCode::InvalidArgument,
                "fd must be non-negative",
            ));
        }
        let handle = port.add_poll(fd, events, callback)?;
        self.fds.insert(handle);
        Ok(handle)
    }

    pub fn remove_fd<P: MainLoopPort>(&mut self, port: &mut P, handle: FdHandle) {
        if self.fds.remove(&handle) {
            port.remove_poll(handle);
        }
    }

    pub fn add_timer<P: MainLoopPort>(
        &mut self,
        port: &mut P,
        delay_ms: u64,
        callback: TimerCallback,
        repeat: bool,
    ) -> FlameResult<TimerHandle> {
        let handle = port.add_timer(delay_ms, callback, repeat)?;
        self.timers.insert(handle);
        Ok(handle)
    }

    pub fn defer<P: MainLoopPort>(
        &mut self,
        port: &mut P,
        callback: TimerCallback,
    ) -> FlameResult<TimerHandle> {
        let handle = port.defer(callback)?;
        self.timers.insert(handle);
        Ok(handle)
    }

    pub fn remove_timer<P: MainLoopPort>(&mut self, port: &mut P, handle: TimerHandle) {
        if self.timers.remove(&handle) {
            port.remove_timer(handle);
        }
    }

    pub fn stop_all<P: MainLoopPort>(&mut self, port: &mut P) {
        let fds = std::mem::take(&mut self.fds);
        for handle in fds {
            port.remove_poll(handle);
        }
        let timers = std::mem::take(&mut self.timers);
        for handle in timers {
            port.remove_timer(handle);
        }
    }
}
