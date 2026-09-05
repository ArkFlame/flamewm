#include "flamewm/shell/clock_view.h"
#include "flamewm/ui/backend.h"
#include "flamewm/ui/label.h"
#include "flamewm/ui/button.h"
#include "flamewm/ui/list.h"
#include "flamewm/ui/popover.h"
#include "flamewm/ui/shellstyle.h"
#include "flamewm/shell/popovers.h"

#include <ctime>
#include <cstring>
#include <chrono>
#include <algorithm>

namespace flamewm {
namespace shell {

struct ClockView::TimerState {
    TimerState() : owner(nullptr), generation(0) {}

    ClockView* owner;
    unsigned long generation;
};

ClockView::ClockView()
    : container_(nullptr)
    , loop_(nullptr)
    , timerHandle_()
    , timerArmed_(false)
    , timerGeneration_(0)
    , timerState_(new TimerState())
    , format_("%H:%M")
    , dateFormat_("%d/%m/%y")
    , text_()
    , dateText_()
    , timeLabel_(nullptr)
    , dateLabel_(nullptr)
    , calendarPopover_(nullptr)
    , calendarList_(nullptr)
    , outputRect_()
    , panelEdge_(api::PanelEdge::Bottom) {
    timerHandle_.id = 0;
    updateTextFromNow();
}

ClockView::~ClockView() {
    timerState_->owner = nullptr;
    stopTimer();
    if (calendarPopover_) calendarPopover_->close();
    delete calendarList_;
    delete calendarPopover_;
    clearLabels();
}

void ClockView::setContainer(flamewm::ui::Window* c) {
    if (container_ == c) return;
    if (calendarPopover_) calendarPopover_->close();
    delete calendarList_; calendarList_ = nullptr;
    delete calendarPopover_; calendarPopover_ = nullptr;
    clearLabels();
    container_ = c;
}

void ClockView::setOutputRect(const api::Rect& rect) {
    outputRect_ = rect;
    if (calendarVisible()) showCalendar();
}
void ClockView::setPanelEdge(api::PanelEdge edge) {
    panelEdge_ = edge;
    if (calendarVisible()) showCalendar();
}

void ClockView::setMainLoop(api::MainLoopPort* loop) {
    if (loop_ == loop) return;
    stopTimer();
    loop_ = loop;
    if (loop_) startTimer();
}

void ClockView::clearLabels() {
    delete timeLabel_; timeLabel_ = nullptr;
    delete dateLabel_; dateLabel_ = nullptr;
}

void ClockView::ensureLabels() {
    if (!timeLabel_) {
        timeLabel_ = flamewm::ui::UiBackend::createButton(container_);
        if (timeLabel_) {
            timeLabel_->setVisual(flamewm::ui::style::Visual(
                flamewm::ui::style::VisualRoleControl,
                flamewm::ui::style::VisualNormal));
            timeLabel_->setOnClick([this]() { toggleCalendar(); });
            timeLabel_->setText(text_);
        }
    }
    if (!dateLabel_) {
        dateLabel_ = flamewm::ui::UiBackend::createLabel(container_);
        dateLabel_->setWrap(false);
        dateLabel_->setTypography(flamewm::ui::style::TypographyCaption);
        dateLabel_->setVisual(flamewm::ui::style::Visual(
            flamewm::ui::style::VisualRoleText,
            flamewm::ui::style::VisualNormal));
    }
}

void ClockView::ensureCalendar() {
    if (calendarPopover_) return;
    calendarPopover_ = flamewm::ui::UiBackend::createPopover(container_);
    if (!calendarPopover_) return;
    calendarPopover_->setRole(flamewm::ui::WindowRole::Popup);
    calendarPopover_->setVisual(flamewm::ui::style::Visual(
        flamewm::ui::style::VisualRoleRaised,
        flamewm::ui::style::VisualNormal));
    calendarList_ = flamewm::ui::UiBackend::createList(calendarPopover_);
    if (!calendarList_) {
        delete calendarPopover_;
        calendarPopover_ = nullptr;
        return;
    }
    calendarList_->setVisual(flamewm::ui::style::Visual(
        flamewm::ui::style::VisualRoleSurface,
        flamewm::ui::style::VisualNormal));
}

void ClockView::showCalendar() {
    ensureCalendar();
    if (!calendarPopover_ || !calendarList_ || !container_) return;
    time_t now = time(nullptr);
    struct tm tmv;
    localtime_r(&now, &tmv);
    CalendarCell cells[kCalRows][kCalCols];
    calendarModel_.buildCalendar(tmv.tm_year + 1900, tmv.tm_mon + 1, tmv.tm_mday, cells);
    std::vector<flamewm::ui::ListRow> rows;
    rows.reserve(kCalRows + 2);
    rows.push_back(flamewm::ui::ListRow("month", formatTime(now, "%B %Y")));
    rows.push_back(flamewm::ui::ListRow("weekdays", "Mon  Tue  Wed  Thu  Fri  Sat  Sun"));
    for (int r = 0; r < kCalRows; ++r) {
        std::string row;
        for (int c = 0; c < kCalCols; ++c) {
            if (c) row += "  ";
            if (!cells[r][c].day) {
                row += "  ";
                continue;
            }
            const std::string day = cells[r][c].day < 10
                ? "0" + std::to_string(cells[r][c].day)
                : std::to_string(cells[r][c].day);
            // List has no per-cell state; retain today semantic in material.
            row += cells[r][c].isToday ? "[" + day + "]" : day;
        }
        rows.push_back(flamewm::ui::ListRow("week" + std::to_string(r), row));
    }
    calendarList_->setRows(rows);
    const ShellStyle style = ShellStyle::defaults();
    const int width = style.clockPopoverWidth;
    const int height = style.popoverPadding * 2 + static_cast<int>(rows.size()) * 32;
    const api::Rect local = container_->geometry();
    if (!outputRect_.valid() || !local.valid()) return;
    api::Rect anchor = local;
    anchor.x += outputRect_.x;
    anchor.y += outputRect_.y;
    if (panelEdge_ == api::PanelEdge::Bottom || panelEdge_ == api::PanelEdge::Top)
        anchor.y += panelEdge_ == api::PanelEdge::Bottom
            ? outputRect_.h - style.panelHeight : 0;
    else
        anchor.x += panelEdge_ == api::PanelEdge::Right
            ? outputRect_.w - style.panelHeight : 0;
    PopoverAnchor placement(anchor, panelEdge_, outputRect_, style.panelHeight, 0);
    const api::Rect corrected = Popovers::anchoredRect(placement, api::Size(width, height));
    calendarList_->setGeometry(api::Rect(style.popoverPadding, style.popoverPadding,
                                         std::max(0, corrected.w - style.popoverPadding * 2),
                                         std::max(0, corrected.h - style.popoverPadding * 2)));
    // Native showAt positions below its anchor, so pass a root-coordinate
    // synthetic anchor whose lower edge is the corrected top edge.
    calendarPopover_->setGeometry(corrected);
    calendarPopover_->showAt(api::Rect(corrected.x, corrected.y - corrected.h,
                                       corrected.w, corrected.h));
}

void ClockView::hideCalendar() { if (calendarPopover_) calendarPopover_->close(); }
void ClockView::toggleCalendar() {
    if (calendarPopover_ && calendarPopover_->isVisible()) hideCalendar();
    else showCalendar();
}
bool ClockView::calendarVisible() const {
    return calendarPopover_ && calendarPopover_->isVisible();
}

void ClockView::buildCalendar(int year, int month, int todayDay,
                              CalendarCell out[kCalRows][kCalCols]) const {
    calendarModel_.buildCalendar(year, month, todayDay, out);
}

std::string ClockView::formatTime(time_t t, const std::string& fmt) const {
    struct tm tmv;
#ifdef _POSIX_VERSION
    localtime_r(&t, &tmv);
#else
    tmv = *localtime(&t);
#endif
    char buf[256];
    if (fmt.empty()) return std::string();
    std::size_t n = strftime(buf, sizeof(buf), fmt.c_str(), &tmv);
    if (n == 0) return std::string();
    return std::string(buf, n);
}

void ClockView::updateTextFromNow() {
    time_t now = time(nullptr);
    text_ = formatTime(now, format_);
    dateText_ = formatTime(now, dateFormat_);
    if (timeLabel_) timeLabel_->setText(text_);
    if (dateLabel_) dateLabel_->setText(dateText_);
}

void ClockView::startTimer() {
    if (!loop_ || timerArmed_) return;

    const bool secondsFormat = format_.find("%S") != std::string::npos
        || format_.find("%T") != std::string::npos
        || format_.find("%X") != std::string::npos
        || format_.find("%r") != std::string::npos;
    const uint64_t interval = secondsFormat ? 1000 : 60000;
    const uint64_t now = static_cast<uint64_t>(
        std::chrono::duration_cast<std::chrono::milliseconds>(
            std::chrono::system_clock::now().time_since_epoch()).count());
    const uint64_t delay = interval - (now % interval);
    const unsigned long generation = ++timerGeneration_;
    timerState_->owner = this;
    timerState_->generation = generation;
    std::weak_ptr<TimerState> weakState(timerState_);
    timerHandle_ = loop_->addTimer(delay, [weakState, generation]() {
        std::shared_ptr<TimerState> state = weakState.lock();
        if (!state || !state->owner || state->generation != generation) return;
        ClockView* owner = state->owner;
        owner->timerArmed_ = false;
        owner->timerHandle_.id = 0;
        owner->tick();
        owner->startRepeatingTimer();
    }, false);
    timerArmed_ = true;
}

void ClockView::startRepeatingTimer() {
    if (!loop_ || timerArmed_) return;
    const bool secondsFormat = format_.find("%S") != std::string::npos
        || format_.find("%T") != std::string::npos
        || format_.find("%X") != std::string::npos
        || format_.find("%r") != std::string::npos;
    const uint64_t interval = secondsFormat ? 1000 : 60000;
    const unsigned long generation = timerGeneration_;
    std::weak_ptr<TimerState> weakState(timerState_);
    timerHandle_ = loop_->addTimer(interval, [weakState, generation]() {
        std::shared_ptr<TimerState> state = weakState.lock();
        if (state && state->owner && state->generation == generation) state->owner->tick();
    }, true);
    timerArmed_ = true;
}

void ClockView::stopTimer() {
    ++timerGeneration_;
    timerState_->generation = timerGeneration_;
    if (!timerArmed_ || !loop_) {
        timerArmed_ = false;
        timerHandle_.id = 0;
        return;
    }
    loop_->removeTimer(timerHandle_);
    timerArmed_ = false;
    timerHandle_.id = 0;
}

void ClockView::render() {
    ensureLabels();
    updateTextFromNow();
    if (container_) {
        container_->show();
        api::Rect g = container_->geometry();
        const ShellStyle style = ShellStyle::defaults();
        const int contentWidth = std::max(style.clockMinWidth,
                                          std::max(0, g.w - style.clockPadLeft - style.clockPadRight));
        if (timeLabel_) timeLabel_->setGeometry(api::Rect(style.clockPadLeft,
                                                           style.clockPadTop,
                                                           contentWidth, style.clockLineHeight));
        if (dateLabel_) dateLabel_->setGeometry(api::Rect(style.clockPadLeft,
                                                           style.clockPadTop + style.clockLineHeight,
                                                           contentWidth, style.clockLineHeight));
        container_->repaint();
    }
    if (timeLabel_) {
        timeLabel_->show();
        timeLabel_->repaint();
    }
    if (dateLabel_) {
        dateLabel_->show();
        dateLabel_->repaint();
    }
    if (loop_ && !timerArmed_) startTimer();
}

void ClockView::invalidate() {
    updateTextFromNow();
    render();
    if (container_) container_->repaint();
}

void ClockView::tick() {
    updateTextFromNow();
    if (timeLabel_) timeLabel_->repaint();
    if (dateLabel_) dateLabel_->repaint();
    if (container_) container_->repaint();
}

std::string ClockView::text() const { return text_; }
std::string ClockView::dateText() const { return dateText_; }
std::string ClockView::timeText() const { return text_; }
void ClockView::setFormat(const std::string& f) {
    if (format_ == f) return;
    format_ = f;
    updateTextFromNow();
    if (loop_) {
        stopTimer();
        startTimer();
    }
}
void ClockView::setDateFormat(const std::string& f) { dateFormat_ = f; updateTextFromNow(); }
std::string ClockView::format() const { return format_; }
std::string ClockView::dateFormat() const { return dateFormat_; }

} // namespace shell
} // namespace flamewm
