#ifndef FLAMEWM_LAUNCHER_SEARCH_H
#define FLAMEWM_LAUNCHER_SEARCH_H

#include <string>
#include <vector>

namespace flamewm {
namespace launcher {

// Search helper: filter by name/exec/keywords case-insensitive.
// No shell interpolation; Exec field codes (%F %U etc) are treated as data.
struct SearchEntry {
    std::string id;       // desktopId or normalized identity
    std::string name;     // Display name
    std::string exec;     // Exec string
    std::string keywords; // Keywords / GenericName / Comment combined
};

class SearchHelper {
public:
    // Returns true if entry matches query (substring, case-insensitive) on
    // name, exec (with field codes preserved as literal), or keywords.
    static bool matches(const SearchEntry& entry, const std::string& query);

    // Filter list; empty query returns all.
    static std::vector<SearchEntry> filter(const std::vector<SearchEntry>& entries,
                                           const std::string& query);

    // Field-code safety: strip/neutralize field codes for display only; exec
    // launch must use preserved argv vector, not shell-interpolated string.
    static std::string sanitizeExecForDisplay(const std::string& exec);

    // Returns true if string contains shell metacharacters that would be
    // dangerous if interpolated (caller must use argv vector).
    static bool containsShellMeta(const std::string& s);

private:
    static std::string toLower(const std::string& s);
};

} // namespace launcher
} // namespace flamewm

#endif // FLAMEWM_LAUNCHER_SEARCH_H
