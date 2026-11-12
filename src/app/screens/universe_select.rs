use crossterm::event::{KeyCode, KeyEvent};

use crate::app::context::AppContext;
use crate::app::screens::{Screen, ScreenAction, ScreenController};

#[derive(Default)]
pub struct UniverseSelectScreen {
    pub universe_idx: usize,
}

impl ScreenController for UniverseSelectScreen {
    fn screen(&self) -> Screen {
        Screen::UniverseSelect
    }

    fn title(&self) -> &'static str {
        "Universes"
    }

    fn footer_hints(&self, _compact: bool) -> &'static str {
        "[↑/↓] Select Universe  [Enter] Open  [?] Help  [Esc] Back"
    }

    fn handle_key(&mut self, key: KeyEvent, ctx: &mut AppContext) -> ScreenAction {
        let len = ctx.universes.len();
        match key.code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                ScreenAction::Navigate(Screen::MainMenu)
            }
            KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                self.universe_idx = self.universe_idx.checked_sub(1).unwrap_or(len - 1);
                ScreenAction::None
            }
            KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                self.universe_idx = (self.universe_idx + 1) % len;
                ScreenAction::None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(u) = ctx.universes.get(self.universe_idx).cloned() {
                    if u.is_implemented {
                        ctx.selected_universe_id = u.id;
                        ctx.refresh_cached_stats(None);
                        return ScreenAction::Navigate(Screen::CollectionSelect);
                    }
                    ctx.set_banner(
                        format!("'{}' is a planned future universe slot!", u.name),
                        false,
                    );
                }
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }
}

