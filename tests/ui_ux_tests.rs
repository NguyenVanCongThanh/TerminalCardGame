use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use terminal_card_game::app::{ActiveModal, App, CollectionSort, PackPhase, Screen};
use terminal_card_game::domain::{Card, CardMetadata, PackConfig, Rarity};
use terminal_card_game::images::ImageCache;
use terminal_card_game::images::renderer::ImageRenderer;
use terminal_card_game::providers::ProviderRegistry;
use terminal_card_game::storage::sqlite::SqliteStore;
use terminal_card_game::storage::{CardRepository, InventoryRepository};
use terminal_card_game::ui;

fn temp_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("tcg_ui_test_{prefix}_{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn sample_cards() -> Vec<Card> {
    let mut attrs = BTreeMap::new();
    attrs.insert("HP".to_string(), "120".to_string());
    attrs.insert("Market Trend".to_string(), "€42.50".to_string());

    vec![
        Card {
            id: "base1-4".to_string(),
            universe_id: "pokemon".to_string(),
            collection_id: "base1".to_string(),
            card_number: "4".to_string(),
            name: "Charizard".to_string(),
            rarity: Rarity::HoloRare,
            raw_rarity: "Rare Holo".to_string(),
            image_url: None,
            thumbnail_url: None,
            local_image_path: None,
            metadata: CardMetadata {
                subtitle: Some("Fire • Stage 2".to_string()),
                creator: Some("Mitsuhiro Arita".to_string()),
                flavor_text: Some("Spits fire that is hot enough to melt boulders.".to_string()),
                tags: vec!["Fire".to_string()],
                attributes: attrs.clone(),
                highlights: vec![
                    "[Power] Energy Burn: Turn Energy into Fire.".to_string(),
                    "[Fire/Fire/Fire/Fire] Fire Spin (100 dmg)".to_string(),
                ],
            },
        },
        Card {
            id: "base1-58".to_string(),
            universe_id: "pokemon".to_string(),
            collection_id: "base1".to_string(),
            card_number: "58".to_string(),
            name: "Pikachu".to_string(),
            rarity: Rarity::Common,
            raw_rarity: "Common".to_string(),
            image_url: None,
            thumbnail_url: None,
            local_image_path: None,
            metadata: CardMetadata {
                subtitle: Some("Lightning • Basic".to_string()),
                creator: Some("Mitsuhiro Arita".to_string()),
                flavor_text: Some("When several of these Pokémon gather...".to_string()),
                tags: vec!["Lightning".to_string()],
                attributes: attrs,
                highlights: vec!["[Lightning] Gnaw (10 dmg)".to_string()],
            },
        },
    ]
}

fn setup_test_app() -> (App, std::path::PathBuf) {
    let dir = temp_dir("app");
    let store = Arc::new(SqliteStore::open(&dir.join("vault.db")).unwrap());
    let cards = sample_cards();
    store.upsert_cards(&cards).unwrap();

    let pack = PackConfig::default_configs().remove(0);
    store
        .record_pack_opening(
            &pack,
            "pokemon",
            "base1",
            &[
                ("Base", cards[1].clone()),
                ("Hit", cards[0].clone()),
            ],
        )
        .unwrap();

    let providers = Arc::new(ProviderRegistry::new());
    let image_cache = Arc::new(ImageCache::new(dir.join("cache")).unwrap());
    let renderer = ImageRenderer::halfblocks_only();
    let app = App::new(store, providers, image_cache, renderer).unwrap();
    (app, dir)
}

#[tokio::test]
async fn settings_clear_cache_requires_modal_confirmation() {
    let (mut app, dir) = setup_test_app();
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();

    app.handle_terminal_event(key(KeyCode::Char('4')));
    assert_eq!(app.screen, Screen::Settings);

    // Move to row 4 (Clear Local Image Cache)
    for _ in 0..4 {
        app.handle_terminal_event(key(KeyCode::Down));
    }
    assert_eq!(app.settings_idx, 4);

    // Left/Right must NOT open confirm or clear cache accidentally
    app.handle_terminal_event(key(KeyCode::Right));
    assert!(app.modal.is_none());

    // Enter opens confirmation modal
    app.handle_terminal_event(key(KeyCode::Enter));
    assert!(matches!(app.modal, Some(ActiveModal::Confirm { .. })));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    // Esc cancels
    app.handle_terminal_event(key(KeyCode::Esc));
    assert!(app.modal.is_none());

    // Enter -> y confirms
    app.handle_terminal_event(key(KeyCode::Enter));
    app.handle_terminal_event(key(KeyCode::Char('y')));
    assert!(app.modal.is_none());

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn help_modal_toggles_and_collection_search_sort_work() {
    let (mut app, dir) = setup_test_app();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

    // Global Help modal at MainMenu
    app.handle_terminal_event(key(KeyCode::Char('?')));
    assert!(matches!(app.modal, Some(ActiveModal::Help)));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    app.handle_terminal_event(key(KeyCode::Esc));
    assert!(app.modal.is_none());

    // Enter My Collection
    app.handle_terminal_event(key(KeyCode::Char('3')));
    assert_eq!(app.screen, Screen::MyCollection);
    assert_eq!(app.filtered_owned_cards().len(), 2);

    // Cycle sort
    app.handle_terminal_event(key(KeyCode::Char('s')));
    assert_eq!(app.sort_mode, CollectionSort::RarityDesc);
    assert_eq!(app.filtered_owned_cards()[0].card.name, "Charizard");

    // Live search '/'
    app.handle_terminal_event(key(KeyCode::Char('/')));
    assert!(app.search_active);
    for ch in "pika".chars() {
        app.handle_terminal_event(key(KeyCode::Char(ch)));
    }
    assert_eq!(app.filtered_owned_cards().len(), 1);
    assert_eq!(app.filtered_owned_cards()[0].card.name, "Pikachu");
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    // Exit search and clear
    app.handle_terminal_event(key(KeyCode::Enter));
    assert!(!app.search_active);
    app.handle_terminal_event(key(KeyCode::Char('x')));
    assert_eq!(app.filtered_owned_cards().len(), 2);

    // Open Card Details & test scrolling
    app.handle_terminal_event(key(KeyCode::Enter));
    assert_eq!(app.screen, Screen::CardDetails);
    app.handle_terminal_event(key(KeyCode::Down));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn pack_opening_and_wide_grid_render_cleanly() {
    let (mut app, dir) = setup_test_app();
    let mut wide_term = Terminal::new(TestBackend::new(170, 38)).unwrap();

    // Open a 10-card standard booster offline from cached base1 pool
    app.handle_terminal_event(key(KeyCode::Char('1')));
    assert_eq!(app.screen, Screen::CollectionSelect);
    wide_term.draw(|f| ui::render(f, &mut app)).unwrap();

    app.handle_terminal_event(key(KeyCode::Enter));
    assert_eq!(app.screen, Screen::PackSelect);
    app.handle_terminal_event(key(KeyCode::Down)); // booster-10
    wide_term.draw(|f| ui::render(f, &mut app)).unwrap();

    app.handle_terminal_event(key(KeyCode::Enter));
    assert_eq!(app.screen, Screen::PackOpening);
    wide_term.draw(|f| ui::render(f, &mut app)).unwrap(); // Ripping

    // Skip ripping -> Revealing -> Flip -> Summary
    app.handle_terminal_event(key(KeyCode::Char(' ')));
    assert!(matches!(
        app.pack_opening.as_ref().unwrap().phase,
        PackPhase::Revealing { .. }
    ));
    wide_term.draw(|f| ui::render(f, &mut app)).unwrap();

    app.handle_terminal_event(key(KeyCode::Char(' ')));
    wide_term.draw(|f| ui::render(f, &mut app)).unwrap();

    app.handle_terminal_event(key(KeyCode::Char('a')));
    assert!(matches!(
        app.pack_opening.as_ref().unwrap().phase,
        PackPhase::Summary { .. }
    ));
    wide_term.draw(|f| ui::render(f, &mut app)).unwrap();

    // Switch to My Collection wide 3x2 grid
    app.handle_terminal_event(key(KeyCode::Char('c')));
    app.handle_terminal_event(key(KeyCode::Char('v')));
    wide_term.draw(|f| ui::render(f, &mut app)).unwrap();
    assert_eq!(app.grid_page_size, 6);

    let _ = std::fs::remove_dir_all(&dir);
}
