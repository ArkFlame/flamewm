#ifndef FLAMEWM_UI_LIST_H
#define FLAMEWM_UI_LIST_H

#include "flamewm/ui/window.h"

#include <functional>
#include <string>
#include <vector>

namespace flamewm {
namespace ui {

struct ListRow {
    std::string id;
    std::string label;
    std::string iconName;
    ListRow() {}
    ListRow(const std::string& i, const std::string& l, const std::string& ic = std::string())
        : id(i), label(l), iconName(ic) {}
};

class List : public Window {
public:
    virtual void setRows(const std::vector<ListRow>& rows) = 0;
    virtual std::vector<ListRow> rows() const = 0;
    virtual void setSelected(int index) = 0;
    virtual int selected() const = 0;
    virtual std::string selectedId() const = 0;
    virtual void setOnSelectionChanged(std::function<void(int)> cb) = 0;
    virtual void setOnActivated(std::function<void(int)> cb) = 0;
    virtual ~List() {}
};

} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_LIST_H
