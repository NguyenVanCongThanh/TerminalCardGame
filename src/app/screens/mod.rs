pub mod card_details;
pub mod collection_select;
pub mod main_menu;
pub mod my_collection;
pub mod pack_opening;
pub mod pack_select;
pub mod settings;
pub mod universe_select;

use crossterm::event::KeyEvent;

use crate::app::context::AppContext;
use crate::app::screens::card_details::InspectableCard;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    UniverseSelect,
    CollectionSelect,
    PackSelect,
    PackOpening,
    MyCollection,
    CardDetails,
    Settings,
}

/// High-level navigation & cross-screen effects returned by a `ScreenController`.
pub enum ScreenAction {
    None,
    Navigate(Screen),
    OpenPack(usize),
    OpenCardDetails {
        cards: Vec<InspectableCard>,
        idx: usize,
        return_screen: Screen,
    },
    Quit,
}

/// Contract implemented by each screen controller to encapsulate its own state and input handling.
pub trait ScreenController {
    fn screen(&self) -> Screen;
    fn title(&self) -> &'static str;
    fn footer_hints(&self, compact: bool) -> &'static str;

    /// Return `true` when the screen is capturing raw text input (e.g. search bar)
    /// and global shortcuts like `?` should not intercept keys.
    fn blocks_global_help(&self) -> bool {
        false
    }

    fn on_enter(&mut self, _ctx: &mut AppContext) {}
    fn on_tick(&mut self, _ctx: &mut AppContext) {}
    fn handle_key(&mut self, key: KeyEvent, ctx: &mut AppContext) -> ScreenAction;
}

