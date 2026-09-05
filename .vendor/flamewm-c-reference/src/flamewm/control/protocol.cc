#include "flamewm/control/protocol.h"

#include "flamewm/api/capabilities.h"
#include "flamewm/core/version.h"

#include <cstdio>
#include <cstdlib>

namespace flamewm {
namespace control {

const char kBusName[] = "com.arkflame.FlameWM1";
const char kObjectPath[] = "/com/arkflame/FlameWM1";

const char kIfaceRoot[] = "com.arkflame.FlameWM1";
const char kIfaceSettings[] = "com.arkflame.FlameWM1.Settings";
const char kIfaceWorkspaces[] = "com.arkflame.FlameWM1.Workspaces";
const char kIfaceDisplays[] = "com.arkflame.FlameWM1.Displays";
const char kIfaceShortcuts[] = "com.arkflame.FlameWM1.Shortcuts";
const char kIfacePanels[] = "com.arkflame.FlameWM1.Panels";
const char kIfaceSession[] = "com.arkflame.FlameWM1.Session";

const char kErrorInvalidArgument[] = "com.arkflame.FlameWM1.Error.InvalidArgument";
const char kErrorNotFound[] = "com.arkflame.FlameWM1.Error.NotFound";
const char kErrorStaleRevision[] = "com.arkflame.FlameWM1.Error.StaleRevision";
const char kErrorConflict[] = "com.arkflame.FlameWM1.Error.Conflict";
const char kErrorBusy[] = "com.arkflame.FlameWM1.Error.Busy";
const char kErrorUnsupported[] = "com.arkflame.FlameWM1.Error.Unsupported";
const char kErrorUnavailable[] = "com.arkflame.FlameWM1.Error.Unavailable";
const char kErrorPermissionDenied[] = "com.arkflame.FlameWM1.Error.PermissionDenied";
const char kErrorInternalFailure[] = "com.arkflame.FlameWM1.Error.InternalFailure";

std::string Version::toString() const {
    char buf[64];
    std::snprintf(buf, sizeof(buf), "%u.%u.%u",
                  static_cast<unsigned>(major),
                  static_cast<unsigned>(minor),
                  static_cast<unsigned>(patch));
    return std::string(buf);
}

Version CurrentVersion() {
    Version v;
    const char* s = flamewm::flameVersion();
    if (s) {
        // best effort parse FLAMEWM_VERSION "0.0.5"
        ParseVersion(std::string(s), &v);
    }
    return v;
}

Version MakeVersion(uint32_t major, uint32_t minor, uint32_t patch) {
    return Version(major, minor, patch);
}

bool ParseVersion(const std::string& s, Version* out) {
    if (!out) return false;
    if (s.empty()) return false;
    // Expect "major.minor.patch" with decimal unsigned ints.
    int n = 0;
    unsigned ma = 0, mi = 0, pa = 0;
    // Use sscanf but verify full consumption.
    char extra = 0;
    int matched = std::sscanf(s.c_str(), "%u.%u.%u %c", &ma, &mi, &pa, &extra);
    // sscanf with " %c" will match trailing non-space; we want exactly 3 numbers.
    // Also ensure there is no extra char.
    if (matched != 3) return false;
    // Verify canonical form: toString round-trips.
    Version tmp(ma, mi, pa);
    if (tmp.toString() != s) {
        // Allow leading zeros? Canonical check would reject them; enforce exact.
        // To keep parser lenient for canonical only, reject non-canonical.
        // But also handle that toString of 0.0.5 is "0.0.5" -> matches.
        // Check that s contains only digits and dots.
        // If mismatch, still verify s format manually.
        // Re-validate by scanning dots.
        // For now, require exact round-trip.
        return false;
    }
    out->major = ma;
    out->minor = mi;
    out->patch = pa;
    // Also set n unused to silence warning; alternative manual parse:
    (void)n;
    return true;
}

std::vector<std::string> GetCapabilities() {
    return flamewm::api::Capabilities::current().items;
}

bool HasCapability(const std::string& cap) {
    flamewm::api::Capabilities c = flamewm::api::Capabilities::current();
    return c.has(cap);
}

} // namespace control
} // namespace flamewm
