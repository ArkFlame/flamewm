#include "flamewm/platform/displays/service.h"

#include <chrono>
#include <map>

namespace flamewm {
namespace platform {
namespace displays {

namespace {

bool isAllowedScale(int pct) {
    return pct == 100 || pct == 125 || pct == 150 || pct == 175 || pct == 200;
}

uint64_t nowMsMonotonic() {
    using namespace std::chrono;
    return static_cast<uint64_t>(
        duration_cast<milliseconds>(steady_clock::now().time_since_epoch()).count());
}

} // namespace

struct DisplayService::Impl {
    api::DisplayPort* display;
    api::MainLoopPort* loop;
    api::DisplaySnapshot snap;
    uint64_t generation;
    uint64_t nextTxId;

    struct PendingTx {
        api::TransactionId tx;
        api::OutputId output;
        api::ModeId mode;
        uint64_t deadlineMs;
        DisplayTxState state;
        bool active;
        uint64_t baseGeneration;
        PendingTx() : tx(), output(), mode(), deadlineMs(0), state(DisplayTxState::Idle), active(false), baseGeneration(0) {}
    } pending;

    int nextListenerId;
    std::map<int, std::function<void(uint64_t)> > listeners;

    api::MainLoopPort::TimerHandle timerHandle;
    bool timerArmed;

    Impl(api::DisplayPort* d, api::MainLoopPort* l)
        : display(d), loop(l), snap(), generation(0), nextTxId(1), pending(), nextListenerId(1), timerHandle(), timerArmed(false) {
        snap.generation = 0;
        generation = 0;
        timerHandle.id = 0;
    }

    void bumpGeneration() {
        ++generation;
        snap.generation = generation;
    }

    void notifyListeners() {
        std::map<int, std::function<void(uint64_t)> > copy = listeners;
        for (std::map<int, std::function<void(uint64_t)> >::iterator it = copy.begin(); it != copy.end(); ++it) {
            if (it->second) it->second(generation);
        }
    }

    api::OutputSnapshot* findOutput(const api::OutputId& id) {
        for (size_t i = 0; i < snap.outputs.size(); ++i) {
            if (snap.outputs[i].id == id) return &snap.outputs[i];
        }
        return 0;
    }

    const api::OutputSnapshot* findOutputConst(const api::OutputId& id) const {
        for (size_t i = 0; i < snap.outputs.size(); ++i) {
            if (snap.outputs[i].id == id) return &snap.outputs[i];
        }
        return 0;
    }

    void clearPending() {
        pending.active = false;
        pending.state = DisplayTxState::Idle;
        snap.pending.active = false;
    }

    void syncPendingToSnapshot() {
        if (pending.active) {
            snap.pending.active = true;
            snap.pending.tx = pending.tx;
            snap.pending.output = pending.output;
            snap.pending.mode = pending.mode;
            snap.pending.deadlineMs = pending.deadlineMs;
        } else {
            snap.pending.active = false;
        }
    }

    void disarmTimer() {
        if (timerArmed && loop) {
            loop->removeTimer(timerHandle);
            timerArmed = false;
            timerHandle.id = 0;
        }
    }

    void doRevertLocked(bool bump) {
        // Platform owns Revert decision; adapter owns CRTC restore state.
        if (display && pending.active) {
            // Restore via adapter — adapter holds captured CRTC state.
            display->restore(pending.tx);
        }
        // Try refresh snapshot after restore to observe reverted topology
        if (display) {
            api::Result<api::DisplaySnapshot> fresh = display->queryFresh();
            if (fresh.ok()) {
                snap = fresh.value();
                generation = snap.generation;
            } else if (bump) {
                bumpGeneration();
            }
        } else if (bump) {
            bumpGeneration();
        }
        disarmTimer();
        clearPending();
        syncPendingToSnapshot();
        if (bump) {
            notifyListeners();
        }
    }
};

DisplayService::DisplayService(api::DisplayPort* display, api::MainLoopPort* loop)
    : impl_(new Impl(display, loop)) {
    // Prime snapshot from DisplayPort if available; no X11 here.
    if (impl_->display) {
        api::Result<api::DisplaySnapshot> fres = impl_->display->queryFresh();
        if (fres.ok()) {
            impl_->snap = fres.value();
            impl_->generation = impl_->snap.generation;
        }
    }
}

DisplayService::~DisplayService() {
    if (impl_) {
        impl_->disarmTimer();
        delete impl_;
        impl_ = 0;
    }
}

api::DisplaySnapshot DisplayService::snapshot() const {
    return impl_->snap;
}

uint64_t DisplayService::generation() const {
    return impl_->generation;
}

api::Result<api::TransactionId> DisplayService::beginModeChange(
    const api::OutputId& output, api::ModeId mode, uint64_t topologyGeneration) {

    if (!output.valid()) {
        return api::Result<api::TransactionId>::Err(api::Error::InvalidArgument, "invalid output");
    }
    if (!mode.valid()) {
        return api::Result<api::TransactionId>::Err(api::Error::InvalidArgument, "invalid mode");
    }
    if (!impl_->display) {
        return api::Result<api::TransactionId>::Err(api::Error::Unavailable, "no display port");
    }
    if (!impl_->loop) {
        return api::Result<api::TransactionId>::Err(api::Error::Unavailable, "no main loop port");
    }
    if (topologyGeneration != impl_->generation) {
        return api::Result<api::TransactionId>::Err(api::Error::StaleRevision, "stale generation");
    }
    if (impl_->pending.active) {
        return api::Result<api::TransactionId>::Err(api::Error::Busy, "tx already pending");
    }

    // Fresh snapshot via DisplayPort — validates durable output/mode against current topology.
    {
        api::Result<api::DisplaySnapshot> fresh = impl_->display->queryFresh();
        if (!fresh.ok()) {
            return api::Result<api::TransactionId>::Err(fresh.status());
        }
        impl_->snap = fresh.value();
        impl_->generation = impl_->snap.generation;
        // Re-validate generation after refresh — concurrent hotplug invalidates.
        if (topologyGeneration != impl_->generation) {
            // Caller based on stale topology; fresh generation differs.
            // We already checked before refresh, but if fresh advanced, treat as stale.
            // However we updated generation to fresh; need to return stale.
            // To keep monotonic semantics, revert generation update? No — keep fresh.
            return api::Result<api::TransactionId>::Err(api::Error::StaleRevision, "stale generation after refresh");
        }
    }

    api::OutputSnapshot* out = impl_->findOutput(output);
    if (!out) {
        return api::Result<api::TransactionId>::Err(api::Error::NotFound, "output not found");
    }
    bool modeFound = false;
    for (size_t i = 0; i < out->modes.size(); ++i) {
        if (out->modes[i].id == mode) { modeFound = true; break; }
    }
    if (!modeFound) {
        return api::Result<api::TransactionId>::Err(api::Error::InvalidArgument, "mode not available");
    }

    // Adapter owns CRTC restore state — capture before apply is encapsulated in applyMode.
    // Platform owns transaction ID/deadline/generation.
    api::TransactionId adapterTx;
    api::Status st = impl_->display->applyMode(output, mode, &adapterTx);
    if (!st.ok()) {
        return api::Result<api::TransactionId>::Err(st);
    }

    api::TransactionId txId;
    if (adapterTx.valid()) {
        txId = adapterTx;
    } else {
        txId = api::TransactionId(impl_->nextTxId++);
        if (impl_->nextTxId == 0) impl_->nextTxId = 1;
    }
    // Ensure nextTxId stays ahead of adapter-provided id to avoid collision on fallback.
    if (adapterTx.valid() && adapterTx.value >= impl_->nextTxId) {
        impl_->nextTxId = adapterTx.value + 1;
        if (impl_->nextTxId == 0) impl_->nextTxId = 1;
    }

    impl_->pending.tx = txId;
    impl_->pending.output = output;
    impl_->pending.mode = mode;
    impl_->pending.deadlineMs = nowMsMonotonic() + 15000;
    impl_->pending.state = DisplayTxState::Pending;
    impl_->pending.active = true;
    impl_->pending.baseGeneration = impl_->generation;

    // Refresh snapshot after apply to observe new topology generation if adapter updated it.
    {
        api::Result<api::DisplaySnapshot> after = impl_->display->queryFresh();
        if (after.ok()) {
            impl_->snap = after.value();
            impl_->generation = impl_->snap.generation;
        } else {
            // Fallback: provisional update currentMode in existing snapshot
            api::OutputSnapshot* out2 = impl_->findOutput(output);
            if (out2) out2->currentMode = mode;
            impl_->bumpGeneration();
        }
    }
    impl_->syncPendingToSnapshot();
    impl_->notifyListeners();

    // 15s deadline via MainLoopPort timer — expiry => Revert (adapter restore).
    DisplayService* self = this;
    Impl* implPtr = impl_;
    impl_->disarmTimer();
    api::MainLoopPort::TimerHandle h = impl_->loop->addTimer(15000, [self, implPtr]() {
        if (!self || !implPtr) return;
        if (!implPtr->pending.active) return;
        if (nowMsMonotonic() < implPtr->pending.deadlineMs) {
            // Timer fired early (coalesced); re-arm remaining.
            uint64_t remain = implPtr->pending.deadlineMs - nowMsMonotonic();
            if (remain == 0) remain = 1;
            if (remain > 15000) remain = 15000;
            implPtr->disarmTimer();
            DisplayService* s2 = self;
            Impl* ip2 = implPtr;
            implPtr->timerHandle = implPtr->loop->addTimer(remain, [s2, ip2]() {
                if (!s2 || !ip2) return;
                if (!ip2->pending.active) return;
                // Timeout Revert: adapter restore, bump generation, notify.
                if (nowMsMonotonic() >= ip2->pending.deadlineMs) {
                    ip2->doRevertLocked(true);
                    // doRevertLocked already notified; ensure snapshot pending cleared
                }
            }, false);
            implPtr->timerArmed = true;
            return;
        }
        // Deadline expiry => Revert via adapter restore
        implPtr->doRevertLocked(true);
    }, false);
    impl_->timerHandle = h;
    impl_->timerArmed = true;

    return api::Result<api::TransactionId>::Ok(txId);
}

api::Status DisplayService::keep(api::TransactionId tx) {
    if (!tx.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid tx");
    if (!impl_->pending.active) return api::Status::make(api::Error::NotFound, "no pending tx");
    if (impl_->pending.tx != tx) return api::Status::make(api::Error::NotFound, "tx mismatch");
    if (nowMsMonotonic() >= impl_->pending.deadlineMs) {
        // Expired — timeout Revert via adapter
        impl_->doRevertLocked(true);
        return api::Status::make(api::Error::Timeout, "tx expired");
    }
    // Keep commits Platform transaction state — stop timer, clear pending.
    // No generation bump (already bumped at begin/apply); adapter keeps applied CRTC.
    impl_->pending.state = DisplayTxState::Keeping;
    impl_->disarmTimer();
    impl_->clearPending();
    impl_->syncPendingToSnapshot();
    // Refresh to confirm topology after keep
    if (impl_->display) {
        api::Result<api::DisplaySnapshot> fresh = impl_->display->queryFresh();
        if (fresh.ok()) {
            impl_->snap = fresh.value();
            impl_->generation = impl_->snap.generation;
            impl_->syncPendingToSnapshot();
        }
    }
    return api::Status::Ok();
}

api::Status DisplayService::revert(api::TransactionId tx) {
    if (!tx.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid tx");
    if (!impl_->pending.active) return api::Status::make(api::Error::NotFound, "no pending tx");
    if (impl_->pending.tx != tx) return api::Status::make(api::Error::NotFound, "tx mismatch");
    impl_->pending.state = DisplayTxState::Reverting;
    // Revert calls native restore — adapter owns CRTC state
    impl_->doRevertLocked(true);
    return api::Status::Ok();
}

void DisplayService::onTopologyChanged(const api::DisplaySnapshot& fresh) {
    // Hotplug increments topology generation. Pending tx from older generation is invalidated.
    impl_->snap = fresh;
    impl_->generation = fresh.generation;
    if (impl_->pending.active) {
        // If fresh generation jumped beyond pending base + 1, or output disappeared, auto-revert.
        bool shouldRevert = false;
        if (fresh.generation > impl_->pending.baseGeneration + 1) {
            shouldRevert = true;
        } else {
            bool found = false;
            for (size_t i = 0; i < fresh.outputs.size(); ++i) {
                if (fresh.outputs[i].id == impl_->pending.output) { found = true; break; }
            }
            if (!found) shouldRevert = true;
        }
        if (shouldRevert) {
            // Timeout-style revert via adapter restore
            if (impl_->display) {
                impl_->display->restore(impl_->pending.tx);
            }
            impl_->disarmTimer();
            impl_->clearPending();
            // generation already set to fresh.generation; sync pending
            impl_->syncPendingToSnapshot();
            impl_->notifyListeners();
            return;
        }
    }
    impl_->syncPendingToSnapshot();
    impl_->notifyListeners();
}

api::Status DisplayService::setShellScale(const api::OutputId& output,
                                           int percent,
                                           uint64_t expectedRevision) {
    if (!output.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid output");
    if (!isAllowedScale(percent)) return api::Status::make(api::Error::InvalidArgument, "scale must be 100/125/150/175/200");
    if (expectedRevision != impl_->generation) return api::Status::make(api::Error::StaleRevision, "stale revision");
    api::OutputSnapshot* out = impl_->findOutput(output);
    if (!out) return api::Status::make(api::Error::NotFound, "output not found");
    out->shellScalePercent = percent;
    impl_->bumpGeneration();
    impl_->notifyListeners();
    return api::Status::Ok();
}

int DisplayService::addListener(std::function<void(uint64_t)> cb) {
    if (!cb) return 0;
    int id = impl_->nextListenerId++;
    impl_->listeners[id] = cb;
    return id;
}

void DisplayService::removeListener(int listenerId) {
    std::map<int, std::function<void(uint64_t)> >::iterator it = impl_->listeners.find(listenerId);
    if (it != impl_->listeners.end()) impl_->listeners.erase(it);
}

} // namespace displays
} // namespace platform
} // namespace flamewm
