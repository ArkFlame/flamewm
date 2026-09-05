#include "flamewm/control/server.h"

#include "flamewm/platform/host.h"
#include "flamewm/core/version.h"
#include "flamewm/api/capabilities.h"
#include "flamewm/api/errors.h"
#include "flamewm/api/settings.h"
#include "flamewm/api/workspace.h"
#include "flamewm/api/display.h"
#include "flamewm/api/shortcuts.h"
#include "flamewm/api/panels.h"
#include "flamewm/api/session.h"
#include "flamewm/api/ids.h"

#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
#define FLAMEWM_HAS_DBUS 1
#endif

#if defined(FLAMEWM_HAS_DBUS)
#if defined(__has_include)
#if __has_include(<dbus/dbus.h>)
#include <dbus/dbus.h>
#define FLAMEWM_DBUS_HEADERS 1
#endif
#else
#include <dbus/dbus.h>
#define FLAMEWM_DBUS_HEADERS 1
#endif
#ifndef FLAMEWM_DBUS_HEADERS
#undef FLAMEWM_HAS_DBUS
#endif
#endif

#include <string>
#include <vector>
#include <map>
#include <cstring>
#include <cstdlib>

namespace flamewm {
namespace control {

namespace {

static const char kBusName[] = "com.arkflame.FlameWM1";
static const char kObjectPath[] = "/com/arkflame/FlameWM1";
static const char kIfaceRoot[] = "com.arkflame.FlameWM1";
static const char kIfaceSettings[] = "com.arkflame.FlameWM1.Settings";
static const char kIfaceWorkspaces[] = "com.arkflame.FlameWM1.Workspaces";
static const char kIfaceDisplays[] = "com.arkflame.FlameWM1.Displays";
static const char kIfaceShortcuts[] = "com.arkflame.FlameWM1.Shortcuts";
static const char kIfacePanels[] = "com.arkflame.FlameWM1.Panels";
static const char kIfaceSession[] = "com.arkflame.FlameWM1.Session";
static const char kErrorPrefix[] = "com.arkflame.FlameWM1.Error.";

inline std::string controlErrorName(api::Error e) {
    return std::string(kErrorPrefix) + api::errorName(e);
}

#ifdef FLAMEWM_HAS_DBUS
inline void appendDictVariantUint64(DBusMessageIter* arr, const char* key, dbus_uint64_t v) {
    DBusMessageIter entry, var;
    dbus_message_iter_open_container(arr, DBUS_TYPE_DICT_ENTRY, NULL, &entry);
    dbus_message_iter_append_basic(&entry, DBUS_TYPE_STRING, &key);
    dbus_message_iter_open_container(&entry, DBUS_TYPE_VARIANT, "t", &var);
    dbus_message_iter_append_basic(&var, DBUS_TYPE_UINT64, &v);
    dbus_message_iter_close_container(&entry, &var);
    dbus_message_iter_close_container(arr, &entry);
}
inline void appendDictVariantUint32(DBusMessageIter* arr, const char* key, dbus_uint32_t v) {
    DBusMessageIter entry, var;
    dbus_message_iter_open_container(arr, DBUS_TYPE_DICT_ENTRY, NULL, &entry);
    dbus_message_iter_append_basic(&entry, DBUS_TYPE_STRING, &key);
    dbus_message_iter_open_container(&entry, DBUS_TYPE_VARIANT, "u", &var);
    dbus_message_iter_append_basic(&var, DBUS_TYPE_UINT32, &v);
    dbus_message_iter_close_container(&entry, &var);
    dbus_message_iter_close_container(arr, &entry);
}
inline void appendDictVariantInt32(DBusMessageIter* arr, const char* key, dbus_int32_t v) {
    DBusMessageIter entry, var;
    dbus_message_iter_open_container(arr, DBUS_TYPE_DICT_ENTRY, NULL, &entry);
    dbus_message_iter_append_basic(&entry, DBUS_TYPE_STRING, &key);
    dbus_message_iter_open_container(&entry, DBUS_TYPE_VARIANT, "i", &var);
    dbus_message_iter_append_basic(&var, DBUS_TYPE_INT32, &v);
    dbus_message_iter_close_container(&entry, &var);
    dbus_message_iter_close_container(arr, &entry);
}
inline void appendDictVariantString(DBusMessageIter* arr, const char* key, const char* v) {
    DBusMessageIter entry, var;
    dbus_message_iter_open_container(arr, DBUS_TYPE_DICT_ENTRY, NULL, &entry);
    dbus_message_iter_append_basic(&entry, DBUS_TYPE_STRING, &key);
    dbus_message_iter_open_container(&entry, DBUS_TYPE_VARIANT, "s", &var);
    dbus_message_iter_append_basic(&var, DBUS_TYPE_STRING, &v);
    dbus_message_iter_close_container(&entry, &var);
    dbus_message_iter_close_container(arr, &entry);
}
inline void appendDictVariantBool(DBusMessageIter* arr, const char* key, dbus_bool_t v) {
    DBusMessageIter entry, var;
    dbus_message_iter_open_container(arr, DBUS_TYPE_DICT_ENTRY, NULL, &entry);
    dbus_message_iter_append_basic(&entry, DBUS_TYPE_STRING, &key);
    dbus_message_iter_open_container(&entry, DBUS_TYPE_VARIANT, "b", &var);
    dbus_message_iter_append_basic(&var, DBUS_TYPE_BOOLEAN, &v);
    dbus_message_iter_close_container(&entry, &var);
    dbus_message_iter_close_container(arr, &entry);
}
#endif

api::PanelEdge panelEdgeFromString(const std::string& s, bool* ok) {
    if (ok) *ok = true;
    if (s == "bottom") return api::PanelEdge::Bottom;
    if (s == "top") return api::PanelEdge::Top;
    if (s == "left") return api::PanelEdge::Left;
    if (s == "right") return api::PanelEdge::Right;
    if (ok) *ok = false;
    return api::PanelEdge::Bottom;
}

} // namespace

struct ControlServer::Impl {
    platform::PlatformHost* host;
    bool running;

#ifdef FLAMEWM_HAS_DBUS
    DBusConnection* conn;
    api::MainLoopPort* loop;
    struct WatchCtx {
        DBusWatch* watch;
        api::MainLoopPort::FdHandle handle;
        int fd;
        bool enabled;
    };
    struct TimeoutCtx {
        DBusTimeout* timeout;
        api::MainLoopPort::TimerHandle handle;
        bool enabled;
        int intervalMs;
    };
    std::map<DBusWatch*, WatchCtx> watches;
    std::map<DBusTimeout*, TimeoutCtx> timeouts;
    DBusObjectPathVTable vtable;
    uint64_t generation;

    static dbus_bool_t watchAdd(DBusWatch* w, void* data) {
        Impl* self = static_cast<Impl*>(data);
        if (!self || !w) return FALSE;
        if (!dbus_watch_get_enabled(w)) return TRUE;
        int fd = dbus_watch_get_unix_fd(w);
        unsigned flags = dbus_watch_get_flags(w);
        bool read = (flags & DBUS_WATCH_READABLE) != 0;
        bool write = (flags & DBUS_WATCH_WRITABLE) != 0;
        int events = 0;
        if (read) events |= 1;
        if (write) events |= 4;
        if (self->loop) {
            DBusWatch* ww = w;
            Impl* ss = self;
            api::MainLoopPort::FdCallback cb = [ss, ww](int, int ev) {
                unsigned f = 0;
                if (ev & 1) f |= DBUS_WATCH_READABLE;
                if (ev & 4) f |= DBUS_WATCH_WRITABLE;
                dbus_watch_handle(ww, f);
                while (ss->conn && dbus_connection_get_dispatch_status(ss->conn) == DBUS_DISPATCH_DATA_REMAINS) {
                    dbus_connection_dispatch(ss->conn);
                }
            };
            api::MainLoopPort::FdHandle h = self->loop->addPoll(fd, events, cb);
            WatchCtx ctx;
            ctx.watch = w;
            ctx.handle = h;
            ctx.fd = fd;
            ctx.enabled = true;
            self->watches[w] = ctx;
        } else {
            WatchCtx ctx;
            ctx.watch = w;
            ctx.handle = api::MainLoopPort::FdHandle();
            ctx.handle.id = -1;
            ctx.fd = fd;
            ctx.enabled = true;
            self->watches[w] = ctx;
        }
        return TRUE;
    }

    static void watchRemove(DBusWatch* w, void* data) {
        Impl* self = static_cast<Impl*>(data);
        if (!self || !w) return;
        std::map<DBusWatch*, WatchCtx>::iterator it = self->watches.find(w);
        if (it == self->watches.end()) return;
        if (self->loop && it->second.handle.id >= 0) {
            self->loop->removePoll(it->second.handle);
        }
        self->watches.erase(it);
    }

    static void watchToggle(DBusWatch* w, void* data) {
        Impl* self = static_cast<Impl*>(data);
        if (!self || !w) return;
        bool enabled = dbus_watch_get_enabled(w) != FALSE;
        std::map<DBusWatch*, WatchCtx>::iterator it = self->watches.find(w);
        if (it == self->watches.end()) return;
        it->second.enabled = enabled;
        if (!enabled && self->loop && it->second.handle.id >= 0) {
            self->loop->removePoll(it->second.handle);
            it->second.handle.id = -1;
        } else if (enabled && self->loop && it->second.handle.id < 0) {
            int fd = it->second.fd;
            unsigned flags = dbus_watch_get_flags(w);
            int events = 0;
            if (flags & DBUS_WATCH_READABLE) events |= 1;
            if (flags & DBUS_WATCH_WRITABLE) events |= 4;
            DBusWatch* ww = w;
            Impl* ss = self;
            api::MainLoopPort::FdCallback cb = [ss, ww](int, int ev) {
                unsigned f = 0;
                if (ev & 1) f |= DBUS_WATCH_READABLE;
                if (ev & 4) f |= DBUS_WATCH_WRITABLE;
                dbus_watch_handle(ww, f);
                while (ss->conn && dbus_connection_get_dispatch_status(ss->conn) == DBUS_DISPATCH_DATA_REMAINS) {
                    dbus_connection_dispatch(ss->conn);
                }
            };
            it->second.handle = self->loop->addPoll(fd, events, cb);
        }
    }

    static dbus_bool_t timeoutAdd(DBusTimeout* t, void* data) {
        Impl* self = static_cast<Impl*>(data);
        if (!self || !t) return FALSE;
        if (!dbus_timeout_get_enabled(t)) return TRUE;
        int interval = dbus_timeout_get_interval(t);
        if (self->loop) {
            DBusTimeout* tt = t;
            Impl* ss = self;
            api::MainLoopPort::TimerCallback cb = [ss, tt]() {
                dbus_timeout_handle(tt);
                while (ss->conn && dbus_connection_get_dispatch_status(ss->conn) == DBUS_DISPATCH_DATA_REMAINS) {
                    dbus_connection_dispatch(ss->conn);
                }
            };
            api::MainLoopPort::TimerHandle h = self->loop->addTimer(static_cast<uint64_t>(interval), cb, false);
            TimeoutCtx ctx;
            ctx.timeout = t;
            ctx.handle = h;
            ctx.enabled = true;
            ctx.intervalMs = interval;
            self->timeouts[t] = ctx;
        } else {
            TimeoutCtx ctx;
            ctx.timeout = t;
            ctx.handle = api::MainLoopPort::TimerHandle();
            ctx.handle.id = -1;
            ctx.enabled = true;
            ctx.intervalMs = interval;
            self->timeouts[t] = ctx;
        }
        return TRUE;
    }

    static void timeoutRemove(DBusTimeout* t, void* data) {
        Impl* self = static_cast<Impl*>(data);
        if (!self || !t) return;
        std::map<DBusTimeout*, TimeoutCtx>::iterator it = self->timeouts.find(t);
        if (it == self->timeouts.end()) return;
        if (self->loop && it->second.handle.id >= 0) {
            self->loop->removeTimer(it->second.handle);
        }
        self->timeouts.erase(it);
    }

    static void timeoutToggle(DBusTimeout* t, void* data) {
        Impl* self = static_cast<Impl*>(data);
        if (!self || !t) return;
        bool enabled = dbus_timeout_get_enabled(t) != FALSE;
        std::map<DBusTimeout*, TimeoutCtx>::iterator it = self->timeouts.find(t);
        if (it == self->timeouts.end()) return;
        it->second.enabled = enabled;
        if (!enabled && self->loop && it->second.handle.id >= 0) {
            self->loop->removeTimer(it->second.handle);
            it->second.handle.id = -1;
        } else if (enabled && self->loop && it->second.handle.id < 0) {
            int interval = it->second.intervalMs;
            DBusTimeout* tt = t;
            Impl* ss = self;
            api::MainLoopPort::TimerCallback cb = [ss, tt]() {
                dbus_timeout_handle(tt);
                while (ss->conn && dbus_connection_get_dispatch_status(ss->conn) == DBUS_DISPATCH_DATA_REMAINS) {
                    dbus_connection_dispatch(ss->conn);
                }
            };
            it->second.handle = self->loop->addTimer(static_cast<uint64_t>(interval), cb, false);
        }
    }

    static void dispatchStatus(DBusConnection* c, DBusDispatchStatus status, void* data) {
        (void)c;
        (void)status;
        Impl* self = static_cast<Impl*>(data);
        if (!self || !self->conn) return;
        while (self->conn && dbus_connection_get_dispatch_status(self->conn) == DBUS_DISPATCH_DATA_REMAINS) {
            dbus_connection_dispatch(self->conn);
        }
    }

    static DBusHandlerResult messageFilter(DBusConnection* c, DBusMessage* msg, void* data) {
        Impl* self = static_cast<Impl*>(data);
        if (!self || !c || !msg) return DBUS_HANDLER_RESULT_NOT_YET_HANDLED;
        if (dbus_message_get_type(msg) != DBUS_MESSAGE_TYPE_METHOD_CALL) return DBUS_HANDLER_RESULT_NOT_YET_HANDLED;
        const char* path = dbus_message_get_path(msg);
        const char* iface = dbus_message_get_interface(msg);
        if (!path || !iface) return DBUS_HANDLER_RESULT_NOT_YET_HANDLED;
        if (std::strcmp(path, kObjectPath) != 0) return DBUS_HANDLER_RESULT_NOT_YET_HANDLED;
        // Dispatch across all known interfaces
        bool known = (std::strcmp(iface, kIfaceRoot) == 0
                   || std::strcmp(iface, kIfaceSettings) == 0
                   || std::strcmp(iface, kIfaceWorkspaces) == 0
                   || std::strcmp(iface, kIfaceDisplays) == 0
                   || std::strcmp(iface, kIfaceShortcuts) == 0
                   || std::strcmp(iface, kIfacePanels) == 0
                   || std::strcmp(iface, kIfaceSession) == 0);
        if (!known) return DBUS_HANDLER_RESULT_NOT_YET_HANDLED;
        return self->handleMethod(c, msg);
    }

    DBusHandlerResult handleMethod(DBusConnection* c, DBusMessage* msg) {
        const char* member = dbus_message_get_member(msg);
        const char* iface = dbus_message_get_interface(msg);
        if (!member || !iface) return sendError(c, msg, api::Error::InvalidArgument, "missing method");
        std::string m(member);
        std::string iff(iface);
        // Root interface
        if (iff == kIfaceRoot) {
            if (m == "GetVersion") return handleGetVersion(c, msg);
            if (m == "GetCapabilities") return handleGetCapabilities(c, msg);
            if (m == "Ping") return handlePing(c, msg);
            return sendError(c, msg, api::Error::Unsupported, "unknown method: " + m);
        }
        if (iff == kIfaceSettings) {
            if (m == "GetSnapshot") return handleSettingsGetSnapshot(c, msg);
            if (m == "Apply") return handleSettingsApply(c, msg);
            if (m == "ResetSection") return handleSettingsResetSection(c, msg);
            return sendError(c, msg, api::Error::Unsupported, "unknown method: " + m);
        }
        if (iff == kIfaceWorkspaces) {
            if (m == "GetSnapshot") return handleWorkspacesGetSnapshot(c, msg);
            if (m == "Activate") return handleWorkspacesActivate(c, msg);
            if (m == "InsertAfter") return handleWorkspacesInsertAfter(c, msg);
            if (m == "Remove") return handleWorkspacesRemove(c, msg);
            return sendError(c, msg, api::Error::Unsupported, "unknown method: " + m);
        }
        if (iff == kIfaceDisplays) {
            if (m == "GetSnapshot") return handleDisplaysGetSnapshot(c, msg);
            if (m == "BeginModeChange") return handleDisplaysBeginModeChange(c, msg);
            if (m == "Keep") return handleDisplaysKeep(c, msg);
            if (m == "Revert") return handleDisplaysRevert(c, msg);
            if (m == "SetShellScale") return handleDisplaysSetShellScale(c, msg);
            return sendError(c, msg, api::Error::Unsupported, "unknown method: " + m);
        }
        if (iff == kIfaceShortcuts) {
            if (m == "GetSnapshot") return handleShortcutsGetSnapshot(c, msg);
            if (m == "SetBinding") return handleShortcutsSetBinding(c, msg);
            if (m == "ClearBinding") return handleShortcutsClearBinding(c, msg);
            if (m == "ResetBinding") return handleShortcutsResetBinding(c, msg);
            return sendError(c, msg, api::Error::Unsupported, "unknown method: " + m);
        }
        if (iff == kIfacePanels) {
            if (m == "GetSnapshot") return handlePanelsGetSnapshot(c, msg);
            if (m == "SetEdge") return handlePanelsSetEdge(c, msg);
            if (m == "SetSize") return handlePanelsSetSize(c, msg);
            if (m == "Pin") return handlePanelsPin(c, msg);
            if (m == "Unpin") return handlePanelsUnpin(c, msg);
            if (m == "Reorder") return handlePanelsReorder(c, msg);
            return sendError(c, msg, api::Error::Unsupported, "unknown method: " + m);
        }
        if (iff == kIfaceSession) {
            if (m == "GetCapabilities") return handleSessionGetCapabilities(c, msg);
            if (m == "Lock") return handleSessionAction(c, msg, api::SessionAction::Lock);
            if (m == "Logout") return handleSessionAction(c, msg, api::SessionAction::Logout);
            if (m == "Suspend") return handleSessionAction(c, msg, api::SessionAction::Suspend);
            if (m == "Reboot") return handleSessionAction(c, msg, api::SessionAction::Reboot);
            if (m == "Shutdown") return handleSessionAction(c, msg, api::SessionAction::Shutdown);
            return sendError(c, msg, api::Error::Unsupported, "unknown method: " + m);
        }
        return sendError(c, msg, api::Error::Unsupported, "unknown method: " + m);
    }

    DBusHandlerResult sendError(DBusConnection* c, DBusMessage* msg, api::Error e, const std::string& detail) {
        std::string name = controlErrorName(e);
        DBusMessage* err = dbus_message_new_error(msg, name.c_str(), detail.c_str());
        if (!err) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_bool_t ok = dbus_connection_send(c, err, NULL);
        dbus_message_unref(err);
        (void)ok;
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult sendStatusError(DBusConnection* c, DBusMessage* msg, const api::Status& s) {
        api::Error code = s.code;
        if (code == api::Error::None) code = api::Error::InternalFailure;
        return sendError(c, msg, code, s.message);
    }

    // Root handlers

    DBusHandlerResult handleGetVersion(DBusConnection* c, DBusMessage* msg) {
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        const char* v = flameVersion();
        if (!v) v = "0.0.0";
        dbus_message_append_args(reply, DBUS_TYPE_STRING, &v, DBUS_TYPE_INVALID);
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleGetCapabilities(DBusConnection* c, DBusMessage* msg) {
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        api::Capabilities caps = api::Capabilities::current();
        DBusMessageIter iter, arr;
        dbus_message_iter_init_append(reply, &iter);
        dbus_message_iter_open_container(&iter, DBUS_TYPE_ARRAY, DBUS_TYPE_STRING_AS_STRING, &arr);
        for (std::size_t i = 0; i < caps.items.size(); ++i) {
            const char* s = caps.items[i].c_str();
            dbus_message_iter_append_basic(&arr, DBUS_TYPE_STRING, &s);
        }
        dbus_message_iter_close_container(&iter, &arr);
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handlePing(DBusConnection* c, DBusMessage* msg) {
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        const char* pong = "pong";
        dbus_message_append_args(reply, DBUS_TYPE_STRING, &pong, DBUS_TYPE_INVALID);
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    // Settings: GetSnapshot a{sv}, Apply(t,a{sv})->u, ResetSection(t,s)->u
    DBusHandlerResult handleSettingsGetSnapshot(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->settings()) {
            return sendError(c, msg, api::Error::Unavailable, "settings unavailable");
        }
        api::SettingsSnapshot snap = host->settings()->snapshot();
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        DBusMessageIter iter, arr;
        dbus_message_iter_init_append(reply, &iter);
        dbus_message_iter_open_container(&iter, DBUS_TYPE_ARRAY, "{sv}", &arr);
        dbus_uint64_t rev = snap.revision;
        appendDictVariantUint64(&arr, "revision", rev);
        for (std::map<std::string,std::string>::const_iterator it = snap.values.begin(); it != snap.values.end(); ++it) {
            const char* v = it->second.c_str();
            // key needs stable pointer
            appendDictVariantString(&arr, it->first.c_str(), v);
        }
        dbus_message_iter_close_container(&iter, &arr);
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleSettingsApply(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->settings()) {
            return sendError(c, msg, api::Error::Unavailable, "settings unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t expected = 0;
        dbus_message_iter_get_basic(&iter, &expected);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_ARRAY) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected a{sv} changes");
        }
        DBusMessageIter arr;
        dbus_message_iter_recurse(&iter, &arr);
        std::vector<api::SettingsChange> changes;
        while (dbus_message_iter_get_arg_type(&arr) == DBUS_TYPE_DICT_ENTRY) {
            DBusMessageIter entry;
            dbus_message_iter_recurse(&arr, &entry);
            if (dbus_message_iter_get_arg_type(&entry) != DBUS_TYPE_STRING) { dbus_message_iter_next(&arr); continue; }
            const char* k = 0;
            dbus_message_iter_get_basic(&entry, &k);
            dbus_message_iter_next(&entry);
            if (dbus_message_iter_get_arg_type(&entry) != DBUS_TYPE_VARIANT) { dbus_message_iter_next(&arr); continue; }
            DBusMessageIter var;
            dbus_message_iter_recurse(&entry, &var);
            if (dbus_message_iter_get_arg_type(&var) == DBUS_TYPE_STRING) {
                const char* v = 0;
                dbus_message_iter_get_basic(&var, &v);
                if (k && v) changes.push_back(api::SettingsChange(k, v));
            }
            dbus_message_iter_next(&arr);
        }
        api::Status st = host->settings()->apply(static_cast<uint64_t>(expected), changes);
        if (!st.ok()) return sendStatusError(c, msg, st);
        uint64_t newRev = host->settings()->revision();
        // XML says newRevision type u (uint32) but we keep u64 (t) — send both compatible: send u64; legacy clients handle truncation; use t.
        // To satisfy xml introspection exact type we send as UINT64 variant would be wrong. The method returns single u/t.
        // We send UINT64 (t) which matches generation staying u64 per task.
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_uint64_t out = newRev;
        // Return as UINT64 to preserve u64; if client expects UINT32 it will still read low 32.
        dbus_message_append_args(reply, DBUS_TYPE_UINT64, &out, DBUS_TYPE_INVALID);
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        // emit after commit
        emitSettingsChanged(newRev, changes);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleSettingsResetSection(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->settings()) {
            return sendError(c, msg, api::Error::Unavailable, "settings unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t expected = 0;
        dbus_message_iter_get_basic(&iter, &expected);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_STRING) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected string section");
        }
        const char* sec = 0;
        dbus_message_iter_get_basic(&iter, &sec);
        std::string section = sec ? sec : "";
        api::Status st = host->settings()->resetSection(static_cast<uint64_t>(expected), section);
        if (!st.ok()) return sendStatusError(c, msg, st);
        uint64_t newRev = host->settings()->revision();
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_uint64_t out = newRev;
        dbus_message_append_args(reply, DBUS_TYPE_UINT64, &out, DBUS_TYPE_INVALID);
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        std::vector<std::string> keys;
        keys.push_back(section);
        emitSettingsChanged(newRev, keys);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    // Workspaces
    DBusHandlerResult handleWorkspacesGetSnapshot(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->workspaces()) {
            return sendError(c, msg, api::Error::Unavailable, "workspaces unavailable");
        }
        api::WorkspaceSnapshot snap = host->workspaces()->snapshot();
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        DBusMessageIter iter, arr;
        dbus_message_iter_init_append(reply, &iter);
        dbus_message_iter_open_container(&iter, DBUS_TYPE_ARRAY, "{sv}", &arr);
        dbus_uint64_t rev = snap.revision;
        appendDictVariantUint64(&arr, "revision", rev);
        appendDictVariantUint32(&arr, "count", static_cast<dbus_uint32_t>(snap.count));
        appendDictVariantInt32(&arr, "activeIndex", static_cast<dbus_int32_t>(snap.activeIndex));
        appendDictVariantInt32(&arr, "lastIndex", static_cast<dbus_int32_t>(snap.lastIndex));
        if (!snap.names.empty()) {
            DBusMessageIter entry, var, narr;
            const char* k = "names";
            dbus_message_iter_open_container(&arr, DBUS_TYPE_DICT_ENTRY, NULL, &entry);
            dbus_message_iter_append_basic(&entry, DBUS_TYPE_STRING, &k);
            dbus_message_iter_open_container(&entry, DBUS_TYPE_VARIANT, "as", &var);
            dbus_message_iter_open_container(&var, DBUS_TYPE_ARRAY, DBUS_TYPE_STRING_AS_STRING, &narr);
            for (std::size_t i = 0; i < snap.names.size(); ++i) {
                const char* s = snap.names[i].c_str();
                dbus_message_iter_append_basic(&narr, DBUS_TYPE_STRING, &s);
            }
            dbus_message_iter_close_container(&var, &narr);
            dbus_message_iter_close_container(&entry, &var);
            dbus_message_iter_close_container(&arr, &entry);
        }
        dbus_message_iter_close_container(&iter, &arr);
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleWorkspacesActivate(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->workspaces()) {
            return sendError(c, msg, api::Error::Unavailable, "workspaces unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT32) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint32 index");
        }
        dbus_uint32_t idx = 0;
        dbus_message_iter_get_basic(&iter, &idx);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t rev = 0;
        dbus_message_iter_get_basic(&iter, &rev);
        api::Status st = host->workspaces()->activate(static_cast<int>(idx), static_cast<uint64_t>(rev));
        if (!st.ok()) return sendStatusError(c, msg, st);
        uint64_t newRev = host->workspaces()->revision();
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitWorkspacesChanged(newRev);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleWorkspacesInsertAfter(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->workspaces()) {
            return sendError(c, msg, api::Error::Unavailable, "workspaces unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT32) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint32 index");
        }
        dbus_uint32_t idx = 0;
        dbus_message_iter_get_basic(&iter, &idx);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t rev = 0;
        dbus_message_iter_get_basic(&iter, &rev);
        api::Status st = host->workspaces()->insertAfter(static_cast<int>(idx), static_cast<uint64_t>(rev));
        if (!st.ok()) return sendStatusError(c, msg, st);
        uint64_t newRev = host->workspaces()->revision();
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitWorkspacesChanged(newRev);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleWorkspacesRemove(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->workspaces()) {
            return sendError(c, msg, api::Error::Unavailable, "workspaces unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT32) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint32 index");
        }
        dbus_uint32_t idx = 0;
        dbus_message_iter_get_basic(&iter, &idx);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t rev = 0;
        dbus_message_iter_get_basic(&iter, &rev);
        api::Status st = host->workspaces()->remove(static_cast<int>(idx), static_cast<uint64_t>(rev));
        if (!st.ok()) return sendStatusError(c, msg, st);
        uint64_t newRev = host->workspaces()->revision();
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitWorkspacesChanged(newRev);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    // Displays
    DBusHandlerResult handleDisplaysGetSnapshot(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->displays()) {
            return sendError(c, msg, api::Error::Unavailable, "displays unavailable");
        }
        api::DisplaySnapshot snap = host->displays()->snapshot();
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        DBusMessageIter iter, arr;
        dbus_message_iter_init_append(reply, &iter);
        dbus_message_iter_open_container(&iter, DBUS_TYPE_ARRAY, "{sv}", &arr);
        dbus_uint64_t gen = snap.generation;
        appendDictVariantUint64(&arr, "generation", gen);
        appendDictVariantUint32(&arr, "outputCount", static_cast<dbus_uint32_t>(snap.outputs.size()));
        appendDictVariantBool(&arr, "hasPending", snap.hasPending() ? TRUE : FALSE);
        if (snap.hasPending()) {
            appendDictVariantUint64(&arr, "pendingTx", snap.pending.tx.value);
            appendDictVariantUint64(&arr, "pendingDeadlineMs", snap.pending.deadlineMs);
        }
        // Minimal per-output identity: concat key
        if (!snap.outputs.empty()) {
            DBusMessageIter entry, var, oarr;
            const char* k = "outputs";
            dbus_message_iter_open_container(&arr, DBUS_TYPE_DICT_ENTRY, NULL, &entry);
            dbus_message_iter_append_basic(&entry, DBUS_TYPE_STRING, &k);
            dbus_message_iter_open_container(&entry, DBUS_TYPE_VARIANT, "as", &var);
            dbus_message_iter_open_container(&var, DBUS_TYPE_ARRAY, DBUS_TYPE_STRING_AS_STRING, &oarr);
            for (std::size_t i = 0; i < snap.outputs.size(); ++i) {
                const char* s = snap.outputs[i].id.key.c_str();
                dbus_message_iter_append_basic(&oarr, DBUS_TYPE_STRING, &s);
            }
            dbus_message_iter_close_container(&var, &oarr);
            dbus_message_iter_close_container(&entry, &var);
            dbus_message_iter_close_container(&arr, &entry);
        }
        dbus_message_iter_close_container(&iter, &arr);
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleDisplaysBeginModeChange(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->displays()) {
            return sendError(c, msg, api::Error::Unavailable, "displays unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_STRING) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected string outputKey");
        }
        const char* out = 0;
        dbus_message_iter_get_basic(&iter, &out);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 modeId");
        }
        dbus_uint64_t mode = 0;
        dbus_message_iter_get_basic(&iter, &mode);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 generation");
        }
        dbus_uint64_t gen = 0;
        dbus_message_iter_get_basic(&iter, &gen);
        api::OutputId oid(out ? out : "");
        api::ModeId mid(static_cast<uint64_t>(mode));
        api::Result<api::TransactionId> res = host->displays()->beginModeChange(oid, mid, static_cast<uint64_t>(gen));
        if (!res.ok()) return sendStatusError(c, msg, res.status());
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_uint64_t tx = res.value().value;
        dbus_message_append_args(reply, DBUS_TYPE_UINT64, &tx, DBUS_TYPE_INVALID);
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        // pending transaction signal if needed
        emitDisplayTransactionChanged(res.value().value, "pending", 15000);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleDisplaysKeep(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->displays()) {
            return sendError(c, msg, api::Error::Unavailable, "displays unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 transaction");
        }
        dbus_uint64_t tx = 0;
        dbus_message_iter_get_basic(&iter, &tx);
        api::Status st = host->displays()->keep(api::TransactionId(static_cast<uint64_t>(tx)));
        if (!st.ok()) return sendStatusError(c, msg, st);
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitDisplaysChanged(host->displays()->generation());
        emitDisplayTransactionChanged(static_cast<uint64_t>(tx), "kept", 0);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleDisplaysRevert(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->displays()) {
            return sendError(c, msg, api::Error::Unavailable, "displays unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 transaction");
        }
        dbus_uint64_t tx = 0;
        dbus_message_iter_get_basic(&iter, &tx);
        api::Status st = host->displays()->revert(api::TransactionId(static_cast<uint64_t>(tx)));
        if (!st.ok()) return sendStatusError(c, msg, st);
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitDisplaysChanged(host->displays()->generation());
        emitDisplayTransactionChanged(static_cast<uint64_t>(tx), "reverted", 0);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleDisplaysSetShellScale(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->displays()) {
            return sendError(c, msg, api::Error::Unavailable, "displays unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_STRING) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected string outputKey");
        }
        const char* out = 0;
        dbus_message_iter_get_basic(&iter, &out);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT32) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint32 percent");
        }
        dbus_uint32_t pct = 0;
        dbus_message_iter_get_basic(&iter, &pct);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t rev = 0;
        dbus_message_iter_get_basic(&iter, &rev);
        api::OutputId oid(out ? out : "");
        api::Status st = host->displays()->setShellScale(oid, static_cast<int>(pct), static_cast<uint64_t>(rev));
        if (!st.ok()) return sendStatusError(c, msg, st);
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitDisplaysChanged(host->displays()->generation());
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    // Shortcuts
    DBusHandlerResult handleShortcutsGetSnapshot(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->shortcuts()) {
            return sendError(c, msg, api::Error::Unavailable, "shortcuts unavailable");
        }
        api::ShortcutSnapshot snap = host->shortcuts()->snapshot();
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        DBusMessageIter iter, arr;
        dbus_message_iter_init_append(reply, &iter);
        dbus_message_iter_open_container(&iter, DBUS_TYPE_ARRAY, "{sv}", &arr);
        dbus_uint64_t rev = snap.revision;
        appendDictVariantUint64(&arr, "revision", rev);
        for (std::map<std::string, api::KeyBinding>::const_iterator it = snap.bindings.begin(); it != snap.bindings.end(); ++it) {
            const char* v = it->second.key.c_str();
            appendDictVariantString(&arr, it->first.c_str(), v);
        }
        dbus_message_iter_close_container(&iter, &arr);
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleShortcutsSetBinding(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->shortcuts()) {
            return sendError(c, msg, api::Error::Unavailable, "shortcuts unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_STRING) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected string action");
        }
        const char* act = 0;
        dbus_message_iter_get_basic(&iter, &act);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_STRING) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected string binding");
        }
        const char* bind = 0;
        dbus_message_iter_get_basic(&iter, &bind);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t rev = 0;
        dbus_message_iter_get_basic(&iter, &rev);
        std::string action = act ? act : "";
        api::KeyBinding kb(bind ? bind : "");
        api::Status st = host->shortcuts()->setBinding(action, kb, static_cast<uint64_t>(rev));
        if (!st.ok()) return sendStatusError(c, msg, st);
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitShortcutsChanged(host->shortcuts()->revision());
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleShortcutsClearBinding(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->shortcuts()) {
            return sendError(c, msg, api::Error::Unavailable, "shortcuts unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_STRING) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected string action");
        }
        const char* act = 0;
        dbus_message_iter_get_basic(&iter, &act);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t rev = 0;
        dbus_message_iter_get_basic(&iter, &rev);
        std::string action = act ? act : "";
        api::Status st = host->shortcuts()->clearBinding(action, static_cast<uint64_t>(rev));
        if (!st.ok()) return sendStatusError(c, msg, st);
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitShortcutsChanged(host->shortcuts()->revision());
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleShortcutsResetBinding(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->shortcuts()) {
            return sendError(c, msg, api::Error::Unavailable, "shortcuts unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_STRING) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected string action");
        }
        const char* act = 0;
        dbus_message_iter_get_basic(&iter, &act);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t rev = 0;
        dbus_message_iter_get_basic(&iter, &rev);
        std::string action = act ? act : "";
        api::Status st = host->shortcuts()->resetBinding(action, static_cast<uint64_t>(rev));
        if (!st.ok()) return sendStatusError(c, msg, st);
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitShortcutsChanged(host->shortcuts()->revision());
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    // Panels
    DBusHandlerResult handlePanelsGetSnapshot(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->panels()) {
            return sendError(c, msg, api::Error::Unavailable, "panels unavailable");
        }
        api::PanelsSnapshot snap = host->panels()->snapshot();
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        DBusMessageIter iter, arr;
        dbus_message_iter_init_append(reply, &iter);
        dbus_message_iter_open_container(&iter, DBUS_TYPE_ARRAY, "{sv}", &arr);
        dbus_uint64_t rev = snap.revision;
        appendDictVariantUint64(&arr, "revision", rev);
        appendDictVariantBool(&arr, "startOpen", snap.startOpen ? TRUE : FALSE);
        if (!snap.startOutput.key.empty()) {
            const char* v = snap.startOutput.key.c_str();
            appendDictVariantString(&arr, "startOutput", v);
        }
        appendDictVariantUint32(&arr, "panelCount", static_cast<dbus_uint32_t>(snap.panels.size()));
        appendDictVariantUint32(&arr, "taskCount", static_cast<dbus_uint32_t>(snap.tasks.size()));
        appendDictVariantUint32(&arr, "pinnedCount", static_cast<dbus_uint32_t>(snap.pinnedApps.size()));
        if (!snap.panels.empty()) {
            // encode edges as string array
            DBusMessageIter entry, var, earr;
            const char* k = "edges";
            dbus_message_iter_open_container(&arr, DBUS_TYPE_DICT_ENTRY, NULL, &entry);
            dbus_message_iter_append_basic(&entry, DBUS_TYPE_STRING, &k);
            dbus_message_iter_open_container(&entry, DBUS_TYPE_VARIANT, "as", &var);
            dbus_message_iter_open_container(&var, DBUS_TYPE_ARRAY, DBUS_TYPE_STRING_AS_STRING, &earr);
            for (std::size_t i = 0; i < snap.panels.size(); ++i) {
                const char* e = "";
                switch (snap.panels[i].edge) {
                    case api::PanelEdge::Bottom: e = "bottom"; break;
                    case api::PanelEdge::Top: e = "top"; break;
                    case api::PanelEdge::Left: e = "left"; break;
                    case api::PanelEdge::Right: e = "right"; break;
                }
                dbus_message_iter_append_basic(&earr, DBUS_TYPE_STRING, &e);
            }
            dbus_message_iter_close_container(&var, &earr);
            dbus_message_iter_close_container(&entry, &var);
            dbus_message_iter_close_container(&arr, &entry);
        }
        if (!snap.pinnedApps.empty()) {
            DBusMessageIter entry, var, parr;
            const char* k = "pinnedApps";
            dbus_message_iter_open_container(&arr, DBUS_TYPE_DICT_ENTRY, NULL, &entry);
            dbus_message_iter_append_basic(&entry, DBUS_TYPE_STRING, &k);
            dbus_message_iter_open_container(&entry, DBUS_TYPE_VARIANT, "as", &var);
            dbus_message_iter_open_container(&var, DBUS_TYPE_ARRAY, DBUS_TYPE_STRING_AS_STRING, &parr);
            for (std::size_t i = 0; i < snap.pinnedApps.size(); ++i) {
                const char* s = snap.pinnedApps[i].value.c_str();
                dbus_message_iter_append_basic(&parr, DBUS_TYPE_STRING, &s);
            }
            dbus_message_iter_close_container(&var, &parr);
            dbus_message_iter_close_container(&entry, &var);
            dbus_message_iter_close_container(&arr, &entry);
        }
        dbus_message_iter_close_container(&iter, &arr);
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handlePanelsSetEdge(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->panels()) {
            return sendError(c, msg, api::Error::Unavailable, "panels unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_STRING) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected string outputKey");
        }
        const char* out = 0;
        dbus_message_iter_get_basic(&iter, &out);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_STRING) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected string edge");
        }
        const char* edgeStr = 0;
        dbus_message_iter_get_basic(&iter, &edgeStr);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t rev = 0;
        dbus_message_iter_get_basic(&iter, &rev);
        bool ok = false;
        api::PanelEdge e = panelEdgeFromString(edgeStr ? edgeStr : "", &ok);
        if (!ok) return sendError(c, msg, api::Error::InvalidArgument, "unknown panel edge");
        api::OutputId oid(out ? out : "");
        api::Status st = host->panels()->setEdge(oid, e, static_cast<uint64_t>(rev));
        if (!st.ok()) return sendStatusError(c, msg, st);
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitPanelsChanged(host->panels()->revision());
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handlePanelsSetSize(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->panels()) {
            return sendError(c, msg, api::Error::Unavailable, "panels unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_STRING) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected string outputKey");
        }
        const char* out = 0;
        dbus_message_iter_get_basic(&iter, &out);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT32) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint32 size");
        }
        dbus_uint32_t sz = 0;
        dbus_message_iter_get_basic(&iter, &sz);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t rev = 0;
        dbus_message_iter_get_basic(&iter, &rev);
        api::OutputId oid(out ? out : "");
        api::Status st = host->panels()->setSize(oid, static_cast<int>(sz), static_cast<uint64_t>(rev));
        if (!st.ok()) return sendStatusError(c, msg, st);
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitPanelsChanged(host->panels()->revision());
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handlePanelsPin(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->panels()) {
            return sendError(c, msg, api::Error::Unavailable, "panels unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_STRING) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected string appId");
        }
        const char* app = 0;
        dbus_message_iter_get_basic(&iter, &app);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t rev = 0;
        dbus_message_iter_get_basic(&iter, &rev);
        api::DesktopAppId aid(app ? app : "");
        api::Status st = host->panels()->pin(aid, static_cast<uint64_t>(rev));
        if (!st.ok()) return sendStatusError(c, msg, st);
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitPanelsChanged(host->panels()->revision());
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handlePanelsUnpin(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->panels()) {
            return sendError(c, msg, api::Error::Unavailable, "panels unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_STRING) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected string appId");
        }
        const char* app = 0;
        dbus_message_iter_get_basic(&iter, &app);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t rev = 0;
        dbus_message_iter_get_basic(&iter, &rev);
        api::DesktopAppId aid(app ? app : "");
        api::Status st = host->panels()->unpin(aid, static_cast<uint64_t>(rev));
        if (!st.ok()) return sendStatusError(c, msg, st);
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitPanelsChanged(host->panels()->revision());
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handlePanelsReorder(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->panels()) {
            return sendError(c, msg, api::Error::Unavailable, "panels unavailable");
        }
        DBusMessageIter iter;
        if (!dbus_message_iter_init(msg, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_STRING) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected string entryId");
        }
        const char* eid = 0;
        dbus_message_iter_get_basic(&iter, &eid);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT32) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint32 index");
        }
        dbus_uint32_t idx = 0;
        dbus_message_iter_get_basic(&iter, &idx);
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_UINT64) {
            return sendError(c, msg, api::Error::InvalidArgument, "expected uint64 expectedRevision");
        }
        dbus_uint64_t rev = 0;
        dbus_message_iter_get_basic(&iter, &rev);
        api::TaskEntryId tid(eid ? eid : "");
        api::Status st = host->panels()->reorder(tid, static_cast<int>(idx), static_cast<uint64_t>(rev));
        if (!st.ok()) return sendStatusError(c, msg, st);
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        emitPanelsChanged(host->panels()->revision());
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    // Session
    DBusHandlerResult handleSessionGetCapabilities(DBusConnection* c, DBusMessage* msg) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->session()) {
            return sendError(c, msg, api::Error::Unavailable, "session unavailable");
        }
        api::SessionCapabilities caps = host->session()->capabilities();
        // Return as array of strings per new XML (names of allowed actions)
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        DBusMessageIter iter, arr;
        dbus_message_iter_init_append(reply, &iter);
        dbus_message_iter_open_container(&iter, DBUS_TYPE_ARRAY, DBUS_TYPE_STRING_AS_STRING, &arr);
        if (caps.canLock) { const char* s="lock"; dbus_message_iter_append_basic(&arr, DBUS_TYPE_STRING, &s); }
        if (caps.canLogout) { const char* s="logout"; dbus_message_iter_append_basic(&arr, DBUS_TYPE_STRING, &s); }
        if (caps.canSuspend) { const char* s="suspend"; dbus_message_iter_append_basic(&arr, DBUS_TYPE_STRING, &s); }
        if (caps.canReboot) { const char* s="reboot"; dbus_message_iter_append_basic(&arr, DBUS_TYPE_STRING, &s); }
        if (caps.canShutdown) { const char* s="shutdown"; dbus_message_iter_append_basic(&arr, DBUS_TYPE_STRING, &s); }
        dbus_message_iter_close_container(&iter, &arr);
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    DBusHandlerResult handleSessionAction(DBusConnection* c, DBusMessage* msg, api::SessionAction act) {
        if (!host) return sendError(c, msg, api::Error::Unavailable, "host not attached");
        if (!host->isStarted() || !host->session()) {
            return sendError(c, msg, api::Error::Unavailable, "session unavailable");
        }
        api::Status st;
        switch (act) {
            case api::SessionAction::Lock: st = host->session()->lock(); break;
            case api::SessionAction::Logout: st = host->session()->logout(); break;
            case api::SessionAction::Suspend: st = host->session()->suspend(); break;
            case api::SessionAction::Reboot: st = host->session()->reboot(); break;
            case api::SessionAction::Shutdown: st = host->session()->shutdown(); break;
            default: st = api::Status::make(api::Error::InvalidArgument, "unknown action"); break;
        }
        if (!st.ok()) return sendStatusError(c, msg, st);
        DBusMessage* reply = dbus_message_new_method_return(msg);
        if (!reply) return DBUS_HANDLER_RESULT_NEED_MEMORY;
        dbus_connection_send(c, reply, NULL);
        dbus_message_unref(reply);
        return DBUS_HANDLER_RESULT_HANDLED;
    }

    void emitSignalOnIface(const char* iface, const char* name, dbus_uint64_t revision) {
        if (!conn) return;
        DBusMessage* sig = dbus_message_new_signal(kObjectPath, iface, name);
        if (!sig) return;
        dbus_message_append_args(sig, DBUS_TYPE_UINT64, &revision, DBUS_TYPE_INVALID);
        dbus_connection_send(conn, sig, NULL);
        dbus_message_unref(sig);
    }
    void emitSignalWithKeys(const char* iface, const char* name, uint64_t rev, const std::vector<std::string>& keys) {
        if (!conn) return;
        DBusMessage* sig = dbus_message_new_signal(kObjectPath, iface, name);
        if (!sig) return;
        DBusMessageIter iter, arr;
        dbus_uint64_t r = rev;
        dbus_message_iter_init_append(sig, &iter);
        dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &r);
        dbus_message_iter_open_container(&iter, DBUS_TYPE_ARRAY, DBUS_TYPE_STRING_AS_STRING, &arr);
        for (std::size_t i=0;i<keys.size();++i) { const char* s=keys[i].c_str(); dbus_message_iter_append_basic(&arr, DBUS_TYPE_STRING, &s); }
        dbus_message_iter_close_container(&iter, &arr);
        dbus_connection_send(conn, sig, NULL);
        dbus_message_unref(sig);
    }
    void emitDisplayTransactionChanged(uint64_t tx, const char* state, uint64_t deadlineMs) {
        if (!conn) return;
        DBusMessage* sig = dbus_message_new_signal(kObjectPath, kIfaceDisplays, "DisplayTransactionChanged");
        if (!sig) return;
        DBusMessageIter iter;
        dbus_message_iter_init_append(sig, &iter);
        dbus_uint64_t t = tx;
        const char* s = state ? state : "";
        dbus_uint64_t d = deadlineMs;
        dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &t);
        dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &s);
        dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &d);
        dbus_connection_send(conn, sig, NULL);
        dbus_message_unref(sig);
    }

    void emitSettingsChanged(uint64_t rev, const std::vector<std::string>& keys) { emitSignalWithKeys(kIfaceSettings, "SettingsChanged", rev, keys); }
    void emitSettingsChanged(uint64_t rev, const std::vector<api::SettingsChange>& changes) {
        std::vector<std::string> keys;
        keys.reserve(changes.size());
        for (std::size_t i=0;i<changes.size();++i) keys.push_back(changes[i].key);
        emitSignalWithKeys(kIfaceSettings, "SettingsChanged", rev, keys);
    }
    void emitWorkspacesChanged(uint64_t rev) { emitSignalOnIface(kIfaceWorkspaces, "WorkspacesChanged", rev); }
    void emitDisplaysChanged(uint64_t gen) { emitSignalOnIface(kIfaceDisplays, "TopologyChanged", gen); }
    void emitShortcutsChanged(uint64_t rev) { emitSignalOnIface(kIfaceShortcuts, "ShortcutsChanged", rev); }
    void emitPanelsChanged(uint64_t rev) { emitSignalOnIface(kIfacePanels, "PanelsChanged", rev); }

    explicit Impl(platform::PlatformHost* h)
        : host(h), running(false), conn(NULL), loop(NULL), generation(0) {
        std::memset(&vtable, 0, sizeof(vtable));
        vtable.message_function = messageFilter;
    }

#else
    uint64_t generation;
    explicit Impl(platform::PlatformHost* h) : host(h), running(false), generation(0) {}
#endif
};

ControlServer::ControlServer(platform::PlatformHost* host)
    : impl_(new Impl(host)) {}

ControlServer::~ControlServer() {
    stop();
    delete impl_;
    impl_ = NULL;
}

bool ControlServer::isRunning() const {
    return impl_ && impl_->running;
}

#ifdef FLAMEWM_HAS_DBUS

bool ControlServer::start() {
    if (!impl_ || impl_->running) return false;
    if (!impl_->host) return false;

    DBusError err;
    dbus_error_init(&err);
    DBusConnection* conn = dbus_bus_get(DBUS_BUS_SESSION, &err);
    if (dbus_error_is_set(&err)) {
        dbus_error_free(&err);
        if (conn) dbus_connection_unref(conn);
        return false;
    }
    if (!conn) return false;

    int ret = dbus_bus_request_name(conn, kBusName,
        DBUS_NAME_FLAG_DO_NOT_QUEUE, &err);
    if (dbus_error_is_set(&err)) {
        dbus_error_free(&err);
        dbus_connection_unref(conn);
        return false;
    }
    if (ret != DBUS_REQUEST_NAME_REPLY_PRIMARY_OWNER) {
        dbus_connection_unref(conn);
        return false;
    }

    impl_->loop = impl_->host->reactor() ? impl_->host->reactor()->port() : NULL;
    impl_->conn = conn;

    dbus_connection_set_watch_functions(conn,
        Impl::watchAdd, Impl::watchRemove, Impl::watchToggle,
        impl_, NULL);
    dbus_connection_set_timeout_functions(conn,
        Impl::timeoutAdd, Impl::timeoutRemove, Impl::timeoutToggle,
        impl_, NULL);
    dbus_connection_set_dispatch_status_function(conn,
        Impl::dispatchStatus, impl_, NULL);

    impl_->running = true;
    impl_->generation++;

    if (!dbus_connection_add_filter(conn, Impl::messageFilter, impl_, NULL)) {
        stop();
        return false;
    }

    impl_->vtable.message_function = Impl::messageFilter;

    return true;
}

void ControlServer::stop() {
    if (!impl_ || (!impl_->running && !impl_->conn)) return;
    Impl* self = impl_;
    self->generation++;
    if (self->conn) {
        dbus_connection_remove_filter(self->conn, Impl::messageFilter, self);
        DBusError err;
        dbus_error_init(&err);
        dbus_bus_release_name(self->conn, kBusName, &err);
        if (dbus_error_is_set(&err)) dbus_error_free(&err);
        for (std::map<DBusWatch*, Impl::WatchCtx>::iterator it = self->watches.begin(); it != self->watches.end(); ++it) {
            if (self->loop && it->second.handle.id >= 0) self->loop->removePoll(it->second.handle);
        }
        self->watches.clear();
        for (std::map<DBusTimeout*, Impl::TimeoutCtx>::iterator it = self->timeouts.begin(); it != self->timeouts.end(); ++it) {
            if (self->loop && it->second.handle.id >= 0) self->loop->removeTimer(it->second.handle);
        }
        self->timeouts.clear();
        dbus_connection_flush(self->conn);
        dbus_connection_unref(self->conn);
        self->conn = NULL;
    }
    self->running = false;
}

#else // !FLAMEWM_HAS_DBUS

bool ControlServer::start() {
    (void)impl_;
    return false;
}

void ControlServer::stop() {
    if (!impl_) return;
    impl_->running = false;
}

#endif

} // namespace control
} // namespace flamewm
