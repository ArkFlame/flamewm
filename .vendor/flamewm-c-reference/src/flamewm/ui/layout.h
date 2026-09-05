#ifndef FLAMEWM_UI_LAYOUT_H
#define FLAMEWM_UI_LAYOUT_H

#include "../api/geometry.h"
#include <string>
#include <vector>

namespace flamewm {
namespace ui {

// Lightweight, no-X11 layout descriptors and geometry solver.
typedef api::Rect Rect;
typedef api::Size Size;

struct Insets {
    int left;
    int top;
    int right;
    int bottom;
    Insets() : left(0), top(0), right(0), bottom(0) {}
    explicit Insets(int all) : left(all), top(all), right(all), bottom(all) {}
    Insets(int horizontal, int vertical)
        : left(horizontal), top(vertical), right(horizontal), bottom(vertical) {}
    Insets(int l, int t, int r, int b) : left(l), top(t), right(r), bottom(b) {}
};

enum MeasureMode { Fixed = 0, Content = 1, Flex = 2 };
enum Alignment { AlignStart = 0, AlignCenter = 1, AlignEnd = 2, AlignStretch = 3 };

struct Measure {
    MeasureMode mode;
    int value;
    int minimum;
    int maximum;
    Measure(MeasureMode mode_ = Content, int value_ = 0, int min_ = 0, int max_ = -1)
        : mode(mode_), value(value_), minimum(min_), maximum(max_) {}
};

struct LayoutSpec {
    Measure width;
    Measure height;
    Alignment horizontal;
    Alignment vertical;
    bool visible;
    Size content;
    LayoutSpec()
        : width(), height(), horizontal(AlignStretch), vertical(AlignStretch),
          visible(true), content() {}
};

struct LayoutItem {
    std::string id;
    int span;
    bool fill;
    LayoutSpec spec;
    LayoutItem() : span(1), fill(false), spec() {}
    explicit LayoutItem(const std::string& id_, int span_ = 1, bool fill_ = false)
        : id(id_), span(span_), fill(fill_), spec() {
        spec.width = Measure(fill_ ? Flex : Content, fill_ ? 1 : 0);
        spec.height = Measure(fill_ ? Flex : Content, fill_ ? 1 : 0);
    }
    LayoutItem& setSpec(const LayoutSpec& value) { spec = value; return *this; }
    LayoutItem& setVisible(bool value) { spec.visible = value; return *this; }
};

struct LayoutResult {
    std::string id;
    Rect rect;
    bool visible;
    LayoutResult() : id(), rect(), visible(false) {}
    LayoutResult(const std::string& id_, const Rect& rect_, bool visible_)
        : id(id_), rect(rect_), visible(visible_) {}
};

struct LayoutOptions {
    int gap;
    Insets padding;
    Alignment horizontal;
    Alignment vertical;
    LayoutOptions() : gap(0), padding(), horizontal(AlignStretch), vertical(AlignStretch) {}
};

class Form {
public:
    void addRow(const std::string& label, const std::string& widgetId) {
        labels_.push_back(label);
        items_.push_back(LayoutItem(widgetId));
    }
    void addRow(const std::string& label, const LayoutItem& item) {
        labels_.push_back(label);
        items_.push_back(item);
    }
    size_t rows() const { return labels_.size(); }
    const std::string& labelAt(size_t i) const { return labels_[i]; }
    const LayoutItem& itemAt(size_t i) const { return items_[i]; }
    void clear() { labels_.clear(); items_.clear(); }
private:
    std::vector<std::string> labels_;
    std::vector<LayoutItem> items_;
};

class Stack {
public:
    enum Direction { Vertical = 0, Horizontal = 1 };
    explicit Stack(Direction dir = Vertical) : dir_(dir) {}
    void add(const LayoutItem& item) { items_.push_back(item); }
    void add(const std::string& widgetId, bool fill = false) {
        items_.push_back(LayoutItem(widgetId, 1, fill));
    }
    Direction direction() const { return dir_; }
    size_t count() const { return items_.size(); }
    const LayoutItem& at(size_t i) const { return items_[i]; }
    void clear() { items_.clear(); }
private:
    Direction dir_;
    std::vector<LayoutItem> items_;
};

std::vector<LayoutResult> solveStack(const Stack& stack, const Rect& bounds,
                                     const LayoutOptions& options = LayoutOptions());

class Grid {
public:
    Grid() : items_() {}
    void add(const LayoutItem& item) { items_.push_back(item); }
    void add(const std::string& widgetId, int span = 1) { items_.push_back(LayoutItem(widgetId, span)); }
    size_t count() const { return items_.size(); }
    const LayoutItem& at(size_t i) const { return items_[i]; }
    void clear() { items_.clear(); }
private:
    std::vector<LayoutItem> items_;
};

std::vector<LayoutResult> solveGrid(const Grid& grid, const Rect& bounds,
                                    int columns, const LayoutOptions& options = LayoutOptions());

class FormRows {
public:
    void addRow(const std::string& label, const LayoutItem& item) {
        labels_.push_back(label); items_.push_back(item);
    }
    void addRow(const std::string& label, const std::string& widgetId) {
        // String rows represent widgets occupying the row's available space.
        addRow(label, LayoutItem(widgetId, 1, true));
    }
    size_t rows() const { return labels_.size(); }
    const std::string& labelAt(size_t i) const { return labels_[i]; }
    const LayoutItem& itemAt(size_t i) const { return items_[i]; }
    void clear() { labels_.clear(); items_.clear(); }
private:
    std::vector<std::string> labels_;
    std::vector<LayoutItem> items_;
};

std::vector<LayoutResult> solveFormRows(const FormRows& form, const Rect& bounds,
                                        int labelWidth, int rowGap = 0,
                                        const LayoutOptions& options = LayoutOptions());

} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_LAYOUT_H
