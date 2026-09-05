#include "mpris.h"
#include "dbusdispatcher.h"
#include <algorithm>
#include <cstring>

#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
#include <dbus/dbus.h>
#endif

namespace flamewm { namespace integrations {

static const char* kService = "mpris";

#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
struct WatchData { Mpris* owner; int id; };
struct TimeoutData { Mpris* owner; int id; };
struct CallData { Mpris* owner; uint64_t generation; std::string bus; bool properties; };
static void freeCallData(void* data) { delete static_cast<CallData*>(data); }

static dbus_bool_t addWatch(DBusWatch* watch, void* data) {
    WatchData* wd = new WatchData;
    wd->owner = static_cast<Mpris*>(data);
    wd->id = wd->owner->dispatcher()->addWatch(dbus_watch_get_unix_fd(watch),
        (dbus_watch_get_flags(watch) & DBUS_WATCH_READABLE) != 0,
        (dbus_watch_get_flags(watch) & DBUS_WATCH_WRITABLE) != 0, kService);
    dbus_watch_set_data(watch, wd, 0);
    return wd->id >= 0 ? TRUE : FALSE;
}
static void removeWatch(DBusWatch* watch, void*) {
    WatchData* wd = static_cast<WatchData*>(dbus_watch_get_data(watch));
    if (!wd) return;
    wd->owner->dispatcher()->removeWatch(wd->id);
    delete wd;
    dbus_watch_set_data(watch, 0, 0);
}
static void toggleWatch(DBusWatch* watch, void*) {
    WatchData* wd = static_cast<WatchData*>(dbus_watch_get_data(watch));
    if (wd) wd->owner->dispatcher()->setWatchEnabled(wd->id, dbus_watch_get_enabled(watch));
}
static dbus_bool_t addTimeout(DBusTimeout* timeout, void* data) {
    TimeoutData* td = new TimeoutData;
    td->owner = static_cast<Mpris*>(data);
    td->id = td->owner->dispatcher()->addTimeout(dbus_timeout_get_interval(timeout), kService);
    dbus_timeout_set_data(timeout, td, 0);
    return td->id >= 0 ? TRUE : FALSE;
}
static void removeTimeout(DBusTimeout* timeout, void*) {
    TimeoutData* td = static_cast<TimeoutData*>(dbus_timeout_get_data(timeout));
    if (!td) return;
    td->owner->dispatcher()->removeTimeout(td->id);
    delete td;
    dbus_timeout_set_data(timeout, 0, 0);
}
static void toggleTimeout(DBusTimeout* timeout, void*) {
    TimeoutData* td = static_cast<TimeoutData*>(dbus_timeout_get_data(timeout));
    if (td) td->owner->dispatcher()->setTimeoutEnabled(td->id, dbus_timeout_get_enabled(timeout));
}

static void updateString(DBusMessageIter* dict, const char* key, std::string* out) {
    DBusMessageIter entry, value;
    dbus_message_iter_recurse(dict, &entry);
    const char* name = 0;
    dbus_message_iter_get_basic(&entry, &name);
    if (!name || std::string(name) != key) return;
    dbus_message_iter_next(&entry);
    dbus_message_iter_recurse(&entry, &value);
    if (dbus_message_iter_get_arg_type(&value) == DBUS_TYPE_STRING) {
        const char* v = 0; dbus_message_iter_get_basic(&value, &v); if (v) *out = v;
    } else if (dbus_message_iter_get_arg_type(&value) == DBUS_TYPE_ARRAY) {
        DBusMessageIter array; dbus_message_iter_recurse(&value, &array);
        if (dbus_message_iter_get_arg_type(&array) == DBUS_TYPE_STRING) { const char* v = 0; dbus_message_iter_get_basic(&array, &v); if (v) *out = v; }
    }
}
static void updateBool(DBusMessageIter* dict, const char* key, bool* out) {
    DBusMessageIter entry, value;
    dbus_message_iter_recurse(dict, &entry);
    const char* name = 0; dbus_message_iter_get_basic(&entry, &name);
    if (!name || std::string(name) != key) return;
    dbus_message_iter_next(&entry); dbus_message_iter_recurse(&entry, &value);
    if (dbus_message_iter_get_arg_type(&value) == DBUS_TYPE_BOOLEAN) {
        dbus_bool_t v = FALSE; dbus_message_iter_get_basic(&value, &v); *out = v != FALSE;
    }
}
static void updateMetadata(DBusMessageIter* dict, PlayerInfo* p) {
    DBusMessageIter entry, value, metadata;
    dbus_message_iter_recurse(dict, &entry);
    const char* name = 0; dbus_message_iter_get_basic(&entry, &name);
    if (!name || std::string(name) != "Metadata") return;
    dbus_message_iter_next(&entry); dbus_message_iter_recurse(&entry, &value);
    if (dbus_message_iter_get_arg_type(&value) != DBUS_TYPE_ARRAY) return;
    dbus_message_iter_recurse(&value, &metadata);
    while (dbus_message_iter_get_arg_type(&metadata) != DBUS_TYPE_INVALID) {
        updateString(&metadata, "xesam:title", &p->trackTitle);
        updateString(&metadata, "xesam:artist", &p->artist);
        dbus_message_iter_next(&metadata);
    }
}
static void readProperties(DBusMessage* message, PlayerInfo* p) {
    DBusMessageIter root;
    if (!dbus_message_iter_init(message, &root)) return;
    if (dbus_message_iter_get_arg_type(&root) == DBUS_TYPE_STRING) {
        const char* iface = 0; dbus_message_iter_get_basic(&root, &iface);
        if (!dbus_message_iter_next(&root)) return;
    }
    if (dbus_message_iter_get_arg_type(&root) != DBUS_TYPE_ARRAY) return;
    DBusMessageIter dict; dbus_message_iter_recurse(&root, &dict);
    while (dbus_message_iter_get_arg_type(&dict) != DBUS_TYPE_INVALID) {
        updateString(&dict, "Identity", &p->identity);
        updateString(&dict, "Title", &p->trackTitle);
        updateString(&dict, "Artist", &p->artist);
        updateMetadata(&dict, p);
        updateBool(&dict, "CanPlay", &p->canPlay);
        updateBool(&dict, "CanPause", &p->canPause);
        updateBool(&dict, "CanGoNext", &p->canNext);
        updateBool(&dict, "CanGoPrevious", &p->canPrev);
        DBusMessageIter entry, value; dbus_message_iter_recurse(&dict, &entry);
        const char* key = 0; dbus_message_iter_get_basic(&entry, &key);
        if (key && std::string(key) == "PlaybackStatus") {
            dbus_message_iter_next(&entry); dbus_message_iter_recurse(&entry, &value);
            const char* status = 0; dbus_message_iter_get_basic(&value, &status);
            if (status && std::string(status) == "Playing") p->status = PlaybackPlaying;
            else if (status && std::string(status) == "Paused") p->status = PlaybackPaused;
            else p->status = PlaybackStopped;
        }
        dbus_message_iter_next(&dict);
    }
}
static void callFinished(DBusPendingCall* pending, void* data) {
    CallData* cd = static_cast<CallData*>(data);
    Mpris* owner = cd ? cd->owner : 0;
    if (owner) owner->untrackPending(pending);
    DBusMessage* reply = dbus_pending_call_steal_reply(pending);
    if (reply && cd && cd->properties && cd->owner && !cd->owner->isStale(cd->generation) && cd->owner->hasPlayer(cd->bus)) {
        PlayerInfo p; p.busName = cd->bus; readProperties(reply, &p); cd->owner->onPlayerPropertiesChanged(cd->bus, p);
    }
    if (reply) dbus_message_unref(reply);
    dbus_pending_call_unref(pending);
}
static void discoverFinished(DBusPendingCall* pending, void* data) {
    CallData* cd = static_cast<CallData*>(data);
    Mpris* owner = cd ? cd->owner : 0;
    const uint64_t generation = cd ? cd->generation : 0;
    if (owner) owner->untrackPending(pending);
    DBusMessage* reply = dbus_pending_call_steal_reply(pending);
    DBusMessageIter i, array;
    if (reply && owner && !owner->isStale(generation) &&
        dbus_message_iter_init(reply, &i) && dbus_message_iter_get_arg_type(&i) == DBUS_TYPE_ARRAY) {
        dbus_message_iter_recurse(&i, &array);
        while (dbus_message_iter_get_arg_type(&array) != DBUS_TYPE_INVALID) {
            const char* name = 0; dbus_message_iter_get_basic(&array, &name);
            if (name && !strncmp(name, "org.mpris.MediaPlayer2.", 24)) owner->onPlayerAppeared(name, name + 24);
            dbus_message_iter_next(&array);
        }
    }
    if (reply) dbus_message_unref(reply);
    dbus_pending_call_unref(pending);
}
static DBusHandlerResult filterMessage(DBusConnection*, DBusMessage* m, void* data) {
    Mpris* owner = static_cast<Mpris*>(data);
    if (dbus_message_is_signal(m, "org.freedesktop.DBus", "NameOwnerChanged")) {
        DBusMessageIter i; const char *name = 0, *oldOwner = 0, *newOwner = 0;
        if (dbus_message_iter_init(m, &i)) { dbus_message_iter_get_basic(&i, &name); dbus_message_iter_next(&i); dbus_message_iter_get_basic(&i, &oldOwner); dbus_message_iter_next(&i); dbus_message_iter_get_basic(&i, &newOwner); }
        if (name && !strncmp(name, "org.mpris.MediaPlayer2.", 24)) {
            if (newOwner && *newOwner) { owner->mapOwner(newOwner, name); owner->onPlayerAppeared(name, name + 24); }
            else { if (oldOwner) owner->unmapOwner(oldOwner); owner->onPlayerVanished(name); }
        }
    } else if (dbus_message_is_signal(m, "org.freedesktop.DBus.Properties", "PropertiesChanged")) {
        const char* path = dbus_message_get_path(m);
        if (path && !strncmp(path, "/org/mpris/MediaPlayer2", 23)) {
            const char* sender = dbus_message_get_sender(m);
            std::string bus = sender ? owner->mappedOwner(sender) : std::string();
            if (!bus.empty()) { PlayerInfo p; p.busName = bus; readProperties(m, &p); owner->onPlayerPropertiesChanged(bus, p); }
        }
    }
    return DBUS_HANDLER_RESULT_NOT_YET_HANDLED;
}
#endif

Mpris::Mpris() : generation_(1), reconcilePending_(false), dispatcher_(0), connection_(0), watchData_(0), timeoutData_(0) {}
Mpris::Mpris(DBusDispatcher* dispatcher) : generation_(1), reconcilePending_(false), dispatcher_(dispatcher), connection_(0), watchData_(0), timeoutData_(0) { startSessionBus(); }
Mpris::~Mpris() { stopSessionBus(); listeners_.clear(); }

void Mpris::startSessionBus() {
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    if (!dispatcher_ || connection_) return;
    DBusError error; dbus_error_init(&error);
    DBusConnection* c = dbus_bus_get_private(DBUS_BUS_SESSION, &error);
    if (!c) { dbus_error_free(&error); return; }
    dbus_connection_set_exit_on_disconnect(c, FALSE);
    dbus_connection_set_watch_functions(c, addWatch, removeWatch, toggleWatch, this, 0);
    dbus_connection_set_timeout_functions(c, addTimeout, removeTimeout, toggleTimeout, this, 0);
    dbus_connection_add_filter(c, filterMessage, this, 0);
    dbus_bus_add_match(c, "type='signal',interface='org.freedesktop.DBus',member='NameOwnerChanged'", 0);
    dbus_bus_add_match(c, "type='signal',interface='org.freedesktop.DBus.Properties',member='PropertiesChanged',path='/org/mpris/MediaPlayer2'", 0);
    dbus_connection_flush(c); connection_ = c; dispatcher_->addConnection(c);
    DBusMessage* list = dbus_message_new_method_call("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "ListNames");
    DBusPendingCall* pending = 0;
    if (list && dbus_connection_send_with_reply(c, list, &pending, -1) && pending) {
        CallData* cd = new CallData;
        cd->owner = this; cd->generation = generation_; cd->properties = false;
        if (!dbus_pending_call_set_notify(pending, discoverFinished, cd, freeCallData)) {
            delete cd;
            dbus_pending_call_unref(pending);
        } else {
            trackPending(pending);
        }
        dbus_connection_flush(c);
    }
    if (list) dbus_message_unref(list);
#endif
}
void Mpris::stopSessionBus() {
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    bumpGeneration();
    for (size_t i = 0; i < pendingCalls_.size(); ++i) {
        dbus_pending_call_set_notify(pendingCalls_[i], 0, 0, 0);
        dbus_pending_call_cancel(pendingCalls_[i]);
        dbus_pending_call_unref(pendingCalls_[i]);
    }
    pendingCalls_.clear();
    if (connection_) { DBusConnection* c = connection_; if (dispatcher_) dispatcher_->removeConnection(c); dbus_connection_remove_filter(c, filterMessage, this); dbus_connection_close(c); dbus_connection_unref(c); connection_ = 0; }
#endif
}

void Mpris::addListener(MprisListener* l) { if (l && std::find(listeners_.begin(), listeners_.end(), l) == listeners_.end()) listeners_.push_back(l); }
void Mpris::removeListener(MprisListener* l) { listeners_.erase(std::remove(listeners_.begin(), listeners_.end(), l), listeners_.end()); }
void Mpris::onPlayerAppeared(const std::string& busName, const std::string& identity) {
    if (busName.empty()) return;
    int i = findPlayerIndex(busName);
    if (i < 0) { PlayerInfo p; p.busName = busName; p.identity = identity; players_.push_back(p); }
    else if (!identity.empty()) players_[i].identity = identity;
    if (connection_) requestPlayerProperties(busName, generation_);
    updateActive(); notifyListeners();
}
void Mpris::onPlayerVanished(const std::string& busName) {
    removeStalePlayer(busName); updateActive(); notifyListeners();
}
void Mpris::onPlayerPropertiesChanged(const std::string& busName, const PlayerInfo& updated) {
    int i = findPlayerIndex(busName); if (i < 0) return;
    players_[i] = updated; players_[i].busName = busName; updateActive(); notifyListeners();
}
void Mpris::reconcile() { reconcilePending_ = false; updateActive(); notifyListeners(); }
const PlayerInfo* Mpris::activePlayer() const { int i = findPlayerIndex(activeBusName_); return i < 0 ? 0 : &players_[i]; }
std::string Mpris::pickActive(const std::vector<PlayerInfo>& ps) {
    std::string playing, paused, any;
    for (size_t i = 0; i < ps.size(); ++i) {
        const std::string& n = ps[i].busName;
        if (any.empty() || n < any) any = n;
        if (ps[i].status == PlaybackPlaying && (playing.empty() || n < playing)) playing = n;
        if (ps[i].status == PlaybackPaused && (paused.empty() || n < paused)) paused = n;
    }
    return !playing.empty() ? playing : (!paused.empty() ? paused : any);
}
static bool sendControl(Mpris* self, const std::string& bus, uint64_t gen, const char* method) {
    if (!self || self->isStale(gen) || bus.empty() || !self->hasPlayer(bus)) return false;
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    DBusConnection* c = self->connection();
    if (!c) return false;
    DBusMessage* m = dbus_message_new_method_call(bus.c_str(), "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player", method);
    if (!m) return false;
    CallData* cd = new CallData; cd->owner = self; cd->generation = gen; cd->bus = bus; cd->properties = false;
    DBusPendingCall* pending = 0;
     bool ok = dbus_connection_send_with_reply(c, m, &pending, -1) && pending && dbus_pending_call_set_notify(pending, callFinished, cd, freeCallData);
     dbus_message_unref(m); if (!ok) { if (pending) dbus_pending_call_unref(pending); delete cd; }
     else { self->trackPending(pending); dbus_connection_flush(c); }
    return ok;
#else
    return false;
#endif
}
bool Mpris::requestPlay(const std::string& b, uint64_t g) { return sendControl(this, b, g, "Play"); }
bool Mpris::requestPause(const std::string& b, uint64_t g) { return sendControl(this, b, g, "Pause"); }
bool Mpris::requestPlayPause(const std::string& b, uint64_t g) { return sendControl(this, b, g, "PlayPause"); }
bool Mpris::requestNext(const std::string& b, uint64_t g) { return sendControl(this, b, g, "Next"); }
bool Mpris::requestPrev(const std::string& b, uint64_t g) { return sendControl(this, b, g, "Previous"); }
void Mpris::injectPlayer(const PlayerInfo& p) { int i = findPlayerIndex(p.busName); if (i < 0) players_.push_back(p); else players_[i] = p; updateActive(); notifyListeners(); }
void Mpris::onServiceOwnerChanged(bool hasOwner) { if (!hasOwner) { players_.clear(); activeBusName_.clear(); bumpGeneration(); notifyListeners(); } else startSessionBus(); }
void Mpris::bumpGeneration() { ++generation_; }
void Mpris::notifyListeners() { std::vector<PlayerInfo> copy = players_; std::vector<MprisListener*> ls = listeners_; for (size_t i = 0; i < ls.size(); ++i) if (ls[i]) ls[i]->onPlayersChanged(copy, activeBusName_, generation_); }
void Mpris::removeStalePlayer(const std::string& b) { int i = findPlayerIndex(b); if (i >= 0) players_.erase(players_.begin() + i); }
int Mpris::findPlayerIndex(const std::string& b) const { for (size_t i = 0; i < players_.size(); ++i) if (players_[i].busName == b) return (int)i; return -1; }
void Mpris::updateActive() { activeBusName_ = pickActive(players_); }
void Mpris::requestPlayerProperties(const std::string& bus, uint64_t gen) {
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    if (!connection_ || isStale(gen)) return;
    DBusMessage* m = dbus_message_new_method_call(bus.c_str(), "/org/mpris/MediaPlayer2", "org.freedesktop.DBus.Properties", "GetAll");
    if (!m) return;
    const char* iface = "org.mpris.MediaPlayer2";
    dbus_message_append_args(m, DBUS_TYPE_STRING, &iface, DBUS_TYPE_INVALID);
    DBusPendingCall* p = 0; if (dbus_connection_send_with_reply(connection_, m, &p, -1) && p) { CallData* cd = new CallData; cd->owner = this; cd->generation = gen; cd->bus = bus; cd->properties = true; if (!dbus_pending_call_set_notify(p, callFinished, cd, freeCallData)) { delete cd; dbus_pending_call_unref(p); } else { trackPending(p); } dbus_connection_flush(connection_); } dbus_message_unref(m);
#else
    (void)bus; (void)gen;
#endif
}

void Mpris::trackPending(DBusPendingCall* pending) {
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    if (pending) {
        dbus_pending_call_ref(pending);
        pendingCalls_.push_back(pending);
    }
#else
    (void)pending;
#endif
}

void Mpris::untrackPending(DBusPendingCall* pending) {
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    std::vector<DBusPendingCall*>::iterator it = std::find(pendingCalls_.begin(), pendingCalls_.end(), pending);
    if (it != pendingCalls_.end()) {
        dbus_pending_call_unref(*it);
        pendingCalls_.erase(it);
    }
#else
    (void)pending;
#endif
}

} // namespace integrations
} // namespace flamewm
