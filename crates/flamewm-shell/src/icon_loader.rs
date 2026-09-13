//! Async shell icon loader: one worker thread owns IconResolver.
//! Main thread never resolves synchronously; it try_sends requests and
//! drains completed rasters per tick. No X calls happen in the worker.

use std::io::Write;
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};

use flamewm_integrations_linux::icons::{IconResolver, IconSize, Rgb8Raster};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconTarget {
    StartSlot(u8),
    TaskSlot(u8),
}

#[derive(Debug, Clone)]
pub struct IconRequest {
    pub target: IconTarget,
    pub generation: u64,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependentSurface {
    Panel,
    Start,
}

#[derive(Debug)]
pub struct IconResult {
    pub target: IconTarget,
    pub generation: u64,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub raster: Option<Rgb8Raster>,
}

pub struct IconLoader {
    req_tx: SyncSender<IconRequest>,
    res_rx: Receiver<IconResult>,
    wake_reader: UnixStream,
    generation: u64,
    pending: std::collections::HashSet<(IconTarget, String, u32, u32)>,
    pub queue_full_drops: u64,
    pub stale_drops: u64,
    pub results_applied: u64,
    pub high_enqueued: u64,
    pub low_enqueued: u64,
    pub coalesced: u64,
    pub dropped_low: u64,
}

impl IconLoader {
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn spawn(packaged_root: std::path::PathBuf) -> Self {
        let (req_tx, req_rx) = sync_channel::<IconRequest>(16);
        let (res_tx, res_rx) = sync_channel::<IconResult>(16);
        let (wake_reader, mut wake_writer) = UnixStream::pair().expect("icon loader wake pair");
        wake_reader
            .set_nonblocking(true)
            .expect("wake reader nonblocking");
        std::thread::Builder::new()
            .name("shell-icon-loader".to_owned())
            .spawn(move || {
                let mut resolver = IconResolver::from_environment(packaged_root, 0);
                while let Ok(req) = req_rx.recv() {
                    // Generic app miss must never rasterize brand artwork:
                    // no empty-name retry (it resolves to the brand
                    // fallback) and any brand fallback raster is dropped to
                    // None. Miss -> caller keeps last valid or the
                    // transparent empty placeholder. Applies to every
                    // request type/role uniformly.
                    let raster = resolver
                        .prepare_name(&req.name, IconSize::new(req.width, req.height))
                        .ok()
                        .filter(|raster| !is_brand_fallback(raster));
                    let res = IconResult {
                        target: req.target,
                        generation: req.generation,
                        name: req.name.clone(),
                        width: req.width,
                        height: req.height,
                        raster,
                    };
                    let _ = res_tx.try_send(res);
                    let _ = wake_writer.write_all(&[1]);
                }
            })
            .expect("spawn icon loader worker");
        Self {
            req_tx,
            res_rx,
            wake_reader,
            generation: 1,
            pending: std::collections::HashSet::new(),
            queue_full_drops: 0,
            stale_drops: 0,
            results_applied: 0,
            high_enqueued: 0,
            low_enqueued: 0,
            coalesced: 0,
            dropped_low: 0,
        }
    }

    pub fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1).max(1);
    }

    /// Non-blocking request; queue-full and already-pending (target +
    /// icon key) requests are dropped and counted. Pending dedupe keeps one
    /// worker resolve per key; the result drain caches by key and marks only
    /// dependent surfaces.
    pub fn request(&mut self, target: IconTarget, name: &str, width: u32, height: u32) {
        self.high_enqueued += 1;
        let key = (target, name.to_owned(), width, height);
        if self.pending.contains(&key) {
            return;
        }
        self.pending.insert(key.clone());
        let req = IconRequest {
            target,
            generation: self.generation,
            name: name.to_owned(),
            width,
            height,
        };
        match self.req_tx.try_send(req) {
            Ok(()) => {}
            Err(TrySendError::Full(returned)) => {
                self.pending.remove(&(
                    returned.target,
                    returned.name,
                    returned.width,
                    returned.height,
                ));
                self.queue_full_drops += 1;
            }
            Err(TrySendError::Disconnected(returned)) => {
                self.pending.remove(&(
                    returned.target,
                    returned.name,
                    returned.width,
                    returned.height,
                ));
                self.queue_full_drops += 1;
            }
        }
    }

    /// Low-priority prewarm shim (counts only; same queue, no behavior change).
    pub fn request_low(&mut self, target: IconTarget, name: &str, width: u32, height: u32) {
        self.low_enqueued += 1;
        self.request(target, name, width, height);
    }

    /// Drain ready results; stale generations are dropped and counted.
    /// Ready keys leave the pending set so a later view change can re-queue.
    pub fn drain_ready(&mut self) -> Vec<IconResult> {
        let mut out = Vec::new();
        let _ = self.wake_reader.set_nonblocking(true);
        let mut wake_buf = [0u8; 64];
        use std::io::Read;
        let _ = (&self.wake_reader).read(&mut wake_buf);
        while let Ok(res) = self.res_rx.try_recv() {
            self.pending
                .remove(&(res.target, res.name.clone(), res.width, res.height));
            if res.generation != self.generation {
                self.stale_drops += 1;
                continue;
            }
            out.push(res);
        }
        self.results_applied += out.len() as u64;
        out
    }

    #[must_use]
    pub fn wake_fd(&self) -> std::os::unix::io::RawFd {
        use std::os::unix::io::AsRawFd;
        self.wake_reader.as_raw_fd()
    }

    pub fn wake_drain(&self) {
        use std::io::Read;
        let _ = self.wake_reader.set_nonblocking(true);
        let mut buf = [0u8; 64];
        loop {
            match (&self.wake_reader).read(&mut buf) {
                Ok(0) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
    }

    /// Dependent surfaces for one icon result: Start slots repaint the
    /// submenu, task slots repaint the panel. Callers reproject only these.
    #[must_use]
    pub fn dependent_surface(target: IconTarget) -> DependentSurface {
        match target {
            IconTarget::StartSlot(_) => DependentSurface::Start,
            IconTarget::TaskSlot(_) => DependentSurface::Panel,
        }
    }
}

/// True when a worker raster is brand artwork that a generic app miss
/// must never show. Covers the packaged brand fallback path and the
/// explicit brand names; other packaged/system hits pass through.
fn is_brand_fallback(raster: &Rgb8Raster) -> bool {
    raster.source.ends_with("flamewm-icon.svg") || raster.source.ends_with("flamewm-start.svg")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_channel_is_nonblocking_when_full() {
        let (tx, _rx) = sync_channel::<IconRequest>(1);
        tx.try_send(IconRequest {
            target: IconTarget::StartSlot(0),
            generation: 1,
            name: "a".to_owned(),
            width: 20,
            height: 20,
        })
        .expect("first send fits");
        assert!(tx
            .try_send(IconRequest {
                target: IconTarget::StartSlot(1),
                generation: 1,
                name: "b".to_owned(),
                width: 20,
                height: 20,
            })
            .is_err());
    }

    #[test]
    fn stale_generations_are_dropped() {
        let (req_tx, req_rx) = sync_channel::<IconRequest>(16);
        let (res_tx, res_rx) = sync_channel::<IconResult>(16);
        let (wake_reader, _wake_writer) = UnixStream::pair().expect("wake pair");
        let mut loader = IconLoader {
            req_tx,
            res_rx,
            wake_reader,
            generation: 2,
            pending: std::collections::HashSet::new(),
            queue_full_drops: 0,
            stale_drops: 0,
            results_applied: 0,
            high_enqueued: 0,
            low_enqueued: 0,
            coalesced: 0,
            dropped_low: 0,
        };
        let _ = req_rx;
        res_tx
            .try_send(IconResult {
                target: IconTarget::TaskSlot(0),
                generation: 1,
                name: "old".to_owned(),
                width: 25,
                height: 25,
                raster: None,
            })
            .expect("queue stale");
        res_tx
            .try_send(IconResult {
                target: IconTarget::TaskSlot(1),
                generation: 2,
                name: "new".to_owned(),
                width: 25,
                height: 25,
                raster: None,
            })
            .expect("queue fresh");
        let ready = loader.drain_ready();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].name, "new");
        assert_eq!(loader.stale_drops, 1);
    }

    #[test]
    fn queue_full_drop_counts_without_blocking() {
        let (tx, rx) = sync_channel::<IconRequest>(1);
        let (res_tx, res_rx) = sync_channel::<IconResult>(16);
        let (wake_reader, _w) = UnixStream::pair().expect("wake pair");
        let _ = rx;
        let _ = res_tx;
        let mut loader = IconLoader {
            req_tx: tx.clone(),
            res_rx,
            wake_reader,
            generation: 1,
            pending: std::collections::HashSet::new(),
            queue_full_drops: 0,
            stale_drops: 0,
            results_applied: 0,
            high_enqueued: 0,
            low_enqueued: 0,
            coalesced: 0,
            dropped_low: 0,
        };
        tx.try_send(IconRequest {
            target: IconTarget::StartSlot(0),
            generation: 1,
            name: "fill".to_owned(),
            width: 20,
            height: 20,
        })
        .expect("fill");
        loader.request(IconTarget::StartSlot(1), "y", 20, 20);
        assert_eq!(loader.queue_full_drops, 1);
    }

    #[test]
    fn brand_fallback_sources_are_rejected() {
        use flamewm_integrations_linux::icons::IconOrigin;
        for source in [
            "packaged/assets/web/flamewm-icon.svg",
            "x/assets/branding/flamewm-start.svg",
        ] {
            assert!(is_brand_fallback(&Rgb8Raster {
                source: source.to_owned(),
                width: 20,
                height: 20,
                pixels: vec![1; 20 * 20 * 4],
                origin: IconOrigin::PackagedFallback,
                fallback_reason: None,
            }));
        }
        assert!(!is_brand_fallback(&Rgb8Raster {
            source: "packaged/assets/web/breeze/folder.svg".to_owned(),
            width: 20,
            height: 20,
            pixels: vec![1; 20 * 20 * 4],
            origin: IconOrigin::PackagedBreeze,
            fallback_reason: None,
        }));
    }
}
