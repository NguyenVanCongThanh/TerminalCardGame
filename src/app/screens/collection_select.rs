use crossterm::event::{KeyCode, KeyEvent};

use crate::app::context::AppContext;
use crate::app::screens::{Screen, ScreenAction, ScreenController};

#[derive(Default)]
pub struct CollectionSelectScreen {
    pub collection_idx: usize,
}

impl CollectionSelectScreen {
    fn sync_selected_collection(&self, ctx: &mut AppContext) {
        if let Some(col) = ctx.collections.get(self.collection_idx) {
            ctx.selected_collection_id = col.id.clone();
            ctx.refresh_selected_collection_rarities();
        }
    }
}

impl ScreenController for CollectionSelectScreen {
    fn screen(&self) -> Screen {
        Screen::CollectionSelect
    }

    fn title(&self) -> &'static str {
        "Collections"
    }

    fn footer_hints(&self, compact: bool) -> &'static str {
        if compact {
            "[↑/↓] Set  [Enter] Open  [s] Sync  [r] Refresh  [Esc] Back"
        } else {
            "[↑/↓] Select Set  [Enter] Open Packs  [s] Sync Cards  [r] Refresh Sets  [?] Help  [Esc] Back"
        }
    }

    fn on_enter(&mut self, ctx: &mut AppContext) {
        ctx.refresh_cached_stats(None);
        if self.collection_idx >= ctx.collections.len() {
            self.collection_idx = 0;
        }
        self.sync_selected_collection(ctx);
    }

    fn handle_key(&mut self, key: KeyEvent, ctx: &mut AppContext) -> ScreenAction {
        let len = ctx.collections.len();
        match key.code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                ScreenAction::Navigate(Screen::MainMenu)
            }
            KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                self.collection_idx = self.collection_idx.checked_sub(1).unwrap_or(len - 1);
                self.sync_selected_collection(ctx);
                ScreenAction::None
            }
            KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                self.collection_idx = (self.collection_idx + 1) % len;
                self.sync_selected_collection(ctx);
                ScreenAction::None
            }
            KeyCode::Char('s') => {
                if let Some(col) = ctx.collections.get(self.collection_idx).cloned() {
                    ctx.spawn_sync_collection_cards(col.id, false);
                }
                ScreenAction::None
            }
            KeyCode::Char('r') => {
                ctx.spawn_sync_collections_list();
                ScreenAction::None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(col) = ctx.collections.get(self.collection_idx).cloned() {
                    ctx.selected_collection_id = col.id.clone();
                    ctx.refresh_selected_collection_rarities();
                    let cached = ctx.cached_card_counts.get(&col.id).copied().unwrap_or(0);
                    if cached > 0 {
                        return ScreenAction::Navigate(Screen::PackSelect);
                    }
                    ctx.spawn_sync_collection_cards(col.id, true);
                }
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }
}

