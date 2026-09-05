#include "flamewm/shell/workspace_view.h"
#include "flamewm/api/ports.h"
#include "flamewm/platform/workspaces/service.h"
#include "flamewm/ui/backend.h"
#include "flamewm/ui/button.h"
#include "flamewm/ui/shellstyle.h"

#include <string>

namespace flamewm {
namespace shell {

WorkspaceView::WorkspaceView(api::WorkspacePort* p)
    : port_(p), service_(nullptr), container_(nullptr), vertical_(false) {}
WorkspaceView::~WorkspaceView() { clearButtons(); }

void WorkspaceView::setContainer(flamewm::ui::Window* c) {
    container_ = c;
    if (container_ && container_->geometry().valid()) {
        vertical_ = container_->geometry().h > container_->geometry().w;
    }
    // Apply the current snapshot as soon as the panel slot is bound. This
    // keeps the single-workspace pager at hidden, zero intrinsic geometry.
    render();
}
void WorkspaceView::setVertical(bool vertical) { vertical_ = vertical; invalidate(); }
void WorkspaceView::setWorkspacePort(api::WorkspacePort* p) { port_ = p; }
void WorkspaceView::setWorkspaceService(platform::workspaces::WorkspaceService* svc) { service_ = svc; }

bool WorkspaceView::shouldShow() const { return snapshot_.count >= 2; }
void WorkspaceView::setSnapshot(const api::WorkspaceSnapshot& s) { snapshot_ = s; invalidate(); }
api::WorkspaceSnapshot WorkspaceView::snapshot() const { return snapshot_; }

void WorkspaceView::refreshFromService() {
    if (service_) {
        snapshot_ = service_->snapshot();
    } else if (port_) {
        api::Result<api::WorkspaceSnapshot> r = port_->snapshot();
        if (r.ok()) snapshot_ = r.value();
    }
    invalidate();
}

void WorkspaceView::clearButtons() {
    for (std::size_t i = 0; i < buttons_.size(); ++i) delete buttons_[i];
    buttons_.clear();
}

void WorkspaceView::ensureButtons() {
    syncButtons();
}

void WorkspaceView::syncButtons() {
    if (!shouldShow()) {
        clearButtons();
        return;
    }
    const int want = snapshot_.count;
    if ((int)buttons_.size() != want) {
        clearButtons();
        buttons_.reserve(want);
        for (int i = 0; i < want; ++i) {
            flamewm::ui::Toggle* b = flamewm::ui::UiBackend::createToggle(container_, IconRoleTaskbar);
            b->setText(std::to_string(i + 1));
            buttons_.push_back(b);
        }
    } else {
        for (int i = 0; i < want; ++i) {
            buttons_[i]->setText(std::to_string(i + 1));
        }
    }
    for (int i = 0; i < want; ++i) {
        const bool active = (i == snapshot_.activeIndex);
        // Synchronization must not turn a checked-state update into a new
        // activation; the snapshot revision remains the mutation authority.
        buttons_[i]->setOnToggled(std::function<void(bool)>());
        buttons_[i]->setChecked(active);
        const int idx = i;
        buttons_[i]->setOnToggled([this, idx](bool checked) {
            if (checked) this->onClick(idx);
        });
        buttons_[i]->setEnabled(true);
        buttons_[i]->setVisual(flamewm::ui::style::Visual(
            flamewm::ui::style::VisualRoleNavigation,
            active ? flamewm::ui::style::VisualSelected
                   : flamewm::ui::style::VisualNormal));
        if (active) buttons_[i]->setIconRole(IconRoleTaskbarStatus);
        else buttons_[i]->setIconRole(IconRoleTaskbar);
        if (buttons_[i]->isVisible() == false) buttons_[i]->show();
    }
}

void WorkspaceView::render() {
    if (!container_) return;
    if (!shouldShow()) {
        clearButtons();
        container_->hide();
        container_->setGeometry(api::Rect(0,0,0,0));
        return;
    }
    container_->show();
    ensureButtons();
    const ShellStyle style = ShellStyle::defaults();
    // Vertical panels keep the product's two-row ordering but use portrait
    // cells so the pager remains narrow without changing semantic names.
    const int cellWidth = vertical_ ? 16 : style.workspaceButtonWidth;
    const int cellHeight = vertical_ ? 20 : style.workspaceButtonHeight;
    for (int i = 0; i < snapshot_.count; ++i) {
        const workspace::IndexPos pos = indexToPos(i, snapshot_.count);
        buttons_[i]->setGeometry(api::Rect(
            style.workspacePadX + pos.col * (cellWidth + style.workspaceColumnGap),
            style.workspacePadY + pos.row * (cellHeight + style.workspaceRowGap),
            cellWidth, cellHeight));
    }
    container_->repaint();
    for (std::size_t i = 0; i < buttons_.size(); ++i) buttons_[i]->repaint();
}

void WorkspaceView::invalidate() {
    render();
    if (container_) container_->repaint();
}

workspace::IndexPos WorkspaceView::indexToPos(int idx, int total) {
    return workspace::TwoRowTopology::indexToPos(idx, total);
}
int WorkspaceView::posToIndex(int row, int col, int total) {
    return workspace::TwoRowTopology::posToIndex(row, col, total);
}

void WorkspaceView::activate(int index) {
    if (index < 0 || index >= snapshot_.count) return;
    if (index == snapshot_.activeIndex) return;
    if (service_) {
        service_->activate(index, snapshot_.revision);
        refreshFromService();
        return;
    }
    if (port_) {
        api::Status s = port_->activate(index, snapshot_.revision);
        (void)s;
        api::Result<api::WorkspaceSnapshot> r = port_->snapshot();
        if (r.ok()) setSnapshot(r.value());
    }
}
void WorkspaceView::onClick(int idx) { activate(idx); }

} // namespace shell
} // namespace flamewm
