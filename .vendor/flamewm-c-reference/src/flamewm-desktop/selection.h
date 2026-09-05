#ifndef FLAMEWM_DESKTOP_SELECTION_H
#define FLAMEWM_DESKTOP_SELECTION_H

#include <string>
#include <set>
#include <vector>
#include <map>
#include "item.h"

namespace flamewm {
namespace desktop {

// Selection + rubber-band rectangle. Authoritative selected set is item identity.
class SelectionModel {
public:
    SelectionModel(): opacity_(20), hasRubber_(false) {}

    void setOpacity(int v){ if(v<0)v=0; if(v>60)v=60; opacity_=v; }
    int opacity() const { return opacity_; }

    void select(const std::string& path){ selected_.insert(path); }
    void deselect(const std::string& path){ selected_.erase(path); }
    void clear(){ selected_.clear(); }
    void setSelected(const std::set<std::string>& s){ selected_=s; }
    const std::set<std::string>& selected() const { return selected_; }
    bool isSelected(const std::string& p) const { return selected_.find(p)!=selected_.end(); }

    // Rubber band: blank-desktop drag creates rectangle (threshold handled by caller)
    void setRubberRect(const ItemRect& r){ rubber_=r; hasRubber_=true; }
    void clearRubber(){ hasRubber_=false; }
    bool hasRubber() const { return hasRubber_; }
    const ItemRect& rubber() const { return rubber_; }

    // Intersection exact: select hit rects intersecting rubber
    // Returns new selected set (identity). Caller must supply item rects.
    std::set<std::string> hitTest(const std::map<std::string, ItemRect>& itemRects) const;

    // Colors: strong border always visible; fill uses opacity 0..60
    // Returns fill alpha 0..153 (60% of 255)
    static int fillAlphaForOpacity(int opacity) { return (opacity * 255 + 50)/100; }
    // Border is always opaque accent; fill at 0% still shows border

private:
    std::set<std::string> selected_;
    int opacity_; // 0..60
    ItemRect rubber_;
    bool hasRubber_;
};

} // namespace desktop
} // namespace flamewm

#endif
