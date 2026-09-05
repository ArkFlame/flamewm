#ifndef FLAMEWM_DESKTOP_MODEL_H
#define FLAMEWM_DESKTOP_MODEL_H

#include <string>
#include <vector>
#include <map>

namespace flamewm {
namespace desktop {

// Item kinds per DESKTOP.md
enum ItemKind {
    KindDirectory = 0,
    KindRegularFile = 1,
    KindDesktopLauncher = 2,
    KindSymlink = 3,
    KindTrashPseudoItem = 4
};

struct DesktopItem {
    std::string name;            // basename
    std::string path;            // absolute path (or trash pseudo)
    ItemKind kind;
    int cellCol;                 // logical grid column
    int cellRow;                 // logical grid row
    std::string outputIdentity;  // durable output id for persistence
    bool selected;

    DesktopItem() : kind(KindRegularFile), cellCol(-1), cellRow(-1), selected(false) {}
};

// XDG Desktop dir resolution - no shell eval
class DesktopDirResolver {
public:
    // Resolve XDG desktop dir. Reads $XDG_CONFIG_HOME/user-dirs.dirs or ~/.config/user-dirs.dirs
    // Parses XDG_DESKTOP_DIR="..." with no shell expansion. Falls back to ~/Desktop.
    static std::string resolve(std::string* error = 0);
    // Pure helper for testing: parse content of user-dirs.dirs
    static std::string parseUserDirsContent(const std::string& content,
                                            const std::string& home);
    // Expand $HOME and ${HOME} only, no shell eval
    static std::string expandHomeVars(const std::string& raw, const std::string& home);
    static std::string homeDir();
    static std::string xdgDataHome();
};

// Logical placement persistence: outputIdentity + col + row
struct CellPos {
    std::string outputIdentity;
    int col;
    int row;
    CellPos() : col(-1), row(-1) {}
    CellPos(const std::string& o,int c,int r):outputIdentity(o),col(c),row(r){}
};

class DesktopModel {
public:
    DesktopModel();
    explicit DesktopModel(const std::string& desktopPath);

    const std::string& desktopPath() const { return desktopPath_; }
    void setDesktopPath(const std::string& p) { desktopPath_ = p; }

    const std::vector<DesktopItem>& items() const { return items_; }
    std::vector<DesktopItem>& items() { return items_; }

    // Filesystem truth: rescan directory, reconcile items
    bool rescan(std::string* error);
    // Lookup by path/name
    DesktopItem* findByPath(const std::string& path);
    const DesktopItem* findByPath(const std::string& path) const;
    DesktopItem* findByName(const std::string& name);

    // Rename via move cookie pairing: migrate layout record atomically
    bool handleRename(const std::string& oldPath, const std::string& newPath);

    // Persistence: atomic persist logical cells (tmp+rename)
    bool loadLayout(const std::string& layoutFile, std::string* error);
    bool saveLayout(const std::string& layoutFile, std::string* error) const;
    // In-memory layout map: path -> CellPos
    void setCell(const std::string& path, const CellPos& pos);
    bool getCell(const std::string& path, CellPos* out) const;

    // First deterministic free cell for new item
    CellPos firstFreeCell(const std::string& outputIdentity,
                          int cols, int rows) const;
    // Reflow invalid cells after topology change: retain valid, reflow invalid to first-free on primary
    void reflowAfterTopologyChange(const std::string& primaryOutput, int cols, int rows);

    static ItemKind kindForPath(const std::string& path);

    // For testing: inject items directly
    void setItems(const std::vector<DesktopItem>& v) { items_ = v; rebuildCellMap(); }

private:
    std::string desktopPath_;
    std::vector<DesktopItem> items_;
    std::map<std::string, CellPos> cellMap_; // path -> pos

    void rebuildCellMap();
    bool isOccupied(int col, int row, const std::string& output) const;
};

} // namespace desktop
} // namespace flamewm

#endif
