#ifndef FLAMEWM_API_EVENTS_H
#define FLAMEWM_API_EVENTS_H

#include <cstdint>
#include <string>
#include <vector>
#include <functional>

namespace flamewm {
namespace api {

// RAII subscription — move-only, calls disconnect on destruction/reset.
class Subscription {
public:
    Subscription() {}
    explicit Subscription(std::function<void()> disconnect)
        : disconnect_(std::move(disconnect)) {}

    ~Subscription() { reset(); }

    Subscription(const Subscription&) = delete;
    Subscription& operator=(const Subscription&) = delete;

    Subscription(Subscription&& o) : disconnect_(std::move(o.disconnect_)) {
        o.disconnect_ = std::function<void()>();
    }

    Subscription& operator=(Subscription&& o) {
        if (this != &o) {
            reset();
            disconnect_ = std::move(o.disconnect_);
            o.disconnect_ = std::function<void()>();
        }
        return *this;
    }

    void reset() {
        if (disconnect_) {
            std::function<void()> fn = std::move(disconnect_);
            disconnect_ = std::function<void()>();
            fn();
        }
    }

    bool active() const { return static_cast<bool>(disconnect_); }
    explicit operator bool() const { return active(); }

private:
    std::function<void()> disconnect_;
};

// Composite helper to hold multiple subscriptions.
class ScopedSubscriptions {
public:
    ScopedSubscriptions() {}
    ~ScopedSubscriptions() { clear(); }

    ScopedSubscriptions(const ScopedSubscriptions&) = delete;
    ScopedSubscriptions& operator=(const ScopedSubscriptions&) = delete;

    void add(Subscription&& s) {
        if (s.active()) subs_.push_back(std::move(s));
    }

    void clear() {
        for (std::size_t i = 0; i < subs_.size(); ++i) subs_[i].reset();
        subs_.clear();
    }

    bool empty() const { return subs_.empty(); }
    std::size_t size() const { return subs_.size(); }

private:
    std::vector<Subscription> subs_;
};

// --- Typed domain event structs (placeholders, no X11) ---

struct SettingsChangedEvent {
    uint64_t revision;
    std::vector<std::string> keys;

    SettingsChangedEvent() : revision(0) {}
    SettingsChangedEvent(uint64_t rev, const std::vector<std::string>& k)
        : revision(rev), keys(k) {}
    explicit SettingsChangedEvent(uint64_t rev) : revision(rev) {}
};

struct TopologyChangedEvent {
    uint64_t generation;

    TopologyChangedEvent() : generation(0) {}
    explicit TopologyChangedEvent(uint64_t g) : generation(g) {}
};

struct WorkspaceChangedEvent {
    uint64_t revision;
    int activeIndex;

    WorkspaceChangedEvent() : revision(0), activeIndex(-1) {}
    WorkspaceChangedEvent(uint64_t rev, int idx) : revision(rev), activeIndex(idx) {}
};

struct TaskOrderChangedEvent {
    uint64_t revision;
    std::vector<std::string> order;

    TaskOrderChangedEvent() : revision(0) {}
    TaskOrderChangedEvent(uint64_t rev, const std::vector<std::string>& o)
        : revision(rev), order(o) {}
};

} // namespace api
} // namespace flamewm

#endif // FLAMEWM_API_EVENTS_H
