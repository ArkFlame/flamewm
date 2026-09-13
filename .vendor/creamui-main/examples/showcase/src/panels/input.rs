use crate::prelude::*;

/// The "Input" panel: every `TextInput`/`TextArea` variation side by side —
/// plain, with a placeholder, and a multi-line editor. Each is bound to its
/// own [`TextController`] rather than a hand-wired `value`/`on_change` (and,
/// for the `TextArea`, `cursor`/`selection`) pair — the controller owns that
/// state and the widget just reads and writes through it.
#[component]
pub fn InputPanel(
    plain: TextController,
    with_placeholder: TextController,
    notes: TextController,
    notes_wrapped: TextController,
) -> BoxedWidget {
    let theme = use_theme();
    fn textarea_style() -> Style {
        Style {
            size: creamui_core::layout::Size {
                width: Dimension::Length(280.0),
                height: Dimension::Length(180.0),
            },
            ..Default::default()
        }
    }
    let error_field: BoxedWidget = Box::new(jsx! {
        <RawView style={column(theme.spacing_small)}>
            <BorderedInput border={theme.danger} />
            <Text align={TextAlign::Start} color={theme.danger} style={label_style()}>"Error: this field is required"</Text>
        </RawView>
    });
    let warning_field: BoxedWidget = Box::new(jsx! {
        <RawView style={column(theme.spacing_small)}>
            <BorderedInput border={theme.warning} />
            <Text align={TextAlign::Start} color={theme.warning} style={label_style()}>"Warning: verify this value"</Text>
        </RawView>
    });

    Box::new(jsx! {
        <RawView style={column(section_gap())}>
            <SectionHeader title={"Input".to_owned()} subtitle={"Write, select, and edit. Each field keeps its own content.".to_owned()} />
            <Card gap={theme.spacing_large}>
                <StackedField label={"Default".to_owned()} control={Box::new(jsx!{<TextInput controller={&plain} />}) as BoxedWidget} />
                <StackedField label={"With placeholder".to_owned()} control={Box::new(jsx!{<TextInput controller={&with_placeholder} placeholder={"Type something…".to_owned()} />}) as BoxedWidget} />
            </Card>
            <Card gap={theme.spacing_medium}>
                <FieldLabel text={"Validation states".to_owned()} />
                <CardRow>
                    {error_field}
                    {warning_field}
                </CardRow>
            </Card>
            <Card gap={theme.spacing_medium}>
                <FieldLabel text={"Text area".to_owned()} />
                <CardRow>
                    <StackedField label={"Horizontal scrolling".to_owned()} control={Box::new(jsx!{<TextArea controller={&notes} style={textarea_style()} placeholder={"Notes…".to_owned()} />}) as BoxedWidget} />
                    <StackedField label={"Wrap to fit".to_owned()} control={Box::new(jsx!{<TextArea controller={&notes_wrapped} style={textarea_style()} wrap={true} placeholder={"Notes…".to_owned()} />}) as BoxedWidget} />
                </CardRow>
            </Card>
        </RawView>
    })
}
