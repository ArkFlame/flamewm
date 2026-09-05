#ifndef FLAMEWM_SNAP_TYPES_H
#define FLAMEWM_SNAP_TYPES_H

namespace flamewm {
namespace snap {

// Rect — YRect-like physical geometry (outer frame). Integer pixels.
struct Rect {
    int x;
    int y;
    int w;
    int h;
    Rect() : x(0), y(0), w(0), h(0) {}
    Rect(int x_, int y_, int w_, int h_) : x(x_), y(y_), w(w_), h(h_) {}
    bool operator==(const Rect& o) const { return x==o.x && y==o.y && w==o.w && h==o.h; }
    bool operator!=(const Rect& o) const { return !(*this==o); }
};

// Snap target. Aliases keep both spec names and legacy IceWM tile names.
// X11/X.h defines `None` as 0L — temporarily hide it so the enumerator
// `None` survives inclusion order (ymsgbox.cc -> ypixmap.h -> X11.h -> wmframe.h).
#ifdef None
#pragma push_macro("None")
#undef None
#define FLAMEWM_SNAP_RESTORE_NONE
#endif
enum SnapTarget {
    SnapNone = 0,
    SnapLeftHalf,
    SnapRightHalf,
    SnapTopHalf,
    SnapBottomHalf,
    SnapTopLeft,
    SnapTopRight,
    SnapBottomLeft,
    SnapBottomRight,
    SnapCenter,   // centered half-size (IceWM tile center) — not V4 drag target
    SnapMaximize, // top-center maximize — full work area (V4 drag target)

    // V4 spec aliases (WINDOWS.md drag targets)
    None = SnapNone,
    TargetNone = SnapNone,
    LeftHalf = SnapLeftHalf,
    RightHalf = SnapRightHalf,
    TopLeftQuarter = SnapTopLeft,
    TopRightQuarter = SnapTopRight,
    BottomLeftQuarter = SnapBottomLeft,
    BottomRightQuarter = SnapBottomRight,
    Maximize = SnapMaximize
};
#ifdef FLAMEWM_SNAP_RESTORE_NONE
#pragma pop_macro("None")
#undef FLAMEWM_SNAP_RESTORE_NONE
#endif

// SnapGeometry aliases Rect for backward compatibility with test_contracts
struct SnapGeometry {
    int x, y, w, h;
    SnapGeometry() : x(0), y(0), w(0), h(0) {}
    SnapGeometry(int x_, int y_, int w_, int h_) : x(x_), y(y_), w(w_), h(h_) {}
    Rect toRect() const { return Rect(x,y,w,h); }
};

inline Rect toRect(const SnapGeometry& g) { return Rect(g.x,g.y,g.w,g.h); }
inline SnapGeometry toSnapGeometry(const Rect& r) { SnapGeometry g; g.x=r.x; g.y=r.y; g.w=r.w; g.h=r.h; return g; }

// Core geometry calculation — single source for preview and commit.
// Commits use manager work area (mx,my,Mx,My) which already excludes
// taskbar struts. Detection uses physical output rect (see controller).
// Preview and commit MUST call the same function — no duplicated math.
//
// Invariant: geometryFor is pure and handles any origin (0, positive, negative)
// because it operates on mx/my/Mx/My directly, never assuming 0 origin.
// Center formula preserved: x = mx + (Mx - mx - w)/2 (bug fixed in wmTile).
inline SnapGeometry geometryFor(SnapTarget t, int mx, int my, int Mx, int My, int curW, int curH) {
    SnapGeometry g;
    int W = Mx - mx;
    int H = My - my;
    switch (t) {
        case SnapLeftHalf:      g.x=mx; g.y=my; g.w=W/2; g.h=H; break;
        case SnapRightHalf:     g.x=mx+W/2; g.y=my; g.w=W - W/2; g.h=H; break;
        case SnapTopHalf:       g.x=mx; g.y=my; g.w=W; g.h=H/2; break;
        case SnapBottomHalf:    g.x=mx; g.y=my+H/2; g.w=W; g.h=H - H/2; break;
        case SnapTopLeft:       g.x=mx; g.y=my; g.w=W/2; g.h=H/2; break;
        case SnapTopRight:      g.x=mx+W/2; g.y=my; g.w=W - W/2; g.h=H/2; break;
        case SnapBottomLeft:    g.x=mx; g.y=my+H/2; g.w=W/2; g.h=H - H/2; break;
        case SnapBottomRight:   g.x=mx+W/2; g.y=my+H/2; g.w=W - W/2; g.h=H - H/2; break;
        case SnapCenter:        g.w = (curW>0?curW:W/2); g.h=(curH>0?curH:H/2);
                                g.x = mx + (Mx - mx - g.w)/2;
                                g.y = my + (My - my - g.h)/2;
                                break;
        case SnapMaximize:      g.x=mx; g.y=my; g.w=W; g.h=H; break;
        default:                g.x=mx; g.y=my; g.w=curW; g.h=curH; break;
    }
    return g;
}

// Overload using Rect work area — preview==commit invariant preserved.
// workArea: manager work area (strut-aware)
// outputRect: physical output rect — documented for detection side; commit
//             still uses workArea. When provided this overload ignores it
//             for geometry and exists only to make call sites explicit.
inline SnapGeometry geometryFor(SnapTarget t, const Rect& workArea, int curW, int curH) {
    return geometryFor(t, workArea.x, workArea.y,
                          workArea.x + workArea.w, workArea.y + workArea.h,
                          curW, curH);
}
inline SnapGeometry geometryFor(SnapTarget t, const Rect& workArea, const Rect& /*outputRect*/, int curW, int curH) {
    // Documented: target detection uses physical outputRect, commit uses workArea.
    // This function is commit/preview geometry — identical for both.
    return geometryFor(t, workArea, curW, curH);
}
inline Rect geometryRectFor(SnapTarget t, const Rect& workArea, int curW, int curH) {
    return toRect(geometryFor(t, workArea, curW, curH));
}
inline Rect geometryRectFor(SnapTarget t, const Rect& workArea, const Rect& outputRect, int curW, int curH) {
    return toRect(geometryFor(t, workArea, outputRect, curW, curH));
}

} // namespace snap
} // namespace flamewm
#endif
