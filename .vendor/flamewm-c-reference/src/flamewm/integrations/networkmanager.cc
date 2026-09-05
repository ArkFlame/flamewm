#include "networkmanager.h"
#include "dbusdispatcher.h"
#include <algorithm>

#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
#include <dbus/dbus.h>
#endif

namespace flamewm {
namespace integrations {

#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
struct NMReplyContext {
    NetworkManager* manager;
    uint64_t generation;
    NMReplyContext(NetworkManager* m, uint64_t g) : manager(m), generation(g) {}
};
static void freeNMReplyContext(void* data) { delete static_cast<NMReplyContext*>(data); }
#endif

struct NetworkManager::Impl {
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    DBusConnection* connection;
    std::map<DBusWatch*, int> watchIds;
    std::map<DBusTimeout*, int> timeoutIds;
    std::vector<DBusPendingCall*> pendingCalls;

    Impl() : connection(0) {}
#else
    Impl() {}
#endif
};

NetworkManager::NetworkManager()
    : NetworkManager(static_cast<DBusDispatcher*>(0))
{
}

NetworkManager::NetworkManager(DBusDispatcher* dispatcher)
    : generation_(1)
    , available_(false)
    , pendingReconnect_(false)
    , backoffAttempts_(0)
    , currentBackoffMs_(0)
    , apRefreshPending_(false)
    , reconnectTimerId_(-1)
    , dispatcher_(dispatcher)
    , ownsDispatcher_(false)
    , impl_(new Impl())
{
    status_.state = NetworkUnavailable;
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    if (!dispatcher_) {
        dispatcher_ = new DBusDispatcher(0);
        ownsDispatcher_ = true;
    }

    DBusError error;
    dbus_error_init(&error);
    impl_->connection = dbus_bus_get_private(DBUS_BUS_SYSTEM, &error);
    if (impl_->connection) {
        dbus_connection_set_exit_on_disconnect(impl_->connection, FALSE);
        dispatcher_->addConnection(impl_->connection);
        dbus_connection_set_watch_functions(impl_->connection, &NetworkManager::addWatch,
                                             &NetworkManager::removeWatch,
                                             &NetworkManager::toggleWatch, this, 0);
        dbus_connection_set_timeout_functions(impl_->connection, &NetworkManager::addTimeout,
                                               &NetworkManager::removeTimeout,
                                               &NetworkManager::toggleTimeout, this, 0);
        DBusError matchError;
        dbus_error_init(&matchError);
        dbus_bus_add_match(impl_->connection,
            "type='signal',interface='org.freedesktop.DBus',member='NameOwnerChanged',arg0='org.freedesktop.NetworkManager'", &matchError);
        if (dbus_error_is_set(&matchError)) dbus_error_free(&matchError);

        dbus_error_init(&matchError);
        dbus_bus_add_match(impl_->connection,
            "type='signal',interface='org.freedesktop.DBus.Properties',member='PropertiesChanged',arg0='org.freedesktop.NetworkManager'", &matchError);
        if (dbus_error_is_set(&matchError)) dbus_error_free(&matchError);
        dbus_connection_add_filter(impl_->connection, &NetworkManager::filterMessage, this, 0);
        dbus_connection_flush(impl_->connection);
        if (dbus_bus_name_has_owner(impl_->connection, "org.freedesktop.NetworkManager", 0))
            onServiceAppeared();
    }
    dbus_error_free(&error);
#endif
}

NetworkManager::~NetworkManager() {
    cancelReconnect();
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    for (size_t i = 0; i < impl_->pendingCalls.size(); ++i) {
        dbus_pending_call_set_notify(impl_->pendingCalls[i], 0, 0, 0);
        dbus_pending_call_cancel(impl_->pendingCalls[i]);
        dbus_pending_call_unref(impl_->pendingCalls[i]);
    }
    impl_->pendingCalls.clear();
    if (impl_->connection) {
        dispatcher_->removeConnection(impl_->connection);
        dbus_connection_remove_filter(impl_->connection, &NetworkManager::filterMessage, this);
        dbus_connection_close(impl_->connection);
        dbus_connection_unref(impl_->connection);
        impl_->connection = 0;
    }
#endif
    if (ownsDispatcher_) delete dispatcher_;
    listeners_.clear();
    delete impl_;
}

void NetworkManager::addListener(NetworkManagerListener* l) {
    if (!l) return;
    if (std::find(listeners_.begin(), listeners_.end(), l) != listeners_.end()) return;
    listeners_.push_back(l);
}

void NetworkManager::removeListener(NetworkManagerListener* l) {
    listeners_.erase(std::remove(listeners_.begin(), listeners_.end(), l), listeners_.end());
}

void NetworkManager::onServiceOwnerChanged(bool hasOwner, uint64_t dispatcherGen) {
    if (dispatcher_ && dispatcher_->isStale("nm", dispatcherGen)) return;
    if (hasOwner) onServiceAppeared();
    else onServiceVanished();
}

void NetworkManager::onServiceAppeared() {
    available_ = true;
    if (status_.state == NetworkUnavailable) status_.state = NetworkDisconnected;
    resetBackoff();
    cancelReconnect();
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    if (impl_->connection) {
        DBusMessage* call = dbus_message_new_method_call(
            "org.freedesktop.NetworkManager", "/org/freedesktop/NetworkManager",
            "org.freedesktop.DBus.Properties", "GetAll");
        if (call) {
            const char* iface = "org.freedesktop.NetworkManager";
            dbus_message_append_args(call, DBUS_TYPE_STRING, &iface, DBUS_TYPE_INVALID);
            DBusPendingCall* pending = 0;
            if (dbus_connection_send_with_reply(impl_->connection, call, &pending, -1) && pending) {
                NMReplyContext* captured = new NMReplyContext(this, generation_);
                if (!dbus_pending_call_set_notify(pending, &NetworkManager::rootReply, captured, freeNMReplyContext)) {
                    delete captured;
                    dbus_pending_call_unref(pending);
                } else {
                    trackPending(pending);
                }
            } else if (pending) {
                dbus_pending_call_unref(pending);
            }
            dbus_message_unref(call);
        }
    }
#endif
    notifyListeners();
}

void NetworkManager::onServiceVanished() {
    available_ = false;
    if (dispatcher_) dispatcher_->bumpGeneration("nm");
    clearStaleState();
    bumpGeneration();
    status_.state = NetworkUnavailable;
    scheduleReconnect();
    notifyListeners();
}

bool NetworkManager::setWifiEnabled(bool enabled) {
    if (!available_) return false;
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    if (!impl_->connection) return false;
    DBusMessage* call = dbus_message_new_method_call("org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager", "org.freedesktop.NetworkManager", "Enable");
    if (!call) return false;
    dbus_bool_t value = enabled ? TRUE : FALSE;
    dbus_message_append_args(call, DBUS_TYPE_BOOLEAN, &value, DBUS_TYPE_INVALID);
    DBusPendingCall* pending = 0;
    bool sent = dbus_connection_send_with_reply(impl_->connection, call, &pending, -1) != FALSE;
    if (sent && pending) {
        NMReplyContext* captured = new NMReplyContext(this, generation_);
        if (!dbus_pending_call_set_notify(pending, &NetworkManager::reply, captured, freeNMReplyContext)) {
            delete captured;
            dbus_pending_call_unref(pending);
        } else {
            trackPending(pending);
        }
    } else if (pending) {
        dbus_pending_call_unref(pending);
    }
    dbus_message_unref(call);
    if (!sent) return false;
#else
    return false;
#endif
    status_.wifiEnabled = enabled;
    notifyListeners();
    return true;
}

bool NetworkManager::requestConnectKnown(const std::string& apPath, uint64_t callerGen) {
    if (isStale(callerGen)) return false;
    if (!available_) return false;
    if (apPath.empty()) return false;
    // Only object paths are sent. Secrets never enter this adapter.
    if (knownProfiles_.find(apPath) == knownProfiles_.end()) {
        // not known — caller should use secure path
        return false;
    }
    status_.state = NetworkWifiConnecting;
    notifyListeners();
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    if (impl_->connection) {
        DBusMessage* call = dbus_message_new_method_call("org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager", "org.freedesktop.NetworkManager", "ActivateConnection");
        if (!call) return false;
        const char* connection = knownProfiles_[apPath].c_str();
        const char* device = "/";
        const char* specific = apPath.c_str();
        dbus_message_append_args(call, DBUS_TYPE_OBJECT_PATH, &connection,
            DBUS_TYPE_OBJECT_PATH, &device, DBUS_TYPE_OBJECT_PATH, &specific, DBUS_TYPE_INVALID);
        DBusPendingCall* pending = 0;
        bool sent = dbus_connection_send_with_reply(impl_->connection, call, &pending, -1) != FALSE;
        if (sent && pending) {
            NMReplyContext* captured = new NMReplyContext(this, generation_);
            if (!dbus_pending_call_set_notify(pending, &NetworkManager::reply, captured, freeNMReplyContext)) {
                delete captured;
                dbus_pending_call_unref(pending);
            } else {
                trackPending(pending);
            }
        } else if (pending) {
            dbus_pending_call_unref(pending);
        }
        dbus_message_unref(call);
        return sent;
    }
#endif
    return true;
}

bool NetworkManager::requestConnectNewSecure(const std::string& apPath, uint64_t callerGen) {
    if (isStale(callerGen)) return false;
    if (!available_) return false;
    if (apPath.empty()) return false;
    // The secret agent owns credentials; this path sends no secret data.
    status_.state = NetworkWifiConnecting;
    notifyListeners();
    return true;
}

bool NetworkManager::requestDisconnect(uint64_t callerGen) {
    if (isStale(callerGen)) return false;
    if (!available_) return false;
    status_.state = NetworkDisconnected;
    status_.activeSsid.clear();
    status_.activeStrength = 0;
    status_.activePath.clear();
    notifyListeners();
    return true;
}

bool NetworkManager::requestScan() {
    if (!available_) return false;
    requestApRefreshCoalesced();
    return true;
}

void NetworkManager::requestApRefreshCoalesced() {
    if (apRefreshPending_) return;
    apRefreshPending_ = true;
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    if (impl_->connection) {
        DBusMessage* call = dbus_message_new_method_call("org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager", "org.freedesktop.NetworkManager", "GetDevices");
        if (call) {
            DBusPendingCall* pending = 0;
            if (dbus_connection_send_with_reply(impl_->connection, call, &pending, -1) && pending) {
                NMReplyContext* captured = new NMReplyContext(this, generation_);
                if (!dbus_pending_call_set_notify(pending, &NetworkManager::reply, captured, freeNMReplyContext)) {
                    delete captured;
                    dbus_pending_call_unref(pending);
                } else {
                    trackPending(pending);
                }
            } else if (pending) {
                dbus_pending_call_unref(pending);
            }
            dbus_message_unref(call);
        }
    }
#endif
}

void NetworkManager::onReconnectTimer() {
    pendingReconnect_ = false;
    if (dispatcher_ && reconnectTimerId_ >= 0) {
        dispatcher_->removeTimeout(reconnectTimerId_);
        reconnectTimerId_ = -1;
    }
    if (available_) return;
    if (backoffAttempts_ >= backoff_.maxAttempts) {
        // bounded: stop storm, wait for NameOwnerChanged to appear
        return;
    }
    ++backoffAttempts_;
    currentBackoffMs_ = nextBackoffMs();
    // Real: attempt async reconnect (check bus name has owner)
    // Schedule next attempt if still unavailable
    if (backoffAttempts_ < backoff_.maxAttempts) {
        scheduleReconnect();
    }
}

int NetworkManager::nextBackoffMs() const {
    if (backoffAttempts_ <= 0) return backoff_.initialMs;
    int ms = backoff_.initialMs;
    for (int i = 1; i < backoffAttempts_; ++i) {
        ms *= 2;
        if (ms > backoff_.maxMs) { ms = backoff_.maxMs; break; }
    }
    if (ms > backoff_.maxMs) ms = backoff_.maxMs;
    return ms;
}

void NetworkManager::resetBackoff() {
    backoffAttempts_ = 0;
    currentBackoffMs_ = 0;
}

void NetworkManager::injectStatus(const NetworkStatus& s, uint64_t gen) {
    if (isStale(gen)) return;
    status_ = s;
    available_ = (s.state != NetworkUnavailable);
    if (available_) resetBackoff();
    notifyListeners();
}

void NetworkManager::bumpGeneration() {
    ++generation_;
    // invalidate pending AP refresh that might deliver old paths
    apRefreshPending_ = false;
}

void NetworkManager::clearStaleState() {
    // Owner loss clears stale object paths before reconnect (FAILURES.md)
    status_.activePath.clear();
    status_.visibleAps.clear();
    knownProfiles_.clear();
    apRefreshPending_ = false;
}

void NetworkManager::notifyListeners() {
    uint64_t gen = generation_;
    NetworkStatus copy = status_;
    // Copy listener list to allow self-removal during callback
    std::vector<NetworkManagerListener*> ls = listeners_;
    for (size_t i = 0; i < ls.size(); ++i) {
        if (ls[i]) ls[i]->onNetworkStatus(copy, gen);
    }
}

void NetworkManager::scheduleReconnect() {
    if (pendingReconnect_) return;
    if (backoffAttempts_ >= backoff_.maxAttempts) return;
    pendingReconnect_ = true;
    if (dispatcher_) reconnectTimerId_ = dispatcher_->addTimeout(nextBackoffMs(), "nm");
}

void NetworkManager::cancelReconnect() {
    pendingReconnect_ = false;
    if (dispatcher_ && reconnectTimerId_ >= 0) {
        dispatcher_->removeTimeout(reconnectTimerId_);
        reconnectTimerId_ = -1;
    }
}

#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
dbus_bool_t NetworkManager::addWatch(DBusWatch* watch, void* data) {
    NetworkManager* manager = static_cast<NetworkManager*>(data);
    if (!manager || !manager->dispatcher_ || !watch) return FALSE;
    int id = manager->dispatcher_->addWatch(dbus_watch_get_unix_fd(watch),
        (dbus_watch_get_flags(watch) & DBUS_WATCH_READABLE) != 0,
        (dbus_watch_get_flags(watch) & DBUS_WATCH_WRITABLE) != 0, "nm");
    if (id <= 0) return FALSE;
    manager->impl_->watchIds[watch] = id;
    return TRUE;
}

void NetworkManager::removeWatch(DBusWatch* watch, void* data) {
    NetworkManager* manager = static_cast<NetworkManager*>(data);
    if (!manager || !manager->dispatcher_) return;
    std::map<DBusWatch*, int>::iterator it = manager->impl_->watchIds.find(watch);
    if (it != manager->impl_->watchIds.end()) {
        manager->dispatcher_->removeWatch(it->second);
        manager->impl_->watchIds.erase(it);
    }
}

void NetworkManager::toggleWatch(DBusWatch* watch, void* data) {
    NetworkManager* manager = static_cast<NetworkManager*>(data);
    if (!manager || !manager->dispatcher_) return;
    std::map<DBusWatch*, int>::iterator it = manager->impl_->watchIds.find(watch);
    if (it != manager->impl_->watchIds.end())
        manager->dispatcher_->setWatchEnabled(it->second, dbus_watch_get_enabled(watch) != FALSE);
}

dbus_bool_t NetworkManager::addTimeout(DBusTimeout* timeout, void* data) {
    NetworkManager* manager = static_cast<NetworkManager*>(data);
    if (!manager || !manager->dispatcher_ || !timeout) return FALSE;
    int id = manager->dispatcher_->addTimeout(dbus_timeout_get_interval(timeout), "nm");
    if (id <= 0) return FALSE;
    manager->impl_->timeoutIds[timeout] = id;
    return TRUE;
}

void NetworkManager::removeTimeout(DBusTimeout* timeout, void* data) {
    NetworkManager* manager = static_cast<NetworkManager*>(data);
    if (!manager || !manager->dispatcher_) return;
    std::map<DBusTimeout*, int>::iterator it = manager->impl_->timeoutIds.find(timeout);
    if (it != manager->impl_->timeoutIds.end()) {
        manager->dispatcher_->removeTimeout(it->second);
        manager->impl_->timeoutIds.erase(it);
    }
}
void NetworkManager::toggleTimeout(DBusTimeout* timeout, void* data) {
    NetworkManager* manager = static_cast<NetworkManager*>(data);
    if (!manager || !manager->dispatcher_) return;
    std::map<DBusTimeout*, int>::iterator it = manager->impl_->timeoutIds.find(timeout);
    if (it != manager->impl_->timeoutIds.end())
        manager->dispatcher_->setTimeoutEnabled(it->second, dbus_timeout_get_enabled(timeout) != FALSE);
}

DBusHandlerResult NetworkManager::filterMessage(DBusConnection*, DBusMessage* message, void* data) {
    NetworkManager* manager = static_cast<NetworkManager*>(data);
    if (manager) manager->handleSignal(message);
    return DBUS_HANDLER_RESULT_NOT_YET_HANDLED;
}

void NetworkManager::rootReply(DBusPendingCall* pending, void* data) {
    NMReplyContext* captured = static_cast<NMReplyContext*>(data);
    NetworkManager* manager = captured ? captured->manager : 0;
    uint64_t generation = captured ? captured->generation : 0;
    if (manager) manager->untrackPending(pending);
    DBusMessage* message = dbus_pending_call_steal_reply(pending);
    if (message) {
        // The pending callback has no ownership cycle: the connection filter owns
        // the manager and generation rejects replies crossing owner loss.
        if (manager && !manager->isStale(generation) &&
            dbus_message_get_type(message) == DBUS_MESSAGE_TYPE_METHOD_RETURN)
            manager->handleRootProperties(message, generation);
        dbus_message_unref(message);
    }
    dbus_pending_call_unref(pending);
    (void)generation;
}

void NetworkManager::reply(DBusPendingCall* pending, void* data) {
    rootReply(pending, data);
}

void NetworkManager::handleSignal(DBusMessage* message) {
    if (dbus_message_is_signal(message, "org.freedesktop.DBus", "NameOwnerChanged")) {
        const char* name = 0;
        const char* oldOwner = 0;
        const char* newOwner = 0;
        if (dbus_message_get_args(message, 0, DBUS_TYPE_STRING, &name,
                                   DBUS_TYPE_STRING, &oldOwner,
                                   DBUS_TYPE_STRING, &newOwner, DBUS_TYPE_INVALID) &&
            name && std::string(name) == "org.freedesktop.NetworkManager") {
            if (newOwner && *newOwner) onServiceAppeared();
            else if (oldOwner && *oldOwner) onServiceVanished();
        }
        return;
    }
    if (!dbus_message_is_signal(message, "org.freedesktop.DBus.Properties", "PropertiesChanged")) return;
    DBusMessageIter args;
    if (!dbus_message_iter_init(message, &args) || dbus_message_iter_get_arg_type(&args) != DBUS_TYPE_STRING) return;
    const char* iface = 0;
    dbus_message_iter_get_basic(&args, &iface);
    if (!iface || std::string(iface) != "org.freedesktop.NetworkManager") return;
    if (!dbus_message_iter_next(&args) || dbus_message_iter_get_arg_type(&args) != DBUS_TYPE_ARRAY) return;
    DBusMessageIter changed;
    dbus_message_iter_recurse(&args, &changed);
    while (dbus_message_iter_get_arg_type(&changed) == DBUS_TYPE_DICT_ENTRY) {
        DBusMessageIter entry;
        dbus_message_iter_recurse(&changed, &entry);
        const char* key = 0;
        if (dbus_message_iter_get_arg_type(&entry) == DBUS_TYPE_STRING) dbus_message_iter_get_basic(&entry, &key);
        if (key && dbus_message_iter_next(&entry) && dbus_message_iter_get_arg_type(&entry) == DBUS_TYPE_VARIANT) {
            DBusMessageIter value;
            dbus_message_iter_recurse(&entry, &value);
            if (std::string(key) == "WirelessEnabled" && dbus_message_iter_get_arg_type(&value) == DBUS_TYPE_BOOLEAN) {
                dbus_bool_t enabled = FALSE; dbus_message_iter_get_basic(&value, &enabled); status_.wifiEnabled = enabled != FALSE;
            } else if (std::string(key) == "NetworkingEnabled" && dbus_message_iter_get_arg_type(&value) == DBUS_TYPE_BOOLEAN) {
                dbus_bool_t enabled = FALSE; dbus_message_iter_get_basic(&value, &enabled); status_.networkingEnabled = enabled != FALSE;
            } else if (std::string(key) == "State" && dbus_message_iter_get_arg_type(&value) == DBUS_TYPE_UINT32) {
                uint32_t state = 0; dbus_message_iter_get_basic(&value, &state);
                status_.state = state >= 50 ? NetworkWifiConnected : (state >= 40 ? NetworkWifiConnecting : NetworkDisconnected);
            }
        }
        dbus_message_iter_next(&changed);
    }
    if (available_) notifyListeners();
}

void NetworkManager::handleRootProperties(DBusMessage* message, uint64_t capturedGeneration) {
    DBusMessageIter args;
    if (!dbus_message_iter_init(message, &args) || dbus_message_iter_get_arg_type(&args) != DBUS_TYPE_ARRAY) return;
    DBusMessageIter values;
    dbus_message_iter_recurse(&args, &values);
    while (dbus_message_iter_get_arg_type(&values) == DBUS_TYPE_DICT_ENTRY) {
        DBusMessageIter entry;
        dbus_message_iter_recurse(&values, &entry);
        const char* key = 0;
        if (dbus_message_iter_get_arg_type(&entry) == DBUS_TYPE_STRING) dbus_message_iter_get_basic(&entry, &key);
        if (key && dbus_message_iter_next(&entry) && dbus_message_iter_get_arg_type(&entry) == DBUS_TYPE_VARIANT) {
            DBusMessageIter value;
            dbus_message_iter_recurse(&entry, &value);
            if (std::string(key) == "WirelessEnabled" && dbus_message_iter_get_arg_type(&value) == DBUS_TYPE_BOOLEAN) {
                dbus_bool_t enabled = FALSE; dbus_message_iter_get_basic(&value, &enabled); status_.wifiEnabled = enabled != FALSE;
            } else if (std::string(key) == "NetworkingEnabled" && dbus_message_iter_get_arg_type(&value) == DBUS_TYPE_BOOLEAN) {
                dbus_bool_t enabled = FALSE; dbus_message_iter_get_basic(&value, &enabled); status_.networkingEnabled = enabled != FALSE;
            } else if (std::string(key) == "State" && dbus_message_iter_get_arg_type(&value) == DBUS_TYPE_UINT32) {
                uint32_t state = 0; dbus_message_iter_get_basic(&value, &state);
                status_.state = state >= 50 ? NetworkWifiConnected : (state >= 40 ? NetworkWifiConnecting : NetworkDisconnected);
            }
        }
        dbus_message_iter_next(&values);
    }
    if (!isStale(capturedGeneration)) notifyListeners();
}
void NetworkManager::trackPending(DBusPendingCall* pending) {
    if (pending) {
        dbus_pending_call_ref(pending);
        impl_->pendingCalls.push_back(pending);
    }
}

void NetworkManager::untrackPending(DBusPendingCall* pending) {
    std::vector<DBusPendingCall*>::iterator it = std::find(impl_->pendingCalls.begin(), impl_->pendingCalls.end(), pending);
    if (it != impl_->pendingCalls.end()) {
        dbus_pending_call_unref(*it);
        impl_->pendingCalls.erase(it);
    }
}
#endif

} // namespace integrations
} // namespace flamewm
