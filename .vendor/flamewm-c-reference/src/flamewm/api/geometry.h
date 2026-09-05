#ifndef FLAMEWM_API_GEOMETRY_H
#define FLAMEWM_API_GEOMETRY_H

#include <algorithm>

namespace flamewm {
namespace api {

struct Point {
    int x;
    int y;

    Point() : x(0), y(0) {}
    Point(int x_, int y_) : x(x_), y(y_) {}

    bool operator==(const Point& o) const { return x == o.x && y == o.y; }
    bool operator!=(const Point& o) const { return !(*this == o); }
};

struct Size {
    int w;
    int h;

    Size() : w(0), h(0) {}
    Size(int w_, int h_) : w(w_), h(h_) {}

    bool valid() const { return w > 0 && h > 0; }

    bool operator==(const Size& o) const { return w == o.w && h == o.h; }
    bool operator!=(const Size& o) const { return !(*this == o); }
};

struct Rect {
    int x;
    int y;
    int w;
    int h;

    Rect() : x(0), y(0), w(0), h(0) {}
    Rect(int x_, int y_, int w_, int h_) : x(x_), y(y_), w(w_), h(h_) {}

    static Rect from(const Point& p, const Size& s) { return Rect(p.x, p.y, s.w, s.h); }

    bool valid() const { return w > 0 && h > 0; }

    int right() const { return x + w; }
    int bottom() const { return y + h; }

    Point origin() const { return Point(x, y); }
    Size size() const { return Size(w, h); }

    bool contains(const Point& p) const {
        return p.x >= x && p.x < right() && p.y >= y && p.y < bottom();
    }

    bool intersects(const Rect& o) const {
        return x < o.right() && o.x < right() && y < o.bottom() && o.y < bottom();
    }

    bool operator==(const Rect& o) const { return x == o.x && y == o.y && w == o.w && h == o.h; }
    bool operator!=(const Rect& o) const { return !(*this == o); }
};

inline Rect intersect(const Rect& a, const Rect& b) {
    int nx = std::max(a.x, b.x);
    int ny = std::max(a.y, b.y);
    int nr = std::min(a.right(), b.right());
    int nb = std::min(a.bottom(), b.bottom());
    int nw = nr - nx;
    int nh = nb - ny;
    if (nw <= 0 || nh <= 0) return Rect(0, 0, 0, 0);
    return Rect(nx, ny, nw, nh);
}

inline Rect unite(const Rect& a, const Rect& b) {
    if (!a.valid()) return b;
    if (!b.valid()) return a;
    int nx = std::min(a.x, b.x);
    int ny = std::min(a.y, b.y);
    int nr = std::max(a.right(), b.right());
    int nb = std::max(a.bottom(), b.bottom());
    return Rect(nx, ny, nr - nx, nb - ny);
}

} // namespace api
} // namespace flamewm

#endif // FLAMEWM_API_GEOMETRY_H
