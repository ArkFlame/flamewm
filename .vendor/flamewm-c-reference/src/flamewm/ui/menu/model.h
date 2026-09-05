#ifndef FLAMEWM_UI_MENU_MODEL_H
#define FLAMEWM_UI_MENU_MODEL_H

#include "../iconroles.h"

#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

namespace flamewm {
namespace ui {
namespace menu {

typedef std::uint64_t MenuItemId;

class MenuItem {
public:
    MenuItem();
    MenuItem(MenuItemId id, const std::string& label, flamewm::IconRole icon,
             bool enabled = true, bool checked = false);

    MenuItemId id() const;
    const std::string& label() const;
    flamewm::IconRole icon() const;
    bool enabled() const;
    bool checked() const;

private:
    MenuItemId id_;
    std::string label_;
    flamewm::IconRole icon_;
    bool enabled_;
    bool checked_;
};

class MenuSeparator {
public:
    MenuSeparator();
    explicit MenuSeparator(MenuItemId id);

    MenuItemId id() const;

private:
    MenuItemId id_;
};

class MenuEntry {
public:
    enum Kind { Item, Separator };

    explicit MenuEntry(const MenuItem& item);
    explicit MenuEntry(const MenuSeparator& separator);

    Kind kind() const;
    bool isItem() const;
    bool isSeparator() const;
    const MenuItem& item() const;
    const MenuSeparator& separator() const;
    MenuItemId id() const;

private:
    Kind kind_;
    MenuItem item_;
    MenuSeparator separator_;
};

class MenuModel {
public:
    bool addItem(const MenuItem& item);
    bool addSeparator(const MenuSeparator& separator);
    bool contains(MenuItemId id) const;
    std::size_t size() const;
    const MenuEntry& at(std::size_t index) const;
    const std::vector<MenuEntry>& entries() const;

private:
    std::vector<MenuEntry> entries_;
};

} // namespace menu
} // namespace ui
} // namespace flamewm

#endif
