use crossterm::event::{KeyCode, KeyEvent};

use crate::app::context::AppContext;
use crate::app::screens::{Screen, ScreenAction, ScreenController};

#[derive(Default)]
pub struct PackSelectScreen {
    pub pack_idx: usize,
}

impl ScreenController for PackSelectScreen {
    fn screen(&self) -> Screen {
        Screen::PackSelect
    }

    fn title(&self) -> &'static str {
        "Pack Selection"
    }

    fn footer_hints(&self, _compact: bool) -> &'static str {
        "[↑/↓] Choose Pack  [Enter/Space] Rip Pack!  [s] Re-sync  [?] Help  [Esc] Back"
    }

    fn on_enter(&mut self, ctx: &mut AppContext) {
        self.pack_idx = 0;
        ctx.active_pack_idx = 0;
    }

    fn handle_key(&mut self, key: KeyEvent, ctx: &mut AppContext) -> ScreenAction {
        let len = ctx.pack_configs.len();
        match key.code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                ScreenAction::Navigate(Screen::CollectionSelect)
            }
            KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                self.pack_idx = self.pack_idx.checked_sub(1).unwrap_or(len - 1);
                ctx.active_pack_idx = self.pack_idx;
                ScreenAction::None
            }
            KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                self.pack_idx = (self.pack_idx + 1) % len;
                ctx.active_pack_idx = self.pack_idx;
                ScreenAction::None
            }
            KeyCode::Char('s') => {
                ctx.spawn_sync_collection_cards(ctx.selected_collection_id.clone(), false);
                ScreenAction::None
            }
            KeyCode::Enter | KeyCode::Char(' ') => ScreenAction::OpenPack(self.pack_idx),
            _ => ScreenAction::None,
        }
    }
}

