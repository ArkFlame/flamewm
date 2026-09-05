#ifndef FLAMEWM_LAUNCHER_APPMODEL_H
#define FLAMEWM_LAUNCHER_APPMODEL_H

// FDO app model reuse concept: cached desktop entries, visible categories tree,
// local in-memory search (filter per keystroke, no fs scan), visible search icon,
// keyboard arrows/Enter/Escape, Power/Session actions via existing IceWM backends (stub).
// Real FDO discovery (fdomenu.cc / desktop file parsing) is reused at integration
// time; this model is the presentation/search layer.

#include "search.h"
#include <string>
#include <vector>
#include <map>

namespace flamewm {
namespace launcher {

enum CategoryId {
    CatAll = 0,
    CatAccessories,
    CatDevelopment,
    CatGraphics,
    CatMultimedia,
    CatNetwork,
    CatOffice,
    CatSystem,
    CatPowerSession,
    CatCount
};

struct AppCategory {
    CategoryId id;
    std::string name;
    std::string iconName; // visible icon per category
};

struct AppItem {
    SearchEntry entry;
    CategoryId category;
    bool isPowerAction; // Power/Session actions use IceWM backends
};

enum PowerActionId {
    PowerLock = 0,
    PowerLogout,
    PowerReboot,
    PowerShutdown,
    PowerSuspend
};

struct PowerAction {
    PowerActionId id;
    std::string label;
    std::string iconName;
};

class AppModel {
public:
    AppModel();

    // Cache population (called once at load, not per keystroke).
    void setEntries(const std::vector<AppItem>& items);
    void addEntry(const AppItem& item);
    void clear();

    // Categories tree (visible).
    std::vector<AppCategory> categories() const;
    std::vector<AppItem> itemsForCategory(CategoryId cat) const;

    // In-memory search: filter per keystroke, no fs scan.
    // Visible search icon: caller checks hasSearchIcon().
    std::vector<AppItem> search(const std::string& query) const;
    bool hasSearchIcon() const { return true; }

    // Keyboard navigation over a result set.
    // arrows/Enter/Escape — pure model index tracking.
    void setCurrentResults(const std::vector<AppItem>& results);
    bool moveSelection(int delta); // arrows: +1 down, -1 up, clamps
    void resetSelection();
    int selectedIndex() const { return selectedIndex_; }
    const AppItem* selectedItem() const;
    bool handleKey(const std::string& key); // "Up","Down","Enter","Escape" -> true if handled
    bool wasActivated() const { return wasActivated_; }
    bool wasDismissed() const { return wasDismissed_; }
    void clearActivationFlags() { wasActivated_ = false; wasDismissed_ = false; }

    // Power/Session actions (stub: delegates to IceWM backends at integration).
    std::vector<PowerAction> powerActions() const;
    bool triggerPowerAction(PowerActionId id, std::string* outCommand) const;

    size_t size() const { return items_.size(); }

private:
    std::vector<AppItem> items_;
    std::vector<AppItem> currentResults_;
    int selectedIndex_;
    bool wasActivated_;
    bool wasDismissed_;
};

} // namespace launcher
} // namespace flamewm

#endif // FLAMEWM_LAUNCHER_APPMODEL_H
