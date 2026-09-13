use crate::prelude::*;

#[component]
pub fn ImagesPanel(png: ImageData, jpeg: ImageData, webp: ImageData) -> BoxedWidget {
    let theme = use_theme();
    let square = Style {
        size: fixed(168., 168.),
        flex_shrink: 0.,
        ..Default::default()
    };
    let landscape = Style {
        size: fixed(210., 148.),
        flex_shrink: 0.,
        ..Default::default()
    };
    Box::new(jsx! {
        <RawView style={column(section_gap())}>
            <SectionHeader
                title={"Images".to_owned()}
                subtitle={"Local PNG, JPEG, and WebP assets, each cropped with a different fit and shape.".to_owned()}
            />
            <CardRow>
                <FieldCard label={"PNG · square".to_owned()} control={Box::new(jsx!{<Image data={png} style={square.clone()} fit={ImageFit::Cover} />}) as BoxedWidget} />
                <FieldCard label={"JPEG · rounded corners".to_owned()} control={Box::new(jsx!{<Image data={jpeg} style={landscape} fit={ImageFit::Cover} corner_radius={theme.card_radius} />}) as BoxedWidget} />
                <FieldCard label={"WebP · full circle".to_owned()} control={Box::new(jsx!{<Image data={webp} style={square} fit={ImageFit::Cover} corner_radius={84.0} />}) as BoxedWidget} />
            </CardRow>
        </RawView>
    })
}
