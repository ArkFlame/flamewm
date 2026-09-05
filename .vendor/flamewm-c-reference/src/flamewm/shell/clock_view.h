#ifndef FLAMEWM_SHELL_CLOCK_VIEW_H
#define FLAMEWM_SHELL_CLOCK_VIEW_H

#include "flamewm/api/ports.h"
#include "flamewm/api/panels.h"
#include "flamewm/ui/window.h"
#include "flamewm/panel/clock.h"
#include <memory>
#include <string>
#include <ctime>

namespace flamewm {
namespace ui { class Label; class Button; class List; class Popover; }
namespace shell {

class ClockView {
public:
    ClockView();
    ~ClockView();

    ClockView(const ClockView&) = delete;
    ClockView& operator=(const ClockView&) = delete;

    void setContainer(flamewm::ui::Window* c);
    void setOutputRect(const api::Rect& rect);
    void setPanelEdge(api::PanelEdge edge);
    void setMainLoop(api::MainLoopPort* loop);

    void render();
    void invalidate();
    void tick();

    std::string text() const;
    std::string dateText() const;
    std::string timeText() const;
    void setFormat(const std::string& fmt);
    void setDateFormat(const std::string& fmt);
    std::string format() const;
    std::string dateFormat() const;

    // Calendar data stays in panel::ClockView; shell only presents it.
    typedef panel::ClockView::CalendarCell CalendarCell;
    static const int kCalRows = panel::ClockView::kCalRows;
    static const int kCalCols = panel::ClockView::kCalCols;
    void buildCalendar(int year, int month, int todayDay,
                       CalendarCell out[kCalRows][kCalCols]) const;
    void toggleCalendar();
    bool calendarVisible() const;

private:
    struct TimerState;

    void ensureLabels();
    void clearLabels();
    void updateTextFromNow();
    void startTimer();
    void startRepeatingTimer();
    void stopTimer();
    std::string formatTime(time_t t, const std::string& fmt) const;
    void ensureCalendar();
    void showCalendar();
    void hideCalendar();

    flamewm::ui::Window* container_;
    api::MainLoopPort* loop_;
    api::MainLoopPort::TimerHandle timerHandle_;
    bool timerArmed_;
    unsigned long timerGeneration_;
    std::shared_ptr<TimerState> timerState_;
    std::string format_;
    std::string dateFormat_;
    std::string text_;
    std::string dateText_;
    flamewm::ui::Button* timeLabel_;
    flamewm::ui::Label* dateLabel_;
    flamewm::ui::Popover* calendarPopover_;
    flamewm::ui::List* calendarList_;
    panel::ClockView calendarModel_;
    api::Rect outputRect_;
    api::PanelEdge panelEdge_;
};

} // namespace shell
} // namespace flamewm

#endif // FLAMEWM_SHELL_CLOCK_VIEW_H
