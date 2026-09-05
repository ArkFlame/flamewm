#include "view.h"
#include "fileactions.h"
#include "ypaint.h"
#include "yicon.h"
#include "render/material_x11.h"
#include <X11/Xlib.h>
#include <fstream>
#include <algorithm>

namespace {
std::string desktopIconName(const flamewm::desktop::DesktopItem& item) {
    if (item.kind == flamewm::desktop::KindDirectory) return "folder";
    if (item.kind == flamewm::desktop::KindSymlink) return "emblem-symbolic-link";
    if (item.kind == flamewm::desktop::KindTrashPseudoItem) return "user-trash";
    if (item.kind == flamewm::desktop::KindDesktopLauncher) {
        std::ifstream file(item.path.c_str());
        std::string line;
        while (std::getline(file, line)) {
            if (line.compare(0, 5, "Icon=") == 0 && line.size() > 5)
                return line.substr(5);
        }
    }
    return "text-x-generic";
}

ref<YIcon> desktopIcon(const flamewm::desktop::DesktopItem& item) {
    ref<YIcon> icon = YIcon::getIcon(desktopIconName(item).c_str());
    if (icon != null && icon->small() != null) return icon;
    // Keep desktop entries visible when a selected theme lacks a semantic icon.
    icon = YIcon::getIcon("text-x-generic");
    if (icon != null && icon->small() != null) return icon;
    icon = YIcon::getIcon("file");
    return icon;
}
}

namespace flamewm {
namespace desktop {

DesktopView::DesktopView(): workArea_(0,0,1,1), scalePct_(100), model_(0),
    selectionFillOpacity_(20), selectionBorderOpacity_(60){ recompute(); }
void DesktopView::recompute(){ grid_ = LayoutEngine::compute(workArea_, scalePct_); }
ItemRect DesktopView::rectForCell(int col,int row) const { return itemRectForCell(grid_, col, row); }
std::map<std::string, ItemRect> DesktopView::buildItemRects() const {
    std::map<std::string, ItemRect> out;
    if(!model_) return out;
    const std::vector<DesktopItem>& items = model_->items();
    for(size_t i=0;i<items.size();++i){
        const DesktopItem& it=items[i];
        if(it.cellCol<0||it.cellRow<0) continue;
        out[it.path]=itemRectForCell(grid_, it.cellCol, it.cellRow);
    }
    return out;
}
void DesktopView::render(void* d,unsigned long w,void* g) const {
    Display* display=(Display*)d; GC gc=(GC)g;
    if(!model_) return;
    XWindowAttributes attributes;
    if (!XGetWindowAttributes(display, (Window)w, &attributes)) return;
    Graphics graphics((Drawable)w, attributes.width, attributes.height,
                      attributes.depth);
    std::map<std::string,ItemRect> rects=buildItemRects();
    const std::vector<DesktopItem>& items=model_->items();
    for(size_t i=0;i<items.size();++i){
        std::map<std::string,ItemRect>::const_iterator r=rects.find(items[i].path);
        if(r==rects.end()) continue;
        ItemRect q=r->second;
        const int visualInset = 4;
        const int visualWidth = std::max(1, q.w - visualInset * 2);
        const int visualHeight = std::max(1, q.h - visualInset * 2 - 16);
        if (items[i].selected) {
            const unsigned long fill = MaterialX11::blend(0xff5533UL, 0x111111UL,
                                                            selectionFillOpacity_);
            MaterialX11::fill(display, w, gc, q.x + visualInset, q.y + visualInset,
                              static_cast<unsigned>(visualWidth),
                              static_cast<unsigned>(visualHeight), fill);
            XSetForeground(display, gc,
                           MaterialX11::blend(0xff5533UL, 0xe6e6e6UL,
                                               selectionBorderOpacity_));
        } else {
            XSetForeground(display,gc,0xffe6e6e6UL);
        }
        XDrawRectangle(display,w,gc,q.x+visualInset,q.y+visualInset,
                       visualWidth - 1, visualHeight - 1);
        ref<YIcon> icon = desktopIcon(items[i]);
        if (icon != null) {
            const int size = static_cast<int>(YIcon::smallSize());
            if (!icon->draw(graphics, q.x + (q.w - size) / 2, q.y + 10, size))
                XDrawString(display,w,gc,q.x+(q.w-12)/2,q.y+10+size/2,"[F]",3);
        } else {
            XSetForeground(display, gc, 0xffe6e6e6UL);
            XDrawString(display,w,gc,q.x+(q.w-12)/2,q.y+26,"[F]",3);
        }
        XSetForeground(display,gc,0xffe6e6e6UL);
        XDrawString(display,w,gc,q.x+4,q.y+q.h-10,items[i].name.c_str(),(int)items[i].name.size());
    }
}
std::vector<std::string> DesktopView::blankContextMenuItems(bool stickyEnabled){
    std::vector<std::string> v;
    v.push_back("Open Terminal");
    v.push_back("Create New Folder");
    if(stickyEnabled) v.push_back("New Sticky Note");
    v.push_back("Desktop and Wallpaper");
    return v;
}
bool DesktopView::executeBlankContextAction(const std::string& action,const std::string& desktopDir,std::string* error){
    if(action=="Open Terminal") return FileActions::openTerminal(desktopDir,error);
    if(action=="Create New Folder") return !FileActions::createNewFolder(desktopDir,error).empty();
    if(action=="Desktop and Wallpaper") return FileActions::openDesktopAndWallpaper(error);
    if(error) *error="unsupported desktop action";
    return false;
}

} // namespace desktop
} // namespace flamewm
