pub mod context;
pub mod screens;

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::error::AppResult;
use crate::images::ImageCache;
use crate::images::renderer::ImageRenderer;
use crate::providers::ProviderRegistry;
use crate::storage::sqlite::SqliteStore;

pub use context::{ActiveModal, AppContext, AsyncMessage, ConfirmAction, StatusBanner};
pub use screens::Screen;
pub use screens::card_details::{CardDetailsScreen, InspectableCard};
pub use screens::collection_select::CollectionSelectScreen;
pub use screens::main_menu::MainMenuScreen;
pub use screens::my_collection::{CollectionSort, MyCollectionScreen};
pub use screens::pack_opening::{PackOpeningScreen, PackOpeningState, PackPhase};
pub use screens::pack_select::PackSelectScreen;
pub use screens::settings::SettingsScreen;
pub use screens::universe_select::UniverseSelectScreen;
pub use screens::{ScreenAction, ScreenController};

/// Lean top-level application coordinator / screen router.
///
/// Shared infrastructure and cached domain state live in [`AppContext`], while each
/// screen's local state, key handling, and tick updates live in its own [`ScreenController`].
pub struct App {
    pub screen: Screen,
    pub ctx: AppContext,

    pub main_menu: MainMenuScreen,
    pub universe_select: UniverseSelectScreen,
    pub collection_select: CollectionSelectScreen,
    pub pack_select: PackSelectScreen,
    pub pack_opening_screen: PackOpeningScreen,
    pub my_collection: MyCollectionScreen,
    pub card_details: CardDetailsScreen,
    pub settings_screen: SettingsScreen,
}

impl Deref for App {
    type Target = AppContext;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl DerefMut for App {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}

impl App {
    pub fn new(
        store: Arc<SqliteStore>,
        providers: Arc<ProviderRegistry>,
        image_cache: Arc<ImageCache>,
        renderer: ImageRenderer,
    ) -> AppResult<Self> {
        let ctx = AppContext::new(store, providers, image_cache, renderer)?;
        Ok(Self {
            screen: Screen::MainMenu,
            ctx,
            main_menu: MainMenuScreen::default(),
            universe_select: UniverseSelectScreen::default(),
            collection_select: CollectionSelectScreen::default(),
            pack_select: PackSelectScreen::default(),
            pack_opening_screen: PackOpeningScreen::default(),
            my_collection: MyCollectionScreen::default(),
            card_details: CardDetailsScreen::default(),
            settings_screen: SettingsScreen::default(),
        })
    }

    pub fn active_controller(&self) -> &dyn ScreenController {
        match self.screen {
            Screen::MainMenu => &self.main_menu,
            Screen::UniverseSelect => &self.universe_select,
            Screen::CollectionSelect => &self.collection_select,
            Screen::PackSelect => &self.pack_select,
            Screen::PackOpening => &self.pack_opening_screen,
            Screen::MyCollection => &self.my_collection,
            Screen::CardDetails => &self.card_details,
            Screen::Settings => &self.settings_screen,
        }
    }

    fn transition_to(&mut self, next: Screen) {
        self.screen = next;
        let ctx = &mut self.ctx;
        match next {
            Screen::MainMenu => self.main_menu.on_enter(ctx),
            Screen::UniverseSelect => self.universe_select.on_enter(ctx),
            Screen::CollectionSelect => self.collection_select.on_enter(ctx),
            Screen::PackSelect => self.pack_select.on_enter(ctx),
            Screen::PackOpening => self.pack_opening_screen.on_enter(ctx),
            Screen::MyCollection => self.my_collection.on_enter(ctx),
            Screen::CardDetails => self.card_details.on_enter(ctx),
            Screen::Settings => self.settings_screen.on_enter(ctx),
        }
    }

    fn apply_screen_action(&mut self, action: ScreenAction) {
        match action {
            ScreenAction::None => {}
            ScreenAction::Navigate(target) => self.transition_to(target),
            ScreenAction::OpenPack(pack_idx) => {
                if let Some(result) = self.ctx.open_pack(pack_idx) {
                    self.pack_opening_screen
                        .start(result, self.ctx.settings.fast_animations);
                    self.transition_to(Screen::PackOpening);
                }
            }
            ScreenAction::OpenCardDetails {
                cards,
                idx,
                return_screen,
            } => {
                self.card_details
                    .open(cards, idx, return_screen, &mut self.ctx);
                self.screen = Screen::CardDetails;
            }
            ScreenAction::Quit => {
                self.ctx.running = false;
            }
        }
    }

    pub fn on_tick(&mut self) {
        self.ctx.tick_banner();
        let ctx = &mut self.ctx;
        match self.screen {
            Screen::MainMenu => self.main_menu.on_tick(ctx),
            Screen::UniverseSelect => self.universe_select.on_tick(ctx),
            Screen::CollectionSelect => self.collection_select.on_tick(ctx),
            Screen::PackSelect => self.pack_select.on_tick(ctx),
            Screen::PackOpening => self.pack_opening_screen.on_tick(ctx),
            Screen::MyCollection => self.my_collection.on_tick(ctx),
            Screen::CardDetails => self.card_details.on_tick(ctx),
            Screen::Settings => self.settings_screen.on_tick(ctx),
        }
    }

    pub fn handle_async_message(&mut self, msg: AsyncMessage) {
        let filter = self.my_collection.collection_filter.clone();
        if let Some(next_screen) =
            self.ctx
                .handle_async_message(msg, self.screen, filter.as_deref())
        {
            self.transition_to(next_screen);
        }
        self.my_collection.clamp_cursor(&self.ctx.owned_cards);
    }

    pub fn handle_terminal_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return;
            }
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                self.ctx.running = false;
                return;
            }
            self.handle_key(key);
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.ctx.modal.is_some() {
            self.handle_modal_key(key);
            return;
        }
        if key.code == KeyCode::Char('?') && !self.active_controller().blocks_global_help() {
            self.ctx.modal = Some(ActiveModal::Help);
            return;
        }
        let ctx = &mut self.ctx;
        let action = match self.screen {
            Screen::MainMenu => self.main_menu.handle_key(key, ctx),
            Screen::UniverseSelect => self.universe_select.handle_key(key, ctx),
            Screen::CollectionSelect => self.collection_select.handle_key(key, ctx),
            Screen::PackSelect => self.pack_select.handle_key(key, ctx),
            Screen::PackOpening => self.pack_opening_screen.handle_key(key, ctx),
            Screen::MyCollection => self.my_collection.handle_key(key, ctx),
            Screen::CardDetails => self.card_details.handle_key(key, ctx),
            Screen::Settings => self.settings_screen.handle_key(key, ctx),
        };
        self.apply_screen_action(action);
    }

    fn handle_modal_key(&mut self, key: KeyEvent) {
        let Some(modal) = self.ctx.modal.clone() else {
            return;
        };
        match modal {
            ActiveModal::Help => match key.code {
                KeyCode::Esc
                | KeyCode::Enter
                | KeyCode::Char(' ')
                | KeyCode::Char('?')
                | KeyCode::Char('q') => {
                    self.ctx.modal = None;
                }
                _ => {}
            },
            ActiveModal::Confirm { action, .. } => match key.code {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => {
                    self.ctx.modal = None;
                }
                KeyCode::Enter | KeyCode::Char('y') => {
                    self.ctx.modal = None;
                    self.ctx.execute_confirm_action(action);
                }
                _ => {}
            },
        }
    }
}
