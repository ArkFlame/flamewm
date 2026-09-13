//! Bounded background icon resolution with priority-aware admission.
//!
//! The service owns one shared generation-scoped [`IconLookupIndex`], one
//! shared result cache, exactly [`ICON_RESOLVE_CONCURRENCY`] workers, a
//! priority queue with pending-dedupe, and a wake FD signalled whenever a
//! result completes. Workers perform zero FS rescan: they fork from the
//! base resolver and share the immutable index + theme config with fresh
//! small private caches. No UI/X11 calls happen in workers.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

use crate::icon_theme::IconLookupIndex;
use crate::icons::{IconError, IconKey, IconResolver, Rgba8Raster};

/// Resolution urgency. Required jobs are never discarded by the service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconPriority {
    High,
    Low,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconJob {
    pub key: IconKey,
    pub priority: IconPriority,
}

#[derive(Clone, Debug)]
pub struct IconResult {
    pub key: IconKey,
    pub result: Result<Rgba8Raster, IconError>,
    /// Worker completion envelope timestamp. `drain_ready`/`take_ready`
    /// record `icon.result.wake_to_apply` from this instant to apply.
    pub completed_at: std::time::Instant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IconSubmit {
    Enqueued,
    Coalesced,
    DroppedLow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IconSubmitError {
    HighQueueFull(IconJob),
    Stopped(IconJob),
}

/// Worker seam for deterministic service tests and alternate resolver setup.
pub trait IconResolve: Send + 'static {
    fn resolve(&mut self, key: &IconKey) -> Result<Rgba8Raster, IconError>;
}

impl IconResolve for IconResolver {
    fn resolve(&mut self, key: &IconKey) -> Result<Rgba8Raster, IconError> {
        self.prepare_key(key)
    }
}

struct Queue {
    high: VecDeque<Queued>,
    low: VecDeque<Queued>,
    pending: HashSet<IconKey>,
    capacity: usize,
    stopped: bool,
}

#[derive(Clone, Debug)]
struct Queued {
    key: IconKey,
    submitted_at: std::time::Instant,
}

impl Queue {
    fn len(&self) -> usize {
        self.high.len() + self.low.len()
    }

    fn submit(&mut self, job: IconJob) -> Result<IconSubmit, IconSubmitError> {
        if self.stopped {
            return Err(IconSubmitError::Stopped(job));
        }
        if self.pending.contains(&job.key) {
            return Ok(IconSubmit::Coalesced);
        }
        if self.len() == self.capacity {
            match job.priority {
                IconPriority::High => {
                    if let Some(replaced) = self.low.pop_front() {
                        self.pending.remove(&replaced.key);
                    } else {
                        return Err(IconSubmitError::HighQueueFull(job));
                    }
                }
                IconPriority::Low => return Ok(IconSubmit::DroppedLow),
            }
        }
        let priority = job.priority;
        let key = job.key.clone();
        self.pending.insert(key.clone());
        let queued = Queued {
            key,
            submitted_at: std::time::Instant::now(),
        };
        match priority {
            IconPriority::High => self.high.push_back(queued),
            IconPriority::Low => self.low.push_back(queued),
        }
        Ok(IconSubmit::Enqueued)
    }
}

struct SharedQueue {
    queue: Mutex<Queue>,
    ready: Condvar,
}

/// Exact worker count: one-time index build shared by all workers.
pub const ICON_RESOLVE_CONCURRENCY: usize = 20;
/// Bounded pending queue depth (HIGH retry ownership on full, LOW may drop).
pub const ICON_QUEUE_CAPACITY: usize = 256;
/// Final shared result/raster cache bounds.
pub const RESULT_CACHE_ENTRIES: usize = 2048;
const RESULT_CACHE_BYTES: usize = 512 * 1024 * 4;

/// Clamp a requested worker count into 1..=20 (>20 clamps to 20).
#[must_use]
pub fn clamp_worker_count(requested: usize) -> usize {
    requested.clamp(1, ICON_RESOLVE_CONCURRENCY)
}

/// Priority icon service. Results are keyed only by [`IconKey`], never UI slots.
///
/// Owns one shared [`IconLookupIndex`], one shared result/raster cache
/// (workers resolve through the shared index and post results into the
/// shared result map), exactly [`ICON_RESOLVE_CONCURRENCY`] workers, a
/// priority queue with pending-dedupe, and a wake FD signalled on every
/// completed result. Workers perform no UI/X11 calls.
pub struct IconService<R = IconResolver> {
    worker_count: usize,
    queue_capacity: usize,
    shared: Arc<SharedQueue>,
    results: Option<Receiver<IconResult>>,
    workers: Vec<JoinHandle<()>>,
    resolver: std::marker::PhantomData<R>,
    /// One shared generation-scoped lookup index for all workers.
    index: Arc<IconLookupIndex>,
    /// One shared result cache (key -> resolved raster) fed by workers.
    result_cache: Arc<Mutex<HashMap<IconKey, Result<Rgba8Raster, IconError>>>>,
    /// Wake FD: worker writes one byte per completed result; drained by poller.
    wake_reader: Option<UnixStream>,
    submitted: Arc<AtomicU64>,
    completed: Arc<AtomicU64>,
    coalesced: Arc<AtomicU64>,
    dropped_low: Arc<AtomicU64>,
}

impl<R: IconResolve> IconService<R> {
    /// Starts `worker_count` resolver workers clamped to 1..=20
    /// (>20 clamps to 20). Queue capacity is clamped to 1..=256 and
    /// excludes active jobs.
    ///
    /// Workers share one generation-scoped index; pass the service index into
    /// each worker factory via [`IconService::shared_index`] when the worker
    /// resolver supports it (see `IconResolver::set_shared_index`).
    pub fn new<F>(worker_count: usize, queue_capacity: usize, factory: F) -> Self
    where
        F: Fn() -> R + Send + Sync + 'static,
    {
        Self::with_index(
            worker_count,
            queue_capacity,
            factory,
            Arc::new(empty_index()),
        )
    }

    /// Starts the service around an existing shared lookup index.
    pub fn with_index<F>(
        worker_count: usize,
        queue_capacity: usize,
        factory: F,
        index: Arc<IconLookupIndex>,
    ) -> Self
    where
        F: Fn() -> R + Send + Sync + 'static,
    {
        let workers_n = clamp_worker_count(worker_count);
        let capacity = queue_capacity.clamp(1, ICON_QUEUE_CAPACITY);
        let shared = Arc::new(SharedQueue {
            queue: Mutex::new(Queue {
                high: VecDeque::new(),
                low: VecDeque::new(),
                pending: HashSet::new(),
                capacity,
                stopped: false,
            }),
            ready: Condvar::new(),
        });
        let (result_tx, results) = mpsc::sync_channel(capacity);
        let wake_pair: Option<(UnixStream, UnixStream)> = UnixStream::pair().ok();
        let (wake_reader, wake_writer_opt): (Option<UnixStream>, Option<UnixStream>) =
            match wake_pair {
                Some((reader, writer)) => (Some(reader), Some(writer)),
                None => (None, None),
            };
        if let (Some(reader), Some(writer)) = (wake_reader.as_ref(), wake_writer_opt.as_ref()) {
            let _ = reader.set_nonblocking(true);
            let _ = writer.set_nonblocking(true);
        }
        let wake_tx: Option<Arc<Mutex<UnixStream>>> = wake_writer_opt
            .as_ref()
            .and_then(|stream: &UnixStream| stream.try_clone().ok())
            .map(|stream| Arc::new(Mutex::new(stream)));
        let factory = Arc::new(factory);
        let result_cache: Arc<Mutex<HashMap<IconKey, Result<Rgba8Raster, IconError>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let submitted = Arc::new(AtomicU64::new(0));
        let completed = Arc::new(AtomicU64::new(0));
        let coalesced = Arc::new(AtomicU64::new(0));
        let dropped_low = Arc::new(AtomicU64::new(0));
        let mut workers = Vec::new();
        for number in 0..workers_n {
            let shared = Arc::clone(&shared);
            let result_tx = result_tx.clone();
            let wake_tx = wake_tx.clone();
            let cache = Arc::clone(&result_cache);
            let completed = Arc::clone(&completed);
            let factory = Arc::clone(&factory);
            workers.push(
                thread::Builder::new()
                    .name(format!("icon-resolver-{number}"))
                    .spawn(move || {
                        worker_loop(shared, result_tx, wake_tx, cache, completed, factory())
                    })
                    .expect("spawn icon resolver worker"),
            );
        }
        Self {
            worker_count: workers_n,
            queue_capacity: capacity,
            shared,
            results: Some(results),
            workers,
            resolver: std::marker::PhantomData,
            index,
            result_cache,
            wake_reader,
            submitted,
            completed,
            coalesced,
            dropped_low,
        }
    }

    /// The one shared generation-scoped lookup index.
    #[must_use]
    pub fn shared_index(&self) -> &Arc<IconLookupIndex> {
        &self.index
    }

    /// Live worker count (clamped 1..=20 at construction).
    #[must_use]
    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Live queue capacity (clamped 1..=256 at construction).
    #[must_use]
    pub fn queue_capacity(&self) -> usize {
        self.queue_capacity
    }

    /// Live shared result cache length (bounded by [`RESULT_CACHE_ENTRIES`]).
    #[must_use]
    pub fn result_cache_len(&self) -> usize {
        self.result_cache
            .lock()
            .map(|cache| cache.len())
            .unwrap_or(0)
    }

    /// Raw FD readable whenever a result completes (edge: drain `wake_drain`).
    #[must_use]
    pub fn wake_fd(&self) -> Option<RawFd> {
        self.wake_reader.as_ref().map(AsRawFd::as_raw_fd)
    }

    /// Non-blocking drain of pending wake bytes (call after `wake_fd` readable).
    pub fn wake_drain(&mut self) {
        if let Some(reader) = &mut self.wake_reader {
            let mut buf = [0_u8; 64];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }
        }
    }

    /// Cached result for `key` from the single shared result cache.
    #[must_use]
    pub fn cached(&self, key: &IconKey) -> Option<Result<Rgba8Raster, IconError>> {
        self.result_cache.lock().ok()?.get(key).cloned()
    }
}

impl IconService<IconResolver> {
    /// C05 core spawn: one-time index build from the environment base.
    /// `base = IconResolver::from_environment` builds the single
    /// generation-scoped index; every worker is `base.fork_worker()`
    /// (shared immutable index + theme config, fresh 128-entry / 512 KiB
    /// private caches, zero rescan). Exactly 20 workers, queue 256.
    #[must_use]
    pub fn spawn_icon_core(packaged_root: std::path::PathBuf, generation: u64) -> Self {
        let base = IconResolver::from_environment(packaged_root, generation);
        let index = Arc::clone(base.shared_index());
        let base = Arc::new(base);
        Self::with_index(
            ICON_RESOLVE_CONCURRENCY,
            ICON_QUEUE_CAPACITY,
            move || base.fork_worker(),
            index,
        )
    }
}

impl<R: IconResolve> IconService<R> {
    /// Emits a 5000ms-cooldown service summary from existing counters.
    fn emit_service_summary(&self, context: &str) {
        let (submitted, completed, coalesced, dropped_low) = self.counters();
        let queue_len = self
            .shared
            .queue
            .lock()
            .map(|queue| queue.len())
            .unwrap_or(0);
        flamewm_debug::emit(
            flamewm_debug::ICON_SERVICE_SUMMARY,
            flamewm_debug::ICON_SERVICE_SUMMARY_COOLDOWN,
            || {
                format!(
                    "{context} submitted={submitted} completed={completed} coalesced={coalesced} dropped_low={dropped_low} queue_len={queue_len}"
                )
            },
        );
    }

    /// `(submitted, completed, coalesced, dropped_low)` service counters.
    #[must_use]
    pub fn counters(&self) -> (u64, u64, u64, u64) {
        (
            self.submitted.load(Ordering::Relaxed),
            self.completed.load(Ordering::Relaxed),
            self.coalesced.load(Ordering::Relaxed),
            self.dropped_low.load(Ordering::Relaxed),
        )
    }

    /// Attempts non-blocking submission. A full high-priority queue returns
    /// ownership to caller; it is never silently dropped.
    pub fn submit(&self, job: IconJob) -> Result<IconSubmit, IconSubmitError> {
        let submitted = self
            .shared
            .queue
            .lock()
            .expect("icon queue poisoned")
            .submit(job);
        match &submitted {
            Ok(IconSubmit::Enqueued) => {
                self.submitted.fetch_add(1, Ordering::Relaxed);
                self.shared.ready.notify_one();
            }
            Ok(IconSubmit::Coalesced) => {
                self.coalesced.fetch_add(1, Ordering::Relaxed);
            }
            Ok(IconSubmit::DroppedLow) => {
                self.dropped_low.fetch_add(1, Ordering::Relaxed);
            }
            Err(_) => {}
        }
        submitted
    }

    /// Drains completed jobs and releases their dedupe keys for future work.
    /// `&mut self` because the result receiver is consumed through the
    /// service's own forward path; callers hold the service behind `Arc`
    /// and forward from a single owned task.
    pub fn drain_ready(&mut self) -> Vec<IconResult> {
        let mut ready = Vec::new();
        let Some(results) = &self.results else {
            return ready;
        };
        while let Ok(result) = results.try_recv() {
            // Wake-to-apply: worker completion envelope -> applied here.
            let wake_to_apply = result.completed_at.elapsed();
            flamewm_profiler::record_elapsed("icon.result.wake_to_apply", wake_to_apply);
            self.finish(&result);
            emit_apply(&result, wake_to_apply);
            ready.push(result);
        }
        self.emit_service_summary("drain");
        ready
    }

    /// Takes one completed icon job, releasing its dedupe key.
    pub fn take_ready(&mut self) -> Option<IconResult> {
        let result = self.results.as_ref()?.recv().ok()?;
        let wake_to_apply = result.completed_at.elapsed();
        flamewm_profiler::record_elapsed("icon.result.wake_to_apply", wake_to_apply);
        self.finish(&result);
        emit_apply(&result, wake_to_apply);
        self.emit_service_summary("take");
        Some(result)
    }

    fn finish(&self, result: &IconResult) {
        self.shared
            .queue
            .lock()
            .expect("icon queue poisoned")
            .pending
            .remove(&result.key);
        if let Ok(mut cache) = self.result_cache.lock() {
            // Final shared result cache: bounded by entries and bytes.
            let cost = result_memory(&result.result);
            while (cache.len() >= RESULT_CACHE_ENTRIES
                || cache_memory(&cache) + cost > RESULT_CACHE_BYTES)
                && !cache.is_empty()
            {
                if let Some(first) = cache.keys().next().cloned() {
                    cache.remove(&first);
                } else {
                    break;
                }
            }
            cache.insert(result.key.clone(), result.result.clone());
        }
    }
}

fn result_memory(result: &Result<Rgba8Raster, IconError>) -> usize {
    match result {
        Ok(raster) => raster.pixels.len() + 96,
        Err(_) => 96,
    }
}

fn cache_memory(cache: &HashMap<IconKey, Result<Rgba8Raster, IconError>>) -> usize {
    cache.values().map(result_memory).sum()
}

/// Empty generation-0 index used when no theme state is provided yet.
fn empty_index() -> IconLookupIndex {
    IconLookupIndex::new(
        Vec::new(),
        vec!["hicolor".to_owned()],
        HashMap::new(),
        HashMap::new(),
        0,
    )
}

impl<R> Drop for IconService<R> {
    fn drop(&mut self) {
        self.shared
            .queue
            .lock()
            .expect("icon queue poisoned")
            .stopped = true;
        self.shared.ready.notify_all();
        // Wake blocked result sends before waiting for worker termination.
        self.results.take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn worker_loop<R: IconResolve>(
    shared: Arc<SharedQueue>,
    result_tx: SyncSender<IconResult>,
    wake_tx: Option<Arc<Mutex<UnixStream>>>,
    result_cache: Arc<Mutex<HashMap<IconKey, Result<Rgba8Raster, IconError>>>>,
    completed: Arc<AtomicU64>,
    mut resolver: R,
) {
    loop {
        // Peek the next key; serve from the shared result cache when a prior
        // worker already completed it (still forwards so waiter observes it).
        enum Next {
            Cached(IconResult),
            Resolve(IconKey),
        }
        let next: Option<Next> = {
            let mut queue = shared.queue.lock().expect("icon queue poisoned");
            while !queue.stopped && queue.high.is_empty() && queue.low.is_empty() {
                queue = shared.ready.wait(queue).expect("icon queue poisoned");
            }
            if queue.stopped {
                return;
            }
            let front_key = queue
                .high
                .front()
                .map(|q| q.key.clone())
                .or_else(|| queue.low.front().map(|q| q.key.clone()))
                .expect("queued icon");
            if let Some(hit) = result_cache
                .lock()
                .ok()
                .and_then(|c| c.get(&front_key).cloned())
            {
                let queued = if queue.high.front().is_some_and(|q| q.key == front_key) {
                    queue.high.pop_front().expect("queued icon")
                } else {
                    queue.low.pop_front().expect("queued icon")
                };
                // Queue wait: envelope submitted_at -> dequeue, not Condvar idle.
                flamewm_profiler::record_elapsed("icon.queue.wait", queued.submitted_at.elapsed());
                Some(Next::Cached(IconResult {
                    key: queued.key,
                    result: hit,
                    completed_at: std::time::Instant::now(),
                }))
            } else {
                let queued = queue
                    .high
                    .pop_front()
                    .or_else(|| queue.low.pop_front())
                    .expect("queued icon");
                flamewm_profiler::record_elapsed("icon.queue.wait", queued.submitted_at.elapsed());
                Some(Next::Resolve(queued.key))
            }
        };
        match next {
            Some(Next::Cached(done)) => {
                completed.fetch_add(1, Ordering::Relaxed);
                if result_tx.send(done).is_err() {
                    return;
                }
            }
            Some(Next::Resolve(key)) => {
                // Resolution runs without holding the queue lock.
                let result = resolver.resolve(&key);
                let ok = result.is_ok();
                let fallback = result
                    .as_ref()
                    .ok()
                    .and_then(|raster| raster.fallback_reason.clone());
                flamewm_debug::emit(
                    flamewm_debug::ICON_RESOLVE_RESULT,
                    flamewm_debug::ICON_RESOLVE_RESULT_COOLDOWN,
                    || format!("key={key:?} ok={ok}"),
                );
                if let Some(reason) = fallback {
                    flamewm_debug::emit(
                        flamewm_debug::ICON_FALLBACK,
                        flamewm_debug::ICON_FALLBACK_COOLDOWN,
                        || format!("key={key:?} reason={reason}"),
                    );
                }
                completed.fetch_add(1, Ordering::Relaxed);
                if result_tx
                    .send(IconResult {
                        result,
                        key,
                        completed_at: std::time::Instant::now(),
                    })
                    .is_err()
                {
                    return;
                }
            }
            None => return,
        }
        // Wake FD: one byte per completed result.
        if let Some(wake) = &wake_tx {
            if let Ok(mut stream) = wake.lock() {
                let _ = stream.write_all(&[1]);
            }
        }
    }
}

/// Per-icon apply summary at 5000ms cooldown from the finished result.
fn emit_apply(result: &IconResult, wake_to_apply: std::time::Duration) {
    let ok = result.result.is_ok();
    let fallback = result
        .result
        .as_ref()
        .ok()
        .and_then(|raster| raster.fallback_reason.clone());
    if fallback.is_some() {
        flamewm_debug::emit(
            flamewm_debug::ICON_FALLBACK,
            flamewm_debug::ICON_FALLBACK_COOLDOWN,
            || format!("apply key={:?} reason={}", result.key, fallback.unwrap()),
        );
    }
    flamewm_debug::emit(
        flamewm_debug::ICON_APPLY_SUMMARY,
        flamewm_debug::ICON_APPLY_SUMMARY_COOLDOWN,
        || {
            format!(
                "key={:?} ok={ok} wake_to_apply_ms={:.3}",
                result.key,
                wake_to_apply.as_secs_f64() * 1000.0
            )
        },
    );
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::icons::{IconLookupPurpose, IconRequest, IconSize};

    struct FakeResolver(Arc<AtomicUsize>);

    impl IconResolve for FakeResolver {
        fn resolve(&mut self, _: &IconKey) -> Result<Rgba8Raster, IconError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Err(IconError::InvalidSize {
                logical: 0,
                physical: 0,
            })
        }
    }

    fn key(name: &str) -> IconKey {
        IconKey::new(
            IconLookupPurpose::Application,
            IconRequest::name(name),
            IconSize::new(16, 16),
            1,
            1,
        )
    }

    #[test]
    fn core_spawns_exactly_twenty_workers_with_index_built_once() {
        use crate::icon_theme::INDEX_BUILD_COUNT;
        use std::sync::atomic::Ordering;
        // One-build proof: hermetic empty data dirs keep the build cheap
        // (no live Breeze/hicolor tree walk). `INDEX_BUILD_COUNT` is
        // process-global and shared with parallel lib tests, so exact
        // absolute counts are racy; the race-free invariant is `Arc::ptr_eq`
        // (workers literally share the one built allocation, zero rebuilds
        // by construction: `from_shared_index`/`fork_worker` never call
        // `IconLookupIndex::build`). Single pass: no retry loop over full
        // index builds (that loop was the workspace-run CPU hog).
        let before = INDEX_BUILD_COUNT.load(Ordering::SeqCst);
        let base = IconResolver::new(
            std::path::PathBuf::from("/tmp"),
            vec!["hicolor".to_owned()],
            Vec::new(),
            9001,
        );
        assert!(
            INDEX_BUILD_COUNT.load(Ordering::SeqCst) - before >= 1,
            "base performs the one IconLookupIndex::build for its generation"
        );
        let shared = base.shared_index().clone();
        let workers: Vec<IconResolver> = (0..20)
            .map(|_| {
                IconResolver::from_shared_index(
                    std::path::PathBuf::from("/tmp"),
                    vec!["hicolor".to_owned()],
                    Vec::new(),
                    9001,
                    Arc::clone(&shared),
                )
            })
            .collect();
        assert_eq!(workers.len(), 20, "exactly 20 forked workers");
        for worker in &workers {
            assert!(
                Arc::ptr_eq(worker.shared_index(), &shared),
                "fork shares the one built index with zero rebuilds"
            );
        }
        let base41 = Arc::new(base);
        let shared_index = Arc::clone(base41.shared_index());
        let mut service = IconService::with_index(
            20,
            256,
            move || base41.fork_worker(),
            Arc::clone(&shared_index),
        );
        assert!(
            Arc::ptr_eq(service.shared_index(), &shared_index),
            "service shares the one built index with zero rebuilds"
        );
        assert_eq!(service.worker_count(), 20);
        assert_eq!(service.queue_capacity(), 256);
        // clamp: >20 clamps to 20, 0 clamps to 1; queue clamps to 256.
        assert_eq!(crate::icon_service::clamp_worker_count(99), 20);
        assert_eq!(crate::icon_service::clamp_worker_count(0), 1);
        assert_eq!(crate::icon_service::ICON_RESOLVE_CONCURRENCY, 20);
        assert_eq!(crate::icon_service::ICON_QUEUE_CAPACITY, 256);
        // 20 unique HIGH requests complete; duplicate coalesces in pending.
        for index in 0..20 {
            let job = IconJob {
                key: key(&format!("core-{index}")),
                priority: IconPriority::High,
            };
            assert_eq!(service.submit(job), Ok(IconSubmit::Enqueued));
        }
        assert_eq!(
            service.submit(IconJob {
                key: key("core-0"),
                priority: IconPriority::High,
            }),
            Ok(IconSubmit::Coalesced)
        );
        let mut got = 0;
        for _ in 0..20 {
            if service.take_ready().is_some() {
                got += 1;
            }
        }
        assert_eq!(got, 20, "20 unique requests complete");
        // Cache bounds: repeated misses never exceed final shared bounds.
        assert!(service.result_cache_len() <= crate::icon_service::RESULT_CACHE_ENTRIES);
        // Workers only call `IconResolve::resolve` -> `prepare_key`
        // (FS + decode); assert the worker fn has no UI/X11 surface calls.
        let src = include_str!("icon_service.rs");
        let start = src.find("fn worker_loop").expect("worker_loop");
        let tail = &src[start..];
        let end = tail.find("\nfn emit_apply").expect("worker end");
        let worker = &tail[..end];
        assert!(!worker.contains("x11"), "no X11 in workers");
        assert!(!worker.contains("X11"), "no X11 in workers");
        assert!(!worker.contains("UiDocument"), "no UI in workers");
    }

    #[test]
    fn duplicate_key_resolves_once_and_result_has_no_target() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut service = IconService::new(2, 8, {
            let calls = Arc::clone(&calls);
            move || FakeResolver(Arc::clone(&calls))
        });
        let job = IconJob {
            key: key("same"),
            priority: IconPriority::High,
        };
        assert_eq!(service.submit(job.clone()), Ok(IconSubmit::Enqueued));
        assert_eq!(service.submit(job), Ok(IconSubmit::Coalesced));
        let result = service.take_ready().expect("one completed result");
        assert_eq!(result.key, key("same"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn high_replaces_queued_low_but_full_high_queue_returns_job() {
        let mut queue = Queue {
            high: VecDeque::new(),
            low: VecDeque::new(),
            pending: HashSet::new(),
            capacity: 1,
            stopped: false,
        };
        assert_eq!(
            queue.submit(IconJob {
                key: key("first"),
                priority: IconPriority::Low
            }),
            Ok(IconSubmit::Enqueued)
        );
        let high = IconJob {
            key: key("high"),
            priority: IconPriority::High,
        };
        assert_eq!(queue.submit(high), Ok(IconSubmit::Enqueued));
        let required = IconJob {
            key: key("required"),
            priority: IconPriority::High,
        };
        assert_eq!(
            queue.submit(required.clone()),
            Err(IconSubmitError::HighQueueFull(required))
        );
        assert_eq!(
            queue.submit(IconJob {
                key: key("low"),
                priority: IconPriority::Low
            }),
            Ok(IconSubmit::DroppedLow)
        );
    }
}
