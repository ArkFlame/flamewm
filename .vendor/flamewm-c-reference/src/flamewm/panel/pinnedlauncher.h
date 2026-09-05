#ifndef FLAMEWM_PANEL_PINNEDLAUNCHER_H
#define FLAMEWM_PANEL_PINNEDLAUNCHER_H

#include <string>

namespace flamewm { namespace panel {
struct PinnedLauncherEntry {
    std::string appId;
    std::string label;
    bool active;
    PinnedLauncherEntry() : active(false) {}
    PinnedLauncherEntry(const std::string& id, const std::string& text)
        : appId(id), label(text), active(false) {}
    bool isInactive() const { return !active; }
};
} }

class TaskPane;
class TaskButton;

namespace flamewm { namespace panel {
TaskButton* createPinnedLauncherButton(TaskPane* pane, const std::string& appId, const std::string& label);
void destroyPinnedLauncherButton(TaskButton* btn);
} }

#endif // FLAMEWM_PANEL_PINNEDLAUNCHER_H

// Native TaskButton subclass — separate guard so it is visible even when this
// header was first included without ATASKS_H_ (e.g. taskstrip.h -> pinnedlauncher.h
// before atasks.h) and later re-included after ATASKS_H_ becomes defined.
#ifndef FLAMEWM_PANEL_PINNEDLAUNCHER_NATIVE_H
#if defined(ATASKS_H_) && !defined(TASKSTRIP_PURE_FALLBACK)
#define FLAMEWM_PANEL_PINNEDLAUNCHER_NATIVE_H
class TaskBarApp;
class Graphics;
class YRect;
class YRect2;
class YIcon;
template <typename T> class ref;
class YTimer;
namespace flamewm { namespace panel {
class PinnedLauncherButton : public TaskButton {
public:
    PinnedLauncherButton(TaskPane* pane, const std::string& appId, const std::string& label);
    virtual ~PinnedLauncherButton();
    const std::string& launcherAppId() const { return fAppId; }
    const std::string& launcherLabel() const { return fLabel; }
    void setLauncherOrder(int order) { fOrder = order; }
    void setLauncherLabel(const std::string& label) { fLabel = label; updateToolTip(); repaint(); }
    void launch() const;
    virtual bool getShown() override;
    virtual int getOrder() const override;
    virtual int estimate() override;
    virtual void paint(Graphics& g, const YRect& r) override;
    virtual void handleButton(const XButtonEvent& button) override;
    virtual void handleClick(const XButtonEvent& up, int count) override;
    virtual void handleDNDEnter() override;
    virtual void handleDNDLeave() override;
    virtual void updateToolTip() override;
    virtual bool handleBeginDrag(const XButtonEvent& down, const XMotionEvent& motion) override;
    virtual void handleCrossing(const XCrossingEvent& crossing) override;
    virtual void handleExpose(const XExposeEvent& expose) override;
    virtual bool isFocusTraversable() override;
    virtual void configure(const YRect2& r) override;
    virtual void repaint() override;
    virtual void repaintApp(TaskBarApp* app) override;
    virtual bool handleTimer(YTimer* t) override;
private:
    std::string fAppId;
    std::string fLabel;
    int fOrder;
    int fSelected;
    ref<YIcon> resolveIcon() const;
};
} }
#endif
#endif
