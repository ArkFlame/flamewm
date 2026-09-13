//! A custom calculator component built with static JSX and headless widgets.
//!
//! It composes `RawView`, `RawText`, and `RawButton`, the same primitives an
//! application can use for its own design system.

use creamui_core::layout::{AlignItems, Dimension, FlexDirection, JustifyContent, Style};
use creamui_core::{BoxedWidget, Size, TextAlign};
use creamui_macros::{component, jsx};
use creamui_reactive::Signal;
use creamui_render::{run, WindowOptions};
use creamui_theme::Color;
use creamui_widgets::layout::{centered, fixed, row};

const PANEL: Color = Color::rgb(0x18, 0x19, 0x1e);
const KEY: Color = Color::rgb(0x2c, 0x2c, 0x2e);
const UTILITY_KEY: Color = Color::rgb(0xa5, 0xa5, 0xad);
const ORANGE: Color = Color::rgb(0xff, 0x9f, 0x0a);
const BLUE: Color = Color::rgb(0x0a, 0x84, 0xff);
const WHITE: Color = Color::rgb(0xf7, 0xf7, 0xfa);
const DARK_TEXT: Color = Color::rgb(0x1c, 0x1c, 0x1e);

fn apply(expression: &mut String, key: &str) {
    match key {
        "C" => expression.clear(),
        "±" => {
            if expression.starts_with('-') {
                expression.remove(0);
            } else if !expression.is_empty() {
                expression.insert(0, '-');
            }
        }
        "%" => {
            if let Ok(value) = expression.parse::<f64>() {
                *expression = (value / 100.0).to_string();
            }
        }
        "=" => {
            if let Some((index, operator)) = expression
                .char_indices()
                .skip(1)
                .find_map(|(index, c)| matches!(c, '+' | '-' | '×' | '÷').then_some((index, c)))
            {
                let (left, right) = expression.split_at(index);
                if let (Ok(left), Ok(right)) = (
                    left.parse::<f64>(),
                    right[operator.len_utf8()..].parse::<f64>(),
                ) {
                    let value = match operator {
                        '+' => left + right,
                        '-' => left - right,
                        '×' => left * right,
                        '÷' if right != 0.0 => left / right,
                        _ => return,
                    };
                    *expression = if value.fract() == 0.0 {
                        (value as i64).to_string()
                    } else {
                        format!("{value:.8}")
                            .trim_end_matches('0')
                            .trim_end_matches('.')
                            .to_owned()
                    };
                }
            }
        }
        _ => expression.push_str(key),
    }
}

fn key_style(width: f32) -> Style {
    centered(Style {
        size: fixed(width, 70.0),
        ..Default::default()
    })
}

#[component]
fn CalculatorKey(
    label: String,
    width: f32,
    background: Color,
    text_color: Color,
    expression: Signal<String>,
) -> BoxedWidget {
    let label_for_click = label.clone();
    Box::new(jsx! {
        <RawButton style={key_style(width)} background={background} corner_radius={26.0} on_click={move || expression.update(|value| apply(value, &label_for_click))}>
            <RawText color={text_color} font_size={23.0}>{label}</RawText>
        </RawButton>
    })
}

#[component]
fn CalculatorRow(children: Vec<BoxedWidget>) -> BoxedWidget {
    let style = Style {
        size: fixed(420.0, 70.0),
        ..row(12.0)
    };
    Box::new(jsx! { <RawView style={style} children={children} /> })
}

#[component]
fn CalculatorPanel(expression: Signal<String>) -> BoxedWidget {
    let value = expression.get();
    let display = if value.is_empty() { "0" } else { &value };
    let panel_style = Style {
        flex_direction: FlexDirection::Column,
        gap: creamui_core::layout::Size {
            width: creamui_core::layout::LengthPercentage::Length(0.0),
            height: creamui_core::layout::LengthPercentage::Length(12.0),
        },
        ..Default::default()
    };
    let display_style = Style {
        size: fixed(420.0, 128.0),
        flex_direction: FlexDirection::Column,
        justify_content: Some(JustifyContent::FlexEnd),
        align_items: Some(AlignItems::FlexEnd),
        ..Default::default()
    };
    let value_style = Style {
        size: fixed(420.0, 80.0),
        ..Default::default()
    };

    Box::new(jsx! {
        <RawView style={panel_style}>
            <RawView style={display_style}>
                <RawText color={WHITE} font_size={56.0} align={TextAlign::End} style={value_style}>{display}</RawText>
            </RawView>
            <CalculatorRow>
                <CalculatorKey label={"C".to_owned()} width={96.0} background={UTILITY_KEY} text_color={DARK_TEXT} expression={expression.clone()} />
                <CalculatorKey label={"±".to_owned()} width={96.0} background={UTILITY_KEY} text_color={DARK_TEXT} expression={expression.clone()} />
                <CalculatorKey label={"%".to_owned()} width={96.0} background={UTILITY_KEY} text_color={DARK_TEXT} expression={expression.clone()} />
                <CalculatorKey label={"÷".to_owned()} width={96.0} background={ORANGE} text_color={WHITE} expression={expression.clone()} />
            </CalculatorRow>
            <CalculatorRow>
                <CalculatorKey label={"7".to_owned()} width={96.0} background={KEY} text_color={WHITE} expression={expression.clone()} />
                <CalculatorKey label={"8".to_owned()} width={96.0} background={KEY} text_color={WHITE} expression={expression.clone()} />
                <CalculatorKey label={"9".to_owned()} width={96.0} background={KEY} text_color={WHITE} expression={expression.clone()} />
                <CalculatorKey label={"×".to_owned()} width={96.0} background={ORANGE} text_color={WHITE} expression={expression.clone()} />
            </CalculatorRow>
            <CalculatorRow>
                <CalculatorKey label={"4".to_owned()} width={96.0} background={KEY} text_color={WHITE} expression={expression.clone()} />
                <CalculatorKey label={"5".to_owned()} width={96.0} background={KEY} text_color={WHITE} expression={expression.clone()} />
                <CalculatorKey label={"6".to_owned()} width={96.0} background={KEY} text_color={WHITE} expression={expression.clone()} />
                <CalculatorKey label={"-".to_owned()} width={96.0} background={ORANGE} text_color={WHITE} expression={expression.clone()} />
            </CalculatorRow>
            <CalculatorRow>
                <CalculatorKey label={"1".to_owned()} width={96.0} background={KEY} text_color={WHITE} expression={expression.clone()} />
                <CalculatorKey label={"2".to_owned()} width={96.0} background={KEY} text_color={WHITE} expression={expression.clone()} />
                <CalculatorKey label={"3".to_owned()} width={96.0} background={KEY} text_color={WHITE} expression={expression.clone()} />
                <CalculatorKey label={"+".to_owned()} width={96.0} background={ORANGE} text_color={WHITE} expression={expression.clone()} />
            </CalculatorRow>
            <CalculatorRow>
                <CalculatorKey label={"0".to_owned()} width={204.0} background={KEY} text_color={WHITE} expression={expression.clone()} />
                <CalculatorKey label={".".to_owned()} width={96.0} background={KEY} text_color={WHITE} expression={expression.clone()} />
                <CalculatorKey label={"=".to_owned()} width={96.0} background={BLUE} text_color={WHITE} expression={expression.clone()} />
            </CalculatorRow>
        </RawView>
    })
}

fn main() {
    let expression = Signal::new(String::new());
    run(
        WindowOptions {
            title: "CreamUI — Calculator".into(),
            width: 460,
            height: 600,
            ..Default::default()
        },
        PANEL,
        |_| {},
        move |viewport: Size| -> BoxedWidget {
            let root_style = centered(Style {
                size: creamui_core::layout::Size {
                    width: Dimension::Length(viewport.width),
                    height: Dimension::Length(viewport.height),
                },
                flex_direction: FlexDirection::Column,
                justify_content: Some(JustifyContent::Center),
                ..Default::default()
            });
            Box::new(jsx! {
                <RawView style={root_style} background={PANEL}>
                    <CalculatorPanel expression={expression.clone()} />
                </RawView>
            })
        },
    );
}
