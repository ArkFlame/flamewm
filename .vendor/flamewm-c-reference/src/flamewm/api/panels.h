#ifndef FLAMEWM_API_PANELS_H
#define FLAMEWM_API_PANELS_H

#include "ids.h"
#include "geometry.h"
#include "errors.h"

#include <cstdint>
#include <vector>

namespace flamewm {
namespace api {

enum class PanelEdge {
    Bottom = 0,
    Top = 1,
    Left = 2,
    Right = 3
};

struct PanelSnapshot {
    OutputId output;
    PanelEdge edge;
    int logicalSize;
    bool visible;
    Rect geometry;

    PanelSnapshot()
        : edge(PanelEdge::Bottom)
        , logicalSize(0)
        , visible(false)
        , geometry() {}
};

struct TaskEntry {
    TaskEntryId id;
    bool isPinned;
    bool hasWindow;
    WindowRef window;
    DesktopAppId appId;
    int orderIndex;

    TaskEntry()
        : isPinned(false)
        , hasWindow(false)
        , window()
        , orderIndex(0) {}
};

struct PanelsSnapshot {
    uint64_t revision;
    std::vector<PanelSnapshot> panels;
    std::vector<TaskEntry> tasks;
    std::vector<DesktopAppId> pinnedApps;
    OutputId trayOwner;
    bool startOpen;
    OutputId startOutput;

    PanelsSnapshot()
        : revision(0)
        , startOpen(false) {}
};

} // namespace api
} // namespace flamewm

#endif // FLAMEWM_API_PANELS_H
