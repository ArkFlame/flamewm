#include "layout.h"

#include <algorithm>

namespace flamewm { namespace ui {

namespace {
int clampSize(int value, const Measure& m) {
    value = std::max(m.minimum, value);
    if (m.maximum >= 0) value = std::min(m.maximum, value);
    return std::max(0, value);
}

int basis(const LayoutItem& item, bool horizontal) {
    const Measure& m = horizontal ? item.spec.width : item.spec.height;
    const int content = horizontal ? item.spec.content.w : item.spec.content.h;
    if (m.mode == Fixed) return clampSize(m.value, m);
    if (m.mode == Flex) return clampSize(m.value, m);
    return clampSize(content, m);
}

int alignedOffset(int available, int size, Alignment alignment) {
    if (alignment == AlignCenter) return (available - size) / 2;
    if (alignment == AlignEnd) return available - size;
    return 0;
}

std::vector<LayoutResult> solve(const std::vector<LayoutItem>& items, const Rect& bounds,
                                bool horizontal, const LayoutOptions& options) {
    const int origin = horizontal ? bounds.x + options.padding.left : bounds.y + options.padding.top;
    const int crossOrigin = horizontal ? bounds.y + options.padding.top : bounds.x + options.padding.left;
    const int available = std::max(0, (horizontal ? bounds.w - options.padding.left - options.padding.right
                                                    : bounds.h - options.padding.top - options.padding.bottom));
    const int cross = std::max(0, horizontal ? bounds.h - options.padding.top - options.padding.bottom
                                             : bounds.w - options.padding.left - options.padding.right);
    std::vector<int> sizes(items.size(), 0);
    int used = 0;
    int flex = 0;
    int visibleCount = 0;
    for (size_t i = 0; i < items.size(); ++i) {
        if (!items[i].spec.visible) continue;
        sizes[i] = basis(items[i], horizontal);
        used += sizes[i];
        if ((items[i].spec.width.mode == Flex && horizontal) ||
            (items[i].spec.height.mode == Flex && !horizontal))
            flex += std::max(1, horizontal ? items[i].spec.width.value : items[i].spec.height.value);
        ++visibleCount;
    }
    used += std::max(0, visibleCount - 1) * options.gap;
    int extra = available - used;
    if (extra > 0 && flex > 0) {
        int given = 0;
        for (size_t i = 0; i < items.size(); ++i) if (items[i].spec.visible) {
            const Measure& m = horizontal ? items[i].spec.width : items[i].spec.height;
            if (m.mode == Flex) {
                const int weight = std::max(1, m.value);
                const int part = (extra * weight) / flex;
                sizes[i] = clampSize(sizes[i] + part, m);
                given += part;
            }
        }
        extra -= given;
    }
    std::vector<LayoutResult> result;
    int contentExtent = 0;
    for (size_t i = 0; i < sizes.size(); ++i) {
        if (items[i].spec.visible) contentExtent += sizes[i];
    }
    contentExtent += std::max(0, visibleCount - 1) * options.gap;
    const Alignment mainAlignment = horizontal ? options.horizontal : options.vertical;
    int cursor = origin + alignedOffset(available, contentExtent, mainAlignment);
    for (size_t i = 0; i < items.size(); ++i) {
        const LayoutItem& item = items[i];
        if (!item.spec.visible) { result.push_back(LayoutResult(item.id, Rect(), false)); continue; }
        const Measure& crossMeasure = horizontal ? item.spec.height : item.spec.width;
        Alignment optionAlignment = horizontal ? options.vertical : options.horizontal;
        Alignment itemAlignment = horizontal ? item.spec.vertical : item.spec.horizontal;
        const Alignment crossAlignment = optionAlignment == AlignStretch ? itemAlignment : optionAlignment;
        int crossSize = crossMeasure.mode == Flex ||
                        crossAlignment == AlignStretch
                        ? cross : basis(item, !horizontal);
        crossSize = clampSize(crossSize, crossMeasure);
        Alignment align = crossAlignment;
        if (align == AlignStretch) crossSize = clampSize(cross, crossMeasure);
        const int offset = alignedOffset(cross, crossSize, align);
        Rect r = horizontal ? Rect(cursor, crossOrigin + offset, sizes[i], crossSize)
                            : Rect(crossOrigin + offset, cursor, crossSize, sizes[i]);
        result.push_back(LayoutResult(item.id, r, true));
        cursor += sizes[i] + options.gap;
    }
    return result;
}
}

std::vector<LayoutResult> solveStack(const Stack& stack, const Rect& bounds, const LayoutOptions& options) {
    std::vector<LayoutItem> items;
    for (size_t i = 0; i < stack.count(); ++i) items.push_back(stack.at(i));
    return solve(items, bounds, stack.direction() == Stack::Horizontal, options);
}

std::vector<LayoutResult> solveGrid(const Grid& grid, const Rect& bounds, int columns, const LayoutOptions& options) {
    std::vector<LayoutResult> out;
    if (columns <= 0) return out;
    const int innerW = std::max(0, bounds.w - options.padding.left - options.padding.right);
    const int innerH = std::max(0, bounds.h - options.padding.top - options.padding.bottom);
    const int cellW = std::max(0, (innerW - (columns - 1) * options.gap) / columns);
    std::vector<int> itemRows(grid.count(), 0);
    std::vector<int> itemCols(grid.count(), 0);
    std::vector<int> rowUse(1, 0);
    for (size_t i = 0; i < grid.count(); ++i) {
        if (!grid.at(i).spec.visible) continue;
        const int span = std::max(1, std::min(grid.at(i).span, columns));
        size_t row = rowUse.size() - 1;
        while (rowUse[row] + span > columns) {
            ++row;
            if (row == rowUse.size()) rowUse.push_back(0);
        }
        itemRows[i] = static_cast<int>(row);
        itemCols[i] = rowUse[row];
        rowUse[row] += span;
    }
    const int rows = static_cast<int>(rowUse.size());
    const int cellH = rows ? std::max(0, (innerH - (rows - 1) * options.gap) / rows) : 0;
    for (size_t i = 0; i < grid.count(); ++i) {
        const LayoutItem& item = grid.at(i);
        if (!item.spec.visible) { out.push_back(LayoutResult(item.id, Rect(), false)); continue; }
        const int row = itemRows[i];
        const int col = itemCols[i];
        const int span = std::max(1, std::min(item.span, columns - col));
        const int cellX = bounds.x + options.padding.left + col * (cellW + options.gap);
        const int cellY = bounds.y + options.padding.top + row * (cellH + options.gap);
        const int cellWidth = cellW * span + options.gap * (span - 1);
        const Alignment horizontalAlignment = options.horizontal == AlignStretch
                                                  ? item.spec.horizontal : options.horizontal;
        const Alignment verticalAlignment = options.vertical == AlignStretch
                                                ? item.spec.vertical : options.vertical;
        const int w = horizontalAlignment == AlignStretch ? cellWidth
                                                            : clampSize(basis(item, true), item.spec.width);
        const int h = verticalAlignment == AlignStretch ? cellH
                                                         : clampSize(basis(item, false), item.spec.height);
        out.push_back(LayoutResult(item.id,
                                   Rect(cellX + alignedOffset(cellWidth, w, horizontalAlignment),
                                        cellY + alignedOffset(cellH, h, verticalAlignment), w, h), true));
    }
    return out;
}

std::vector<LayoutResult> solveFormRows(const FormRows& form, const Rect& bounds, int labelWidth,
                                        int rowGap, const LayoutOptions& options) {
    LayoutOptions rowOptions = options;
    rowOptions.gap = 0;
    rowOptions.padding = Insets();
    std::vector<LayoutResult> out;
    const int innerW = std::max(0, bounds.w - options.padding.left - options.padding.right);
    const int widgetW = std::max(0, innerW - std::max(0, labelWidth) - options.gap);
    const int innerH = std::max(0, bounds.h - options.padding.top - options.padding.bottom);
    const int rowH = form.rows() ? std::max(0, (innerH - static_cast<int>(form.rows() - 1) * rowGap) /
                                                   static_cast<int>(form.rows())) : 0;
    for (size_t i = 0; i < form.rows(); ++i) {
        std::vector<LayoutItem> one(1, form.itemAt(i));
        Rect row(bounds.x + options.padding.left + std::max(0, labelWidth) + options.gap,
                 bounds.y + options.padding.top + static_cast<int>(i) * (rowH + rowGap), widgetW, rowH);
        std::vector<LayoutResult> solved = solve(one, row, true, rowOptions);
        if (!solved.empty()) out.push_back(solved[0]);
    }
    return out;
}

}} // namespace flamewm::ui
