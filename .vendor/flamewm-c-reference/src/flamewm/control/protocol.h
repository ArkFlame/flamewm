#ifndef FLAMEWM_CONTROL_PROTOCOL_H
#define FLAMEWM_CONTROL_PROTOCOL_H

#include <cstdint>
#include <string>
#include <vector>

namespace flamewm {
namespace control {

// Bus and object path
extern const char kBusName[];
extern const char kObjectPath[];

// Interfaces (7)
extern const char kIfaceRoot[];
extern const char kIfaceSettings[];
extern const char kIfaceWorkspaces[];
extern const char kIfaceDisplays[];
extern const char kIfaceShortcuts[];
extern const char kIfacePanels[];
extern const char kIfaceSession[];

// D-Bus error names (9)
extern const char kErrorInvalidArgument[];
extern const char kErrorNotFound[];
extern const char kErrorStaleRevision[];
extern const char kErrorConflict[];
extern const char kErrorBusy[];
extern const char kErrorUnsupported[];
extern const char kErrorUnavailable[];
extern const char kErrorPermissionDenied[];
extern const char kErrorInternalFailure[];

struct Version {
    uint32_t major;
    uint32_t minor;
    uint32_t patch;

    Version() : major(0), minor(0), patch(0) {}
    Version(uint32_t ma, uint32_t mi, uint32_t pa)
        : major(ma), minor(mi), patch(pa) {}

    std::string toString() const;

    bool operator==(const Version& other) const {
        return major == other.major && minor == other.minor && patch == other.patch;
    }
    bool operator!=(const Version& other) const {
        return !(*this == other);
    }
};

// Helpers
Version CurrentVersion();
Version MakeVersion(uint32_t major, uint32_t minor, uint32_t patch);
bool ParseVersion(const std::string& s, Version* out);

// Capabilities helpers (delegates to flamewm::api::Capabilities::current())
std::vector<std::string> GetCapabilities();
bool HasCapability(const std::string& cap);

} // namespace control
} // namespace flamewm

#endif // FLAMEWM_CONTROL_PROTOCOL_H
