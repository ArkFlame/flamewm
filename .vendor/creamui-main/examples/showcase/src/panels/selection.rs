use crate::prelude::*;

/// Select, radio, and segmented selection share a page. `SegmentedControl`
/// owns a row of the lower-level `Choice` items, so the demo does not repeat
/// the same interaction as two competing controls.
#[component]
pub fn SelectionPanel(
    select: SelectController,
    radio: Signal<usize>,
    segment: Signal<usize>,
    list_scroll: ScrollController,
    list_selected: Signal<usize>,
) -> BoxedWidget {
    const OPTIONS: [&str; 3] = ["System", "Light", "Dark"];
    const FRUITS: [&str; 16] = [
        "Apple",
        "Banana",
        "Cherry",
        "Date",
        "Elderberry",
        "Fig",
        "Grape",
        "Honeydew",
        "Kiwi",
        "Lemon",
        "Mango",
        "Nectarine",
        "Orange",
        "Papaya",
        "Quince",
        "Raspberry",
    ];
    let radio_value = radio.get();
    let segment_value = segment.get();
    let set_radio = radio.clone();
    let set_segment = segment.clone();
    let list_value = list_selected.get();
    let set_list = list_selected.clone();
    let list_box_style = Style {
        size: creamui_core::layout::Size {
            width: Dimension::Length(220.0),
            height: Dimension::Length(200.0),
        },
        flex_shrink: 0.0,
        ..Default::default()
    };
    Box::new(jsx! {
        <RawView style={column(section_gap())}>
            <SectionHeader title={"Selection".to_owned()} subtitle={"Choose one value with a popup, explanatory radios, compact Choice segments, or a scrollable list.".to_owned()} />
            <FieldCard
                label={"Select / ComboBox".to_owned()}
                control={jsx!{<Select options={OPTIONS.iter().map(|o| o.to_string()).collect::<Vec<_>>()} controller={select} />}}
            />
            <CardRow>
                <FieldCard
                    label={"Radio group".to_owned()}
                    control={jsx!{
                        <RadioGroup
                            selected={radio_value}
                            on_change={Box::new(move |index| set_radio.set(index)) as Box<dyn Fn(usize)>}
                            options={vec![
                                "Keep files on this device".to_owned(),
                                "Sync encrypted copies".to_owned(),
                                "Never sync".to_owned(),
                            ]}
                        />
                    }}
                />
                <FieldCard
                    label={"Segmented control".to_owned()}
                    control={jsx!{
                        <SegmentedControl
                            selected={segment_value}
                            on_change={Box::new(move |index| set_segment.set(index)) as Box<dyn Fn(usize)>}
                            options={vec!["Day".to_owned(), "Week".to_owned(), "Month".to_owned()]}
                        />
                    }}
                />
            </CardRow>
            <FieldCard
                label={format!("List box · {}", FRUITS[list_value])}
                control={jsx!{
                    <ListBox
                        style={list_box_style}
                        scroll={list_scroll}
                        selected={list_value}
                        on_change={Box::new(move |index| set_list.set(index)) as Box<dyn Fn(usize)>}
                        options={FRUITS.iter().map(|o| o.to_string()).collect::<Vec<_>>()}
                    />
                }}
            />
        </RawView>
    })
}
