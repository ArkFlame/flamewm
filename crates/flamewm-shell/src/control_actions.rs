//! J07 nonblocking control actions. The UI thread never calls
//! `ControlClient::call` directly: it enqueues onto the bounded
//! `ControlMutationQueue` owned by `MutationDispatcher` and returns
//! immediately. The worker owns its own control connection; results drain
//! via `wake_fd` on the reactor turn. Full/closed queue is a nonfatal
//! diagnostic (counter + stderr), never a panic or blocking call.

use std::sync::{Arc, OnceLock};

use flamewm_control_core::{ControlMutationQueue, ControlRequest, MutationLane};
use flamewm_control_dbus::MutationDispatcher;

/// Enqueue one control mutation; returns the queue id on success, `None`
/// on full/closed queue (counted, stderr diagnostic, loop survives).
pub fn enqueue_action(
    dispatcher: Option<&Arc<MutationDispatcher>>,
    request: ControlRequest,
) -> Option<u64> {
    let dispatcher = dispatcher?;
    enqueue_on_queue(dispatcher.queue(), request)
}

/// Queue-level submit used by tests that cannot open a bus.
pub fn enqueue_on_queue(queue: &Arc<ControlMutationQueue>, request: ControlRequest) -> Option<u64> {
    let lane = MutationLane::for_request(&request);
    match queue.submit(lane, request) {
        Ok(id) => {
            action_ok_counter().increment();
            Some(id)
        }
        Err(error) => {
            action_fail_counter().increment();
            eprintln!("flamewm-shell: mutation enqueue failed: {}", error.message);
            None
        }
    }
}

/// Nonblocking result drain for the reactor turn. Failures are nonfatal
/// diagnostics; the event loop survives either way.
pub fn drain_results(dispatcher: Option<&Arc<MutationDispatcher>>) {
    let Some(dispatcher) = dispatcher else {
        return;
    };
    while let Some(result) = dispatcher.try_recv_result() {
        match &result.outcome {
            Ok(_) => result_ok_counter().increment(),
            Err(error) => {
                result_err_counter().increment();
                eprintln!(
                    "flamewm-shell: mutation {} failed: {}",
                    result.id, error.message
                );
            }
        }
    }
}

fn action_ok_counter() -> &'static flamewm_profiler::CounterPoint {
    static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
    C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.action.enqueue_ok"))
}

fn action_fail_counter() -> &'static flamewm_profiler::CounterPoint {
    static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
    C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.action.enqueue_fail"))
}

fn result_ok_counter() -> &'static flamewm_profiler::CounterPoint {
    static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
    C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.action.result_ok"))
}

fn result_err_counter() -> &'static flamewm_profiler::CounterPoint {
    static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
    C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.action.result_err"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flamewm_control_core::ControlMutationQueue;

    #[test]
    fn enqueue_returns_immediately_and_full_is_nonfatal() {
        let queue = Arc::new(ControlMutationQueue::new(1));
        let first = enqueue_on_queue(&queue, ControlRequest::Ping);
        assert!(first.is_some());
        let second = enqueue_on_queue(&queue, ControlRequest::Ping);
        assert!(
            second.is_none(),
            "full queue must be nonfatal None, not panic"
        );
        assert_eq!(queue.pending_len(), 1, "full entry retained, not dropped");
    }
}
