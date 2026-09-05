#ifndef FLAMEWM_API_DISPLAY_H
#define FLAMEWM_API_DISPLAY_H

#include "ids.h"
#include "geometry.h"
#include "errors.h"

#include <cstdint>
#include <string>
#include <vector>

namespace flamewm {
namespace api {

struct DisplayMode {
    ModeId id;
    Size resolution;
    int refreshMilliHz;
    bool preferred;

    DisplayMode() : id(), resolution(), refreshMilliHz(0), preferred(false) {}
    DisplayMode(const ModeId& i, const Size& s, int r, bool p)
        : id(i), resolution(s), refreshMilliHz(r), preferred(p) {}

    bool operator==(const DisplayMode& o) const {
        return id == o.id && resolution == o.resolution &&
               refreshMilliHz == o.refreshMilliHz && preferred == o.preferred;
    }
    bool operator!=(const DisplayMode& o) const { return !(*this == o); }
};

struct OutputSnapshot {
    OutputId id;
    std::string connector;
    std::string edidIdentity;
    bool connected;
    bool primary;
    Rect geometry;
    ModeId currentMode;
    std::vector<DisplayMode> modes;
    int shellScalePercent;

    OutputSnapshot()
        : id(),
          connector(),
          edidIdentity(),
          connected(false),
          primary(false),
          geometry(),
          currentMode(),
          modes(),
          shellScalePercent(100) {}
};

struct DisplaySnapshot {
    uint64_t generation;
    std::vector<OutputSnapshot> outputs;

    struct PendingModeChange {
        TransactionId tx;
        OutputId output;
        ModeId mode;
        uint64_t deadlineMs;
        bool active;

        PendingModeChange()
            : tx(), output(), mode(), deadlineMs(0), active(false) {}
        PendingModeChange(const TransactionId& t, const OutputId& o,
                          const ModeId& m, uint64_t d, bool a)
            : tx(t), output(o), mode(m), deadlineMs(d), active(a) {}
    } pending;

    DisplaySnapshot() : generation(0), outputs(), pending() {}

    bool hasPending() const { return pending.active; }
};

} // namespace api
} // namespace flamewm

#endif // FLAMEWM_API_DISPLAY_H
