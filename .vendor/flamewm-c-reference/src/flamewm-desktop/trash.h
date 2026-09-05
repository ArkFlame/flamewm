#ifndef FLAMEWM_DESKTOP_TRASH_H
#define FLAMEWM_DESKTOP_TRASH_H

#include <string>
#include <vector>

namespace flamewm {
namespace desktop {

// Freedesktop Trash spec: home + topdir handling, cross-filesystem.
// https://specifications.freedesktop.org/trash-spec/trashspec-latest.html
class TrashService {
public:
    struct TrashResult { bool ok; std::string error; std::string trashedPath; };
    struct EmptyResult { int removed; std::vector<std::string> failures; };

    // Trash a path (file/dir/symlink). Returns per-item result.
    // Implements home trash $XDG_DATA_HOME/Trash and topdir .Trash/$uid or .Trash-$uid fallback.
    static TrashResult trash(const std::string& path, std::string* error);

    // Empty Trash: permanently delete all entries in home + known topdirs.
    // Reports per-item failures via EmptyResult.failures.
    static EmptyResult emptyTrash(std::string* error);

    // Home trash locations
    static std::string homeTrashFiles();
    static std::string homeTrashInfo();
    static std::string homeTrashDir();

    // Helpers exposed for testing
    static std::string trashInfoContent(const std::string& originalPath);
    static std::string uniqueTrashName(const std::string& base, const std::string& trashFilesDir);
    static bool isInTrash(const std::string& path);

private:
    static bool ensureTrashDirs(const std::string& filesDir, const std::string& infoDir, std::string* error);
};

} // namespace desktop
} // namespace flamewm

#endif
