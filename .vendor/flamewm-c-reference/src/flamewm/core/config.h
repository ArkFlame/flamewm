#ifndef FLAMEWM_CORE_CONFIG_H
#define FLAMEWM_CORE_CONFIG_H

#include <string>
#include <map>
#include <stdint.h>

namespace flamewm {

// SettingsSnapshot — immutable typed snapshot with monotonic revision
struct SettingsSnapshot {
    uint64_t revision; // monotonic, 0 = unset

    std::string accent; // hex color or empty=default
    std::string iconTheme;

    // Opacities 0..60 inclusive, distinct keys
    int desktopSelectionFillOpacity;
    int windowSnapPreviewFillOpacity;

    std::string fontFamily;  // IBM Plex Sans default
    bool fontBold;
    int fontSizeOffset; // -2..+4 etc

    std::string taskbarColor;
    int taskbarOpacity;
    int taskbarHeight;
    std::string startButtonText;
    std::string customStartIcon;
    bool stickyNotesEnabled;

    // per-output scales: durableId -> pct (100/125/150/175/200)
    std::map<std::string, int> perOutputScale;

    // hotkey map: action string -> binding string (empty = not assigned)
    std::map<std::string, std::string> hotkeys;

    // unknown compatible keys preserved verbatim
    std::map<std::string, std::string> unknownKeys;

    SettingsSnapshot();

    bool isValid(std::string* error) const;
    static SettingsSnapshot defaults();
};

class ConfigStore {
public:
    ConfigStore();
    ~ConfigStore();

    const SettingsSnapshot& current() const { return current_; }
    uint64_t revision() const { return current_.revision; }

    // Parse full snapshot from key=value text; validate; return false on error
    bool parseSnapshot(const std::string& text, SettingsSnapshot& out, std::string* error) const;
    // Apply validated snapshot if revision > current (monotonic). Returns true if applied.
    bool applySnapshot(const SettingsSnapshot& snap, std::string* error);
    // Ignore stale revision (snap.revision <= current)
    bool isStale(const SettingsSnapshot& snap) const;

    // Atomic persist via tmp+rename. Takes filepath.
    bool persistToFile(const std::string& path, const SettingsSnapshot& snap, std::string* error) const;
    std::string serialize(const SettingsSnapshot& snap) const;

private:
    SettingsSnapshot current_;
};

} // namespace flamewm
#endif
