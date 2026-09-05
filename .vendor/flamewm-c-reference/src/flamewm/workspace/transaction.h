#ifndef FLAMEWM_WORKSPACE_TRANSACTION_H
#define FLAMEWM_WORKSPACE_TRANSACTION_H

#include <vector>
#include <string>

namespace flamewm {
namespace workspace {

// Pure helper for indexed workspace mutation.
// No IceWM manager mutation — returns a model struct for the caller to apply.
//
// Min invariant: at least one workspace must remain (min>=1).
// One transaction must update model count/names, every affected frame index,
// active/last workspace, visibility/focus, _NET_NUMBER_OF_DESKTOPS,
// _NET_CURRENT_DESKTOP, names/viewport/workarea and pager/window menus.
// This model computes the frame index mapping; EWMH field values are
// conceptual (count/names/current/viewport/workarea) — caller applies them.
//
// See FAILURES.md: tail-only shrink reused loses windows; indexed transaction
// with old->new mapping required.
struct WorkspaceTransactionResult {
    bool valid;
    std::string error;
    int newCount;
    std::vector<std::string> newNames; // size newCount
    std::vector<int> frameNewWs; // per-frame new workspace index; -1 means invalid
    int newActive;
    int newLast;
    // Destination for windows on removed workspace when shrinking: migrate to nearest
    int removedMigrateTarget;

    WorkspaceTransactionResult() : valid(false), newCount(0), newActive(0), newLast(0), removedMigrateTarget(-1) {}
};

class WorkspaceTransaction {
public:
    // Validate count/index. isInsert: true for insert, false for remove.
    // count: current count, index: insertion/removal index, minWorkspaces>=1
    static WorkspaceTransactionResult insertWorkspace(
        int count, int index,
        const std::vector<std::string>& names,
        const std::vector<int>& frameWs,
        int active, int last,
        int minWorkspaces = 1);

    static WorkspaceTransactionResult removeWorkspace(
        int count, int index,
        const std::vector<std::string>& names,
        const std::vector<int>& frameWs,
        int active, int last,
        int minWorkspaces = 1);

    // Generic validate
    static bool isValidCount(int count, int minWorkspaces, std::string* err);
    static bool isValidIndexForInsert(int count, int index, std::string* err);
    static bool isValidIndexForRemove(int count, int index, std::string* err);
};

} // namespace workspace
} // namespace flamewm
#endif
