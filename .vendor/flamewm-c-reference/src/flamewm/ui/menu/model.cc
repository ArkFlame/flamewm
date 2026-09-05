#include "model.h"

namespace flamewm {
namespace ui {
namespace menu {

MenuItem::MenuItem()
    : id_(0), icon_(flamewm::IconRoleMenu), enabled_(true), checked_(false) {}

MenuItem::MenuItem(MenuItemId id, const std::string& label,
                   flamewm::IconRole icon, bool enabled, bool checked)
    : id_(id), label_(label), icon_(icon), enabled_(enabled), checked_(checked) {}

MenuItemId MenuItem::id() const { return id_; }
const std::string& MenuItem::label() const { return label_; }
flamewm::IconRole MenuItem::icon() const { return icon_; }
bool MenuItem::enabled() const { return enabled_; }
bool MenuItem::checked() const { return checked_; }

MenuSeparator::MenuSeparator() : id_(0) {}
MenuSeparator::MenuSeparator(MenuItemId id) : id_(id) {}
MenuItemId MenuSeparator::id() const { return id_; }

MenuEntry::MenuEntry(const MenuItem& item)
    : kind_(Item), item_(item), separator_() {}

MenuEntry::MenuEntry(const MenuSeparator& separator)
    : kind_(Separator), item_(), separator_(separator) {}

MenuEntry::Kind MenuEntry::kind() const { return kind_; }
bool MenuEntry::isItem() const { return kind_ == Item; }
bool MenuEntry::isSeparator() const { return kind_ == Separator; }
const MenuItem& MenuEntry::item() const { return item_; }
const MenuSeparator& MenuEntry::separator() const { return separator_; }
MenuItemId MenuEntry::id() const { return isItem() ? item_.id() : separator_.id(); }

bool MenuModel::addItem(const MenuItem& item) {
    if (item.id() == 0 || contains(item.id())) return false;
    entries_.push_back(MenuEntry(item));
    return true;
}

bool MenuModel::addSeparator(const MenuSeparator& separator) {
    if (separator.id() == 0 || contains(separator.id())) return false;
    entries_.push_back(MenuEntry(separator));
    return true;
}

bool MenuModel::contains(MenuItemId id) const {
    for (std::vector<MenuEntry>::const_iterator it = entries_.begin();
         it != entries_.end(); ++it) {
        if (it->id() == id) return true;
    }
    return false;
}

std::size_t MenuModel::size() const { return entries_.size(); }
const MenuEntry& MenuModel::at(std::size_t index) const { return entries_.at(index); }
const std::vector<MenuEntry>& MenuModel::entries() const { return entries_; }

} // namespace menu
} // namespace ui
} // namespace flamewm
