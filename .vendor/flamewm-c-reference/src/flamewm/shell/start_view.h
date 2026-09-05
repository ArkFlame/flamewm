#ifndef FLAMEWM_SHELL_START_VIEW_H
#define FLAMEWM_SHELL_START_VIEW_H

#include "flamewm/api/applications.h"
#include "flamewm/api/geometry.h"
#include "flamewm/api/ids.h"
#include "flamewm/api/panels.h"
#include "flamewm/api/ports.h"
#include "flamewm/ui/list.h"
#include "flamewm/ui/popover.h"
#include "flamewm/ui/textfield.h"
#include "flamewm/ui/window.h"
#include "flamewm/ui/button.h"

namespace flamewm { namespace platform { namespace applications { class ApplicationService; } } }

#include <string>
#include <vector>

namespace flamewm {
namespace shell {

// StartView — application launcher surface.
// Constraints: one-open globally; search field lives inside Start and filters rows in-place.
// Source of truth for apps is ApplicationService cached snapshot (in-memory filter per keystroke, no fs scan).
// Launch goes through ApplicationService when available, otherwise ApplicationPort directly.
// Output target resolver: focused window output > pointer output > primary > first active.
class StartView {
public:
    StartView();
    ~StartView();

    StartView(const StartView&) = delete;
    StartView& operator=(const StartView&) = delete;

    // Dependencies. At least one of ApplicationService / ApplicationPort is required for launch;
    // cache comes from ApplicationService when set.
    void setApplicationService(platform::applications::ApplicationService* svc);
    void setApplicationPort(api::ApplicationPort* p);
    void setContainer(flamewm::ui::Window* c);
    // Empty keeps the compact icon-only Start button.
    void setStartButtonText(const std::string& text);
    // Bind this view to the output hosting its panel. This remains stable while closed.
    void setOutputId(const api::OutputId& output);

    // Optional UiBackend surfaces. When bound, text/list/activation callbacks are wired
    // to keep filtered rows and search state in sync.
    void setTextField(flamewm::ui::TextField* tf);
    void setList(flamewm::ui::List* list);
    void setPopover(flamewm::ui::Popover* popover);

    flamewm::ui::Button* startButton() const { return startButton_; }
    flamewm::ui::TextField* textField() const { return searchField_; }
    flamewm::ui::List* list() const { return appList_; }
    flamewm::ui::Popover* popover() const { return popover_; }

    bool isOpen() const;
    void open(const api::OutputId& onOutput);
    void close();
    void toggle(const api::OutputId& onOutput);

    api::OutputId output() const;

    // Popover anchoring: anchor rect in screen coords (physical), typically Start button rect.
    void setAnchorRect(const api::Rect& r);
    api::Rect anchorRect() const;
    // Supply output rect + edge so anchored popover can be positioned without leaking panel math here.
    void setOutputRect(const api::Rect& r);
    api::Rect outputRect() const;
    void setPanelEdge(api::PanelEdge edge);
    api::PanelEdge panelEdge() const;

    // Refresh cached snapshot from ApplicationService (no fs scan per keystroke — explicit rescan only here).
    void refresh();
    // Rebuild filtered rows from cache_ + query_ (in-memory).
    void rebuildFiltered();

    // Search inside Start — replaces normal rows with filtered results.
    void setQuery(const std::string& q);
    std::string query() const;
    void clearSearch();

    std::vector<api::DesktopApplication> filtered() const;
    // Category helpers derived from cached snapshot (unique, sorted). "All" is not included — caller may prepend.
    std::vector<std::string> categories() const;
    std::vector<api::DesktopApplication> filteredForCategory(const std::string& category) const;

    // Output resolver: focused > pointer > primary > first active. All ids are OutputId.key comparisons.
    static api::OutputId resolveTargetOutput(const api::OutputId& focused,
                                             const api::OutputId& pointer,
                                             const api::OutputId& primary,
                                             const std::vector<api::OutputId>& activeOutputs);
    static api::OutputId resolveTargetOutput(const api::OutputId& focused,
                                             const api::OutputId& pointer,
                                             const api::OutputId& primary,
                                             const std::vector<api::Rect>& activeGeoms,
                                             const std::vector<api::OutputId>& activeIds);

    api::Status launch(const api::DesktopAppId& app);
    api::Status launchWithArgs(const api::DesktopAppId& app, const std::vector<std::string>& args);
    void onSelect(const api::DesktopAppId& app);

    // Explicit UI handlers — also wired as callbacks when TextField/List are bound.
    void handleTextChanged(const std::string& text);
    void handleSubmit(const std::string& text);
    void handleListActivated(int index);

    // Popover helpers.
    void showPopover();
    void hidePopover();
    // Compute anchored rect for a popover of given size using Popovers helper (clamped).
    api::Rect anchoredPopoverRect(const api::Size& popoverSize, int panelThickness, int gapPx) const;

    // For tests: force list row sync.
    void syncListRows();

private:
    void rebuildCache();
    void updateListRows();
    void bindTextFieldCallbacks();
    void bindListCallbacks();
    void ensureControls();
    void updateAnchorFromButton();
    static std::string toLower(const std::string& s);
    static std::string trim(const std::string& s);
    static bool equalsIgnoreCase(const std::string& a, const std::string& b);
    bool matchesQuery(const api::DesktopApplication& app, const std::string& qLower) const;

    platform::applications::ApplicationService* appService_;
    api::ApplicationPort* appPort_;
    flamewm::ui::Window* container_;
    flamewm::ui::Button* startButton_;
    flamewm::ui::TextField* searchField_;
    flamewm::ui::List* appList_;
    flamewm::ui::Popover* popover_;
    bool ownsStartButton_;
    bool ownsSearchField_;
    bool ownsAppList_;
    bool ownsPopover_;

    api::Rect anchorRect_;
    bool anchorExplicit_;
    api::Rect outputRect_;
    api::PanelEdge panelEdge_;
    std::string startButtonText_;

    bool open_;
    api::OutputId boundOutput_;
    api::OutputId output_;
    std::string query_;

    std::vector<api::DesktopApplication> cache_;
    mutable std::vector<api::DesktopApplication> filteredCache_;
    mutable bool filteredDirty_;

    // One Start globally — singleton state (mirrors panel::StartController invariant).
    static bool s_globalOpen;
    static api::OutputId s_globalOutput;
    static StartView* s_globalOwner;
};

} // namespace shell
} // namespace flamewm

#endif // FLAMEWM_SHELL_START_VIEW_H
