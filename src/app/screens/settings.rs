use crossterm::event::{KeyCode, KeyEvent};

use crate::app::context::{ActiveModal, AppContext, ConfirmAction};
use crate::app::screens::{Screen, ScreenAction, ScreenController};
use crate::storage::SettingsRepository;

#[derive(Default)]
pub struct SettingsScreen {
    pub idx: usize,
}

impl SettingsScreen {
    fn adjust_setting(&mut self, ctx: &mut AppContext, forward: bool, is_activate: bool) {
        match self.idx {
            0 => {
                ctx.settings.protocol_mode = if forward {
                    ctx.settings.protocol_mode.next()
                } else {
                    ctx.settings.protocol_mode.prev()
                };
                ctx.renderer.set_mode(ctx.settings.protocol_mode);
                for slot in ctx.image_slots.values_mut() {
                    slot.clear();
                }
                let _ = ctx.store.save_settings(&ctx.settings);
                ctx.set_banner(
                    format!(
                        "Image protocol set to: {}",
                        ctx.settings.protocol_mode.label()
                    ),
                    false,
                );
            }
            1 => {
                ctx.settings.image_quality = ctx.settings.image_quality.toggle();
                let _ = ctx.store.save_settings(&ctx.settings);
                ctx.set_banner(
                    format!("Artwork quality: {}", ctx.settings.image_quality.label()),
                    false,
                );
            }
            2 => {
                ctx.settings.fast_animations = !ctx.settings.fast_animations;
                let _ = ctx.store.save_settings(&ctx.settings);
            }
            3 => {
                const SIZES: [usize; 4] = [16, 28, 40, 60];
                let idx = SIZES
                    .iter()
                    .position(|&s| s == ctx.settings.cards_per_sync)
                    .unwrap_or(1);
                let next = if forward {
                    (idx + 1) % SIZES.len()
                } else {
                    idx.checked_sub(1).unwrap_or(SIZES.len() - 1)
                };
                ctx.settings.cards_per_sync = SIZES[next];
                let _ = ctx.store.save_settings(&ctx.settings);
            }
            4 if is_activate => {
                ctx.modal = Some(ActiveModal::Confirm {
                    title: "Clear Local Image Cache?".to_string(),
                    message: format!(
                        "This will delete {} cached PNG files ({:.1} KB). Your synced cards, inventory, and pack history remain safe.",
                        ctx.cache_stats.file_count,
                        ctx.cache_stats.total_bytes as f64 / 1024.0
                    ),
                    action: ConfirmAction::ClearImageCache,
                });
            }
            _ => {}
        }
    }
}

impl ScreenController for SettingsScreen {
    fn screen(&self) -> Screen {
        Screen::Settings
    }

    fn title(&self) -> &'static str {
        "Settings"
    }

    fn footer_hints(&self, _compact: bool) -> &'static str {
        "[↑/↓] Select Option  [←/→] Change  [Enter] Activate  [?] Help  [Esc] Back"
    }

    fn on_enter(&mut self, ctx: &mut AppContext) {
        ctx.cache_stats = ctx.image_cache.stats();
    }

    fn handle_key(&mut self, key: KeyEvent, ctx: &mut AppContext) -> ScreenAction {
        const SETTINGS_ROWS: usize = 5;
        match key.code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => {
                ScreenAction::Navigate(Screen::MainMenu)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.idx = self.idx.checked_sub(1).unwrap_or(SETTINGS_ROWS - 1);
                ScreenAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.idx = (self.idx + 1) % SETTINGS_ROWS;
                ScreenAction::None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.adjust_setting(ctx, true, true);
                ScreenAction::None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.adjust_setting(ctx, true, false);
                ScreenAction::None
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.adjust_setting(ctx, false, false);
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }
}

