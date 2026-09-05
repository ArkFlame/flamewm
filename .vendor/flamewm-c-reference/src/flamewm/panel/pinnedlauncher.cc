// X-available build pulls full TaskPane authority; pure-model build stays header-only.
#if !defined(TASKSTRIP_PURE_FALLBACK)
#if defined(__has_include)
#  if __has_include(<X11/extensions/Xrender.h>)
#    if __has_include("config.h")
#      include "config.h"
#    endif
#    if __has_include("atasks.h")
#      include "atasks.h"
#    endif
#  endif
#endif
#endif
#include "pinnedlauncher.h"

#if defined(ATASKS_H_) && !defined(TASKSTRIP_PURE_FALLBACK)

#include "yicon.h"
#include "ypaint.h"
#include "ywindow.h"
#include "ypopup.h"
#include "ymenu.h"
#include "ymenuitem.h"
#include "wmapp.h"
#include "base.h"
#include "yprefs.h"
#include "default.h"
#include "yconfig.h"
#include "yfontname.h"
#include "wpixmaps.h"
#include "yimage.h"

namespace flamewm { namespace panel {

static YColorName launcherNormalBg(&clrNormalTaskBarApp);
static YColorName launcherNormalFg(&clrNormalTaskBarAppText);

PinnedLauncherButton::PinnedLauncherButton(TaskPane* pane, const std::string& appId, const std::string& label)
    : TaskButton(pane), fAppId(appId), fLabel(label), fOrder(0), fSelected(0)
{
}

PinnedLauncherButton::~PinnedLauncherButton() {
    if (taskPane() && taskPane()->dragging() == this)
        taskPane()->cancelDrag();
}

bool PinnedLauncherButton::getShown() { return true; }
int PinnedLauncherButton::getOrder() const { return fOrder; }

int PinnedLauncherButton::estimate() {
    int p = 0;
    if (taskbuttonLeftPixbuf != null && taskbuttonRightPixbuf != null) {
        p += taskbuttonLeftPixbuf->width();
        p += taskbuttonRightPixbuf->width();
    } else if (taskbuttonLeftPixmap != null && taskbuttonRightPixmap != null) {
        p += taskbuttonLeftPixmap->width();
        p += taskbuttonRightPixmap->width();
    }
    if (taskBarShowWindowIcons) p += YIcon::smallSize();
    mstring str;
    if (taskBarShowWindowTitles) {
        if (!fLabel.empty()) str = fLabel.c_str();
        else if (!fAppId.empty()) str = fAppId.c_str();
        else str = null;
    } else str = null;
    if (str != null) {
        extern ::YFontName normalTaskBarFontName;
        YFont font; font = ::normalTaskBarFontName;
        if (font != null) { if (taskBarShowWindowIcons) p += 2; p += font->textWidth(str); }
        else { if (taskBarShowWindowIcons) p += 2; p += (int)str.length() * 6; }
    }
    return p + 4;
}

void PinnedLauncherButton::paint(Graphics& g, const YRect& r) {
    (void)r;
    YColor bg = launcherNormalBg;
    YColor fg = launcherNormalFg;
    ref<YPixmap> bgPix = taskbuttonPixmap;
    ref<YPixmap> bgLeft = taskbuttonLeftPixmap;
    ref<YPixmap> bgRight = taskbuttonRightPixmap;
    ref<YImage> bgGrad = taskbuttonPixbuf;
    ref<YImage> bgLeftG = taskbuttonLeftPixbuf;
    ref<YImage> bgRightG = taskbuttonRightPixbuf;
    int p = 0; int left = 0; int style = (fSelected == 3 ? 3 : fSelected == 2 ? 2 : 1);
    if (style == 3) {
        p = 2; g.setColor(YColor::black); g.drawRect(0, 0, width() - 1, height() - 1);
        g.setColor(bg); g.fillRect(1, 1, width() - 2, height() - 2);
    } else {
        g.setColor(bg);
        if (style == 2) {
            p = 2;
            if (wmLook == lookMetal) g.drawBorderM(0, 0, width() - 1, height() - 1, false);
            else if (wmLook == lookGtk) g.drawBorderG(0, 0, width() - 1, height() - 1, false);
            else if (wmLook != lookFlat) g.drawBorderW(0, 0, width() - 1, height() - 1, false);
        } else {
            p = 1;
            if (wmLook == lookMetal) { p = 2; g.drawBorderM(0, 0, width() - 1, height() - 1, true); }
            else if (wmLook == lookGtk) g.drawBorderG(0, 0, width() - 1, height() - 1, true);
            else if (wmLook != lookFlat) g.drawBorderW(0, 0, width() - 1, height() - 1, true);
        }
        int const dp(wmLook == lookFlat ? 0: wmLook == lookMetal ? 2 : p);
        int const ds(wmLook == lookFlat ? 0: wmLook == lookMetal ? 4 : 3);
        if ((int) width() > ds && (int) height() > ds) {
            if (bgGrad != null) {
                int x = dp; int y = dp; unsigned w = width() - ds; unsigned h = height() - ds;
                if (taskbuttonIconOffset && bgLeftG != null && bgRightG != null && 3 * (int)taskbuttonIconOffset <= (int)w) {
                    g.drawGradient(bgLeftG, x, y, bgLeftG->width(), h);
                    x += left = bgLeftG->width();
                    g.drawGradient(bgRightG, (int)w - (int)bgRightG->width(), y, bgRightG->width(), h);
                    w -= bgLeftG->width() + bgRightG->width();
                }
                g.drawGradient(bgGrad, x, y, w, h);
            } else if (bgPix != null) {
                int x = dp; int y = dp; unsigned w = width() - ds; unsigned h = height() - ds;
                if (taskbuttonIconOffset && bgLeft != null && bgRight != null && 3 * (int)taskbuttonIconOffset <= (int)w) {
                    g.fillPixmap(bgLeft, x, y, bgLeft->width(), h);
                    x += left = bgLeft->width();
                    g.fillPixmap(bgRight, w - bgRight->width(), y, bgRight->width(), h);
                    w -= bgLeft->width() + bgRight->width();
                }
                g.fillPixmap(bgPix, x, y, w, h);
            } else g.fillRect(dp, dp, width() - ds, height() - ds);
        }
    }
    ref<YIcon> icon; bool iconDrawn = false; int iconSize = YIcon::smallSize();
    if (taskBarShowWindowIcons) icon = resolveIcon();
    if (icon != null) {
        int const y = (height() - 3 - iconSize - (wmLook == lookMetal)) / 2;
        int ix = p + max(1, left); int iy = p + 1 + y;
        iconDrawn = icon->draw(g, ix, iy, iconSize);
        if (iconDrawn && p + max(1, left) + iconSize + 5 >= int(width())) { if (bgGrad != null) g.maxOpacity(); return; }
    }
    mstring str;
    if (taskBarShowWindowTitles) {
        if (!fLabel.empty()) str = fLabel.c_str();
        else if (!fAppId.empty()) str = fAppId.c_str();
        else str = null;
    } else str = null;
    if (str != null) {
        extern ::YFontName normalTaskBarFontName;
        YFont font; font = ::normalTaskBarFontName;
        if (font != null) {
            g.setColor(fg); g.setFont(font);
            int pad = max(1, left); int iconW = 0;
            if (iconDrawn) { iconW = YIcon::smallSize(); pad += 2; }
            int const tx = pad + iconW;
            int const ty = max(2U, (height() + font->height() - (LOOK(lookMetal | lookFlat) ? 2 : 1)) / 2 - font->descent());
            int const wm = int(width()) - p - pad - iconW - 2;
            if (0 < wm && p + tx + wm < int(width())) { g.setColor(fg); g.drawStringEllipsis(p + tx, p + ty, str, wm); }
        }
    }
    if (bgGrad != null) g.maxOpacity();
}

ref<YIcon> PinnedLauncherButton::resolveIcon() const {
    if (!fAppId.empty()) {
        std::string base = fAppId;
        if (base.size() > 8 && base.compare(base.size() - 8, 8, ".desktop") == 0) base = base.substr(0, base.size() - 8);
        ref<YIcon> ic = YIcon::getIcon(base.c_str()); if (ic != null) return ic;
        ic = YIcon::getIcon(fAppId.c_str()); if (ic != null) return ic;
    }
    if (!fLabel.empty()) { ref<YIcon> ic = YIcon::getIcon(fLabel.c_str()); if (ic != null) return ic; }
    return null;
}

void PinnedLauncherButton::launch() const {
    if (fAppId.empty() || wmapp == nullptr) return;
    std::string base = fAppId;
    if (base.size() > 8 && base.compare(base.size() - 8, 8, ".desktop") == 0) base = base.substr(0, base.size() - 8);
    const char* argv[] = { "gtk-launch", base.c_str(), nullptr };
    wmapp->runProgram(argv[0], argv);
}

void PinnedLauncherButton::handleButton(const XButtonEvent& button) {
    YWindow::handleButton(button);
    if (taskPane() && taskPane()->dragging()) return;
    if (button.type == ButtonPress) {
        if (button.button == Button1 || button.button == Button2) { fSelected = 2; repaint(); }
    } else if (button.type == ButtonRelease) {
        if (button.button == Button1 && fSelected == 2) launch();
        else if (button.button == Button2 && fSelected == 2) launch();
        if (button.button == Button1 || button.button == Button2) { fSelected = 0; repaint(); }
    }
}
void PinnedLauncherButton::handleClick(const XButtonEvent& up, int count) {
    if (up.button == Button3 && count == 1) return;
    if (up.button == Button4 && taskBarUseMouseWheel) { if (taskPane()) taskPane()->switchToPrev(); }
    else if (up.button == Button5 && taskBarUseMouseWheel) { if (taskPane()) taskPane()->switchToNext(); }
}
void PinnedLauncherButton::handleDNDEnter() { fSelected = 3; repaint(); }
void PinnedLauncherButton::handleDNDLeave() { fSelected = 0; repaint(); }
void PinnedLauncherButton::updateToolTip() {
    mstring tip; if (!fLabel.empty()) tip = fLabel.c_str(); else if (!fAppId.empty()) tip = fAppId.c_str(); else tip = null;
    YWindow::setToolTip(tip != null ? tip : null);
}
bool PinnedLauncherButton::handleBeginDrag(const XButtonEvent& down, const XMotionEvent& motion) {
    if (down.button == Button1 && taskPane()) { raise(); taskPane()->startDrag(this, 0, down.x + x(), down.y + y()); taskPane()->processDrag(motion.x + x(), motion.y + y()); return true; }
    return false;
}
void PinnedLauncherButton::handleCrossing(const XCrossingEvent& crossing) {
    if (fSelected > 0) {
        if (crossing.type == EnterNotify) { fSelected = 2; repaint(); }
        else if (crossing.type == LeaveNotify) { fSelected = 0; repaint(); }
    }
    YWindow::handleCrossing(crossing);
}
void PinnedLauncherButton::handleExpose(const XExposeEvent& expose) { if (expose.count == 0) repaint(); }
bool PinnedLauncherButton::isFocusTraversable() { return true; }
void PinnedLauncherButton::configure(const YRect2& r) { if (r.resized()) repaint(); }
void PinnedLauncherButton::repaint() { if (width() > 1 && height() > 1 && getShown()) GraphicsBuffer(this).paint(); }
void PinnedLauncherButton::repaintApp(TaskBarApp*) { repaint(); }
bool PinnedLauncherButton::handleTimer(YTimer*) { return false; }
TaskButton* createPinnedLauncherButton(TaskPane* pane, const std::string& appId, const std::string& label) {
    if (!pane || appId.empty()) return nullptr; return new PinnedLauncherButton(pane, appId, label);
}
void destroyPinnedLauncherButton(TaskButton* btn) { if (!btn) return; TaskPane* p = btn->taskPane(); if (p) p->remove(btn); else delete btn; }

} }
#else
namespace flamewm { namespace panel {
TaskButton* createPinnedLauncherButton(TaskPane*, const std::string&, const std::string&) { return nullptr; }
void destroyPinnedLauncherButton(TaskButton*) {}
} }
#endif
