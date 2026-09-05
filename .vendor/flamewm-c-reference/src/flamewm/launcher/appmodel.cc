#include "appmodel.h"

namespace flamewm {
namespace launcher {

AppModel::AppModel() : selectedIndex_(-1), wasActivated_(false), wasDismissed_(false) {}

void AppModel::setEntries(const std::vector<AppItem>& items) {
    items_ = items;
    currentResults_ = items_;
    selectedIndex_ = currentResults_.empty() ? -1 : 0;
}

void AppModel::addEntry(const AppItem& item) { items_.push_back(item); }

void AppModel::clear() {
    items_.clear();
    currentResults_.clear();
    selectedIndex_ = -1;
}

std::vector<AppCategory> AppModel::categories() const {
    std::vector<AppCategory> cats;
    cats.reserve(CatCount);
    cats.push_back(AppCategory{CatAll, "All", "application-x-executable"});
    cats.push_back(AppCategory{CatAccessories, "Accessories", "applications-accessories"});
    cats.push_back(AppCategory{CatDevelopment, "Development", "applications-development"});
    cats.push_back(AppCategory{CatGraphics, "Graphics", "applications-graphics"});
    cats.push_back(AppCategory{CatMultimedia, "Multimedia", "applications-multimedia"});
    cats.push_back(AppCategory{CatNetwork, "Network", "applications-internet"});
    cats.push_back(AppCategory{CatOffice, "Office", "applications-office"});
    cats.push_back(AppCategory{CatSystem, "System", "applications-system"});
    cats.push_back(AppCategory{CatPowerSession, "Power / Session", "system-shutdown"});
    return cats;
}

std::vector<AppItem> AppModel::itemsForCategory(CategoryId cat) const {
    if (cat == CatAll) return items_;
    if (cat == CatPowerSession) {
        std::vector<AppItem> out;
        for (size_t i = 0; i < items_.size(); ++i) if (items_[i].isPowerAction) out.push_back(items_[i]);
        return out;
    }
    std::vector<AppItem> out;
    for (size_t i = 0; i < items_.size(); ++i) if (items_[i].category == cat) out.push_back(items_[i]);
    return out;
}

std::vector<AppItem> AppModel::search(const std::string& query) const {
    if (query.empty()) return items_;
    std::vector<AppItem> out;
    out.reserve(items_.size());
    for (size_t i = 0; i < items_.size(); ++i) {
        if (SearchHelper::matches(items_[i].entry, query)) out.push_back(items_[i]);
    }
    return out;
}

void AppModel::setCurrentResults(const std::vector<AppItem>& results) {
    currentResults_ = results;
    selectedIndex_ = currentResults_.empty() ? -1 : 0;
    wasActivated_ = false;
    wasDismissed_ = false;
}

bool AppModel::moveSelection(int delta) {
    if (currentResults_.empty()) return false;
    int n = (int)currentResults_.size();
    int ni = selectedIndex_ + delta;
    if (ni < 0) ni = 0;
    if (ni >= n) ni = n - 1;
    if (ni == selectedIndex_) return false;
    selectedIndex_ = ni;
    return true;
}

void AppModel::resetSelection() {
    selectedIndex_ = currentResults_.empty() ? -1 : 0;
}

const AppItem* AppModel::selectedItem() const {
    if (selectedIndex_ < 0 || selectedIndex_ >= (int)currentResults_.size()) return 0;
    return &currentResults_[selectedIndex_];
}

bool AppModel::handleKey(const std::string& key) {
    if (key == "Up") { moveSelection(-1); return true; }
    if (key == "Down") { moveSelection(1); return true; }
    if (key == "Enter") { wasActivated_ = (selectedItem()!=0); return true; }
    if (key == "Escape") { wasDismissed_ = true; return true; }
    return false;
}

std::vector<PowerAction> AppModel::powerActions() const {
    std::vector<PowerAction> v;
    v.push_back(PowerAction{PowerLock, "Lock", "system-lock-screen"});
    v.push_back(PowerAction{PowerLogout, "Logout", "system-log-out"});
    v.push_back(PowerAction{PowerReboot, "Reboot", "system-reboot"});
    v.push_back(PowerAction{PowerShutdown, "Shutdown", "system-shutdown"});
    v.push_back(PowerAction{PowerSuspend, "Suspend", "system-suspend"});
    return v;
}

bool AppModel::triggerPowerAction(PowerActionId id, std::string* outCommand) const {
    if (!outCommand) return false;
    switch (id) {
        case PowerLock:     *outCommand = "icewm --lock"; break; // placeholder: IceWM lock backend
        case PowerLogout:   *outCommand = "icewm --logout"; break;
        case PowerReboot:   *outCommand = "reboot"; break;
        case PowerShutdown: *outCommand = "shutdown -h now"; break;
        case PowerSuspend:  *outCommand = "systemctl suspend"; break;
        default: return false;
    }
    return true;
}

} // namespace launcher
} // namespace flamewm
