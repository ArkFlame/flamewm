#include "transaction.h"

namespace flamewm {
namespace workspace {

bool WorkspaceTransaction::isValidCount(int count, int minWorkspaces, std::string* err) {
    if (count < minWorkspaces) {
        if (err) *err = "count below minimum";
        return false;
    }
    if (count > 1000) {
        if (err) *err = "count exceeds max";
        return false;
    }
    return true;
}

bool WorkspaceTransaction::isValidIndexForInsert(int count, int index, std::string* err) {
    if (index < 0 || index > count) {
        if (err) *err = "insert index out of range";
        return false;
    }
    return true;
}

bool WorkspaceTransaction::isValidIndexForRemove(int count, int index, std::string* err) {
    if (index < 0 || index >= count) {
        if (err) *err = "remove index out of range";
        return false;
    }
    return true;
}

WorkspaceTransactionResult WorkspaceTransaction::insertWorkspace(
    int count, int index,
    const std::vector<std::string>& names,
    const std::vector<int>& frameWs,
    int active, int last,
    int minWorkspaces) {

    WorkspaceTransactionResult r;
    r.valid = false;
    if (!isValidCount(count, minWorkspaces, &r.error)) return r;
    if (!isValidIndexForInsert(count, index, &r.error)) return r;

    r.newCount = count + 1;
    r.newNames.reserve(r.newCount);
    for (int i = 0; i < r.newCount; ++i) {
        if (i < index) {
            r.newNames.push_back(i < (int)names.size() ? names[i] : "");
        } else if (i == index) {
            // New workspace name — auto-generated; caller may rename via pager edit.
            r.newNames.push_back("Workspace " + std::to_string(i+1));
        } else {
            int src = i - 1;
            r.newNames.push_back(src < (int)names.size() ? names[src] : "");
        }
    }
    r.frameNewWs.reserve(frameWs.size());
    for (size_t i = 0; i < frameWs.size(); ++i) {
        int ws = frameWs[i];
        if (ws < 0) { // AllWorkspaces (-1) stays sticky
            r.frameNewWs.push_back(ws);
        } else if (ws >= index) {
            r.frameNewWs.push_back(ws + 1);
        } else {
            r.frameNewWs.push_back(ws);
        }
    }
    // Active/last indices shift if at or after insert point
    r.newActive = active;
    r.newLast = last;
    if (active >= 0 && active >= index) r.newActive = active + 1;
    if (last >= 0 && last >= index) r.newLast = last + 1;
    if (r.newActive >= r.newCount) r.newActive = r.newCount - 1;
    if (r.newLast >= r.newCount) r.newLast = r.newCount - 1;
    r.valid = true;
    return r;
}

WorkspaceTransactionResult WorkspaceTransaction::removeWorkspace(
    int count, int index,
    const std::vector<std::string>& names,
    const std::vector<int>& frameWs,
    int active, int last,
    int minWorkspaces) {

    WorkspaceTransactionResult r;
    r.valid = false;
    if (!isValidCount(count, minWorkspaces, &r.error)) return r;
    if (!isValidIndexForRemove(count, index, &r.error)) return r;
    if (count - 1 < minWorkspaces) {
        r.error = "removing would violate min workspaces";
        return r;
    }
    r.newCount = count - 1;

    r.newNames.reserve(r.newCount);
    for (int i = 0; i < count; ++i) {
        if (i == index) continue;
        r.newNames.push_back(i < (int)names.size() ? names[i] : "");
    }

    // Destination for windows on removed workspace: nearest valid index
    // per WINDOWS.md "migrate to nearest"
    int migrate = index;
    if (migrate >= r.newCount) migrate = r.newCount - 1;
    if (migrate < 0) migrate = 0;
    r.removedMigrateTarget = migrate;

    r.frameNewWs.reserve(frameWs.size());
    for (size_t i = 0; i < frameWs.size(); ++i) {
        int ws = frameWs[i];
        if (ws < 0) {
            r.frameNewWs.push_back(ws);
        } else if (ws == index) {
            r.frameNewWs.push_back(migrate);
        } else if (ws > index) {
            r.frameNewWs.push_back(ws - 1);
        } else {
            r.frameNewWs.push_back(ws);
        }
    }

    // Active/last adjustment
    r.newActive = active;
    r.newLast = last;
    if (active == index) {
        r.newActive = migrate;
    } else if (active > index) {
        r.newActive = active - 1;
    }
    if (last == index) {
        r.newLast = migrate;
    } else if (last > index) {
        r.newLast = last - 1;
    }
    if (r.newActive < 0) r.newActive = 0;
    if (r.newLast < 0) r.newLast = 0;
    if (r.newActive >= r.newCount) r.newActive = r.newCount - 1;
    if (r.newLast >= r.newCount) r.newLast = r.newCount - 1;

    r.valid = true;
    return r;
}

} // namespace workspace
} // namespace flamewm
