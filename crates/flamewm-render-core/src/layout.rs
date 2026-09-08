use crate::model::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutBox {
    pub rect: Rect,
}

#[derive(Clone, Debug)]
pub struct LayoutResult {
    pub boxes: Vec<LayoutBox>,
    /// Content extent per node in surface coordinates (viewport origin +
    /// intrinsic content size). Viewport is `boxes[index].rect`.
    pub contents: Vec<Rect>,
}

impl LayoutResult {
    pub fn content_extent(&self, index: u32) -> Rect {
        self.contents
            .get(index as usize)
            .copied()
            .unwrap_or_default()
    }

    /// Pure viewport/content/offset -> metrics (thumb geometry included).
    /// Thumb minimum is 12px; no thumb when content fits.
    pub fn scroll_metrics(viewport: Rect, content: Rect, offset: ScrollState) -> ScrollMetrics {
        let max_x = (content.width - viewport.width).max(0.0);
        let max_y = (content.height - viewport.height).max(0.0);
        let offset = offset.clamped(max_x, max_y);
        ScrollMetrics {
            viewport,
            content,
            max_x,
            max_y,
            thumb_x: thumb_rect(
                viewport,
                content.width,
                viewport.width,
                offset.offset_x,
                true,
            ),
            thumb_y: thumb_rect(
                viewport,
                content.height,
                viewport.height,
                offset.offset_y,
                false,
            ),
        }
    }

    pub fn metrics_for(&self, index: u32, offset: ScrollState) -> ScrollMetrics {
        let viewport = self
            .boxes
            .get(index as usize)
            .map(|b| b.rect)
            .unwrap_or_default();
        Self::scroll_metrics(viewport, self.content_extent(index), offset)
    }

    pub fn hit_test_action(&self, document: &RuntimeDocument, x: f32, y: f32) -> Option<u32> {
        let mut indices: Vec<usize> = (0..document.document.nodes.len()).collect();
        indices.sort_by_key(|index| (document.effective_z_index(*index as u32), *index as i64));
        for index in indices.into_iter().rev() {
            let node = &document.document.nodes[index];
            if node.action.is_empty() || !document.is_effectively_visible(index as u32) {
                continue;
            }
            if self.boxes.get(index)?.rect.contains(x, y) {
                return Some(index as u32);
            }
        }
        None
    }
}

pub struct LayoutEngine;

impl LayoutEngine {
    pub fn compute(
        document: &RuntimeDocument,
        viewport_width: f32,
        viewport_height: f32,
        interaction: InteractionState,
    ) -> LayoutResult {
        let mut boxes = vec![LayoutBox::default(); document.document.nodes.len()];
        if document.document.nodes.is_empty() {
            return LayoutResult {
                boxes,
                contents: Vec::new(),
            };
        }
        let mut engine = Engine {
            document,
            interaction,
            boxes: &mut boxes,
        };
        engine.layout_node(
            document.document.root,
            0.0,
            0.0,
            viewport_width.max(0.0),
            viewport_height.max(0.0),
            Some(viewport_width.max(0.0)),
            Some(viewport_height.max(0.0)),
        );
        // Content extent: intrinsic measure unconstrained on the scroll axis.
        let mut contents = vec![Rect::default(); document.document.nodes.len()];
        for (index, entry) in boxes.iter().enumerate() {
            let style = document.runtime_style(index as u32, interaction);
            let scrolls_x = matches!(
                style.overflow_x,
                Overflow::Auto | Overflow::Scroll | Overflow::Hidden
            );
            let scrolls_y = matches!(
                style.overflow_y,
                Overflow::Auto | Overflow::Scroll | Overflow::Hidden
            );
            if !scrolls_x && !scrolls_y {
                contents[index] = entry.rect;
                continue;
            }
            let mut empty: Vec<LayoutBox> = Vec::new();
            let probe = Engine {
                document,
                interaction,
                boxes: &mut empty,
            };
            let avail_w = if scrolls_x {
                f32::MAX / 1024.0
            } else {
                entry.rect.width.max(0.0)
            };
            let avail_h = if scrolls_y {
                f32::MAX / 1024.0
            } else {
                entry.rect.height.max(0.0)
            };
            let measured = probe.measure_node(index as u32, avail_w, avail_h);
            let content_w = if scrolls_x {
                measured.0.max(entry.rect.width)
            } else {
                entry.rect.width
            };
            let content_h = if scrolls_y {
                measured.1.max(entry.rect.height)
            } else {
                entry.rect.height
            };
            contents[index] = Rect {
                x: entry.rect.x,
                y: entry.rect.y,
                width: content_w,
                height: content_h,
            };
        }
        LayoutResult { boxes, contents }
    }
}

struct Engine<'a> {
    document: &'a RuntimeDocument,
    interaction: InteractionState,
    boxes: &'a mut [LayoutBox],
}

impl<'a> Engine<'a> {
    fn style(&self, index: u32) -> Style {
        self.document.runtime_style(index, self.interaction)
    }

    fn children(&self, index: u32) -> Vec<u32> {
        let mut result = Vec::new();
        let mut cursor = self.document.document.nodes[index as usize].first_child;
        while let Some(child) = cursor {
            result.push(child);
            cursor = self.document.document.nodes[child as usize].next_sibling;
        }
        result
    }

    fn measure_node(&self, index: u32, avail_w: f32, avail_h: f32) -> (f32, f32) {
        let node = &self.document.document.nodes[index as usize];
        let style = self.style(index);
        if style.display == Display::None {
            return (0.0, 0.0);
        }
        if node.kind == NodeKind::Text {
            let text = self.document.text_for(index);
            let width = text.chars().count() as f32 * style.font_size * 0.58;
            let height = style.font_size * 1.30;
            return (
                clamp_length(width, style.min_width, style.max_width, avail_w),
                clamp_length(height, style.min_height, style.max_height, avail_h),
            );
        }
        if node.kind == NodeKind::Image {
            let asset = node
                .image
                .and_then(|asset| self.document.document.assets.get(asset as usize));
            let natural_w = asset.map(|asset| asset.width as f32).unwrap_or(0.0);
            let natural_h = asset.map(|asset| asset.height as f32).unwrap_or(0.0);
            let width = style.width.resolve(avail_w).unwrap_or(natural_w);
            let height = style.height.resolve(avail_h).unwrap_or(natural_h);
            return (
                clamp_length(width, style.min_width, style.max_width, avail_w),
                clamp_length(height, style.min_height, style.max_height, avail_h),
            );
        }

        let explicit_w = style.width.resolve(avail_w);
        let explicit_h = style.height.resolve(avail_h);
        let inner_w = explicit_w
            .unwrap_or(avail_w)
            .max(style.padding.horizontal())
            - style.padding.horizontal();
        let inner_h =
            explicit_h.unwrap_or(avail_h).max(style.padding.vertical()) - style.padding.vertical();
        let children = self.children(index);
        let mut content_w: f32 = 0.0;
        let mut content_h: f32 = 0.0;
        let mut flow_count = 0usize;

        match style.display {
            Display::None => return (0.0, 0.0),
            Display::Block => {
                for child in children {
                    let child_style = self.style(child);
                    if child_style.position == Position::Absolute
                        || child_style.display == Display::None
                    {
                        continue;
                    }
                    let (cw, ch) = self.measure_node(child, inner_w, inner_h);
                    content_w = content_w.max(cw + child_style.margin.horizontal());
                    content_h += ch + child_style.margin.vertical();
                    if flow_count > 0 {
                        content_h += style.gap;
                    }
                    flow_count += 1;
                }
            }
            Display::Flex => match style.flex_direction {
                FlexDirection::Row => {
                    for child in children {
                        let child_style = self.style(child);
                        if child_style.position == Position::Absolute
                            || child_style.display == Display::None
                        {
                            continue;
                        }
                        let (cw, ch) = self.measure_node(child, inner_w, inner_h);
                        content_w += cw + child_style.margin.horizontal();
                        content_h = content_h.max(ch + child_style.margin.vertical());
                        if flow_count > 0 {
                            content_w += style.gap;
                        }
                        flow_count += 1;
                    }
                }
                FlexDirection::Column => {
                    for child in children {
                        let child_style = self.style(child);
                        if child_style.position == Position::Absolute
                            || child_style.display == Display::None
                        {
                            continue;
                        }
                        let (cw, ch) = self.measure_node(child, inner_w, inner_h);
                        content_w = content_w.max(cw + child_style.margin.horizontal());
                        content_h += ch + child_style.margin.vertical();
                        if flow_count > 0 {
                            content_h += style.gap;
                        }
                        flow_count += 1;
                    }
                }
            },
        }

        let width = explicit_w.unwrap_or(content_w + style.padding.horizontal());
        let height = explicit_h.unwrap_or(content_h + style.padding.vertical());
        (
            clamp_length(width, style.min_width, style.max_width, avail_w),
            clamp_length(height, style.min_height, style.max_height, avail_h),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn layout_node(
        &mut self,
        index: u32,
        x: f32,
        y: f32,
        avail_w: f32,
        avail_h: f32,
        forced_w: Option<f32>,
        forced_h: Option<f32>,
    ) -> (f32, f32) {
        let node = self.document.document.nodes[index as usize].clone();
        let style = self.style(index);
        if style.display == Display::None {
            self.boxes[index as usize].rect = Rect::default();
            return (0.0, 0.0);
        }

        if node.kind == NodeKind::Text || node.kind == NodeKind::Image {
            let (measured_w, measured_h) = self.measure_node(index, avail_w, avail_h);
            let width =
                forced_w.unwrap_or_else(|| style.width.resolve(avail_w).unwrap_or(measured_w));
            let height =
                forced_h.unwrap_or_else(|| style.height.resolve(avail_h).unwrap_or(measured_h));
            let width = clamp_length(width, style.min_width, style.max_width, avail_w);
            let height = clamp_length(height, style.min_height, style.max_height, avail_h);
            self.boxes[index as usize].rect = Rect {
                x,
                y,
                width,
                height,
            };
            return (width, height);
        }

        let measured = self.measure_node(index, avail_w, avail_h);
        let mut width = forced_w
            .or_else(|| style.width.resolve(avail_w))
            .unwrap_or(measured.0.min(avail_w));
        let mut height = forced_h.or_else(|| style.height.resolve(avail_h));
        width = clamp_length(width, style.min_width, style.max_width, avail_w);
        let provisional_height = height.unwrap_or(measured.1.min(avail_h));
        let content_x = x + style.padding.left;
        let content_y = y + style.padding.top;
        let content_w = (width - style.padding.horizontal()).max(0.0);
        let content_h = (provisional_height - style.padding.vertical()).max(0.0);

        self.boxes[index as usize].rect = Rect {
            x,
            y,
            width,
            height: provisional_height,
        };
        let children = self.children(index);
        let consumed = match style.display {
            Display::None => (0.0, 0.0),
            Display::Block => self.layout_block(
                &children, content_x, content_y, content_w, content_h, style.gap,
            ),
            Display::Flex => self.layout_flex(
                &children, content_x, content_y, content_w, content_h, &style,
            ),
        };

        if height.is_none() {
            height = Some(consumed.1 + style.padding.vertical());
        }
        let final_height = clamp_length(
            height.unwrap_or(0.0),
            style.min_height,
            style.max_height,
            avail_h,
        );
        self.boxes[index as usize].rect.height = final_height;

        self.layout_absolute_children(
            index,
            content_x,
            content_y,
            content_w,
            (final_height - style.padding.vertical()).max(0.0),
        );
        (width, final_height)
    }

    fn layout_block(
        &mut self,
        children: &[u32],
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        gap: f32,
    ) -> (f32, f32) {
        let mut cursor_y = y;
        let mut max_w: f32 = 0.0;
        let mut count = 0usize;
        for &child in children {
            let child_style = self.style(child);
            if child_style.position == Position::Absolute || child_style.display == Display::None {
                continue;
            }
            if count > 0 {
                cursor_y += gap;
            }
            cursor_y += child_style.margin.top;
            let child_x = x + child_style.margin.left;
            let child_w = (width - child_style.margin.horizontal()).max(0.0);
            let forced_w = child_style.width.resolve(width).or(Some(child_w));
            let forced_h = child_style.height.resolve(height);
            let (actual_w, actual_h) = self.layout_node(
                child, child_x, cursor_y, child_w, height, forced_w, forced_h,
            );
            cursor_y += actual_h + child_style.margin.bottom;
            max_w = max_w.max(actual_w + child_style.margin.horizontal());
            count += 1;
        }
        (max_w, (cursor_y - y).max(0.0))
    }

    fn layout_flex(
        &mut self,
        children: &[u32],
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: &Style,
    ) -> (f32, f32) {
        let flow: Vec<u32> = children
            .iter()
            .copied()
            .filter(|child| {
                let child_style = self.style(*child);
                child_style.position == Position::Flow && child_style.display != Display::None
            })
            .collect();
        if flow.is_empty() {
            return (0.0, 0.0);
        }

        let is_row = style.flex_direction == FlexDirection::Row;
        let main_available = if is_row { width } else { height };
        let cross_available = if is_row { height } else { width };
        let gap_total = style.gap * flow.len().saturating_sub(1) as f32;
        let mut bases = Vec::with_capacity(flow.len());
        let mut total_base = gap_total;
        let mut total_grow = 0.0f32;

        for &child in &flow {
            let child_style = self.style(child);
            let measured = self.measure_node(child, width, height);
            let main = if is_row {
                child_style.width.resolve(width).unwrap_or(measured.0)
            } else {
                child_style.height.resolve(height).unwrap_or(measured.1)
            };
            let margin_main = if is_row {
                child_style.margin.horizontal()
            } else {
                child_style.margin.vertical()
            };
            total_base += main + margin_main;
            total_grow += child_style.flex_grow.max(0.0);
            bases.push((child, main, measured));
        }

        let remaining = (main_available - total_base).max(0.0);
        let used_main = total_base + if total_grow > 0.0 { remaining } else { 0.0 };
        let slack = (main_available - used_main).max(0.0);
        let (mut cursor, distributed_gap) = match style.justify_content {
            JustifyContent::Start => (0.0, style.gap),
            JustifyContent::Center => (slack / 2.0, style.gap),
            JustifyContent::End => (slack, style.gap),
            JustifyContent::SpaceBetween if flow.len() > 1 => {
                (0.0, style.gap + slack / (flow.len() - 1) as f32)
            }
            JustifyContent::SpaceBetween => (slack / 2.0, style.gap),
        };

        let mut max_cross: f32 = 0.0;
        for (slot, (child, base_main, measured)) in bases.into_iter().enumerate() {
            let child_style = self.style(child);
            if slot > 0 {
                cursor += distributed_gap;
            }
            let grow = if total_grow > 0.0 {
                remaining * (child_style.flex_grow.max(0.0) / total_grow)
            } else {
                0.0
            };
            let main = base_main + grow;
            let margin_main_before = if is_row {
                child_style.margin.left
            } else {
                child_style.margin.top
            };
            let margin_main_after = if is_row {
                child_style.margin.right
            } else {
                child_style.margin.bottom
            };
            let margin_cross_before = if is_row {
                child_style.margin.top
            } else {
                child_style.margin.left
            };
            let margin_cross_after = if is_row {
                child_style.margin.bottom
            } else {
                child_style.margin.right
            };
            cursor += margin_main_before;

            let explicit_cross = if is_row {
                child_style.height.resolve(height)
            } else {
                child_style.width.resolve(width)
            };
            let measured_cross = if is_row { measured.1 } else { measured.0 };
            let available_cross =
                (cross_available - margin_cross_before - margin_cross_after).max(0.0);
            let cross = match (explicit_cross, style.align_items) {
                (Some(value), _) => value.min(available_cross),
                (None, AlignItems::Stretch) => available_cross,
                (None, _) => measured_cross.min(available_cross),
            };
            let cross_offset = match style.align_items {
                AlignItems::Start | AlignItems::Stretch => margin_cross_before,
                AlignItems::Center => {
                    margin_cross_before + (available_cross - cross).max(0.0) / 2.0
                }
                AlignItems::End => margin_cross_before + (available_cross - cross).max(0.0),
            };

            let (child_x, child_y, forced_w, forced_h) = if is_row {
                (x + cursor, y + cross_offset, Some(main), Some(cross))
            } else {
                (x + cross_offset, y + cursor, Some(cross), Some(main))
            };
            let (actual_w, actual_h) =
                self.layout_node(child, child_x, child_y, width, height, forced_w, forced_h);
            let actual_cross = if is_row { actual_h } else { actual_w };
            max_cross = max_cross.max(actual_cross + margin_cross_before + margin_cross_after);
            cursor += main + margin_main_after;
        }

        if is_row {
            (cursor, max_cross)
        } else {
            (max_cross, cursor)
        }
    }

    fn layout_absolute_children(&mut self, parent: u32, x: f32, y: f32, width: f32, height: f32) {
        for child in self.children(parent) {
            let style = self.style(child);
            if style.position != Position::Absolute || style.display == Display::None {
                continue;
            }
            let left = style.left.resolve(width);
            let right = style.right.resolve(width);
            let top = style.top.resolve(height);
            let bottom = style.bottom.resolve(height);
            let forced_w = style.width.resolve(width).or_else(|| match (left, right) {
                (Some(left), Some(right)) => Some((width - left - right).max(0.0)),
                _ => None,
            });
            let forced_h = style
                .height
                .resolve(height)
                .or_else(|| match (top, bottom) {
                    (Some(top), Some(bottom)) => Some((height - top - bottom).max(0.0)),
                    _ => None,
                });
            let measured = self.measure_node(child, width, height);
            let child_w = forced_w.unwrap_or(measured.0);
            let child_h = forced_h.unwrap_or(measured.1);
            let child_x = x + left
                .unwrap_or_else(|| right.map(|value| width - value - child_w).unwrap_or(0.0));
            let child_y = y + top
                .unwrap_or_else(|| bottom.map(|value| height - value - child_h).unwrap_or(0.0));
            self.layout_node(
                child,
                child_x,
                child_y,
                width,
                height,
                Some(child_w),
                Some(child_h),
            );
        }
    }
}

fn clamp_length(value: f32, min: Length, max: Length, available: f32) -> f32 {
    let mut result = value.max(0.0);
    if let Some(minimum) = min.resolve(available) {
        result = result.max(minimum);
    }
    if let Some(maximum) = max.resolve(available) {
        result = result.min(maximum);
    }
    result
}

fn thumb_rect(
    viewport: Rect,
    content_len: f32,
    viewport_len: f32,
    offset: f32,
    horizontal: bool,
) -> Rect {
    if content_len <= viewport_len || viewport_len <= 0.0 || content_len <= 0.0 {
        return Rect::default();
    }
    let track = if horizontal {
        viewport.width
    } else {
        viewport.height
    };
    let thumb_len = (viewport_len / content_len * track).clamp(12.0, track);
    let travel = (track - thumb_len).max(0.0);
    let max_offset = (content_len - viewport_len).max(1.0);
    let pos = (offset.clamp(0.0, max_offset) / max_offset) * travel;
    if horizontal {
        Rect {
            x: viewport.x + pos,
            y: viewport.y + viewport.height - 8.0,
            width: thumb_len,
            height: 8.0,
        }
    } else {
        Rect {
            x: viewport.x + viewport.width - 8.0,
            y: viewport.y + pos,
            width: 8.0,
            height: thumb_len,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_node(parent: u32, next: Option<u32>, text: &str) -> CompiledNode {
        CompiledNode {
            kind: NodeKind::Text,
            parent: Some(parent),
            first_child: None,
            next_sibling: next,
            id: String::new(),
            action: String::new(),
            text: text.into(),
            image: None,
            style: Style::default(),
            hover_style: None,
            active_style: None,
        }
    }

    #[test]
    fn flex_grow_fills_available_width() {
        let mut left = Style::default();
        left.flex_grow = 1.0;
        let mut right = Style::default();
        right.width = Length::Px(20.0);
        let mut root_style = Style::default();
        root_style.display = Display::Flex;
        root_style.height = Length::Px(40.0);
        let document = CompiledDocument {
            source_fingerprint: 0,
            root: 0,
            variables: Vec::new(),
            assets: Vec::new(),
            nodes: vec![
                CompiledNode {
                    kind: NodeKind::Element,
                    parent: None,
                    first_child: Some(1),
                    next_sibling: None,
                    id: String::new(),
                    action: String::new(),
                    text: String::new(),
                    image: None,
                    style: root_style,
                    hover_style: None,
                    active_style: None,
                },
                CompiledNode {
                    kind: NodeKind::Element,
                    parent: Some(0),
                    first_child: Some(3),
                    next_sibling: Some(2),
                    id: String::new(),
                    action: String::new(),
                    text: String::new(),
                    image: None,
                    style: left,
                    hover_style: None,
                    active_style: None,
                },
                CompiledNode {
                    kind: NodeKind::Element,
                    parent: Some(0),
                    first_child: None,
                    next_sibling: None,
                    id: String::new(),
                    action: String::new(),
                    text: String::new(),
                    image: None,
                    style: right,
                    hover_style: None,
                    active_style: None,
                },
                text_node(1, None, "left"),
            ],
        };
        let runtime = RuntimeDocument::new(document).unwrap();
        let layout = LayoutEngine::compute(&runtime, 100.0, 40.0, InteractionState::default());
        assert!((layout.boxes[1].rect.width - 80.0).abs() < 0.01);
    }
}
