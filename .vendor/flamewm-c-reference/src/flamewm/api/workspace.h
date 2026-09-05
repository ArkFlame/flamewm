#ifndef FLAMEWM_API_WORKSPACE_H
#define FLAMEWM_API_WORKSPACE_H

#include "ids.h"
#include "geometry.h"
#include "errors.h"

#include <cstdint>
#include <string>
#include <vector>

namespace flamewm {
namespace api {

// Read-only snapshot of workspace model. No EWMH/IceWM types.
//
// revision: monotonically increasing generation of the whole set.
// count: number of workspaces (>=1).
// activeIndex: currently active workspace [0, count).
// lastIndex: previously active workspace [0, count) or -1 if none.
// workspaces: per-workspace identity ordered by index; size == count.
// names: optional per-workspace names; when non-empty size == count.
struct WorkspaceSnapshot {
    uint64_t revision;
    int count;
    int activeIndex;
    int lastIndex;
    std::vector<WorkspaceRef> workspaces;
    std::vector<std::string> names;

    WorkspaceSnapshot()
        : revision(0), count(0), activeIndex(0), lastIndex(-1) {}
};

// Intent to mutate workspace model. Validated against snapshot revision
// before application; stale expectedRevision must be rejected with
// Error::StaleRevision. No EWMH specifics, no IceWM types.
struct WorkspaceTransform {
    // enum class used to avoid C++ name conflict between unscoped
    // enumerators (Activate, InsertAfter, Remove) and factory helpers
    // of the same name. Enumerator names and factory names match the
    // spec intent exactly: Type::Activate etc. and Activate() etc.
    enum class Type {
        Activate,
        InsertAfter,
        Remove
    };

    Type type;
    int index;
    uint64_t expectedRevision;

    WorkspaceTransform()
        : type(Type::Activate), index(-1), expectedRevision(0) {}

    WorkspaceTransform(Type t, int idx, uint64_t rev)
        : type(t), index(idx), expectedRevision(rev) {}

    static WorkspaceTransform Activate(int idx, uint64_t rev) {
        return WorkspaceTransform(Type::Activate, idx, rev);
    }

    static WorkspaceTransform InsertAfter(int idx, uint64_t rev) {
        return WorkspaceTransform(Type::InsertAfter, idx, rev);
    }

    static WorkspaceTransform Remove(int idx, uint64_t rev) {
        return WorkspaceTransform(Type::Remove, idx, rev);
    }
};

// Validation: count >=1, activeIndex in [0,count), lastIndex in [-1,count),
// workspaces.size() == count, names either empty or size == count,
// each WorkspaceRef index matches its position and is valid.
inline bool isValid(const WorkspaceSnapshot& s) {
    if (s.count < 1) return false;
    if (s.activeIndex < 0 || s.activeIndex >= s.count) return false;
    if (s.lastIndex < -1 || s.lastIndex >= s.count) return false;
    if (s.workspaces.size() != static_cast<std::size_t>(s.count)) return false;
    if (!s.names.empty() && s.names.size() != static_cast<std::size_t>(s.count)) return false;
    for (std::size_t i = 0; i < s.workspaces.size(); ++i) {
        const WorkspaceRef& w = s.workspaces[i];
        if (!w.valid()) return false;
        if (w.index != static_cast<int>(i)) return false;
    }
    return true;
}

} // namespace api
} // namespace flamewm

#endif // FLAMEWM_API_WORKSPACE_H
