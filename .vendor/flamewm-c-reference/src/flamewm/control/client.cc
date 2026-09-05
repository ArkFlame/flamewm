#include "flamewm/control/client.h"

#include <cstring>

#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
#include <dbus/dbus.h>
#endif

namespace flamewm {
namespace control {

namespace {

#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
static const char* kBusName      = "com.arkflame.FlameWM1";
static const char* kObjectPath   = "/com/arkflame/FlameWM1";
static const char* kIfaceRoot    = "com.arkflame.FlameWM1";
static const char* kIfaceSettings   = "com.arkflame.FlameWM1.Settings";
static const char* kIfaceWorkspaces = "com.arkflame.FlameWM1.Workspaces";
static const char* kIfaceDisplays   = "com.arkflame.FlameWM1.Displays";
static const char* kIfaceShortcuts  = "com.arkflame.FlameWM1.Shortcuts";
static const char* kIfacePanels     = "com.arkflame.FlameWM1.Panels";
static const char* kIfaceSession    = "com.arkflame.FlameWM1.Session";
static const int kTimeoutMs = 5000;

api::Error mapDbusError(const char* name) {
    if (!name) return api::Error::InternalFailure;
    if (std::strstr(name, "Timeout")) return api::Error::Timeout;
    if (std::strstr(name, "NotFound") || std::strstr(name, "UnknownMethod")) return api::Error::NotFound;
    if (std::strstr(name, "InvalidArgs") || std::strstr(name, "InvalidArgument")) return api::Error::InvalidArgument;
    if (std::strstr(name, "StaleRevision")) return api::Error::StaleRevision;
    if (std::strstr(name, "Conflict")) return api::Error::Conflict;
    if (std::strstr(name, "Busy") || std::strstr(name, "Exists")) return api::Error::Busy;
    if (std::strstr(name, "Unsupported")) return api::Error::Unsupported;
    if (std::strstr(name, "AccessDenied") || std::strstr(name, "PermissionDenied")) return api::Error::PermissionDenied;
    if (std::strstr(name, "ServiceUnknown") || std::strstr(name, "NameHasNoOwner")) return api::Error::Unavailable;
    if (std::strstr(name, "Unavailable")) return api::Error::Unavailable;
    return api::Error::InternalFailure;
}

api::Status statusFromDbusError(DBusError* err) {
    if (!err || !dbus_error_is_set(err)) return api::Status::make(api::Error::InternalFailure, "unknown dbus error");
    api::Error code = mapDbusError(err->name);
    std::string msg = err->message ? err->message : err->name;
    return api::Status::make(code, msg);
}

static const char* panelEdgeToString(api::PanelEdge e) {
    switch (e) {
        case api::PanelEdge::Bottom: return "bottom";
        case api::PanelEdge::Top: return "top";
        case api::PanelEdge::Left: return "left";
        case api::PanelEdge::Right: return "right";
        default: return "bottom";
    }
}

// Unpack variant containing basic type T; returns true on success.
template <typename T> bool variantGetBasic(DBusMessageIter* varIter, int expectedType, T* out) {
    if (!varIter || !out) return false;
    if (dbus_message_iter_get_arg_type(varIter) != expectedType) return false;
    dbus_message_iter_get_basic(varIter, out);
    return true;
}

// Extract uint64 from a{sv} dict for key.
bool dictLookupUint64(DBusMessageIter* arrIter, const char* key, uint64_t* out) {
    if (!arrIter || !key || !out) return false;
    DBusMessageIter copy = *arrIter;
    while (dbus_message_iter_get_arg_type(&copy) == DBUS_TYPE_DICT_ENTRY) {
        DBusMessageIter entry; dbus_message_iter_recurse(&copy, &entry);
        const char* k = 0;
        if (dbus_message_iter_get_arg_type(&entry) == DBUS_TYPE_STRING) {
            dbus_message_iter_get_basic(&entry, &k);
            dbus_message_iter_next(&entry);
            if (k && std::strcmp(k, key) == 0 && dbus_message_iter_get_arg_type(&entry) == DBUS_TYPE_VARIANT) {
                DBusMessageIter var; dbus_message_iter_recurse(&entry, &var);
                int vt = dbus_message_iter_get_arg_type(&var);
                if (vt == DBUS_TYPE_UINT64) { dbus_uint64_t v=0; dbus_message_iter_get_basic(&var, &v); *out=v; return true; }
                if (vt == DBUS_TYPE_UINT32) { dbus_uint32_t v=0; dbus_message_iter_get_basic(&var, &v); *out=v; return true; }
                if (vt == DBUS_TYPE_INT32)  { dbus_int32_t v=0;  dbus_message_iter_get_basic(&var, &v); *out=static_cast<uint64_t>(v); return true; }
            }
        }
        dbus_message_iter_next(&copy);
    }
    return false;
}

bool dictLookupString(DBusMessageIter* arrIter, const char* key, std::string* out) {
    DBusMessageIter copy = *arrIter;
    while (dbus_message_iter_get_arg_type(&copy) == DBUS_TYPE_DICT_ENTRY) {
        DBusMessageIter entry; dbus_message_iter_recurse(&copy, &entry);
        const char* k = 0;
        if (dbus_message_iter_get_arg_type(&entry) == DBUS_TYPE_STRING) {
            dbus_message_iter_get_basic(&entry, &k);
            dbus_message_iter_next(&entry);
            if (k && std::strcmp(k, key)==0 && dbus_message_iter_get_arg_type(&entry)==DBUS_TYPE_VARIANT) {
                DBusMessageIter var; dbus_message_iter_recurse(&entry, &var);
                if (dbus_message_iter_get_arg_type(&var)==DBUS_TYPE_STRING) {
                    const char* v=0; dbus_message_iter_get_basic(&var, &v);
                    if (v) { *out=v; return true; }
                }
            }
        }
        dbus_message_iter_next(&copy);
    }
    return false;
}
#endif

} // namespace

struct ControlClient::Impl {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    DBusConnection* conn;
    Impl() : conn(0) {}
#else
    Impl() {}
#endif
};

ControlClient::ControlClient() : impl_(new Impl()) {}

ControlClient::~ControlClient() {
    disconnect();
    delete impl_;
}

bool ControlClient::connect() {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (impl_->conn) return true;
    DBusError error;
    dbus_error_init(&error);
    DBusConnection* c = dbus_bus_get(DBUS_BUS_SESSION, &error);
    if (dbus_error_is_set(&error)) {
        dbus_error_free(&error);
        return false;
    }
    if (!c) return false;
    dbus_connection_ref(c);
    impl_->conn = c;
    return true;
#else
    return false;
#endif
}

void ControlClient::disconnect() {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (impl_->conn) {
        dbus_connection_unref(impl_->conn);
        impl_->conn = 0;
    }
#endif
}

bool ControlClient::isConnected() const {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    return impl_->conn != 0;
#else
    return false;
#endif
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)

static api::Result<std::string> callStringMethod(ControlClient::Impl* impl,
                                                 const char* iface,
                                                 const char* method) {
    if (!impl || !impl->conn) {
        return api::Result<std::string>::Err(api::Error::Unavailable, "not connected");
    }
    DBusError error;
    dbus_error_init(&error);
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, iface, method);
    if (!msg) {
        return api::Result<std::string>::Err(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    }
    DBusMessage* reply = dbus_connection_send_with_reply_and_block(impl->conn, msg, kTimeoutMs, &error);
    dbus_message_unref(msg);
    if (dbus_error_is_set(&error)) {
        api::Status s = statusFromDbusError(&error);
        dbus_error_free(&error);
        if (reply) dbus_message_unref(reply);
        return api::Result<std::string>::Err(s);
    }
    if (!reply) {
        return api::Result<std::string>::Err(api::Error::Timeout, "no reply");
    }
    const char* str = 0;
    if (!dbus_message_get_args(reply, &error, DBUS_TYPE_STRING, &str, DBUS_TYPE_INVALID)) {
        api::Status s = statusFromDbusError(&error);
        if (dbus_error_is_set(&error)) dbus_error_free(&error);
        else s = api::Status::make(api::Error::InternalFailure, "invalid reply");
        dbus_message_unref(reply);
        return api::Result<std::string>::Err(s);
    }
    std::string out = str ? str : "";
    dbus_message_unref(reply);
    return api::Result<std::string>::Ok(out);
}

static api::Status callVoidMethod(ControlClient::Impl* impl,
                                   const char* method,
                                   DBusMessage* msg) {
    if (!impl || !impl->conn) {
        if (msg) dbus_message_unref(msg);
        return api::Status::make(api::Error::Unavailable, "not connected");
    }
    if (!msg) {
        return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    }
    DBusError error;
    dbus_error_init(&error);
    DBusMessage* reply = dbus_connection_send_with_reply_and_block(impl->conn, msg, kTimeoutMs, &error);
    dbus_message_unref(msg);
    if (dbus_error_is_set(&error)) {
        api::Status s = statusFromDbusError(&error);
        dbus_error_free(&error);
        if (reply) dbus_message_unref(reply);
        return s;
    }
    if (!reply) {
        return api::Status::make(api::Error::Timeout, "no reply");
    }
    dbus_message_unref(reply);
    return api::Status::Ok();
}

#endif // HAVE_FLAMEWM_DBUS

// ---------------------------------------------------------------------------
// Core / introspection
// ---------------------------------------------------------------------------

api::Result<std::string> ControlClient::ping() {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    return callStringMethod(impl_, kIfaceRoot, "Ping");
#else
    return api::Result<std::string>::Err(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Result<std::string> ControlClient::getVersion() {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    return callStringMethod(impl_, kIfaceRoot, "GetVersion");
#else
    return api::Result<std::string>::Err(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Result<api::Capabilities> ControlClient::getCapabilities() {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) {
        return api::Result<api::Capabilities>::Err(api::Error::Unavailable, "not connected");
    }
    DBusError error;
    dbus_error_init(&error);
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceRoot, "GetCapabilities");
    if (!msg) {
        return api::Result<api::Capabilities>::Err(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    }
    DBusMessage* reply = dbus_connection_send_with_reply_and_block(impl_->conn, msg, kTimeoutMs, &error);
    dbus_message_unref(msg);
    if (dbus_error_is_set(&error)) {
        api::Status s = statusFromDbusError(&error);
        dbus_error_free(&error);
        if (reply) dbus_message_unref(reply);
        return api::Result<api::Capabilities>::Err(s);
    }
    if (!reply) {
        return api::Result<api::Capabilities>::Err(api::Error::Timeout, "no reply");
    }
    DBusMessageIter iter, arr;
    if (!dbus_message_iter_init(reply, &iter) || dbus_message_iter_get_arg_type(&iter) != DBUS_TYPE_ARRAY) {
        dbus_message_unref(reply);
        return api::Result<api::Capabilities>::Err(api::Error::InternalFailure, "invalid reply");
    }
    dbus_message_iter_recurse(&iter, &arr);
    api::Capabilities caps;
    while (dbus_message_iter_get_arg_type(&arr) == DBUS_TYPE_STRING) {
        const char* s = 0;
        dbus_message_iter_get_basic(&arr, &s);
        if (s) caps.items.push_back(s);
        dbus_message_iter_next(&arr);
    }
    dbus_message_unref(reply);
    return api::Result<api::Capabilities>::Ok(caps);
#else
    return api::Result<api::Capabilities>::Err(api::Error::Unavailable, "dbus not available at build time");
#endif
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

api::Result<api::SettingsSnapshot> ControlClient::getSettingsSnapshot() {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) {
        return api::Result<api::SettingsSnapshot>::Err(api::Error::Unavailable, "not connected");
    }
    DBusError error;
    dbus_error_init(&error);
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceSettings, "GetSnapshot");
    if (!msg) {
        return api::Result<api::SettingsSnapshot>::Err(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    }
    DBusMessage* reply = dbus_connection_send_with_reply_and_block(impl_->conn, msg, kTimeoutMs, &error);
    dbus_message_unref(msg);
    if (dbus_error_is_set(&error)) {
        api::Status s = statusFromDbusError(&error);
        dbus_error_free(&error);
        if (reply) dbus_message_unref(reply);
        return api::Result<api::SettingsSnapshot>::Err(s);
    }
    if (!reply) {
        return api::Result<api::SettingsSnapshot>::Err(api::Error::Timeout, "no reply");
    }
    DBusMessageIter iter;
    if (!dbus_message_iter_init(reply, &iter)) {
        dbus_message_unref(reply);
        return api::Result<api::SettingsSnapshot>::Err(api::Error::InternalFailure, "empty reply");
    }
    // New XML: a{sv} dict. Legacy: t + a{ss}. Handle both.
    api::SettingsSnapshot snap;
    int t = dbus_message_iter_get_arg_type(&iter);
    if (t == DBUS_TYPE_ARRAY) {
        // a{sv}
        DBusMessageIter arr; dbus_message_iter_recurse(&iter, &arr);
        uint64_t rev=0;
        if (dictLookupUint64(&arr, "revision", &rev) || dictLookupUint64(&arr, "rev", &rev)) snap.revision=rev;
        // Extract all string variant entries as settings values
        DBusMessageIter copy = arr;
        while (dbus_message_iter_get_arg_type(&copy) == DBUS_TYPE_DICT_ENTRY) {
            DBusMessageIter entry; dbus_message_iter_recurse(&copy, &entry);
            const char* k=0;
            if (dbus_message_iter_get_arg_type(&entry)==DBUS_TYPE_STRING) {
                dbus_message_iter_get_basic(&entry, &k);
                dbus_message_iter_next(&entry);
                if (dbus_message_iter_get_arg_type(&entry)==DBUS_TYPE_VARIANT) {
                    DBusMessageIter var; dbus_message_iter_recurse(&entry, &var);
                    if (dbus_message_iter_get_arg_type(&var)==DBUS_TYPE_STRING) {
                        const char* v=0; dbus_message_iter_get_basic(&var, &v);
                        if (k && v && std::strcmp(k,"revision")!=0 && std::strcmp(k,"rev")!=0) snap.values[k]=v;
                    }
                }
            }
            dbus_message_iter_next(&copy);
        }
        dbus_message_unref(reply);
        return api::Result<api::SettingsSnapshot>::Ok(snap);
    } else if (t == DBUS_TYPE_UINT64) {
        uint64_t rev=0; dbus_message_iter_get_basic(&iter, &rev); snap.revision=rev;
        if (!dbus_message_iter_next(&iter) || dbus_message_iter_get_arg_type(&iter)!=DBUS_TYPE_ARRAY) {
            dbus_message_unref(reply);
            return api::Result<api::SettingsSnapshot>::Err(api::Error::InternalFailure, "invalid reply: expected array");
        }
        DBusMessageIter arr; dbus_message_iter_recurse(&iter, &arr);
        while (dbus_message_iter_get_arg_type(&arr)==DBUS_TYPE_DICT_ENTRY) {
            DBusMessageIter entry; dbus_message_iter_recurse(&arr, &entry);
            const char* k=0; const char* v=0;
            if (dbus_message_iter_get_arg_type(&entry)==DBUS_TYPE_STRING) {
                dbus_message_iter_get_basic(&entry, &k);
                dbus_message_iter_next(&entry);
                if (dbus_message_iter_get_arg_type(&entry)==DBUS_TYPE_STRING) dbus_message_iter_get_basic(&entry, &v);
            }
            if (k && v) snap.values[k]=v;
            dbus_message_iter_next(&arr);
        }
        dbus_message_unref(reply);
        return api::Result<api::SettingsSnapshot>::Ok(snap);
    } else {
        dbus_message_unref(reply);
        return api::Result<api::SettingsSnapshot>::Err(api::Error::InternalFailure, "invalid reply type");
    }
#else
    return api::Result<api::SettingsSnapshot>::Err(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::applySettings(uint64_t expected,
                                         const std::vector<api::SettingsChange>& changes) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceSettings, "Apply");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter, arr;
    dbus_message_iter_init_append(msg, &iter);
    dbus_uint64_t rev = expected;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    dbus_message_iter_open_container(&iter, DBUS_TYPE_ARRAY, "{sv}", &arr);
    for (std::size_t i = 0; i < changes.size(); ++i) {
        DBusMessageIter entry, var;
        dbus_message_iter_open_container(&arr, DBUS_TYPE_DICT_ENTRY, 0, &entry);
        const char* k = changes[i].key.c_str();
        dbus_message_iter_append_basic(&entry, DBUS_TYPE_STRING, &k);
        dbus_message_iter_open_container(&entry, DBUS_TYPE_VARIANT, "s", &var);
        const char* v = changes[i].value.c_str();
        dbus_message_iter_append_basic(&var, DBUS_TYPE_STRING, &v);
        dbus_message_iter_close_container(&entry, &var);
        dbus_message_iter_close_container(&arr, &entry);
    }
    dbus_message_iter_close_container(&iter, &arr);
    DBusError error; dbus_error_init(&error);
    DBusMessage* reply = dbus_connection_send_with_reply_and_block(impl_->conn, msg, kTimeoutMs, &error);
    dbus_message_unref(msg);
    if (dbus_error_is_set(&error)) { api::Status s=statusFromDbusError(&error); dbus_error_free(&error); if(reply) dbus_message_unref(reply); return s; }
    if (!reply) return api::Status::make(api::Error::Timeout, "no reply");
    dbus_message_unref(reply);
    return api::Status::Ok();
#else
    (void)expected; (void)changes;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::resetSettingsSection(uint64_t expected,
                                                const std::string& section) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceSettings, "ResetSection");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    dbus_uint64_t rev = expected;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    const char* s = section.c_str();
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &s);
    return callVoidMethod(impl_, "ResetSection", msg);
#else
    (void)expected; (void)section;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

// ---------------------------------------------------------------------------
// Workspaces
// ---------------------------------------------------------------------------

api::Result<api::WorkspaceSnapshot> ControlClient::getWorkspaceSnapshot() {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) {
        return api::Result<api::WorkspaceSnapshot>::Err(api::Error::Unavailable, "not connected");
    }
    DBusError error;
    dbus_error_init(&error);
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceWorkspaces, "GetSnapshot");
    if (!msg) {
        return api::Result<api::WorkspaceSnapshot>::Err(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    }
    DBusMessage* reply = dbus_connection_send_with_reply_and_block(impl_->conn, msg, kTimeoutMs, &error);
    dbus_message_unref(msg);
    if (dbus_error_is_set(&error)) {
        api::Status s = statusFromDbusError(&error);
        dbus_error_free(&error);
        if (reply) dbus_message_unref(reply);
        return api::Result<api::WorkspaceSnapshot>::Err(s);
    }
    if (!reply) {
        return api::Result<api::WorkspaceSnapshot>::Err(api::Error::Timeout, "no reply");
    }
    DBusMessageIter iter;
    if (!dbus_message_iter_init(reply, &iter)) {
        dbus_message_unref(reply);
        return api::Result<api::WorkspaceSnapshot>::Err(api::Error::InternalFailure, "empty reply");
    }
    api::WorkspaceSnapshot snap;
    int t = dbus_message_iter_get_arg_type(&iter);
    if (t == DBUS_TYPE_UINT64) {
        // Legacy triple: rev, count, active, last  (or rev count active last ...)
        dbus_uint64_t rev=0; dbus_message_iter_get_basic(&iter, &rev); snap.revision=rev;
        if (dbus_message_iter_next(&iter) && dbus_message_iter_get_arg_type(&iter)==DBUS_TYPE_INT32) {
            dbus_int32_t c=0; dbus_message_iter_get_basic(&iter,&c); snap.count=c;
            if (dbus_message_iter_next(&iter) && dbus_message_iter_get_arg_type(&iter)==DBUS_TYPE_INT32) {
                dbus_int32_t a=0; dbus_message_iter_get_basic(&iter,&a); snap.activeIndex=a;
                if (dbus_message_iter_next(&iter) && dbus_message_iter_get_arg_type(&iter)==DBUS_TYPE_INT32) {
                    dbus_int32_t l=0; dbus_message_iter_get_basic(&iter,&l); snap.lastIndex=l;
                }
            }
        }
        // populate workspaces vector to satisfy isValid if count known
        if (snap.count > 0) {
            snap.workspaces.reserve(static_cast<std::size_t>(snap.count));
            for (int i=0;i<snap.count;++i) snap.workspaces.push_back(api::WorkspaceRef(i, snap.revision));
        }
        dbus_message_unref(reply);
        if (snap.count < 1) {
            return api::Result<api::WorkspaceSnapshot>::Err(api::Error::Unavailable, "workspace service unavailable");
        }
        return api::Result<api::WorkspaceSnapshot>::Ok(snap);
    } else if (t == DBUS_TYPE_ARRAY) {
        // a{sv}
        DBusMessageIter arr; dbus_message_iter_recurse(&iter, &arr);
        uint64_t rev=0, cnt=0;
        int64_t active=-1, last=-1;
        bool hasRev = dictLookupUint64(&arr,"revision",&rev) || dictLookupUint64(&arr,"rev",&rev);
        bool hasCnt = dictLookupUint64(&arr,"count",&cnt);
        uint64_t av=0, lv=0;
        if (dictLookupUint64(&arr,"activeIndex",&av) || dictLookupUint64(&arr,"active",&av)) active=static_cast<int64_t>(av);
        if (dictLookupUint64(&arr,"lastIndex",&lv) || dictLookupUint64(&arr,"last",&lv)) last=static_cast<int64_t>(lv);
        if (hasRev) snap.revision=rev;
        if (hasCnt) snap.count=static_cast<int>(cnt);
        if (active>=0) snap.activeIndex=static_cast<int>(active);
        if (last>=0) snap.lastIndex=static_cast<int>(last); else if (last==-1) snap.lastIndex=-1;
        // Try to extract names array if present
        DBusMessageIter copy=arr;
        while (dbus_message_iter_get_arg_type(&copy)==DBUS_TYPE_DICT_ENTRY) {
            DBusMessageIter entry; dbus_message_iter_recurse(&copy,&entry);
            const char* k=0;
            if (dbus_message_iter_get_arg_type(&entry)==DBUS_TYPE_STRING) {
                dbus_message_iter_get_basic(&entry,&k);
                dbus_message_iter_next(&entry);
                if (k && std::strcmp(k,"names")==0 && dbus_message_iter_get_arg_type(&entry)==DBUS_TYPE_VARIANT) {
                    DBusMessageIter var; dbus_message_iter_recurse(&entry,&var);
                    if (dbus_message_iter_get_arg_type(&var)==DBUS_TYPE_ARRAY) {
                        DBusMessageIter narr; dbus_message_iter_recurse(&var,&narr);
                        while (dbus_message_iter_get_arg_type(&narr)==DBUS_TYPE_STRING) {
                            const char* s=0; dbus_message_iter_get_basic(&narr,&s);
                            if (s) snap.names.push_back(s);
                            dbus_message_iter_next(&narr);
                        }
                    }
                }
            }
            dbus_message_iter_next(&copy);
        }
        if (snap.count>0) {
            snap.workspaces.reserve(static_cast<std::size_t>(snap.count));
            for (int i=0;i<snap.count;++i) snap.workspaces.push_back(api::WorkspaceRef(i,snap.revision));
        }
        dbus_message_unref(reply);
        if (snap.count<1) {
            return api::Result<api::WorkspaceSnapshot>::Err(api::Error::Unavailable, "workspace service unavailable");
        }
        return api::Result<api::WorkspaceSnapshot>::Ok(snap);
    } else {
        dbus_message_unref(reply);
        return api::Result<api::WorkspaceSnapshot>::Err(api::Error::InternalFailure, "invalid reply type");
    }
#else
    return api::Result<api::WorkspaceSnapshot>::Err(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::applyWorkspaceTransform(const api::WorkspaceTransform& t) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    const char* method = 0;
    switch (t.type) {
        case api::WorkspaceTransform::Type::Activate:   method = "Activate"; break;
        case api::WorkspaceTransform::Type::InsertAfter: method = "InsertAfter"; break;
        case api::WorkspaceTransform::Type::Remove:     method = "Remove"; break;
        default: return api::Status::make(api::Error::InvalidArgument, "unknown workspace transform");
    }
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceWorkspaces, method);
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    dbus_uint32_t idx = static_cast<dbus_uint32_t>(t.index < 0 ? 0 : t.index);
    dbus_uint64_t rev = t.expectedRevision;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT32, &idx);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    return callVoidMethod(impl_, method, msg);
#else
    (void)t;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::activateWorkspace(uint32_t index, uint64_t expectedRevision) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceWorkspaces, "Activate");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    dbus_uint32_t idx = index;
    dbus_uint64_t rev = expectedRevision;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT32, &idx);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    return callVoidMethod(impl_, "Activate", msg);
#else
    (void)index; (void)expectedRevision;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::insertWorkspaceAfter(uint32_t index, uint64_t expectedRevision) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceWorkspaces, "InsertAfter");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    dbus_uint32_t idx = index;
    dbus_uint64_t rev = expectedRevision;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT32, &idx);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    return callVoidMethod(impl_, "InsertAfter", msg);
#else
    (void)index; (void)expectedRevision;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::removeWorkspace(uint32_t index, uint64_t expectedRevision) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceWorkspaces, "Remove");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    dbus_uint32_t idx = index;
    dbus_uint64_t rev = expectedRevision;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT32, &idx);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    return callVoidMethod(impl_, "Remove", msg);
#else
    (void)index; (void)expectedRevision;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

// ---------------------------------------------------------------------------
// Displays
// ---------------------------------------------------------------------------

api::Result<api::DisplaySnapshot> ControlClient::getDisplaySnapshot() {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) {
        return api::Result<api::DisplaySnapshot>::Err(api::Error::Unavailable, "not connected");
    }
    DBusError error;
    dbus_error_init(&error);
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceDisplays, "GetSnapshot");
    if (!msg) {
        return api::Result<api::DisplaySnapshot>::Err(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    }
    DBusMessage* reply = dbus_connection_send_with_reply_and_block(impl_->conn, msg, kTimeoutMs, &error);
    dbus_message_unref(msg);
    if (dbus_error_is_set(&error)) {
        api::Status s = statusFromDbusError(&error);
        dbus_error_free(&error);
        if (reply) dbus_message_unref(reply);
        return api::Result<api::DisplaySnapshot>::Err(s);
    }
    if (!reply) {
        return api::Result<api::DisplaySnapshot>::Err(api::Error::Timeout, "no reply");
    }
    DBusMessageIter iter;
    if (!dbus_message_iter_init(reply, &iter)) {
        dbus_message_unref(reply);
        return api::Result<api::DisplaySnapshot>::Err(api::Error::InternalFailure, "empty reply");
    }
    api::DisplaySnapshot snap;
    int t = dbus_message_iter_get_arg_type(&iter);
    if (t == DBUS_TYPE_UINT64) {
        dbus_uint64_t gen=0; dbus_message_iter_get_basic(&iter, &gen); snap.generation = gen;
        dbus_message_unref(reply);
        return api::Result<api::DisplaySnapshot>::Ok(snap);
    } else if (t == DBUS_TYPE_ARRAY) {
        DBusMessageIter arr; dbus_message_iter_recurse(&iter, &arr);
        uint64_t gen=0;
        if (dictLookupUint64(&arr, "generation", &gen) || dictLookupUint64(&arr, "gen", &gen)) snap.generation = gen;
        dbus_message_unref(reply);
        return api::Result<api::DisplaySnapshot>::Ok(snap);
    } else {
        dbus_message_unref(reply);
        return api::Result<api::DisplaySnapshot>::Err(api::Error::Unavailable, "display service unavailable");
    }
#else
    return api::Result<api::DisplaySnapshot>::Err(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::beginDisplayModeChange(const api::OutputId& output,
                                                  const api::ModeId& mode,
                                                  uint64_t generation,
                                                  api::TransactionId* outTx) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceDisplays, "BeginModeChange");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    const char* o = output.key.c_str();
    dbus_uint64_t m = mode.value;
    dbus_uint64_t g = generation;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &o);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &m);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &g);
    DBusError error; dbus_error_init(&error);
    DBusMessage* reply = dbus_connection_send_with_reply_and_block(impl_->conn, msg, kTimeoutMs, &error);
    dbus_message_unref(msg);
    if (dbus_error_is_set(&error)) { api::Status s=statusFromDbusError(&error); dbus_error_free(&error); if(reply) dbus_message_unref(reply); return s; }
    if (!reply) return api::Status::make(api::Error::Timeout, "no reply");
    DBusMessageIter ri;
    dbus_message_iter_init(reply, &ri);
    if (dbus_message_iter_get_arg_type(&ri)==DBUS_TYPE_UINT64 && outTx) {
        dbus_uint64_t v=0; dbus_message_iter_get_basic(&ri,&v); outTx->value=v;
    }
    dbus_message_unref(reply);
    return api::Status::Ok();
#else
    (void)output; (void)mode; (void)generation; (void)outTx;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::keepDisplayMode(const api::TransactionId& tx) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceDisplays, "Keep");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    dbus_uint64_t v = tx.value;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &v);
    return callVoidMethod(impl_, "Keep", msg);
#else
    (void)tx;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::revertDisplayMode(const api::TransactionId& tx) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceDisplays, "Revert");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    dbus_uint64_t v = tx.value;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &v);
    return callVoidMethod(impl_, "Revert", msg);
#else
    (void)tx;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::setShellScale(const api::OutputId& output,
                                         uint32_t percent,
                                         uint64_t expectedRevision) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceDisplays, "SetShellScale");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    const char* o = output.key.c_str();
    dbus_uint32_t p = percent;
    dbus_uint64_t rev = expectedRevision;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &o);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT32, &p);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    return callVoidMethod(impl_, "SetShellScale", msg);
#else
    (void)output; (void)percent; (void)expectedRevision;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::applyDisplayMode(const api::OutputId& output,
                                            const api::ModeId& mode,
                                            api::TransactionId* outTx) {
    // Legacy alias: map to BeginModeChange with generation 0.
    return beginDisplayModeChange(output, mode, 0, outTx);
}

api::Status ControlClient::restoreDisplayMode(const api::TransactionId& tx) {
    // Legacy alias: map to Revert (distinct from Keep).
    return revertDisplayMode(tx);
}

// ---------------------------------------------------------------------------
// Shortcuts
// ---------------------------------------------------------------------------

api::Result<api::ShortcutSnapshot> ControlClient::getShortcutSnapshot() {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) {
        return api::Result<api::ShortcutSnapshot>::Err(api::Error::Unavailable, "not connected");
    }
    DBusError error;
    dbus_error_init(&error);
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceShortcuts, "GetSnapshot");
    if (!msg) {
        return api::Result<api::ShortcutSnapshot>::Err(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    }
    DBusMessage* reply = dbus_connection_send_with_reply_and_block(impl_->conn, msg, kTimeoutMs, &error);
    dbus_message_unref(msg);
    if (dbus_error_is_set(&error)) {
        api::Status s = statusFromDbusError(&error);
        dbus_error_free(&error);
        if (reply) dbus_message_unref(reply);
        return api::Result<api::ShortcutSnapshot>::Err(s);
    }
    if (!reply) {
        return api::Result<api::ShortcutSnapshot>::Err(api::Error::Timeout, "no reply");
    }
    DBusMessageIter iter;
    if (!dbus_message_iter_init(reply, &iter)) {
        dbus_message_unref(reply);
        return api::Result<api::ShortcutSnapshot>::Err(api::Error::InternalFailure, "empty reply");
    }
    api::ShortcutSnapshot snap;
    int t = dbus_message_iter_get_arg_type(&iter);
    if (t == DBUS_TYPE_ARRAY) {
        DBusMessageIter arr; dbus_message_iter_recurse(&iter, &arr);
        uint64_t rev=0;
        bool hasRev = dictLookupUint64(&arr, "revision", &rev) || dictLookupUint64(&arr, "rev", &rev);
        if (hasRev) snap.revision = rev;
        // Extract bindings: any string variant entries besides revision
        DBusMessageIter copy=arr;
        while (dbus_message_iter_get_arg_type(&copy)==DBUS_TYPE_DICT_ENTRY) {
            DBusMessageIter entry; dbus_message_iter_recurse(&copy,&entry);
            const char* k=0;
            if (dbus_message_iter_get_arg_type(&entry)==DBUS_TYPE_STRING) {
                dbus_message_iter_get_basic(&entry,&k);
                dbus_message_iter_next(&entry);
                if (k && dbus_message_iter_get_arg_type(&entry)==DBUS_TYPE_VARIANT) {
                    DBusMessageIter var; dbus_message_iter_recurse(&entry,&var);
                    if (dbus_message_iter_get_arg_type(&var)==DBUS_TYPE_STRING) {
                        const char* v=0; dbus_message_iter_get_basic(&var,&v);
                        if (k && v && std::strcmp(k,"revision")!=0 && std::strcmp(k,"rev")!=0) {
                            snap.bindings[k]=api::KeyBinding(v);
                        }
                    }
                }
            }
            dbus_message_iter_next(&copy);
        }
        dbus_message_unref(reply);
        return api::Result<api::ShortcutSnapshot>::Ok(snap);
    } else if (t == DBUS_TYPE_UINT64) {
        dbus_uint64_t rev=0; dbus_message_iter_get_basic(&iter,&rev); snap.revision=rev;
        if (dbus_message_iter_next(&iter) && dbus_message_iter_get_arg_type(&iter)==DBUS_TYPE_ARRAY) {
            DBusMessageIter arr; dbus_message_iter_recurse(&iter,&arr);
            while (dbus_message_iter_get_arg_type(&arr)==DBUS_TYPE_DICT_ENTRY) {
                DBusMessageIter e; dbus_message_iter_recurse(&arr,&e);
                const char* k=0; const char* v=0;
                if (dbus_message_iter_get_arg_type(&e)==DBUS_TYPE_STRING) {
                    dbus_message_iter_get_basic(&e,&k);
                    dbus_message_iter_next(&e);
                    if (dbus_message_iter_get_arg_type(&e)==DBUS_TYPE_STRING) dbus_message_iter_get_basic(&e,&v);
                    else if (dbus_message_iter_get_arg_type(&e)==DBUS_TYPE_VARIANT) {
                        DBusMessageIter var; dbus_message_iter_recurse(&e,&var);
                        if (dbus_message_iter_get_arg_type(&var)==DBUS_TYPE_STRING) dbus_message_iter_get_basic(&var,&v);
                    }
                }
                if (k && v) snap.bindings[k]=api::KeyBinding(v);
                dbus_message_iter_next(&arr);
            }
        }
        dbus_message_unref(reply);
        return api::Result<api::ShortcutSnapshot>::Ok(snap);
    } else {
        dbus_message_unref(reply);
        return api::Result<api::ShortcutSnapshot>::Err(api::Error::InternalFailure, "invalid reply type");
    }
#else
    return api::Result<api::ShortcutSnapshot>::Err(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::setBinding(const std::string& action,
                                      const std::string& binding,
                                      uint64_t expectedRevision) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceShortcuts, "SetBinding");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    const char* a = action.c_str();
    const char* b = binding.c_str();
    dbus_uint64_t rev = expectedRevision;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &a);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &b);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    return callVoidMethod(impl_, "SetBinding", msg);
#else
    (void)action; (void)binding; (void)expectedRevision;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::clearBinding(const std::string& action,
                                        uint64_t expectedRevision) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceShortcuts, "ClearBinding");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    const char* a = action.c_str();
    dbus_uint64_t rev = expectedRevision;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &a);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    return callVoidMethod(impl_, "ClearBinding", msg);
#else
    (void)action; (void)expectedRevision;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::resetBinding(const std::string& action,
                                        uint64_t expectedRevision) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceShortcuts, "ResetBinding");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    const char* a = action.c_str();
    dbus_uint64_t rev = expectedRevision;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &a);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    return callVoidMethod(impl_, "ResetBinding", msg);
#else
    (void)action; (void)expectedRevision;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::applyShortcuts(uint64_t expected,
                                          const std::map<std::string, api::KeyBinding>& bindings) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    // Legacy bulk apply: iterate per-binding SetBinding. If service supports bulk, this still works atomically via per-call.
    // To preserve revision semantics, call SetBinding for each entry; on first failure return error.
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    // If empty, nothing to do.
    if (bindings.empty()) return api::Status::Ok();
    // For compat, if bindings size ==1 we just call SetBinding directly.
    // Otherwise loop; note revision will advance after first call so subsequent would get StaleRevision.
    // So for multi-bindings, fall back to sequential with stale handling: after first success caller should refresh.
    // We attempt to apply via per-binding calls; report first error.
    uint64_t rev = expected;
    for (std::map<std::string, api::KeyBinding>::const_iterator it = bindings.begin(); it != bindings.end(); ++it) {
        api::Status s = setBinding(it->first, it->second.key, rev);
        if (!s.ok()) return s;
        // Do not bump rev; server will reject stale on second iteration if it advanced revision.
        // Break after first to avoid spurious stale: caller using bulk should migrate to setBinding loop with refresh.
        // To still support bulk in one call if server has legacy ApplyShortcuts endpoint, try that endpoint as fallback.
        // Actually try legacy endpoint first: ApplyShortcuts on Shortcuts iface is not in new XML but old server may have it.
        // Prefer bulk if available: send to Shortcuts.ApplyShortcuts if new SetBinding fails with NotFound?
        // Simpler: just apply first and return Ok if single; for multi warn via Unsupported if stale arises.
        if (bindings.size() > 1) {
            // After first successful SetBinding, remaining would be stale; return Busy to indicate need for re-read.
            if (++it != bindings.end()) {
                return api::Status::make(api::Error::Busy, "bulk apply requires per-binding refresh; use setBinding");
            }
            break;
        }
    }
    return api::Status::Ok();
#else
    (void)expected; (void)bindings;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

// ---------------------------------------------------------------------------
// Panels
// ---------------------------------------------------------------------------

api::Result<api::PanelsSnapshot> ControlClient::getPanelsSnapshot() {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) {
        return api::Result<api::PanelsSnapshot>::Err(api::Error::Unavailable, "not connected");
    }
    DBusError error;
    dbus_error_init(&error);
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfacePanels, "GetSnapshot");
    if (!msg) {
        return api::Result<api::PanelsSnapshot>::Err(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    }
    DBusMessage* reply = dbus_connection_send_with_reply_and_block(impl_->conn, msg, kTimeoutMs, &error);
    dbus_message_unref(msg);
    if (dbus_error_is_set(&error)) {
        api::Status s = statusFromDbusError(&error);
        dbus_error_free(&error);
        if (reply) dbus_message_unref(reply);
        return api::Result<api::PanelsSnapshot>::Err(s);
    }
    if (!reply) {
        return api::Result<api::PanelsSnapshot>::Err(api::Error::Timeout, "no reply");
    }
    DBusMessageIter iter;
    if (!dbus_message_iter_init(reply, &iter)) {
        dbus_message_unref(reply);
        return api::Result<api::PanelsSnapshot>::Err(api::Error::InternalFailure, "empty reply");
    }
    api::PanelsSnapshot snap;
    int t = dbus_message_iter_get_arg_type(&iter);
    if (t == DBUS_TYPE_UINT64) {
        dbus_uint64_t rev=0; dbus_message_iter_get_basic(&iter, &rev); snap.revision=rev;
        dbus_message_unref(reply);
        return api::Result<api::PanelsSnapshot>::Ok(snap);
    } else if (t == DBUS_TYPE_ARRAY) {
        DBusMessageIter arr; dbus_message_iter_recurse(&iter, &arr);
        uint64_t rev=0;
        if (dictLookupUint64(&arr,"revision",&rev) || dictLookupUint64(&arr,"rev",&rev)) snap.revision=rev;
        dbus_message_unref(reply);
        return api::Result<api::PanelsSnapshot>::Ok(snap);
    } else {
        dbus_message_unref(reply);
        return api::Result<api::PanelsSnapshot>::Err(api::Error::Unavailable, "panels service unavailable");
    }
#else
    return api::Result<api::PanelsSnapshot>::Err(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::setPanelEdge(const api::OutputId& output, api::PanelEdge edge, uint64_t expectedRevision) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfacePanels, "SetEdge");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    const char* o = output.key.c_str();
    const char* e = panelEdgeToString(edge);
    dbus_uint64_t rev = expectedRevision;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &o);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &e);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    return callVoidMethod(impl_, "SetEdge", msg);
#else
    (void)output; (void)edge; (void)expectedRevision;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::setPanelEdge(const api::OutputId& output, api::PanelEdge edge) {
    return setPanelEdge(output, edge, 0);
}

api::Status ControlClient::setPanelSize(const api::OutputId& output, uint32_t size, uint64_t expectedRevision) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfacePanels, "SetSize");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    const char* o = output.key.c_str();
    dbus_uint32_t sz = size;
    dbus_uint64_t rev = expectedRevision;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &o);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT32, &sz);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    return callVoidMethod(impl_, "SetSize", msg);
#else
    (void)output; (void)size; (void)expectedRevision;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::pin(const api::DesktopAppId& appId, uint64_t expectedRevision) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfacePanels, "Pin");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    const char* a = appId.value.c_str();
    dbus_uint64_t rev = expectedRevision;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &a);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    return callVoidMethod(impl_, "Pin", msg);
#else
    (void)appId; (void)expectedRevision;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::unpin(const api::DesktopAppId& appId, uint64_t expectedRevision) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfacePanels, "Unpin");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    const char* a = appId.value.c_str();
    dbus_uint64_t rev = expectedRevision;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &a);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    return callVoidMethod(impl_, "Unpin", msg);
#else
    (void)appId; (void)expectedRevision;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::reorder(const api::TaskEntryId& entryId, uint32_t index, uint64_t expectedRevision) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfacePanels, "Reorder");
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    DBusMessageIter iter;
    dbus_message_iter_init_append(msg, &iter);
    const char* e = entryId.value.c_str();
    dbus_uint32_t idx = index;
    dbus_uint64_t rev = expectedRevision;
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_STRING, &e);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT32, &idx);
    dbus_message_iter_append_basic(&iter, DBUS_TYPE_UINT64, &rev);
    return callVoidMethod(impl_, "Reorder", msg);
#else
    (void)entryId; (void)index; (void)expectedRevision;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

api::Result<api::SessionCapabilities> ControlClient::getSessionCapabilities() {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) {
        return api::Result<api::SessionCapabilities>::Err(api::Error::Unavailable, "not connected");
    }
    DBusError error;
    dbus_error_init(&error);
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceSession, "GetCapabilities");
    if (!msg) {
        return api::Result<api::SessionCapabilities>::Err(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    }
    DBusMessage* reply = dbus_connection_send_with_reply_and_block(impl_->conn, msg, kTimeoutMs, &error);
    dbus_message_unref(msg);
    if (dbus_error_is_set(&error)) {
        api::Status s = statusFromDbusError(&error);
        dbus_error_free(&error);
        if (reply) dbus_message_unref(reply);
        return api::Result<api::SessionCapabilities>::Err(s);
    }
    if (!reply) {
        return api::Result<api::SessionCapabilities>::Err(api::Error::Timeout, "no reply");
    }
    DBusMessageIter iter;
    if (!dbus_message_iter_init(reply, &iter)) {
        dbus_message_unref(reply);
        return api::Result<api::SessionCapabilities>::Err(api::Error::InternalFailure, "empty reply");
    }
    api::SessionCapabilities caps;
    caps.canLock = caps.canLogout = caps.canSuspend = caps.canReboot = caps.canShutdown = false;
    if (dbus_message_iter_get_arg_type(&iter) == DBUS_TYPE_ARRAY) {
        DBusMessageIter arr; dbus_message_iter_recurse(&iter, &arr);
        while (dbus_message_iter_get_arg_type(&arr)==DBUS_TYPE_STRING) {
            const char* s=0; dbus_message_iter_get_basic(&arr,&s);
            if (s) {
                if (std::strcmp(s,"lock")==0) caps.canLock=true;
                else if (std::strcmp(s,"logout")==0) caps.canLogout=true;
                else if (std::strcmp(s,"suspend")==0) caps.canSuspend=true;
                else if (std::strcmp(s,"reboot")==0) caps.canReboot=true;
                else if (std::strcmp(s,"shutdown")==0) caps.canShutdown=true;
                // also handle legacy booleans-as-strings? ignore
            }
            dbus_message_iter_next(&arr);
        }
        dbus_message_unref(reply);
        return api::Result<api::SessionCapabilities>::Ok(caps);
    } else if (dbus_message_iter_get_arg_type(&iter)==DBUS_TYPE_BOOLEAN) {
        // Legacy 5 booleans fallback
        for (int i=0;i<5;++i) {
            if (dbus_message_iter_get_arg_type(&iter)!=DBUS_TYPE_BOOLEAN) break;
            dbus_bool_t v=0; dbus_message_iter_get_basic(&iter,&v);
            bool b=v?true:false;
            if (i==0) caps.canLock=b; else if(i==1) caps.canLogout=b; else if(i==2) caps.canSuspend=b; else if(i==3) caps.canReboot=b; else caps.canShutdown=b;
            if (!dbus_message_iter_next(&iter)) break;
        }
        dbus_message_unref(reply);
        return api::Result<api::SessionCapabilities>::Ok(caps);
    } else {
        dbus_message_unref(reply);
        return api::Result<api::SessionCapabilities>::Err(api::Error::InternalFailure, "invalid reply type");
    }
#else
    return api::Result<api::SessionCapabilities>::Err(api::Error::Unavailable, "dbus not available at build time");
#endif
}

api::Status ControlClient::requestSessionAction(api::SessionAction action) {
#if defined(HAVE_FLAMEWM_DBUS) || defined(HAVE_DBUS)
    if (!impl_->conn) return api::Status::make(api::Error::Unavailable, "not connected");
    const char* method = 0;
    switch (action) {
        case api::SessionAction::Lock:     method = "Lock"; break;
        case api::SessionAction::Logout:   method = "Logout"; break;
        case api::SessionAction::Suspend:  method = "Suspend"; break;
        case api::SessionAction::Reboot:   method = "Reboot"; break;
        case api::SessionAction::Shutdown: method = "Shutdown"; break;
        default: return api::Status::make(api::Error::InvalidArgument, "unknown session action");
    }
    DBusMessage* msg = dbus_message_new_method_call(kBusName, kObjectPath, kIfaceSession, method);
    if (!msg) return api::Status::make(api::Error::InternalFailure, "dbus_message_new_method_call failed");
    return callVoidMethod(impl_, method, msg);
#else
    (void)action;
    return api::Status::make(api::Error::Unavailable, "dbus not available at build time");
#endif
}

} // namespace control
} // namespace flamewm
