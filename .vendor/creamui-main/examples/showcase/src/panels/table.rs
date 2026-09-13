use crate::prelude::*;

/// The "Table" panel: the Data View category's column-based entry — a CSV
/// viewer's shape. `Table` only needs `Vec<Vec<String>>`, so these rows are
/// parsed from a plain comma-separated string with `str::split` below, not
/// a CSV crate — however an application gets to that shape is up to it.
#[component]
pub fn TablePanel(table_scroll: ScrollController, table_selected: Signal<usize>) -> BoxedWidget {
    const CSV: &str = "Ada Lovelace,Mathematician,1815
Grace Hopper,Programmer,1906
Alan Turing,Mathematician,1912
Margaret Hamilton,Engineer,1936
Katherine Johnson,Physicist,1918
Barbara Liskov,Computer Scientist,1939
Radia Perlman,Engineer,1951";
    let rows: Vec<Vec<String>> = CSV
        .lines()
        .map(|line| line.split(',').map(|cell| cell.to_owned()).collect())
        .collect();
    let columns = vec![
        TableColumn::new("Name", 160.0),
        TableColumn::new("Field", 160.0),
        TableColumn::new("Born", 70.0),
    ];
    let selected = table_selected.get();
    let set_selected = table_selected.clone();
    let table_style = Style {
        size: creamui_core::layout::Size {
            width: Dimension::Length(390.0),
            height: Dimension::Length(280.0),
        },
        flex_shrink: 0.0,
        ..Default::default()
    };
    Box::new(jsx! {
        <RawView style={column(section_gap())}>
            <SectionHeader title={"Table".to_owned()} subtitle={"A CSV-shaped grid: fixed columns, a header that stays put, and a scrollable body. Click a row to select it.".to_owned()} />
            <FieldCard
                label={"Notable computer scientists".to_owned()}
                control={jsx!{
                    <Table
                        style={table_style}
                        scroll={table_scroll}
                        columns={columns}
                        rows={rows}
                        selected={Some(selected)}
                        on_row_click={Box::new(move |index| set_selected.set(index)) as Box<dyn Fn(usize)>}
                    />
                }}
            />
        </RawView>
    })
}
