use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use image::{DynamicImage, ImageFormat};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use terminal_card_game::app::{ActiveModal, App, CollectionSort, PackPhase, Screen, ScreenController};
use terminal_card_game::domain::{Card, CardMetadata, ImageQuality, PackConfig, Rarity};
use terminal_card_game::images::ImageCache;
use terminal_card_game::images::renderer::ImageRenderer;
use terminal_card_game::providers::ProviderRegistry;
use terminal_card_game::storage::sqlite::SqliteStore;
use terminal_card_game::storage::{CardRepository, InventoryRepository};
use terminal_card_game::ui;

static TEST_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = TEST_DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("tcg_ui_test_{prefix}_{pid}_{seq}_{nanos}"));
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
    assert_eq!(app.settings_screen.idx, 4);

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
    assert_eq!(app.my_collection.filtered_owned_cards(&app.owned_cards).len(), 2);

    // Cycle sort
    app.handle_terminal_event(key(KeyCode::Char('s')));
    assert_eq!(app.my_collection.sort_mode, CollectionSort::RarityDesc);
    assert_eq!(
        app.my_collection.filtered_owned_cards(&app.owned_cards)[0].card.name,
        "Charizard"
    );

    // Live search '/'
    app.handle_terminal_event(key(KeyCode::Char('/')));
    assert!(app.my_collection.search_active);
    for ch in "pika".chars() {
        app.handle_terminal_event(key(KeyCode::Char(ch)));
    }
    assert_eq!(app.my_collection.filtered_owned_cards(&app.owned_cards).len(), 1);
    assert_eq!(
        app.my_collection.filtered_owned_cards(&app.owned_cards)[0].card.name,
        "Pikachu"
    );
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    // Exit search and clear
    app.handle_terminal_event(key(KeyCode::Enter));
    assert!(!app.my_collection.search_active);
    app.handle_terminal_event(key(KeyCode::Char('x')));
    assert_eq!(app.my_collection.filtered_owned_cards(&app.owned_cards).len(), 2);

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
        app.pack_opening_screen.state.as_ref().unwrap().phase,
        PackPhase::Revealing { .. }
    ));
    wide_term.draw(|f| ui::render(f, &mut app)).unwrap();

    app.handle_terminal_event(key(KeyCode::Char(' ')));
    wide_term.draw(|f| ui::render(f, &mut app)).unwrap();

    app.handle_terminal_event(key(KeyCode::Char('a')));
    assert!(matches!(
        app.pack_opening_screen.state.as_ref().unwrap().phase,
        PackPhase::Summary { .. }
    ));
    wide_term.draw(|f| ui::render(f, &mut app)).unwrap();

    // Switch to My Collection wide 3x2 grid
    app.handle_terminal_event(key(KeyCode::Char('c')));
    app.handle_terminal_event(key(KeyCode::Char('v')));
    wide_term.draw(|f| ui::render(f, &mut app)).unwrap();
    assert_eq!(app.my_collection.grid_page_size, 6);

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn collection_filter_does_not_corrupt_global_owned_cards() {
    let (mut app, dir) = setup_test_app();
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();

    assert_eq!(app.owned_cards.len(), 2);
    app.handle_terminal_event(key(KeyCode::Char('3')));
    assert_eq!(app.screen, Screen::MyCollection);

    // Cycle Tab filter from All -> base1 -> base2 (0 owned cards in base2)
    app.handle_terminal_event(key(KeyCode::Tab));
    assert_eq!(app.my_collection.collection_filter.as_deref(), Some("base1"));
    app.handle_terminal_event(key(KeyCode::Tab));
    assert_eq!(app.my_collection.collection_filter.as_deref(), Some("base2"));
    assert_eq!(app.my_collection.filtered_owned_cards(&app.owned_cards).len(), 0);

    // Global owned_cards must stay unfiltered even while filtered view is active and after returning
    assert_eq!(app.owned_cards.len(), 2);
    app.handle_terminal_event(key(KeyCode::Esc));
    assert_eq!(app.screen, Screen::MainMenu);
    assert_eq!(app.owned_cards.len(), 2);
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn wrapped_card_details_and_help_modal_scroll_on_80x24() {
    let dir = temp_dir("scroll");
    let store = Arc::new(SqliteStore::open(&dir.join("vault.db")).unwrap());
    let mut long_card = sample_cards().remove(0);
    long_card.metadata.highlights = (1..=6)
        .map(|i| {
            format!(
                "[Fire/Fire/Colorless] Attack {i} — Deals heavy damage and discards energy cards from your Pokémon in play for massive effect."
            )
        })
        .collect();
    long_card.metadata.flavor_text = Some(
        "A very long lore description that wraps onto multiple lines inside an 80x24 terminal viewport to verify scrolling works past unwrapped line count."
            .to_string(),
    );
    store.upsert_cards(&[long_card.clone()]).unwrap();
    let pack = PackConfig::default_configs().remove(0);
    store
        .record_pack_opening(&pack, "pokemon", "base1", &[("Hit", long_card)])
        .unwrap();

    let providers = Arc::new(ProviderRegistry::new());
    let image_cache = Arc::new(ImageCache::new(dir.join("cache")).unwrap());
    let mut app = App::new(store, providers, image_cache, ImageRenderer::halfblocks_only()).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

    // Verify Help modal scrolls on 80x24
    app.handle_terminal_event(key(KeyCode::Char('?')));
    assert!(matches!(app.modal, Some(ActiveModal::Help)));
    app.handle_terminal_event(key(KeyCode::Down));
    app.handle_terminal_event(key(KeyCode::Down));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    assert!(app.help_scroll > 0, "help modal should scroll on 80x24");
    app.handle_terminal_event(key(KeyCode::Esc));

    // Enter MyCollection -> CardDetails and verify wrapped scroll isn't clamped to 0
    app.handle_terminal_event(key(KeyCode::Char('3')));
    app.handle_terminal_event(key(KeyCode::Enter));
    assert_eq!(app.screen, Screen::CardDetails);
    for _ in 0..8 {
        app.handle_terminal_event(key(KeyCode::Down));
    }
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    assert!(
        app.card_details.scroll >= 4,
        "wrapped CardDetails should allow scrolling, got {}",
        app.card_details.scroll
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn pack_opening_hints_out_of_order_flip_and_grid_vertical_nav() {
    let dir = temp_dir("nav");
    let store = Arc::new(SqliteStore::open(&dir.join("vault.db")).unwrap());
    let mut cards = Vec::new();
    for i in 1..=5 {
        let mut c = sample_cards().remove(1);
        c.id = format!("base1-{i}");
        c.card_number = i.to_string();
        c.name = format!("Card {i}");
        cards.push(c);
    }
    store.upsert_cards(&cards).unwrap();
    let pack = PackConfig::default_configs().remove(0);
    let picks: Vec<(&str, Card)> = cards.iter().map(|c| ("Slot", c.clone())).collect();
    store
        .record_pack_opening(&pack, "pokemon", "base1", &picks)
        .unwrap();

    let providers = Arc::new(ProviderRegistry::new());
    let image_cache = Arc::new(ImageCache::new(dir.join("cache")).unwrap());
    let mut app = App::new(store, providers, image_cache, ImageRenderer::halfblocks_only()).unwrap();

    // Test grid vertical navigation preserves column (5 items, 2 cols: [0,1], [2,3], [4])
    app.handle_terminal_event(key(KeyCode::Char('3')));
    app.handle_terminal_event(key(KeyCode::Char('v'))); // grid mode, 2 cols
    app.handle_terminal_event(key(KeyCode::Right)); // cursor = 1 (row 0, col 1)
    assert_eq!(app.my_collection.owned_cursor, 1);
    app.handle_terminal_event(key(KeyCode::Down)); // -> 3 (row 1, col 1)
    assert_eq!(app.my_collection.owned_cursor, 3);
    app.handle_terminal_event(key(KeyCode::Down)); // wraps within col 1 -> 1
    assert_eq!(app.my_collection.owned_cursor, 1);
    app.handle_terminal_event(key(KeyCode::Up)); // wraps within col 1 -> 3
    assert_eq!(app.my_collection.owned_cursor, 3);

    // Return to MainMenu -> Open Pack (quick-jump '1' -> CollectionSelect -> PackSelect -> PackOpening)
    app.handle_terminal_event(key(KeyCode::Char('q'))); // also verify 'q' navigates back from MyCollection
    assert_eq!(app.screen, Screen::MainMenu);
    app.handle_terminal_event(key(KeyCode::Char('1')));
    app.handle_terminal_event(key(KeyCode::Enter));
    app.handle_terminal_event(key(KeyCode::Enter));
    assert_eq!(app.screen, Screen::PackOpening);
    assert!(!app.pack_opening_screen.footer_hints(false).contains("Details"));

    // Verify 'q' exits Revealing phase back to PackSelect and re-open pack
    app.handle_terminal_event(key(KeyCode::Char(' ')));
    assert!(matches!(
        app.pack_opening_screen.state.as_ref().unwrap().phase,
        PackPhase::Revealing { .. }
    ));
    app.handle_terminal_event(key(KeyCode::Char('q')));
    assert_eq!(app.screen, Screen::PackSelect);
    app.handle_terminal_event(key(KeyCode::Enter));
    assert_eq!(app.screen, Screen::PackOpening);

    // Skip Ripping -> Revealing
    app.handle_terminal_event(key(KeyCode::Char(' ')));
    assert!(matches!(
        app.pack_opening_screen.state.as_ref().unwrap().phase,
        PackPhase::Revealing { .. }
    ));

    // Flip out of order: reveal 4 first, then 0..=3, then Space should go straight to Summary
    app.handle_terminal_event(key(KeyCode::Left)); // wraps to 4
    app.handle_terminal_event(key(KeyCode::Char(' '))); // reveal 4
    app.handle_terminal_event(key(KeyCode::Char(' '))); // jumps to next unrevealed (0)
    for _ in 0..4 {
        app.handle_terminal_event(key(KeyCode::Char(' '))); // reveal 0
        app.handle_terminal_event(key(KeyCode::Char(' '))); // advance
    }
    // After revealing all 5 cards, pressing Space transitions straight to Summary
    assert!(matches!(
        app.pack_opening_screen.state.as_ref().unwrap().phase,
        PackPhase::Summary { .. }
    ));
    assert!(app.pack_opening_screen.footer_hints(false).contains("Details"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn quality_switch_downloads_matching_resolution_instead_of_reusing_low_cache() {
    let dir = temp_dir("quality");
    let cache = Arc::new(ImageCache::new(dir.join("cache")).unwrap());

    let mut low_bytes = Vec::new();
    DynamicImage::new_rgba8(2, 2)
        .write_to(&mut Cursor::new(&mut low_bytes), ImageFormat::Png)
        .unwrap();
    let mut high_bytes = Vec::new();
    DynamicImage::new_rgba8(6, 8)
        .write_to(&mut Cursor::new(&mut high_bytes), ImageFormat::Png)
        .unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        for _ in 0..2 {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let low = low_bytes.clone();
            let high = high_bytes.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]);
                let body = if req.contains("/high.png") { high } else { low };
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = sock.write_all(header.as_bytes()).await;
                let _ = sock.write_all(&body).await;
            });
        }
    });

    let mut card = sample_cards().remove(0);
    card.image_url = Some(format!("http://{addr}/high.png"));
    card.thumbnail_url = Some(format!("http://{addr}/low.png"));

    let (low_img, low_path) = cache.load_card_image(&card, ImageQuality::Low).await.unwrap();
    assert_eq!((low_img.width(), low_img.height()), (2, 2));
    card.local_image_path = Some(low_path.to_string_lossy().to_string());

    // Switching to High with local_image_path still pointing to *_low.png must fetch high.png!
    let (high_img, high_path) = cache.load_card_image(&card, ImageQuality::High).await.unwrap();
    assert_eq!((high_img.width(), high_img.height()), (6, 8));
    assert_ne!(high_path, low_path);

    let _ = std::fs::remove_dir_all(&dir);
}
