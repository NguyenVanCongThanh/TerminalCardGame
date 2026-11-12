# Terminal Trading Card Vault (`terminal-card-game`)

An extensible, keyboard-driven terminal trading card collecting & pack-opening game built with **Rust**, **Ratatui**, **Crossterm**, and **`ratatui-image`**.

The initial MVP focuses on **Pokémon TCG** cards fetched from the open **TCGdex** API (`https://api.tcgdex.net/v2/en`), displaying real high-resolution card artwork directly in the terminal and persisting player progress locally in **SQLite**. The domain model, pack engine, storage repositories, and `CardDataProvider` traits are decoupled from Pokémon-specific schemas so additional universes (Yu-Gi-Oh!, Anime, Sports, or local JSON sets) can be plugged in cleanly.

---

## Features

- **Real Card Artwork in the Terminal**: Powered by `ratatui-image` with automatic terminal capability detection (`Picker::from_query_stdio`), supporting:
  - **Kitty Graphics Protocol** (Kitty, Ghostty, compatible terminals)
  - **Sixel Bitmap Graphics** (WezTerm, foot, mlterm, xterm with Sixel, Windows Terminal Preview)
  - **iTerm2 Inline Images** (iTerm2, WezTerm, Tabby)
  - **Unicode Half-Blocks (`▀`)** (Universal 24-bit RGB fallback for any modern terminal)
- **Non-Blocking Async Pipeline**: HTTP requests, PNG decoding, disk caching, and pixel-to-cell resizing (`ThreadProtocol` + `tokio::task::spawn_blocking`) run off the TUI render thread.
- **Interactive Pack Opening Experience**:
  - Foil-rip animation followed by sequential card-back flipping (`[Space]` / `[Enter]`) or Reveal All (`[a]`).
  - Configurable slot-based pack templates (**Demo Pack 5**, **Standard Booster 10**, **Collector's Showcase 5**).
  - Rarity-colored frames, `NEW!` vs `OWNED xN` badges, and instant background artwork pre-fetching.
- **Collection Browser & Inspector**:
  - Switch between **List + Large Preview** and **2×2 Multi-Card Grid** views (`[v]`).
  - Filter by set (`[Tab]`) and rarity (`[f]`), inspect full metadata (`[Enter]`), and track set completion.
- **Local Persistence & Offline Support**:
  - Stores synced sets, card metadata, owned quantities, pack opening history, and settings in SQLite (`rusqlite` bundled).
  - Caches downloaded PNG card artwork on disk so previously synced sets and artwork work offline.

---

## Requirements & Installation

- **Rust toolchain** (1.86+ / edition 2024; verified with `rustc 1.99.1`)
- No external C libraries required:
  - SQLite is compiled via `rusqlite` (`bundled` feature).
  - TLS uses `rustls-tls` (no OpenSSL system headers required).
  - `ratatui-image` uses built-in protocol encoders + native Rust half-blocks (no `libchafa` dependency).

### Build & Run

```bash
# Clone or enter the project directory
cd /home/thanh/TerminalCardGame

# Run unit & integration tests
cargo test

# Launch the interactive TUI
cargo run --release
```

### Data & Cache Location

By default, local SQLite data and cached card artwork are stored at:

- `$XDG_DATA_HOME/terminal-card-game/` (or `$HOME/.local/share/terminal-card-game/`)
  - `vault.db` — SQLite database
  - `cache/images/<collection_id>/<card_id>_<quality>.png` — cached card artwork

Override this path at runtime using:

```bash
TCG_DATA_DIR=/path/to/custom/dir cargo run
```

---

## API Configuration

- **Primary Provider (`TcgDexProvider`)**: Uses the free, open-source **TCGdex API** (`https://api.tcgdex.net/v2/en`).
- **No API key or account setup is required.**
- To keep initial syncs fast and lightweight, each collection syncs a balanced sample of cards (configurable in **Settings**: 16, 28, 40, or 60 cards per set) rather than downloading the entire multi-thousand-card database.

---

## Keyboard Controls

| Screen | Keys | Action |
|---|---|---|
| **Global** | `Ctrl+C` | Exit cleanly |
| **Main Menu** | `↑/↓` or `j/k`, `Enter`, `1-5`, `q` | Navigate / quick-jump / quit |
| **Universe / Collection / Pack** | `↑/↓`, `Enter`, `s`, `r`, `Esc` | Select, sync cards (`s`), refresh sets (`r`), back |
| **Pack Opening** | `Space` / `Enter`, `←/→`, `a`, `d`, `o`, `c` | Flip / next, browse, reveal all (`a`), details (`d`), open another (`o`), collection (`c`) |
| **My Collection** | `↑/↓/←/→`, `v`, `Tab`, `f`, `Enter` | Navigate, toggle List/2×2 Grid (`v`), set filter (`Tab`), rarity filter (`f`), inspect |
| **Card Details** | `←/→`, `Esc` / `Backspace` | Previous / next card, return |
| **Settings** | `↑/↓`, `←/→` / `Enter` | Adjust rendering protocol, artwork quality, animation speed, cache |

---

## Architecture Overview

```text
src/
├── main.rs                 # Terminal setup, Picker auto-detection, Tokio + Crossterm event loop
├── lib.rs                  # Public crate module exports
├── app.rs                  # App state machine, navigation, async task & image slot coordination
├── error.rs                # Unified AppError enum
├── domain/                 # Theme-agnostic core models
│   ├── universe.rs         # Universe entity
│   ├── collection.rs       # Collection / Card Set entity
│   ├── card.rs             # Card, Rarity, flexible CardMetadata (tags, attributes, highlights)
│   ├── pack.rs             # PackConfig, PackSlot, PulledCard, PackOpeningResult
│   └── inventory.rs        # OwnedCard, CollectionProgress, PackHistoryEntry, AppSettings
├── providers/              # External/local card source abstractions
│   ├── mod.rs              # CardDataProvider trait & ProviderRegistry
│   └── tcgdex.rs           # TcgDexProvider (free Pokémon TCG API integration)
├── storage/                # Local persistence layer
│   ├── mod.rs              # CollectionRepository, CardRepository, InventoryRepository, SettingsRepository
│   └── sqlite.rs           # SqliteStore implementation (rusqlite bundled)
├── pack_engine/            # Configurable weighted slot pack generator
│   └── mod.rs              # PackGenerator with rarity fallback & unit tests
├── images/                 # Non-blocking artwork download, caching, and terminal rendering
│   ├── mod.rs              # ImageCache (LRU memory + atomic disk cache)
│   └── renderer.rs         # ImageRenderer (ratatui-image Picker) & CardImageSlot (ThreadProtocol)
└── ui/                     # Ratatui views & styling
    ├── mod.rs              # Frame layout, header, status banner, footer hints
    ├── theme.rs            # Rarity color mapping & UI palette
    ├── widgets.rs          # Card back & fallback card face renderer
    ├── main_menu.rs
    ├── universe_select.rs
    ├── collection_select.rs
    ├── pack_select.rs
    ├── pack_opening.rs
    ├── collection_view.rs
    ├── card_details.rs
    └── settings.rs
```

### Extending to New Universes

1. Implement `CardDataProvider` (in `src/providers/`) for your API or local JSON dataset, mapping cards into the generic `Card` and `CardMetadata` structs.
2. Register the provider in `ProviderRegistry` inside `src/main.rs`.
3. Add or enable the `Universe` entry (`is_implemented = true`) in `Universe::default_universes()`.
No changes to `PackGenerator`, `SqliteStore`, `ImageCache`, or the `ui/` screens are required.

---

## Supported Terminal Image Protocols & Known Limitations

- **Auto-Detection**: On startup (after entering alternate screen), `Picker::from_query_stdio()` queries the terminal for Kitty, Sixel, or iTerm2 support and font pixel cell dimensions. If the terminal does not support graphics protocols (or inside tmux/plain terminals), it falls back automatically to **Unicode Half-Blocks**.
- **Terminal Multiplexers / WSL**: Some terminal setups (such as older Windows Console or certain tmux configurations) may not report native graphics support or cell pixel sizes accurately. You can force **Unicode Half-Blocks** at any time in **Settings -> Image Rendering Protocol**.
- **Sampled Sets**: To avoid downloading thousands of cards on startup, syncing fetches a balanced sample per set (default 28 cards, configurable up to 60 in Settings) covering Chase/Holo Rares, Rares, Uncommons, and Commons.
- **Network Dependency on First Sync**: Initial card metadata and artwork require internet access to `api.tcgdex.net` and `assets.tcgdex.net`. Once synced and viewed, cached sets, inventory, and downloaded PNGs work offline.

