use crate::layout::LayoutResult;
use crate::model::*;

#[derive(Clone, Debug, PartialEq)]
pub enum PaintCommand {
    FillRect {
        rect: Rect,
        color: Color,
        radius: f32,
    },
    StrokeRect {
        rect: Rect,
        color: Color,
        width: f32,
        radius: f32,
    },
    Image {
        rect: Rect,
        asset: u16,
    },
    Text {
        x: f32,
        y: f32,
        color: Color,
        size: f32,
        weight: u16,
        text: String,
    },
}

pub fn build_paint_commands(
    document: &RuntimeDocument,
    layout: &LayoutResult,
    interaction: InteractionState,
) -> Vec<PaintCommand> {
    let mut commands = Vec::with_capacity(document.document.nodes.len() * 2);
    let mut indices: Vec<usize> = (0..document.document.nodes.len()).collect();
    indices.sort_by_key(|index| (document.effective_z_index(*index as u32), *index as i64));
    for index in indices {
        let node = &document.document.nodes[index];
        if !document.is_effectively_visible(index as u32) {
            continue;
        }
        let style = document.runtime_style(index as u32, interaction);
        if style.display == Display::None {
            continue;
        }
        let rect = layout
            .boxes
            .get(index)
            .map(|entry| entry.rect)
            .unwrap_or_default();
        if rect.width <= 0.0 || rect.height <= 0.0 {
            continue;
        }
        if node.kind == NodeKind::Element || node.kind == NodeKind::Image {
            let background = apply_opacity(document.resolve_color(style.background), style.opacity);
            if background.a != 0 {
                commands.push(PaintCommand::FillRect {
                    rect,
                    color: background,
                    radius: style.border_radius.max(0.0),
                });
            }
            let border = apply_opacity(document.resolve_color(style.border_color), style.opacity);
            if style.border_width > 0.0 && border.a != 0 {
                commands.push(PaintCommand::StrokeRect {
                    rect,
                    color: border,
                    width: style.border_width,
                    radius: style.border_radius.max(0.0),
                });
            }
        }
        match node.kind {
            NodeKind::Image => {
                if let Some(asset) = node.image {
                    commands.push(PaintCommand::Image { rect, asset });
                }
            }
            NodeKind::Text => {
                let color = apply_opacity(document.resolve_color(style.color), style.opacity);
                if color.a != 0 {
                    let text = document.text_for(index as u32);
                    if !text.is_empty() {
                        commands.push(PaintCommand::Text {
                            x: rect.x,
                            y: rect.y + style.font_size,
                            color,
                            size: style.font_size,
                            weight: style.font_weight,
                            text: text.to_string(),
                        });
                    }
                }
            }
            NodeKind::Element => {}
        }
    }
    commands
}

fn apply_opacity(mut color: Color, opacity: f32) -> Color {
    let opacity = opacity.clamp(0.0, 1.0);
    color.a = ((color.a as f32) * opacity).round() as u8;
    color
}
