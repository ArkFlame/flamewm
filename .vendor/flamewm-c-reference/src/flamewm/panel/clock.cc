#include "clock.h"
#include <cstring>

namespace flamewm {
namespace panel {

ClockView::ClockView() {}

ClockView::TimeParts ClockView::currentTime(time_t t) const {
    TimeParts p;
    struct tm tmv;
    localtime_r(&t, &tmv);
    p.hour = tmv.tm_hour;
    p.minute = tmv.tm_min;
    p.second = tmv.tm_sec;
    p.year = tmv.tm_year + 1900;
    p.month = tmv.tm_mon + 1;
    p.day = tmv.tm_mday;
    p.wday = tmv.tm_wday;
    char buf[128];
    strftime(buf, sizeof(buf), "%H:%M", &tmv);
    p.timeText = buf;
    strftime(buf, sizeof(buf), "%Y-%m-%d", &tmv);
    p.dateText = buf;
    return p;
}

void ClockView::buildCalendar(int year, int month, int todayDay, CalendarCell out[kCalRows][kCalCols]) const {
    for (int r = 0; r < kCalRows; ++r) for (int c = 0; c < kCalCols; ++c) { out[r][c].day = 0; out[r][c].isToday = false; out[r][c].isCurrentMonth = false; }
    int firstW = weekdayOfFirst(year, month);
    int dim = daysInMonth(year, month);
    int d = 1;
    for (int r = 0; r < kCalRows && d <= dim; ++r) {
        for (int c = 0; c < kCalCols && d <= dim; ++c) {
            if (r == 0 && c < firstW) { out[r][c].day = 0; continue; }
            out[r][c].day = d;
            out[r][c].isToday = (d == todayDay);
            out[r][c].isCurrentMonth = true;
            ++d;
        }
    }
}

PopoverPlacement ClockView::popoverPlacement(const PanelRect& iconRect, int popoverW, int popoverH,
                                             PanelEdge edge, const WorkArea& wa, int gap) const {
    return PopoverAnchor::anchor(iconRect, popoverW, popoverH, edge, wa, gap);
}

std::string ClockView::formatTime(time_t t, const std::string& fmt) const {
    struct tm tmv; localtime_r(&t, &tmv);
    char buf[256]; strftime(buf, sizeof(buf), fmt.c_str(), &tmv); return std::string(buf);
}
std::string ClockView::formatDate(time_t t, const std::string& fmt) const { return formatTime(t, fmt); }

int ClockView::daysInMonth(int y, int m) {
    static int dm[] = {0,31,28,31,30,31,30,31,31,30,31,30,31};
    int d = dm[m];
    if (m == 2 && ((y % 4 == 0 && y % 100 != 0) || (y % 400 == 0))) d = 29;
    return d;
}
int ClockView::weekdayOfFirst(int y, int m) {
    // Zeller-like via mktime for local correctness (Monday=0 not needed; we use Sun=0)
    struct tm tmv; memset(&tmv,0,sizeof(tmv));
    tmv.tm_year = y - 1900; tmv.tm_mon = m - 1; tmv.tm_mday = 1;
    mktime(&tmv);
    return tmv.tm_wday;
}

} // namespace panel
} // namespace flamewm
