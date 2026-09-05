#ifndef FLAMEWM_DESKTOP_VIEW_H
#define FLAMEWM_DESKTOP_VIEW_H

#include "model.h"
#include "layout.h"
#include "selection.h"
#include "item.h"
#include <string>
#include <map>
#include <vector>

namespace flamewm {
namespace desktop {

// View: grid recomputed from work area+scale, taskbar edge/scale/resolution/hotplug bounded reflow,
// items never under panel/outside work area, persist logical cells.
class DesktopView {
public:
    DesktopView();

    void setWorkArea(const WorkArea& wa){ workArea_=wa; recompute(); }
    void setScalePct(int pct){ scalePct_=pct; recompute(); }
    void setModel(DesktopModel* m){ model_=m; recompute(); }
    void setSelectionOpacity(int fill, int border){
        selectionFillOpacity_=fill < 0 ? 0 : (fill > 100 ? 100 : fill);
        selectionBorderOpacity_=border < 0 ? 0 : (border > 100 ? 100 : border);
    }

    const GridConfig& grid() const { return grid_; }
    const WorkArea& workArea() const { return workArea_; }

    // Pixel rect for item at cell
    ItemRect rectForCell(int col,int row) const;
    // Build map path->rect for current model items
    std::map<std::string, ItemRect> buildItemRects() const;
    void render(void* display, unsigned long window, void* gc) const;

    // Blank desktop context menu exact items
    static std::vector<std::string> blankContextMenuItems(bool stickyEnabled = true);
    static bool executeBlankContextAction(const std::string& action,
                                          const std::string& desktopDir,
                                          std::string* error);

private:
    void recompute();
    WorkArea workArea_;
    int scalePct_;
    GridConfig grid_;
    DesktopModel* model_;
    int selectionFillOpacity_;
    int selectionBorderOpacity_;
};

} // namespace desktop
} // namespace flamewm

#endif
