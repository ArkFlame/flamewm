#include "flamewm/shell/start_view.h"

#include "flamewm/platform/applications/service.h"
#include "flamewm/shell/popovers.h"
#include "flamewm/ui/backend.h"
#include "flamewm/ui/assets/asset.h"
#include "flamewm/ui/shellstyle.h"

#include <algorithm>
#include <cctype>
#include <map>

namespace flamewm {
namespace shell {

bool StartView::s_globalOpen = false;
api::OutputId StartView::s_globalOutput = api::OutputId();
StartView* StartView::s_globalOwner = 0;

StartView::StartView()
    : appService_(0)
    , appPort_(0)
    , container_(0)
    , startButton_(0)
    , searchField_(0)
    , appList_(0)
    , popover_(0)
    , ownsStartButton_(false)
    , ownsSearchField_(false)
    , ownsAppList_(false)
    , ownsPopover_(false)
    , anchorRect_()
    , anchorExplicit_(false)
    , outputRect_()
    , panelEdge_(api::PanelEdge::Bottom)
    , startButtonText_()
    , open_(false)
    , boundOutput_()
    , output_()
    , query_()
    , cache_()
    , filteredCache_()
    , filteredDirty_(true)
{}

StartView::~StartView() {
    if (popover_) popover_->setOnClosed(std::function<void()>());
    close();
    if (searchField_) {
        searchField_->setOnChanged(std::function<void(const std::string&)>());
        searchField_->setOnSubmit(std::function<void(const std::string&)>());
    }
    if (appList_) {
        appList_->setOnSelectionChanged(std::function<void(int)>());
        appList_->setOnActivated(std::function<void(int)>());
    }
    if (ownsStartButton_) delete startButton_;
    if (ownsSearchField_) delete searchField_;
    if (ownsAppList_) delete appList_;
    if (ownsPopover_) delete popover_;
    if (s_globalOwner == this) {
        s_globalOwner = 0;
        s_globalOpen = false;
        s_globalOutput = api::OutputId();
    }
}

void StartView::setApplicationService(platform::applications::ApplicationService* svc) {
    appService_ = svc;
    filteredDirty_ = true;
    if (open_) rebuildCache();
}

void StartView::setApplicationPort(api::ApplicationPort* p) { appPort_ = p; }

void StartView::setOutputId(const api::OutputId& output) {
    boundOutput_ = output;
    if (!open_) output_ = output;
}

void StartView::setContainer(flamewm::ui::Window* c) {
    container_ = c;
    ensureControls();
}

void StartView::setStartButtonText(const std::string& text) {
    startButtonText_ = text;
    if (startButton_) startButton_->setText(startButtonText_);
}

void StartView::setTextField(flamewm::ui::TextField* tf) {
    if (searchField_) {
        searchField_->setOnChanged(std::function<void(const std::string&)>());
        searchField_->setOnSubmit(std::function<void(const std::string&)>());
    }
    if (ownsSearchField_) delete searchField_;
    searchField_ = tf;
    ownsSearchField_ = false;
    bindTextFieldCallbacks();
    if (searchField_ && !query_.empty()) searchField_->setText(query_);
}

void StartView::setList(flamewm::ui::List* list) {
    if (appList_) {
        appList_->setOnSelectionChanged(std::function<void(int)>());
        appList_->setOnActivated(std::function<void(int)>());
    }
    if (ownsAppList_) delete appList_;
    appList_ = list;
    ownsAppList_ = false;
    bindListCallbacks();
    syncListRows();
}

void StartView::setPopover(flamewm::ui::Popover* popover) {
    if (popover == popover_) return;
    if (popover_) popover_->setOnClosed(std::function<void()>());
    if (ownsPopover_) delete popover_;
    popover_ = popover;
    ownsPopover_ = false;
    if (popover_) {
        // Lifetime: when popover closes externally, reflect global closed.
        StartView* self = this;
        popover_->setOnClosed([self]() {
            // Guard re-entrancy: close() updates global state, hide is idempotent.
            if (self) self->close();
        });
    }
}

bool StartView::isOpen() const { return s_globalOpen && open_ && s_globalOutput == output_; }

void StartView::open(const api::OutputId& onOutput) {
    if (!onOutput.valid()) return;
    // One Start globally — switching output is close prior then open target.
    if (s_globalOpen && s_globalOutput == onOutput && open_) {
        // Already open on same target — no-op.
        output_ = onOutput;
        return;
    }
    if (s_globalOwner && s_globalOwner != this) s_globalOwner->close();
    s_globalOpen = true;
    s_globalOutput = onOutput;
    s_globalOwner = this;
    output_ = onOutput;
    open_ = true;
    rebuildCache();
    filteredDirty_ = true;
    rebuildFiltered();
    syncListRows();
    showPopover();
    if (searchField_) searchField_->focus();
}

void StartView::close() {
    if (!open_ && !s_globalOpen) return;
    bool wasGlobal = s_globalOpen && s_globalOutput == output_;
    open_ = false;
    if (wasGlobal) {
        s_globalOpen = false;
        s_globalOutput = api::OutputId();
        if (s_globalOwner == this) s_globalOwner = 0;
    }
    // State is cleared before close(), so an onClosed callback cannot re-enter work.
    if (popover_ && popover_->isVisible()) {
        popover_->close();
    }
}

void StartView::toggle(const api::OutputId& onOutput) {
    if (isOpen() && output_ == onOutput) close();
    else open(onOutput);
}

api::OutputId StartView::output() const { return output_; }

void StartView::setAnchorRect(const api::Rect& r) {
    anchorRect_ = r;
    anchorExplicit_ = r.valid();
    if (popover_) popover_->setAnchor(r);
}

api::Rect StartView::anchorRect() const { return anchorRect_; }

void StartView::setOutputRect(const api::Rect& r) {
    outputRect_ = r;
    updateAnchorFromButton();
}

api::Rect StartView::outputRect() const { return outputRect_; }

void StartView::setPanelEdge(api::PanelEdge edge) {
    panelEdge_ = edge;
    if (startButton_) {
        const ShellStyle style = ShellStyle::defaults();
        if (edge == api::PanelEdge::Left || edge == api::PanelEdge::Right)
            startButton_->setGeometry(api::Rect(0, 0, style.panelHeight, style.startButtonWidth));
        else
            startButton_->setGeometry(api::Rect(0, 0, style.startButtonWidth, style.panelHeight));
    }
    ensureControls();
    updateAnchorFromButton();
    if (open_) showPopover();
}

api::PanelEdge StartView::panelEdge() const { return panelEdge_; }

void StartView::refresh() {
    rebuildCache();
    filteredDirty_ = true;
    rebuildFiltered();
    syncListRows();
}

void StartView::rebuildFiltered() {
    if (!filteredDirty_) return;
    std::string qLower = toLower(trim(query_));
    filteredCache_.clear();
    if (qLower.empty()) {
        filteredCache_ = cache_;
    } else {
        filteredCache_.reserve(cache_.size());
        for (size_t i = 0; i < cache_.size(); ++i) {
            if (matchesQuery(cache_[i], qLower)) filteredCache_.push_back(cache_[i]);
        }
    }
    filteredDirty_ = false;
}

void StartView::setQuery(const std::string& q) {
    if (query_ == q) return;
    query_ = q;
    filteredDirty_ = true;
    rebuildFiltered();
    // Keep TextField in sync when query set programmatically, without triggering recursion.
    if (searchField_ && searchField_->text() != q) {
        // Temporarily detach to avoid onChanged firing rebuild again.
        searchField_->setOnChanged(std::function<void(const std::string&)>());
        searchField_->setText(q);
        bindTextFieldCallbacks();
    }
    syncListRows();
}

std::string StartView::query() const { return query_; }

void StartView::clearSearch() { setQuery(std::string()); }

std::vector<api::DesktopApplication> StartView::filtered() const {
    // const method: lazily rebuild if dirty (mutable cache).
    if (filteredDirty_) {
        // Need to mutate — safe because mutable and single-threaded X loop.
        const_cast<StartView*>(this)->rebuildFiltered();
    }
    return filteredCache_;
}

std::vector<std::string> StartView::categories() const {
    std::map<std::string, bool> seen;
    std::vector<std::string> out;
    for (size_t i = 0; i < cache_.size(); ++i) {
        const std::vector<std::string>& cats = cache_[i].categories;
        for (size_t j = 0; j < cats.size(); ++j) {
            std::string k = trim(cats[j]);
            if (k.empty()) continue;
            if (seen.find(k) == seen.end()) {
                seen[k] = true;
                out.push_back(k);
            }
        }
    }
    std::sort(out.begin(), out.end());
    return out;
}

std::vector<api::DesktopApplication> StartView::filteredForCategory(const std::string& category) const {
    std::vector<api::DesktopApplication> base = filtered();
    std::string cat = trim(category);
    if (cat.empty() || equalsIgnoreCase(cat, "All")) return base;
    std::vector<api::DesktopApplication> out;
    out.reserve(base.size());
    for (size_t i = 0; i < base.size(); ++i) {
        const std::vector<std::string>& cats = base[i].categories;
        for (size_t j = 0; j < cats.size(); ++j) {
            if (equalsIgnoreCase(trim(cats[j]), cat)) { out.push_back(base[i]); break; }
        }
    }
    return out;
}

api::OutputId StartView::resolveTargetOutput(const api::OutputId& focused,
                                             const api::OutputId& pointer,
                                             const api::OutputId& primary,
                                             const std::vector<api::OutputId>& activeOutputs) {
    // Order: focused > pointer > primary > first active. Only return if present in activeOutputs.
    // If activeOutputs empty, fall back to validity checks alone.
    if (activeOutputs.empty()) {
        if (focused.valid()) return focused;
        if (pointer.valid()) return pointer;
        if (primary.valid()) return primary;
        return api::OutputId();
    }
    for (size_t i = 0; i < activeOutputs.size(); ++i) if (activeOutputs[i] == focused && focused.valid()) return focused;
    for (size_t i = 0; i < activeOutputs.size(); ++i) if (activeOutputs[i] == pointer && pointer.valid()) return pointer;
    for (size_t i = 0; i < activeOutputs.size(); ++i) if (activeOutputs[i] == primary && primary.valid()) return primary;
    return activeOutputs[0];
}

api::OutputId StartView::resolveTargetOutput(const api::OutputId& focused,
                                             const api::OutputId& pointer,
                                             const api::OutputId& primary,
                                             const std::vector<api::Rect>& activeGeoms,
                                             const std::vector<api::OutputId>& activeIds) {
    (void)activeGeoms;
    return resolveTargetOutput(focused, pointer, primary, activeIds);
}

api::Status StartView::launch(const api::DesktopAppId& app) {
    std::vector<std::string> empty;
    return launchWithArgs(app, empty);
}

api::Status StartView::launchWithArgs(const api::DesktopAppId& app, const std::vector<std::string>& args) {
    if (app.empty()) return api::Status::make(api::Error::InvalidArgument, "empty app id");
    // Validate against cache when available.
    bool found = cache_.empty();
    if (!cache_.empty()) {
        found = false;
        for (size_t i = 0; i < cache_.size(); ++i) if (cache_[i].id == app) { found = true; break; }
        if (!found) return api::Status::make(api::Error::NotFound, "application not found in cache");
    }
    for (size_t i = 0; i < args.size(); ++i) {
        if (args[i].find('\0') != std::string::npos) return api::Status::make(api::Error::InvalidArgument, "arg contains NUL");
    }
    api::Status st = api::Status::make(api::Error::Unavailable, "no application backend");
    if (appService_) st = appService_->launch(app, args);
    else if (appPort_) st = appPort_->launch(app, args);
    // History: keep launch path argv-vector only — no shell interpolation.
    return st;
}

void StartView::onSelect(const api::DesktopAppId& app) {
    api::Status st = launch(app);
    (void)st;
    close();
}

void StartView::handleTextChanged(const std::string& text) { setQuery(text); }

void StartView::handleSubmit(const std::string& text) {
    // Update query first, then activate first filtered result (Enter semantics).
    if (query_ != text) {
        query_ = text;
        filteredDirty_ = true;
        rebuildFiltered();
        syncListRows();
    }
    std::vector<api::DesktopApplication> f = filtered();
    if (f.empty()) return;
    // Prefer an exact case-insensitive ID or display-name match; otherwise use the first result.
    api::DesktopAppId target = f[0].id;
    std::string tl = toLower(trim(text));
    for (size_t i = 0; i < f.size(); ++i) {
        if (toLower(f[i].displayName) == tl || toLower(f[i].id.value) == tl) {
            target = f[i].id;
            break;
        }
    }
    onSelect(target);
}

void StartView::handleListActivated(int index) {
    std::vector<api::DesktopApplication> f = filtered();
    if (index < 0 || index >= (int)f.size()) return;
    onSelect(f[(size_t)index].id);
}

void StartView::ensureControls() {
    if (!container_) return;
    const ShellStyle style = ShellStyle::defaults();
    if (!startButton_) {
        const flamewm::ui::AssetRef startAsset(flamewm::ui::AssetIdStart, IconRoleLauncher);
        startButton_ = flamewm::ui::UiBackend::createButton(container_, startAsset.iconRole());
        ownsStartButton_ = startButton_ != 0;
        if (startButton_) {
            startButton_->setVisual(flamewm::ui::style::Visual(
                flamewm::ui::style::VisualRoleNavigation,
                flamewm::ui::style::VisualNormal));
            startButton_->setText(startButtonText_);
            startButton_->setIconRole(IconRoleLauncher);
            startButton_->setOnClick([this]() {
                api::OutputId target = this->boundOutput_;
                if (!target.valid()) target = this->output_;
                if (!target.valid()) target = StartView::s_globalOutput;
                if (target.valid()) this->toggle(target);
            });
            if (panelEdge_ == api::PanelEdge::Left || panelEdge_ == api::PanelEdge::Right)
                startButton_->setGeometry(api::Rect(0, 0, style.panelHeight, style.startButtonWidth));
            else
                startButton_->setGeometry(api::Rect(0, 0, style.startButtonWidth, style.panelHeight));
            startButton_->show();
        }
    }
    if (!popover_) {
        popover_ = flamewm::ui::UiBackend::createPopover(container_);
        ownsPopover_ = popover_ != 0;
        if (popover_) {
            popover_->setVisual(flamewm::ui::style::Visual(
                flamewm::ui::style::VisualRoleMenu,
                flamewm::ui::style::VisualNormal));
            popover_->setRole(flamewm::ui::WindowRole::Popup);
            ownsPopover_ = false;
            setPopover(popover_);
            // setPopover deliberately marks injected widgets as non-owned; restore
            // ownership because this instance created the surface.
            ownsPopover_ = true;
            popover_->hide();
        }
    }
    if (popover_ && !searchField_) {
        searchField_ = flamewm::ui::UiBackend::createTextField(popover_);
        ownsSearchField_ = searchField_ != 0;
        if (searchField_) {
            searchField_->setPlaceholder("Search applications");
            bindTextFieldCallbacks();
            searchField_->hide();
        }
    }
    if (popover_ && !appList_) {
        appList_ = flamewm::ui::UiBackend::createList(popover_);
        ownsAppList_ = appList_ != 0;
        if (appList_) {
            bindListCallbacks();
            syncListRows();
            appList_->hide();
        }
    }
    updateAnchorFromButton();
    // V8 keeps the search field in a 45px strip at the bottom of Start.
    if (searchField_) searchField_->setGeometry(api::Rect(
        7, style.startMenuMinHeight - 45 + 6,
        style.startMenuWidth - 14, 32));
    if (appList_) appList_->setGeometry(api::Rect(
        5, 6,
        style.startMenuWidth - 10,
        style.startMenuMinHeight - 45 - 11));
}

void StartView::updateAnchorFromButton() {
    if (anchorExplicit_ || !startButton_ || !outputRect_.valid()) return;
    api::Rect b = startButton_->geometry();
    if (!b.valid()) return;
    anchorRect_ = api::Rect(outputRect_.x + b.x, outputRect_.y + b.y, b.w, b.h);
    if (panelEdge_ == api::PanelEdge::Bottom)
        anchorRect_.y = outputRect_.y + outputRect_.h - b.h;
    else if (panelEdge_ == api::PanelEdge::Right)
        anchorRect_.x = outputRect_.x + outputRect_.w - b.w;
    if (popover_) popover_->setAnchor(anchorRect_);
}

void StartView::showPopover() {
    ensureControls();
    if (!popover_) return;
    const ShellStyle style = ShellStyle::defaults();
    if (!anchorRect_.valid() && outputRect_.valid()) {
        // Derive anchor from panel edge + output when no explicit button rect yet.
        // Use small default anchor at panel edge center.
        int panelThickness = style.panelHeight;
        if (outputRect_.valid()) {
            if (panelEdge_ == api::PanelEdge::Bottom) anchorRect_ = api::Rect(outputRect_.x, outputRect_.y + outputRect_.h - panelThickness, style.startButtonWidth, panelThickness);
            else if (panelEdge_ == api::PanelEdge::Top) anchorRect_ = api::Rect(outputRect_.x, outputRect_.y, style.startButtonWidth, panelThickness);
            else if (panelEdge_ == api::PanelEdge::Left) anchorRect_ = api::Rect(outputRect_.x, outputRect_.y, panelThickness, style.startButtonWidth);
            else anchorRect_ = api::Rect(outputRect_.x + outputRect_.w - panelThickness, outputRect_.y, panelThickness, style.startButtonWidth);
        }
    }
    popover_->setAnchor(anchorRect_);
    if (outputRect_.valid()) {
        popover_->setGeometry(anchoredPopoverRect(
            api::Size(style.startMenuWidth, style.startMenuMinHeight),
            style.panelHeight,
            style.popoverOffset));
    } else {
        popover_->setGeometry(api::Rect(anchorRect_.x, anchorRect_.y,
                                        style.startMenuWidth,
                                        style.startMenuMinHeight));
    }
    // showAt() applies an unclamped native position. Geometry must be committed
    // before showing so native and fallback backends use the same anchored rect.
    popover_->show();
    if (searchField_) searchField_->show();
    if (appList_) appList_->show();
}

void StartView::hidePopover() {
    if (popover_ && popover_->isVisible()) popover_->close();
    if (searchField_) searchField_->hide();
    if (appList_) appList_->hide();
}

api::Rect StartView::anchoredPopoverRect(const api::Size& popoverSize, int panelThickness, int gapPx) const {
    if (!outputRect_.valid() || !popoverSize.valid()) return api::Rect(anchorRect_.x, anchorRect_.y, popoverSize.w, popoverSize.h);
    PopoverAnchor a;
    a.anchorRect = anchorRect_;
    a.outputRect = outputRect_;
    a.edge = panelEdge_;
    a.panelThickness = panelThickness;
    a.gapPx = gapPx;
    return Popovers::anchoredRect(a, popoverSize);
}

void StartView::syncListRows() {
    if (!appList_) return;
    std::vector<api::DesktopApplication> f = filtered();
    std::vector<flamewm::ui::ListRow> rows;
    rows.reserve(f.size());
    for (size_t i = 0; i < f.size(); ++i) {
        rows.push_back(flamewm::ui::ListRow(f[i].id.value, f[i].displayName, f[i].iconName));
    }
    // Avoid re-entrant selection callback during setRows.
    appList_->setOnSelectionChanged(std::function<void(int)>());
    appList_->setOnActivated(std::function<void(int)>());
    appList_->setRows(rows);
    if (!rows.empty()) appList_->setSelected(0);
    else appList_->setSelected(-1);
    bindListCallbacks();
}

void StartView::rebuildCache() {
    if (appService_) cache_ = appService_->all();
    // If no service, retain existing cache (may be empty initially).
    // Ensure deterministic order: sort by displayName case-insensitive? Keep service order (map sorted by id) stable.
    filteredDirty_ = true;
}

void StartView::bindTextFieldCallbacks() {
    if (!searchField_) return;
    StartView* self = this;
    searchField_->setOnChanged([self](const std::string& t) { if (self) self->handleTextChanged(t); });
    searchField_->setOnSubmit([self](const std::string& t) { if (self) self->handleSubmit(t); });
}

void StartView::bindListCallbacks() {
    if (!appList_) return;
    StartView* self = this;
    appList_->setOnActivated([self](int idx) { if (self) self->handleListActivated(idx); });
    // Selection changed does not auto-launch; keep for keyboard navigation sync if needed.
    appList_->setOnSelectionChanged(std::function<void(int)>());
}

std::string StartView::toLower(const std::string& s) {
    std::string o;
    o.reserve(s.size());
    for (size_t i = 0; i < s.size(); ++i) o.push_back((char)std::tolower((unsigned char)s[i]));
    return o;
}

std::string StartView::trim(const std::string& s) {
    size_t a = 0;
    while (a < s.size() && std::isspace((unsigned char)s[a])) ++a;
    size_t b = s.size();
    while (b > a && std::isspace((unsigned char)s[b-1])) --b;
    return s.substr(a, b-a);
}

bool StartView::equalsIgnoreCase(const std::string& a, const std::string& b) {
    if (a.size() != b.size()) {
        std::string la = toLower(a), lb = toLower(b);
        return la == lb;
    }
    for (size_t i = 0; i < a.size(); ++i) if (std::tolower((unsigned char)a[i]) != std::tolower((unsigned char)b[i])) return false;
    return true;
}

bool StartView::matchesQuery(const api::DesktopApplication& app, const std::string& qLower) const {
    if (qLower.empty()) return true;
    std::string id = toLower(app.id.value);
    if (id.find(qLower) != std::string::npos) return true;
    std::string name = toLower(app.displayName);
    if (name.find(qLower) != std::string::npos) return true;
    std::string exec = toLower(app.execCmd);
    if (exec.find(qLower) != std::string::npos) return true;
    std::string icon = toLower(app.iconName);
    if (icon.find(qLower) != std::string::npos) return true;
    std::string wm = toLower(app.startupWMClass);
    if (!wm.empty() && wm.find(qLower) != std::string::npos) return true;
    std::string fb = toLower(app.wmClassFallback);
    if (!fb.empty() && fb.find(qLower) != std::string::npos) return true;
    for (size_t i = 0; i < app.categories.size(); ++i) {
        std::string c = toLower(app.categories[i]);
        if (c.find(qLower) != std::string::npos) return true;
    }
    return false;
}

} // namespace shell
} // namespace flamewm
