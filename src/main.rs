use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{self, Event};
use tokio::sync::mpsc::unbounded_channel;

use terminal_card_game::app::App;
use terminal_card_game::error::AppResult;
use terminal_card_game::images::ImageCache;
use terminal_card_game::images::renderer::ImageRenderer;
use terminal_card_game::providers::ProviderRegistry;
use terminal_card_game::providers::tcgdex::TcgDexProvider;
use terminal_card_game::storage::SettingsRepository;
use terminal_card_game::storage::sqlite::SqliteStore;
use terminal_card_game::ui;

fn resolve_data_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("TCG_DATA_DIR")
        && !custom.trim().is_empty()
    {
        return PathBuf::from(custom);
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME")
        && !xdg.trim().is_empty()
    {
        return PathBuf::from(xdg).join("terminal-card-game");
    }
    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("terminal-card-game");
    }
    PathBuf::from(".terminal-card-game")
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let data_dir = resolve_data_dir();
    let db_path = data_dir.join("vault.db");
    let image_cache_dir = data_dir.join("cache").join("images");

    let store = Arc::new(SqliteStore::open(&db_path)?);
    let initial_settings = store.load_settings()?;

    let mut registry = ProviderRegistry::new();
    registry.register(Arc::new(TcgDexProvider::new()?));
    let providers = Arc::new(registry);

    let image_cache = Arc::new(ImageCache::new(image_cache_dir)?);

    // Initialize terminal first so Picker::from_query_stdio can safely probe capabilities
    // inside alternate screen / raw mode before the input reader thread starts.
    let mut terminal = ratatui::init();
    let renderer = ImageRenderer::detect(initial_settings.protocol_mode);

    let result = run_tui(&mut terminal, store, providers, image_cache, renderer).await;
    ratatui::restore();
    result
}

async fn run_tui(
    terminal: &mut ratatui::DefaultTerminal,
    store: Arc<SqliteStore>,
    providers: Arc<ProviderRegistry>,
    image_cache: Arc<ImageCache>,
    renderer: ImageRenderer,
) -> AppResult<()> {
    let mut app = App::new(store, providers, image_cache, renderer)?;

    let (term_tx, mut term_rx) = unbounded_channel::<Event>();
    std::thread::spawn(move || {
        while let Ok(ev) = event::read() {
            if term_tx.send(ev).is_err() {
                break;
            }
        }
    });

    let mut ticker = tokio::time::interval(Duration::from_millis(75));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    while app.running {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        tokio::select! {
            Some(ev) = term_rx.recv() => {
                app.handle_terminal_event(ev);
                // Drain queued key bursts before next frame for snappy navigation
                while let Ok(queued) = term_rx.try_recv() {
                    app.handle_terminal_event(queued);
                    if !app.running {
                        break;
                    }
                }
            }
            Some(msg) = app.async_rx.recv() => {
                app.handle_async_message(msg);
            }
            _ = ticker.tick() => {
                app.on_tick();
            }
        }
    }

    Ok(())
}
