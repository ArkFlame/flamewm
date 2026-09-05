#ifndef FLAMEWM_DESKTOP_DESKTOPAPP_H
#define FLAMEWM_DESKTOP_DESKTOPAPP_H

#include "model.h"
#include "view.h"
#include "watcher.h"
#include "selection.h"
#include "stickynote.h"
#include "flamewm/control/client.h"
#include "flamewm/api/workspace.h"
#include <string>
#include <vector>
#include <set>

struct _XDisplay;
typedef struct _XDisplay Display;
typedef unsigned long Window;

namespace flamewm {
namespace desktop {

// App lifecycle owning model/view/watcher/selection, bounded restart backoff.
// WM truth via Control: snapshots from ControlClient (revision-guarded intents),
// subscriptions delivered externally; no wmmgr/wmframe/wmtaskbar dependency.
class DesktopApp : public DesktopWatcher::Listener {
public:
    DesktopApp();
    ~DesktopApp();

    bool init(std::string* error);
    void shutdown();
    bool isRunning() const { return running_; }

    // Bounded restart backoff: max attempts, exponential
    struct BackoffState {
        int attempts;
        int maxAttempts;
        int baseMs;
        int maxMs;
        BackoffState(): attempts(0), maxAttempts(5), baseMs(500), maxMs(8000) {}
        int nextDelayMs();
        bool shouldGiveUp() const { return attempts >= maxAttempts; }
        void reset(){ attempts=0; }
    };

    // DesktopWatcher::Listener
    void onWatcherEvents(const std::vector<DesktopWatcher::Event>& ev) override;
    void onOverflowNeedsRescan() override;
    void onWatchInvalidated() override;

    DesktopModel& model(){ return model_; }
    DesktopView& view(){ return view_; }
    SelectionModel& selection(){ return selection_; }
    StickyNoteStore& stickyNotes(){ return stickyNotes_; }
    bool createStickyNote(const std::string& output, int workspace, std::string* error);
    bool deleteStickyNote(const std::string& id, std::string* error);
    bool updateStickyNoteText(const std::string& id, const std::string& text, std::string* error);
    bool updateStickyNoteGeometry(const std::string& id, const StickyRect& rect, std::string* error);
    bool updateStickyNoteSettings(const std::string& id, const StickyColor& bg, const StickyColor& fg, int textSize, std::string* error);
    void requestDisableStickyNotes(){ stickyNotes_.requestDisable(); }
    void cancelDisableStickyNotes(){ stickyNotes_.cancelDisable(); }
    bool confirmDisableStickyNotes(std::string* error){ return stickyNotes_.confirmDisable(error); }
    // Workspace ownership: called when Control workspace snapshot changes; isolates note ownership without duplicating WM truth.
    void onWorkspaceSnapshotChanged(const flamewm::api::WorkspaceSnapshot& snap);
    BackoffState& backoff(){ return backoff_; }

    // Flame Control client for WM/product actions (workspaces, settings).
    // Non-owning; caller retains lifetime. May be null when WM unavailable (headless/test).
    void setControlClient(flamewm::control::ControlClient* c) { control_ = c; }
    flamewm::control::ControlClient* controlClient() const { return control_; }
    bool controlConnected() const;

    // WM intents via ControlClient (C-DESKTOP): workspace creation and settings route.
    // Return false with error on unavailable/disconnected; do not mutate WM state directly.
    bool requestAddWorkspace(std::string* error);
    bool requestActivateWorkspace(int index, std::string* error);
    bool openDesktopSettings(std::string* error);
    // Route blank desktop context actions through ControlClient where applicable.
    bool executeDesktopAction(const std::string& action, std::string* error);

    // X window creation: advertise _NET_WM_WINDOW_TYPE_DESKTOP before map, skip taskbar/pager/QuickSwitch
    // Returns true if window created (when X available), false otherwise (headless ok)
    bool createDesktopWindow(std::string* error);
    // Wait for and process X/inotify events. Returns false when no longer running.
    bool processEvents(int timeoutMs);

private:
    DesktopModel model_;
    DesktopView view_;
    SelectionModel selection_;
    StickyNoteStore stickyNotes_;
    DesktopWatcher* watcher_;
    bool running_;
    BackoffState backoff_;
    std::string layoutFile_;
    Display* display_;
    Window window_;
    Window menuWindow_;
    unsigned long gc_;
    unsigned long menuGc_;
    int menuX_, menuY_;
    std::vector<std::string> menuItems_;
    bool draggingSelection_;
    bool draggingNote_;
    bool resizingNote_;
    bool editingNote_;
    int dragX_, dragY_;
    StickyNote* activeNote_;
    std::string dragPath_;
    std::map<std::string, std::pair<int,int> > dragCells_;
    unsigned long lastClickTime_;
    std::string lastClickPath_;
    flamewm::control::ControlClient* control_;

    void handleMovePairing(const std::vector<DesktopWatcher::Event>& ev);
    void updateWorkArea();
    void redraw();
    void handleXEvent(const void* event);
    StickyNote* noteAt(int x,int y);
    void clampNote(StickyNote* note);
    void showBlankContextMenu(int x, int y);
    void hideBlankContextMenu();
    void drawBlankContextMenu();
};

} // namespace desktop
} // namespace flamewm

#endif
