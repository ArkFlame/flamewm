pub mod button;
pub mod card;
pub mod grid;
pub mod icon;
pub mod label;
pub mod list;
pub mod menu;
pub mod overlay;
pub mod popup;
pub mod scroll_view;
pub mod selection_rect;
pub mod separator;
pub mod slider;
pub mod spacer;
pub mod stack;
pub mod text_input;
pub mod toggle;

use button::Button;
use card::Card;
use grid::Grid;
use icon::Icon;
use label::Label;
use list::List;
use menu::Menu;
use overlay::Overlay;
use popup::Popup;
use scroll_view::ScrollView;
use selection_rect::SelectionRect;
use separator::Separator;
use slider::Slider;
use spacer::Spacer;
use stack::Stack;
use text_input::TextInput;
use toggle::Toggle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetKind {
    Button(Button),
    Toggle(Toggle),
    TextInput(TextInput),
    Label(Label),
    Icon(Icon),
    List(List),
    Menu(Menu),
    Popup(Popup),
    Slider(Slider),
    Separator(Separator),
    Spacer(Spacer),
    Stack(Stack),
    Grid(Grid),
    ScrollView(ScrollView),
    Card(Card),
    Overlay(Overlay),
    SelectionRect(SelectionRect),
}
