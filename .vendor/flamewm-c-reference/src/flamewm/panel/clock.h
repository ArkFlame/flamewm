#ifndef FLAMEWM_PANEL_CLOCK_H
#define FLAMEWM_PANEL_CLOCK_H

#include "../core/types.h"
#include "anchor.h"
#include <string>
#include <ctime>

namespace flamewm {
namespace panel {

// Clock/calendar view model — keeps IceWM strftime engine where sufficient.
// Calendar uses local date arithmetic, no daemon. Today highlight is square cell
// then rounded 50% -> circle. Popover anchors via PopoverAnchor on panel edge/output.
class ClockView {
public:
    ClockView();

    struct TimeParts {
        int hour, minute, second;
        int year, month, day, wday; // month 1..12
        std::string timeText; // formatted per IceWM prefs
        std::string dateText;
    };

    struct CalendarCell {
        int day; // 0 = empty (padding)
        bool isToday;
        bool isCurrentMonth;
    };

    // 6 rows x 7 cols = 42 cells
    static const int kCalRows = 6;
    static const int kCalCols = 7;

    TimeParts currentTime(time_t t) const;
    void buildCalendar(int year, int month, int todayDay, CalendarCell out[kCalRows][kCalCols]) const;

    // Popover placement via shared anchor
    PopoverPlacement popoverPlacement(const PanelRect& iconRect, int popoverW, int popoverH,
                                      PanelEdge edge, const WorkArea& wa, int gap) const;

    // Format using strftime patterns (IceWM engine)
    std::string formatTime(time_t t, const std::string& fmt) const; // e.g. "%H:%M"
    std::string formatDate(time_t t, const std::string& fmt) const; // e.g. "%Y-%m-%d"

    // Today diameter helper (square -> circle)
    int todayDiameter(int cellW, int cellH) const { return PopoverAnchor::todayCellDiameter(cellW, cellH); }

private:
    static int daysInMonth(int y, int m);
    static int weekdayOfFirst(int y, int m); // 0=Sun
};

} // namespace panel
} // namespace flamewm

#endif
