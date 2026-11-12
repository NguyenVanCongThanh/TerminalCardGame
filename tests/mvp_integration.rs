use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use rand::SeedableRng;
use rand::rngs::StdRng;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use terminal_card_game::app::{App, PackPhase, Screen};
use terminal_card_game::domain::{AppSettings, ImageQuality, PackConfig, ProtocolMode};
use terminal_card_game::images::ImageCache;
use terminal_card_game::images::renderer::ImageRenderer;
use terminal_card_game::pack_engine::PackGenerator;
use terminal_card_game::providers::tcgdex::TcgDexProvider;
use terminal_card_game::providers::{CardDataProvider, ProviderRegistry};
use terminal_card_game::storage::sqlite::SqliteStore;
use terminal_card_game::storage::{
    CardRepository, InventoryRepository, SettingsRepository,
};
use terminal_card_game::ui;

fn temp_test_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("tcg_test_{prefix}_{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

#[tokio::test]
async fn live_api_image_cache_halfblocks_render_and_persistence_mvp() {
    let test_dir = temp_test_dir("live_mvp");
    let db_path = test_dir.join("vault.db");
    let cache_dir = test_dir.join("cache").join("images");

    // 1. Initialize provider & storage
    let provider = Arc::new(TcgDexProvider::new().expect("provider init"));
    let mut registry = ProviderRegistry::new();
    registry.register(provider.clone());
    let providers = Arc::new(registry);

    let store = Arc::new(SqliteStore::open(&db_path).expect("sqlite open"));
    let image_cache = Arc::new(ImageCache::new(cache_dir.clone()).expect("image cache init"));

    // 2. Fetch a small real sample of Pokémon Base Set cards from TCGdex
    let cards = provider
        .fetch_collection_cards("base1", 12)
        .await
        .expect("fetch real base1 cards from TCGdex");
    assert_eq!(cards.len(), 12);
    assert!(
        cards.iter().any(|c| c.image_url.is_some()),
        "expected real card image URLs"
    );
    store.upsert_cards(&cards).expect("persist synced cards");

    // 3. Download and cache a real card artwork image locally, then verify disk cache hit
    let charizard_or_first = cards
        .iter()
        .find(|c| c.name.contains("Charizard"))
        .unwrap_or(&cards[0])
        .clone();

    let (dyn_img, disk_path) = image_cache
        .load_card_image(&charizard_or_first, ImageQuality::Low)
        .await
        .expect("download real card image");
    assert!(disk_path.exists(), "image should be cached on disk");
    assert!(dyn_img.width() > 0 && dyn_img.height() > 0);
    store
        .update_local_image_path(&charizard_or_first.id, &disk_path.to_string_lossy())
        .unwrap();

    // 4. Verify terminal Halfblocks rendering into TestBackend produces colored half-block cells
    let renderer = ImageRenderer::halfblocks_only();
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new(
        store.clone(),
        providers.clone(),
        image_cache.clone(),
        renderer,
    )
    .expect("app init");
    app.settings.image_quality = ImageQuality::Low;

    // Render Main Menu
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    // Navigate Main Menu -> Open Packs -> Collection Select -> Pack Select -> Pack Opening
    app.handle_terminal_event(key(KeyCode::Enter));
    assert_eq!(app.screen, Screen::CollectionSelect);
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    app.handle_terminal_event(key(KeyCode::Enter));
    assert_eq!(app.screen, Screen::PackSelect);
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    // Tear open demo pack
    app.handle_terminal_event(key(KeyCode::Enter));
    assert_eq!(app.screen, Screen::PackOpening);

    // Advance ripping animation to revealing
    for _ in 0..12 {
        app.on_tick();
    }
    assert!(matches!(
        app.pack_opening.as_ref().unwrap().phase,
        PackPhase::Revealing { .. }
    ));

    // Flip first card and bind our cached image into main slot, then drive resize worker
    app.handle_terminal_event(key(KeyCode::Char(' ')));
    app.request_card_into_slot("main", &charizard_or_first);

    // Draw once to trigger resize request, pump async resize response, draw again to render halfblocks
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        let mut got_resize = false;
        while let Ok(msg) = app.async_rx.try_recv() {
            if matches!(msg, terminal_card_game::app::AsyncMessage::ImageResized(_)) {
                got_resize = true;
            }
            app.handle_async_message(msg);
        }
        if got_resize && app.image_slots["main"].has_ready_protocol() {
            break;
        }
    }
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    let buf = terminal.backend().buffer();
    let has_halfblock = buf
        .content()
        .iter()
        .any(|cell| cell.symbol() == "▀" || cell.symbol() == "▄");
    assert!(
        has_halfblock,
        "expected Unicode half-block characters ('▀'/'▄') rendered for real card artwork"
    );

    // Reveal all -> Summary -> Card Details -> My Collection (List + 2x2 Grid) -> Settings
    app.handle_terminal_event(key(KeyCode::Char('a')));
    assert!(matches!(
        app.pack_opening.as_ref().unwrap().phase,
        PackPhase::Summary { .. }
    ));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    app.handle_terminal_event(key(KeyCode::Char('d')));
    assert_eq!(app.screen, Screen::CardDetails);
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    app.handle_terminal_event(key(KeyCode::Esc));
    assert_eq!(app.screen, Screen::PackOpening);

    app.handle_terminal_event(key(KeyCode::Char('c')));
    assert_eq!(app.screen, Screen::MyCollection);
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    // Toggle 2x2 grid view
    app.handle_terminal_event(key(KeyCode::Char('v')));
    assert!(app.grid_mode);
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    // Visit Universe Select & Settings
    app.handle_terminal_event(key(KeyCode::Esc));
    assert_eq!(app.screen, Screen::MainMenu);
    app.handle_terminal_event(key(KeyCode::Char('2')));
    assert_eq!(app.screen, Screen::UniverseSelect);
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    app.handle_terminal_event(key(KeyCode::Esc));
    app.handle_terminal_event(key(KeyCode::Char('4')));
    assert_eq!(app.screen, Screen::Settings);
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    // Persist settings & verify reopening SQLite preserves inventory, pack history, and settings
    let custom_settings = AppSettings {
        protocol_mode: ProtocolMode::Halfblocks,
        fast_animations: true,
        ..Default::default()
    };
    store.save_settings(&custom_settings).unwrap();

    drop(app);
    drop(store);

    // Simulate app restart & offline usage
    let reopened = SqliteStore::open(&db_path).expect("reopen sqlite");
    let persisted_cards = reopened.list_cards_in_collection("base1").unwrap();
    assert_eq!(persisted_cards.len(), 12);

    let persisted_owned = reopened.list_owned_cards(None).unwrap();
    assert!(!persisted_owned.is_empty(), "owned cards must persist across restart");
    let total_copies: u32 = persisted_owned.iter().map(|o| o.quantity).sum();
    assert_eq!(total_copies, 5, "demo pack should have added 5 cards");

    let persisted_history = reopened.list_pack_history(10).unwrap();
    assert_eq!(persisted_history.len(), 1);
    assert_eq!(reopened.load_settings().unwrap(), custom_settings);

    // Verify cached image still loads offline from disk (even if card had no network)
    let mut offline_card = charizard_or_first.clone();
    offline_card.image_url = None;
    offline_card.thumbnail_url = None;
    let offline_cache = Arc::new(ImageCache::new(cache_dir).unwrap());
    let (offline_img, _) = offline_cache
        .load_card_image(&offline_card, ImageQuality::Low)
        .await
        .expect("cached image loads offline from disk");
    assert_eq!(offline_img.width(), dyn_img.width());

    // Verify PackGenerator works deterministically offline on cached pool
    let cfg = PackConfig::default_configs().remove(0);
    let mut rng = StdRng::seed_from_u64(99);
    let generated = PackGenerator::generate_cards(&cfg, &persisted_cards, &mut rng).unwrap();
    assert_eq!(generated.len(), 5);

    let _ = std::fs::remove_dir_all(&test_dir);
}
