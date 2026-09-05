#include <cstdlib>
#include <iostream>
#include <map>
#include <sstream>
#include <string>
#include <vector>

#include "flamewm/control/client.h"
#include "flamewm/api/capabilities.h"
#include "flamewm/api/errors.h"

namespace {

void printHelp(const char* prog) {
    std::cout
        << "Usage: " << prog << " [OPTIONS] <command> [args]\n"
        << "\n"
        << "Options:\n"
        << "  -h, --help     Show this help\n"
        << "\n"
        << "Commands:\n"
        << "  Root:\n"
        << "    status                              Check daemon liveness (ping)\n"
        << "    ping                                Ping daemon\n"
        << "    get-version                         Print daemon version\n"
        << "    get-capabilities                    List capabilities\n"
        << "  Settings:\n"
        << "    settings get                        Print settings snapshot\n"
        << "    settings apply <rev> <k=v>...       Apply settings changes\n"
        << "    settings reset <rev> <section>      Reset settings section\n"
        << "  Workspaces:\n"
        << "    workspaces get                      Print workspace snapshot\n"
        << "    workspaces activate <idx> <rev>     Activate workspace\n"
        << "    workspaces insert <idx> <rev>       Insert workspace after idx\n"
        << "    workspaces remove <idx> <rev>       Remove workspace at idx\n"
        << "    workspaces transform <type> <idx> <rev>  Type: activate|insert|remove (compat)\n"
        << "  Displays:\n"
        << "    displays get                        Print display snapshot\n"
        << "    displays begin <output> <mode> <gen>  Begin mode change (prints tx)\n"
        << "    displays keep <tx>                  Keep pending mode change\n"
        << "    displays revert <tx>                Revert pending mode change\n"
        << "    displays set-scale <output> <percent> <rev>  Set shell scale\n"
        << "    displays set-mode <output> <mode>   Apply display mode (legacy alias)\n"
        << "    displays restore <tx>               Restore display mode (legacy alias for revert)\n"
        << "  Shortcuts:\n"
        << "    shortcuts get                       Print shortcut snapshot\n"
        << "    shortcuts set <action> <binding> <rev>    Set binding\n"
        << "    shortcuts clear <action> <rev>      Clear binding\n"
        << "    shortcuts reset <action> <rev>      Reset binding to default\n"
        << "    shortcuts apply <rev> <a=b>...      Apply shortcuts (legacy bulk)\n"
        << "  Panels:\n"
        << "    panels get                          Print panels snapshot\n"
        << "    panels set-edge <output> <edge> [rev]   Set panel edge (bottom|top|left|right)\n"
        << "    panels set-size <output> <size> <rev>   Set panel size\n"
        << "    panels pin <appId> <rev>            Pin app\n"
        << "    panels unpin <appId> <rev>          Unpin app\n"
        << "    panels reorder <entryId> <index> <rev>  Reorder task entry\n"
        << "  Session:\n"
        << "    session caps                        Print session capabilities\n"
        << "    session get-caps                    Alias for caps\n"
        << "    session request <action>            Action: lock|logout|suspend|reboot|shutdown\n"
        << "    session <action>                    Shorthand for request\n"
        << "\n"
        << "Examples:\n"
        << "  " << prog << " ping\n"
        << "  " << prog << " settings get\n"
        << "  " << prog << " settings apply 1 theme=dark\n"
        << "  " << prog << " workspaces get\n"
        << "\n";
}

void printStatusError(const flamewm::api::Status& s) {
    std::cerr << "error: " << flamewm::api::errorName(s.code);
    if (!s.message.empty()) std::cerr << ": " << s.message;
    std::cerr << "\n";
}

bool parseU64(const std::string& str, uint64_t* out) {
    if (str.empty() || !out) return false;
    char* end = 0;
    unsigned long long v = std::strtoull(str.c_str(), &end, 10);
    if (!end || *end != '\0') return false;
    *out = static_cast<uint64_t>(v);
    return true;
}

bool parseU32(const std::string& str, uint32_t* out) {
    if (str.empty() || !out) return false;
    char* end = 0;
    unsigned long v = std::strtoul(str.c_str(), &end, 10);
    if (!end || *end != '\0') return false;
    *out = static_cast<uint32_t>(v);
    return true;
}

bool parseI32(const std::string& str, int* out) {
    if (str.empty() || !out) return false;
    char* end = 0;
    long v = std::strtol(str.c_str(), &end, 10);
    if (!end || *end != '\0') return false;
    *out = static_cast<int>(v);
    return true;
}

int cmdPing(flamewm::control::ControlClient& c) {
    flamewm::api::Result<std::string> r = c.ping();
    if (!r.ok()) { printStatusError(r.status()); return 1; }
    std::cout << r.value() << "\n";
    return 0;
}

int cmdGetVersion(flamewm::control::ControlClient& c) {
    flamewm::api::Result<std::string> r = c.getVersion();
    if (!r.ok()) { printStatusError(r.status()); return 1; }
    std::cout << r.value() << "\n";
    return 0;
}

int cmdGetCapabilities(flamewm::control::ControlClient& c) {
    flamewm::api::Result<flamewm::api::Capabilities> r = c.getCapabilities();
    if (!r.ok()) { printStatusError(r.status()); return 1; }
    for (std::size_t i = 0; i < r.value().items.size(); ++i) {
        std::cout << r.value().items[i] << "\n";
    }
    return 0;
}

int cmdSettingsGet(flamewm::control::ControlClient& c) {
    flamewm::api::Result<flamewm::api::SettingsSnapshot> r = c.getSettingsSnapshot();
    if (!r.ok()) { printStatusError(r.status()); return 1; }
    std::cout << "revision: " << r.value().revision << "\n";
    for (std::map<std::string,std::string>::const_iterator it = r.value().values.begin();
         it != r.value().values.end(); ++it) {
        std::cout << it->first << "=" << it->second << "\n";
    }
    return 0;
}

int cmdSettingsApply(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc < 1) {
        std::cerr << "usage: flamewmctl settings apply <rev> <k=v>...\n";
        return 2;
    }
    uint64_t rev = 0;
    if (!parseU64(argv[0], &rev)) {
        std::cerr << "invalid revision: " << argv[0] << "\n";
        return 2;
    }
    std::vector<flamewm::api::SettingsChange> changes;
    for (int i = 1; i < argc; ++i) {
        std::string kv = argv[i];
        std::size_t pos = kv.find('=');
        if (pos == std::string::npos) {
            std::cerr << "invalid change (expected k=v): " << kv << "\n";
            return 2;
        }
        changes.push_back(flamewm::api::SettingsChange(kv.substr(0,pos), kv.substr(pos+1)));
    }
    flamewm::api::Status s = c.applySettings(rev, changes);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n";
    return 0;
}

int cmdSettingsReset(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "usage: flamewmctl settings reset <rev> <section>\n";
        return 2;
    }
    uint64_t rev = 0;
    if (!parseU64(argv[0], &rev)) {
        std::cerr << "invalid revision: " << argv[0] << "\n";
        return 2;
    }
    flamewm::api::Status s = c.resetSettingsSection(rev, argv[1]);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n";
    return 0;
}

int cmdWorkspacesGet(flamewm::control::ControlClient& c) {
    flamewm::api::Result<flamewm::api::WorkspaceSnapshot> r = c.getWorkspaceSnapshot();
    if (!r.ok()) { printStatusError(r.status()); return 1; }
    const flamewm::api::WorkspaceSnapshot& s = r.value();
    std::cout << "revision: " << s.revision << " count: " << s.count
              << " active: " << s.activeIndex << " last: " << s.lastIndex << "\n";
    for (std::size_t i = 0; i < s.workspaces.size(); ++i) {
        std::cout << "  [" << i << "] idx=" << s.workspaces[i].index
                  << " rev=" << s.workspaces[i].revision;
        if (i < s.names.size()) std::cout << " name=" << s.names[i];
        std::cout << "\n";
    }
    return 0;
}

int cmdWorkspacesActivate(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "usage: flamewmctl workspaces activate <idx> <rev>\n";
        return 2;
    }
    int idx = 0; uint64_t rev = 0;
    if (!parseI32(argv[0], &idx) || !parseU64(argv[1], &rev)) {
        std::cerr << "invalid args\n"; return 2;
    }
    flamewm::api::Status s = c.activateWorkspace(static_cast<uint32_t>(idx < 0 ? 0 : idx), rev);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdWorkspacesInsert(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "usage: flamewmctl workspaces insert <idx> <rev>\n";
        return 2;
    }
    int idx = 0; uint64_t rev = 0;
    if (!parseI32(argv[0], &idx) || !parseU64(argv[1], &rev)) {
        std::cerr << "invalid args\n"; return 2;
    }
    flamewm::api::Status s = c.insertWorkspaceAfter(static_cast<uint32_t>(idx < 0 ? 0 : idx), rev);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdWorkspacesRemove(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "usage: flamewmctl workspaces remove <idx> <rev>\n";
        return 2;
    }
    int idx = 0; uint64_t rev = 0;
    if (!parseI32(argv[0], &idx) || !parseU64(argv[1], &rev)) {
        std::cerr << "invalid args\n"; return 2;
    }
    flamewm::api::Status s = c.removeWorkspace(static_cast<uint32_t>(idx < 0 ? 0 : idx), rev);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdWorkspacesTransform(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 3) {
        std::cerr << "usage: flamewmctl workspaces transform <type> <idx> <rev>\n";
        std::cerr << "  types: activate|insert|remove\n";
        return 2;
    }
    std::string type = argv[0];
    int idx = 0; uint64_t rev = 0;
    if (!parseI32(argv[1], &idx) || !parseU64(argv[2], &rev)) {
        std::cerr << "invalid args\n"; return 2;
    }
    flamewm::api::WorkspaceTransform t;
    if (type == "activate") t = flamewm::api::WorkspaceTransform::Activate(idx, rev);
    else if (type == "insert") t = flamewm::api::WorkspaceTransform::InsertAfter(idx, rev);
    else if (type == "remove") t = flamewm::api::WorkspaceTransform::Remove(idx, rev);
    else { std::cerr << "unknown transform type: " << type << "\n"; return 2; }
    flamewm::api::Status s = c.applyWorkspaceTransform(t);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdDisplaysGet(flamewm::control::ControlClient& c) {
    flamewm::api::Result<flamewm::api::DisplaySnapshot> r = c.getDisplaySnapshot();
    if (!r.ok()) { printStatusError(r.status()); return 1; }
    std::cout << "generation: " << r.value().generation
              << " outputs: " << r.value().outputs.size() << "\n";
    for (std::size_t i = 0; i < r.value().outputs.size(); ++i) {
        const flamewm::api::OutputSnapshot& o = r.value().outputs[i];
        std::cout << "  " << o.connector << " id=" << o.id.key
                  << (o.connected ? " connected" : " disconnected")
                  << (o.primary ? " primary" : "") << "\n";
    }
    if (r.value().hasPending()) {
        std::cout << "pending: tx=" << r.value().pending.tx.value
                  << " output=" << r.value().pending.output.key << "\n";
    }
    return 0;
}

int cmdDisplaysBegin(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 3) {
        std::cerr << "usage: flamewmctl displays begin <output> <mode> <generation>\n";
        return 2;
    }
    flamewm::api::OutputId oid(argv[0]);
    uint64_t mv = 0; uint64_t gen = 0;
    if (!parseU64(argv[1], &mv)) { std::cerr << "invalid mode\n"; return 2; }
    if (!parseU64(argv[2], &gen)) { std::cerr << "invalid generation\n"; return 2; }
    flamewm::api::ModeId mid(mv);
    flamewm::api::TransactionId tx;
    flamewm::api::Status s = c.beginDisplayModeChange(oid, mid, gen, &tx);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok tx=" << tx.value << "\n";
    return 0;
}

int cmdDisplaysKeep(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 1) {
        std::cerr << "usage: flamewmctl displays keep <tx>\n";
        return 2;
    }
    uint64_t v = 0;
    if (!parseU64(argv[0], &v)) { std::cerr << "invalid tx\n"; return 2; }
    flamewm::api::Status s = c.keepDisplayMode(flamewm::api::TransactionId(v));
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdDisplaysRevert(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 1) {
        std::cerr << "usage: flamewmctl displays revert <tx>\n";
        return 2;
    }
    uint64_t v = 0;
    if (!parseU64(argv[0], &v)) { std::cerr << "invalid tx\n"; return 2; }
    flamewm::api::Status s = c.revertDisplayMode(flamewm::api::TransactionId(v));
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdDisplaysSetScale(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 3) {
        std::cerr << "usage: flamewmctl displays set-scale <output> <percent> <rev>\n";
        return 2;
    }
    flamewm::api::OutputId oid(argv[0]);
    uint32_t pct = 0; uint64_t rev = 0;
    if (!parseU32(argv[1], &pct)) { std::cerr << "invalid percent\n"; return 2; }
    if (!parseU64(argv[2], &rev)) { std::cerr << "invalid revision\n"; return 2; }
    flamewm::api::Status s = c.setShellScale(oid, pct, rev);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdDisplaysSetMode(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "usage: flamewmctl displays set-mode <output> <mode>\n";
        return 2;
    }
    flamewm::api::OutputId oid(argv[0]);
    uint64_t mv = 0;
    if (!parseU64(argv[1], &mv)) { std::cerr << "invalid mode\n"; return 2; }
    flamewm::api::ModeId mid(mv);
    flamewm::api::TransactionId tx;
    flamewm::api::Status s = c.applyDisplayMode(oid, mid, &tx);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok tx=" << tx.value << "\n";
    return 0;
}

int cmdDisplaysRestore(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 1) {
        std::cerr << "usage: flamewmctl displays restore <tx>\n";
        return 2;
    }
    uint64_t v = 0;
    if (!parseU64(argv[0], &v)) { std::cerr << "invalid tx\n"; return 2; }
    flamewm::api::Status s = c.restoreDisplayMode(flamewm::api::TransactionId(v));
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdShortcutsGet(flamewm::control::ControlClient& c) {
    flamewm::api::Result<flamewm::api::ShortcutSnapshot> r = c.getShortcutSnapshot();
    if (!r.ok()) { printStatusError(r.status()); return 1; }
    std::cout << "revision: " << r.value().revision << "\n";
    for (std::map<std::string,flamewm::api::KeyBinding>::const_iterator it = r.value().bindings.begin();
         it != r.value().bindings.end(); ++it) {
        std::cout << it->first << "=" << it->second.key << "\n";
    }
    return 0;
}

int cmdShortcutsSet(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 3) {
        std::cerr << "usage: flamewmctl shortcuts set <action> <binding> <rev>\n";
        return 2;
    }
    uint64_t rev = 0;
    if (!parseU64(argv[2], &rev)) { std::cerr << "invalid revision\n"; return 2; }
    flamewm::api::Status s = c.setBinding(argv[0], argv[1], rev);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdShortcutsClear(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "usage: flamewmctl shortcuts clear <action> <rev>\n";
        return 2;
    }
    uint64_t rev = 0;
    if (!parseU64(argv[1], &rev)) { std::cerr << "invalid revision\n"; return 2; }
    flamewm::api::Status s = c.clearBinding(argv[0], rev);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdShortcutsReset(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "usage: flamewmctl shortcuts reset <action> <rev>\n";
        return 2;
    }
    uint64_t rev = 0;
    if (!parseU64(argv[1], &rev)) { std::cerr << "invalid revision\n"; return 2; }
    flamewm::api::Status s = c.resetBinding(argv[0], rev);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdShortcutsApply(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc < 1) {
        std::cerr << "usage: flamewmctl shortcuts apply <rev> <action=binding>...\n";
        return 2;
    }
    uint64_t rev = 0;
    if (!parseU64(argv[0], &rev)) { std::cerr << "invalid revision\n"; return 2; }
    std::map<std::string,flamewm::api::KeyBinding> bindings;
    for (int i = 1; i < argc; ++i) {
        std::string kv = argv[i];
        std::size_t pos = kv.find('=');
        if (pos == std::string::npos) {
            std::cerr << "invalid binding (expected action=binding): " << kv << "\n";
            return 2;
        }
        bindings[kv.substr(0,pos)] = flamewm::api::KeyBinding(kv.substr(pos+1));
    }
    flamewm::api::Status s = c.applyShortcuts(rev, bindings);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdPanelsGet(flamewm::control::ControlClient& c) {
    flamewm::api::Result<flamewm::api::PanelsSnapshot> r = c.getPanelsSnapshot();
    if (!r.ok()) { printStatusError(r.status()); return 1; }
    std::cout << "revision: " << r.value().revision
              << " panels: " << r.value().panels.size()
              << " tasks: " << r.value().tasks.size() << "\n";
    for (std::size_t i = 0; i < r.value().panels.size(); ++i) {
        const flamewm::api::PanelSnapshot& p = r.value().panels[i];
        std::cout << "  output=" << p.output.key << " edge=" << static_cast<int>(p.edge)
                  << " size=" << p.logicalSize << (p.visible ? " visible" : "") << "\n";
    }
    return 0;
}

int cmdPanelsSetEdge(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 2 && argc != 3) {
        std::cerr << "usage: flamewmctl panels set-edge <output> <edge> [rev]\n";
        std::cerr << "  edge: bottom|top|left|right\n";
        return 2;
    }
    flamewm::api::OutputId oid(argv[0]);
    std::string edge = argv[1];
    flamewm::api::PanelEdge e = flamewm::api::PanelEdge::Bottom;
    if (edge == "bottom") e = flamewm::api::PanelEdge::Bottom;
    else if (edge == "top") e = flamewm::api::PanelEdge::Top;
    else if (edge == "left") e = flamewm::api::PanelEdge::Left;
    else if (edge == "right") e = flamewm::api::PanelEdge::Right;
    else { std::cerr << "invalid edge: " << edge << "\n"; return 2; }
    if (argc == 3) {
        uint64_t rev = 0;
        if (!parseU64(argv[2], &rev)) { std::cerr << "invalid revision\n"; return 2; }
        flamewm::api::Status s = c.setPanelEdge(oid, e, rev);
        if (!s.ok()) { printStatusError(s); return 1; }
    } else {
        flamewm::api::Status s = c.setPanelEdge(oid, e);
        if (!s.ok()) { printStatusError(s); return 1; }
    }
    std::cout << "ok\n"; return 0;
}

int cmdPanelsSetSize(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 3) {
        std::cerr << "usage: flamewmctl panels set-size <output> <size> <rev>\n";
        return 2;
    }
    flamewm::api::OutputId oid(argv[0]);
    uint32_t sz = 0; uint64_t rev = 0;
    if (!parseU32(argv[1], &sz)) { std::cerr << "invalid size\n"; return 2; }
    if (!parseU64(argv[2], &rev)) { std::cerr << "invalid revision\n"; return 2; }
    flamewm::api::Status s = c.setPanelSize(oid, sz, rev);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdPanelsPin(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "usage: flamewmctl panels pin <appId> <rev>\n";
        return 2;
    }
    uint64_t rev = 0;
    if (!parseU64(argv[1], &rev)) { std::cerr << "invalid revision\n"; return 2; }
    flamewm::api::Status s = c.pin(flamewm::api::DesktopAppId(argv[0]), rev);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdPanelsUnpin(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "usage: flamewmctl panels unpin <appId> <rev>\n";
        return 2;
    }
    uint64_t rev = 0;
    if (!parseU64(argv[1], &rev)) { std::cerr << "invalid revision\n"; return 2; }
    flamewm::api::Status s = c.unpin(flamewm::api::DesktopAppId(argv[0]), rev);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdPanelsReorder(flamewm::control::ControlClient& c, int argc, char** argv) {
    if (argc != 3) {
        std::cerr << "usage: flamewmctl panels reorder <entryId> <index> <rev>\n";
        return 2;
    }
    uint32_t idx = 0; uint64_t rev = 0;
    if (!parseU32(argv[1], &idx)) { std::cerr << "invalid index\n"; return 2; }
    if (!parseU64(argv[2], &rev)) { std::cerr << "invalid revision\n"; return 2; }
    flamewm::api::Status s = c.reorder(flamewm::api::TaskEntryId(argv[0]), idx, rev);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

int cmdSessionCaps(flamewm::control::ControlClient& c) {
    flamewm::api::Result<flamewm::api::SessionCapabilities> r = c.getSessionCapabilities();
    if (!r.ok()) { printStatusError(r.status()); return 1; }
    std::cout << "lock: " << (r.value().canLock ? "yes" : "no") << "\n";
    std::cout << "logout: " << (r.value().canLogout ? "yes" : "no") << "\n";
    std::cout << "suspend: " << (r.value().canSuspend ? "yes" : "no") << "\n";
    std::cout << "reboot: " << (r.value().canReboot ? "yes" : "no") << "\n";
    std::cout << "shutdown: " << (r.value().canShutdown ? "yes" : "no") << "\n";
    return 0;
}

int cmdSessionAction(flamewm::control::ControlClient& c, const std::string& act) {
    flamewm::api::SessionAction a = flamewm::api::SessionAction::Lock;
    if (act == "lock") a = flamewm::api::SessionAction::Lock;
    else if (act == "logout") a = flamewm::api::SessionAction::Logout;
    else if (act == "suspend") a = flamewm::api::SessionAction::Suspend;
    else if (act == "reboot") a = flamewm::api::SessionAction::Reboot;
    else if (act == "shutdown") a = flamewm::api::SessionAction::Shutdown;
    else { std::cerr << "unknown session action: " << act << "\n"; return 2; }
    flamewm::api::Status s = c.requestSessionAction(a);
    if (!s.ok()) { printStatusError(s); return 1; }
    std::cout << "ok\n"; return 0;
}

} // namespace

int main(int argc, char** argv) {
    const char* prog = argc > 0 ? argv[0] : "flamewmctl";
    if (argc < 2) {
        printHelp(prog);
        return 2;
    }
    std::string first = argv[1];
    if (first == "-h" || first == "--help" || first == "help") {
        printHelp(prog);
        return 0;
    }

    flamewm::control::ControlClient client;
    if (!client.connect()) {
        std::cerr << "error: Unavailable: failed to connect to session bus"
                  << " (is dbus running and FlameWM active?)\n";
        return 1;
    }

    std::string cmd = argv[1];

    if (cmd == "status" || cmd == "ping") {
        return cmdPing(client);
    } else if (cmd == "get-version" || cmd == "version") {
        return cmdGetVersion(client);
    } else if (cmd == "get-capabilities" || cmd == "capabilities") {
        return cmdGetCapabilities(client);
    } else if (cmd == "settings") {
        if (argc < 3) { std::cerr << "usage: flamewmctl settings <get|apply|reset>\n"; return 2; }
        std::string sub = argv[2];
        if (sub == "get") return cmdSettingsGet(client);
        if (sub == "apply") return cmdSettingsApply(client, argc - 3, argv + 3);
        if (sub == "reset") return cmdSettingsReset(client, argc - 3, argv + 3);
        std::cerr << "unknown settings subcommand: " << sub << "\n"; return 2;
    } else if (cmd == "workspaces") {
        if (argc < 3) { std::cerr << "usage: flamewmctl workspaces <get|activate|insert|remove|transform>\n"; return 2; }
        std::string sub = argv[2];
        if (sub == "get") return cmdWorkspacesGet(client);
        if (sub == "activate") return cmdWorkspacesActivate(client, argc - 3, argv + 3);
        if (sub == "insert") return cmdWorkspacesInsert(client, argc - 3, argv + 3);
        if (sub == "remove") return cmdWorkspacesRemove(client, argc - 3, argv + 3);
        if (sub == "transform") return cmdWorkspacesTransform(client, argc - 3, argv + 3);
        std::cerr << "unknown workspaces subcommand: " << sub << "\n"; return 2;
    } else if (cmd == "displays") {
        if (argc < 3) { std::cerr << "usage: flamewmctl displays <get|begin|keep|revert|set-scale|set-mode|restore>\n"; return 2; }
        std::string sub = argv[2];
        if (sub == "get") return cmdDisplaysGet(client);
        if (sub == "begin") return cmdDisplaysBegin(client, argc - 3, argv + 3);
        if (sub == "keep") return cmdDisplaysKeep(client, argc - 3, argv + 3);
        if (sub == "revert") return cmdDisplaysRevert(client, argc - 3, argv + 3);
        if (sub == "set-scale" || sub == "setScale" || sub == "set_scale") return cmdDisplaysSetScale(client, argc - 3, argv + 3);
        if (sub == "set-mode") return cmdDisplaysSetMode(client, argc - 3, argv + 3);
        if (sub == "restore") return cmdDisplaysRestore(client, argc - 3, argv + 3);
        std::cerr << "unknown displays subcommand: " << sub << "\n"; return 2;
    } else if (cmd == "shortcuts") {
        if (argc < 3) { std::cerr << "usage: flamewmctl shortcuts <get|set|clear|reset|apply>\n"; return 2; }
        std::string sub = argv[2];
        if (sub == "get") return cmdShortcutsGet(client);
        if (sub == "set") return cmdShortcutsSet(client, argc - 3, argv + 3);
        if (sub == "clear") return cmdShortcutsClear(client, argc - 3, argv + 3);
        if (sub == "reset") return cmdShortcutsReset(client, argc - 3, argv + 3);
        if (sub == "apply") return cmdShortcutsApply(client, argc - 3, argv + 3);
        std::cerr << "unknown shortcuts subcommand: " << sub << "\n"; return 2;
    } else if (cmd == "panels") {
        if (argc < 3) { std::cerr << "usage: flamewmctl panels <get|set-edge|set-size|pin|unpin|reorder>\n"; return 2; }
        std::string sub = argv[2];
        if (sub == "get") return cmdPanelsGet(client);
        if (sub == "set-edge") return cmdPanelsSetEdge(client, argc - 3, argv + 3);
        if (sub == "set-size") return cmdPanelsSetSize(client, argc - 3, argv + 3);
        if (sub == "pin") return cmdPanelsPin(client, argc - 3, argv + 3);
        if (sub == "unpin") return cmdPanelsUnpin(client, argc - 3, argv + 3);
        if (sub == "reorder") return cmdPanelsReorder(client, argc - 3, argv + 3);
        std::cerr << "unknown panels subcommand: " << sub << "\n"; return 2;
    } else if (cmd == "session") {
        if (argc < 3) { std::cerr << "usage: flamewmctl session <caps|get-caps|getCaps|request|lock|logout|suspend|reboot|shutdown>\n"; return 2; }
        std::string sub = argv[2];
        if (sub == "caps" || sub == "capabilities" || sub == "get-caps" || sub == "getCaps" || sub == "get_caps") return cmdSessionCaps(client);
        if (sub == "request") {
            if (argc < 4) { std::cerr << "usage: flamewmctl session request <action>\n"; return 2; }
            return cmdSessionAction(client, argv[3]);
        }
        return cmdSessionAction(client, sub);
    }

    std::cerr << "unknown command: " << cmd << "\n";
    printHelp(prog);
    return 2;
}
