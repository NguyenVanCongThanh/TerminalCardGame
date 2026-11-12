# Phân tích & Đề xuất Cải tiến UI/UX — Terminal Trading Card Vault

Tài liệu này tổng hợp đánh giá hiện trạng giao diện người dùng (TUI) và đề xuất các cải tiến về Widget, Custom Widget, bố cục, tương tác và hệ thống thiết kế cho dự án `terminal-card-game`.

> **Phạm vi khảo sát:** `src/main.rs`, `src/app.rs`, `src/ui/mod.rs`, `src/ui/theme.rs`, `src/ui/widgets.rs` và toàn bộ 8 màn hình trong `src/ui/`.

---

## 1. Đánh giá hiện trạng tổng thể

### 1.1. Điểm mạnh đã có
- **Kiến trúc tách lớp rõ ràng:** Logic render tách biệt khỏi network, SQLite và quá trình decode/resize ảnh (`CardImageSlot` + `ThreadProtocol`), giúp giao diện không bị khựng khi tải PNG.
- **Khung layout nhất quán:** Các màn hình dùng chung shell Header (3 dòng) – Body – Banner – Footer (2 dòng) tại `src/ui/mod.rs`, bảng màu tối đồng bộ (`src/ui/theme.rs`) và viền màu theo độ hiếm (`Rarity`).
- **Nền tảng animation sẵn sàng:** Vòng lặp tick 75ms (`src/main.rs`) đã hỗ trợ shimmer cho `SecretRare` và animation xé pack cơ bản.

### 1.2. Hạn chế cốt lõi hiện tại
1. **Lạm dụng `Paragraph` + `List`:** Nhiều khu vực dữ liệu có cấu trúc (bảng tỷ lệ slot, danh sách thẻ sở hữu, lịch sử mở pack, thuộc tính thẻ) đang dùng `Line`/`Span` căn lề thủ công (`format!("{:<22}")`). Điều này dễ lệch cột với ký tự Unicode, thiếu thanh cuộn (`Scrollbar`) và khó quét mắt.
2. **Trải nghiệm mở pack còn phẳng:**
   - Pha `Ripping` chỉ dịch chuyển text sang phải 0–8 khoảng trắng trong ~0.75s.
   - Pha `Revealing` đổi tức thì từ mặt sau sang mặt trước, thiếu nhịp hồi hộp cho thẻ hiếm (`is_hit()`), và resize ảnh chỉ bắt đầu sau khi lật thẻ.
   - Pha `Summary` hiển thị giống hệt pha `Revealing`, thiếu màn hình tổng kết thành quả cả pack.
3. **Vấn đề Responsive & Cắt chữ (Clipping):** Footer gợi ý phím tắt (`render_footer`), thanh lọc ở `MyCollection`, và dải thẻ ở `PackOpening` (`Length(4)` cho pack 10 lá) dễ bị cắt chữ trên terminal rộng 80–110 cột.
4. **Thiếu hộp thoại xác nhận & cơ chế tìm kiếm/sắp xếp:**
   - Tại `Settings`, phím `←` / `→` / `Enter` / `Space` trên dòng `Clear Local Image Cache` thực thi xóa cache ngay lập tức mà không hỏi xác nhận (`src/app.rs`).
   - `MyCollection` chưa có tìm kiếm theo tên hay đổi kiểu sắp xếp; `CardDetails` chưa cuộn được khi thẻ có nhiều chiêu thức/mô tả dài.

---

## 2. Đánh giá & Đề xuất chi tiết theo từng màn hình

### 2.1. Khung ứng dụng toàn cục (`src/ui/mod.rs`, `src/app.rs`)
- **Vấn đề hiện tại:**
  - `render_footer` dùng `Paragraph` không wrap nên các chuỗi hướng dẫn dài (~115–135 ký tự) bị cắt trên màn hình hẹp.
  - `StatusBanner` chiếm 1 dòng xen giữa Body và Footer, gây giật bố cục (layout shift) mỗi khi banner xuất hiện/biến mất.
  - Chưa có lớp Modal/Popup dùng chung cho xác nhận hành động hoặc bảng trợ giúp phím tắt (`?`).
- **Giải pháp đề xuất:**
  - Rút gọn gợi ý Footer linh hoạt theo `area.width`, kết hợp phím `?` mở bảng phím tắt đầy đủ.
  - Chuyển `StatusBanner` thành Floating Toast hoặc dành sẵn chiều cao cố định để tránh giật layout.
  - Xây dựng **Modal Popup dùng chung** (xác nhận `Yes/No`, Help `?`) bằng `Clear` + `Block::bordered()` (hoặc crate `tui-popup`).
- **Widget sử dụng:** Built-in `Clear`, `Block`, `Paragraph` (tùy chọn `tui-popup`).
- **Mức độ ưu tiên:** **High**
- **Chi phí & Ảnh hưởng kỹ thuật:** Thấp–Trung bình. Thêm `Option<ModalState>` và `show_help: bool` vào `App`; render lớp overlay cuối cùng trong `ui::render()`. Lưu ý tránh vẽ overlay đè lên vùng bitmap Kitty/Sixel hoặc tạm clear slot ảnh khi mở modal.

---

### 2.2. Màn hình Chính — `MainMenu` (`src/ui/main_menu.rs`)
- **Vấn đề hiện tại:**
  - Logo dùng hộp ASCII cố định chiều rộng; `Collector Profile` và `Recent Pack History` là `Paragraph` tĩnh, không cuộn và không tương tác xem lại pack cũ được.
- **Giải pháp đề xuất:**
  - Thay phần thống kê set trong `Collector Profile` bằng các thanh tiến độ `LineGauge` theo từng set đã sync.
  - Chuyển `Recent Pack History` thành `List` hoặc `Table` có thể focus bằng `Tab` để chọn và mở xem lại các thẻ đã rút trong pack đó (`CardDetails`).
  - Có thể nâng cấp Logo bằng banner lớn hơn khi terminal đủ rộng (ví dụ `tui-big-text`).
- **Widget sử dụng:** Built-in `LineGauge`, `Table` / `List` (tùy chọn `tui-big-text`).
- **Mức độ ưu tiên:** **Medium**
- **Chi phí & Ảnh hưởng kỹ thuật:** Trung bình. Tận dụng `app.collection_progress` và `app.pack_history` (`card_ids`), bổ sung `panel_focus` và query danh sách thẻ từ SQLite khi xem lại lịch sử.

---

### 2.3. Màn hình Chọn Vũ Trụ — `UniverseSelect` (`src/ui/universe_select.rs`)
- **Vấn đề hiện tại:**
  - Panel chi tiết bên phải hiển thị câu giải thích kỹ thuật cho lập trình viên (*"New universes plug in via the `CardDataProvider` trait..."*) thay vì thông tin hướng tới người chơi.
- **Giải pháp đề xuất:**
  - Thay bằng thống kê thực tế của người chơi trong universe đang chọn (tổng số set đã sync, số thẻ đã sở hữu, số pack đã mở) và mô tả trải nghiệm cho các universe sắp ra mắt (`[ACTIVE]` vs `[COMING SOON]`).
- **Widget sử dụng:** Built-in `List`, `Paragraph`, `LineGauge`.
- **Mức độ ưu tiên:** **Low**
- **Chi phí & Ảnh hưởng kỹ thuật:** Rất thấp. Chỉ chỉnh phần render trong `universe_select.rs`.

---

### 2.4. Màn hình Chọn Bộ Sưu Tập — `CollectionSelect` (`src/ui/collection_select.rs`)
- **Vấn đề hiện tại:**
  - Danh sách bên trái thiếu `Scrollbar`.
  - Khối `Cached Pool Rarity Breakdown` bên phải hiển thị dạng dòng text đơn điệu, khó hình dung tỷ lệ phân bố độ hiếm và tiến độ sưu tầm set.
- **Giải pháp đề xuất:**
  - Bổ sung `Scrollbar` cho danh sách set.
  - Dùng `Gauge` / `LineGauge` cho tiến độ hoàn thành set (`unique_owned / total_cached_cards`) và `BarChart` (hoặc thanh tỷ lệ ngang theo màu `Rarity`) cho phân bố độ hiếm trong pool.
  - Đồng bộ spinner loading mượt hơn (tùy chọn `throbber-widgets-tui`).
- **Widget sử dụng:** Built-in `Scrollbar`, `Gauge` / `LineGauge`, `BarChart`.
- **Mức độ ưu tiên:** **Medium**
- **Chi phí & Ảnh hưởng kỹ thuật:** Thấp. Dữ liệu `cached_rarity_counts` và `collection_progress` đã có sẵn trong `App`.

---

### 2.5. Màn hình Chọn Pack — `PackSelect` (`src/ui/pack_select.rs`)
- **Vấn đề hiện tại:**
  - Hình minh họa `Sealed Booster` dùng chung 1 khung ASCII cho cả 3 loại pack (`demo-5`, `booster-10`, `collector-5`), không tạo được cảm giác phân cấp giá trị.
  - Bảng tỷ lệ `Configurable Pack Rules` dùng `Paragraph` nối chuỗi dài, khó đối chiếu khi pack có nhiều slot (như `booster-10`).
- **Giải pháp đề xuất:**
  - Thay panel `Configurable Pack Rules` bằng `Table` có cột rõ ràng (`#`, `Slot`, `Rarity Odds`) kèm `Scrollbar`.
  - Xây dựng **Custom Widget `BoosterPackWidget`** cho vùng `Sealed Booster` với hình dạng bao bì foil răng cưa và màu sắc/hiệu ứng riêng theo từng tier pack.
- **Widget sử dụng:** Built-in `Table`, `Scrollbar` + **Custom Widget `BoosterPackWidget`**.
- **Mức độ ưu tiên:** **High**
- **Chi phí & Ảnh hưởng kỹ thuật:** Thấp–Trung bình. Không ảnh hưởng `PackGenerator`; `BoosterPackWidget` được tái sử dụng trực tiếp ở màn hình `PackOpening`.

---

### 2.6. Màn hình Mở Pack — `PackOpening` (`src/ui/pack_opening.rs`, `src/ui/widgets.rs`)
Đây là màn hình trọng tâm tạo bản sắc game mở thẻ bài.

- **Vấn đề hiện tại:**
  1. Pha `Ripping` quá ngắn và đơn giản (chỉ thụt lề 2 dòng text).
  2. Dải thẻ phía trên (`rows[0]`, `Length(4)`) bị wrap và cắt mất các thẻ cuối với `booster-10` (10 lá).
  3. Pha `Revealing` lật thẻ tức thì, thiếu chuyển động lật/hồi hộp cho thẻ hiếm (`is_hit()`); ảnh thẻ mới bắt đầu gửi yêu cầu resize sau khi lật nên có thể nháy `"Loading artwork..."`.
  4. Pha `Summary` chưa có giao diện tổng kết riêng cho cả pack.
- **Giải pháp đề xuất:**
  1. **Pha `Ripping`:** Dùng `BoosterPackWidget` ở trạng thái đang xé (vết rách tiến triển + tia sáng `✦ ✧` bung ra), hỗ trợ giữ/bấm `Space` để xé hoặc skip.
  2. **Dải thẻ phía trên:** Xây dựng **Custom Widget `PackStripWidget`** hiển thị trên 1 dòng duy nhất, tự cuộn ngang theo `cursor`, có chỉ báo `◀ / ▶` khi vượt bề ngang, tô màu rarity và đánh dấu `NEW!`.
  3. **Hiệu ứng Lật & Hit:**
     - Thêm transition co/giãn khung thẻ khi lật (`CardFlipTransition`) và nhịp dừng hồi hộp ngắn (slow-roll) nếu là thẻ `is_hit()`.
     - Dùng **Custom Widget `SparkleBorderOverlay`** (hoặc `tachyonfx`) tạo hiệu ứng ánh sáng/hạt lấp lánh **trên viền khung và panel chữ**, tuyệt đối không vẽ đè lên vùng pixel ảnh của `StatefulImage` để tương thích Kitty/Sixel.
     - Pre-warm resize ảnh cho thẻ kế tiếp ngay khi người chơi đang xem thẻ trước để khi lật lên là ảnh hiện tức thì.
  4. **Màn hình `Summary` riêng biệt:** Hiển thị banner tổng kết (`Hits`, `New Discoveries`), bảng `Table` toàn bộ các thẻ vừa mở bên trái và preview artwork lớn bên phải.
- **Widget sử dụng:** Built-in `Table` (Summary) + **Custom Widgets**: `BoosterPackWidget`, `PackStripWidget`, `CardFlipTransition`, `SparkleBorderOverlay` (tùy chọn `tachyonfx`).
- **Mức độ ưu tiên:** **High**
- **Chi phí & Ảnh hưởng kỹ thuật:** Trung bình–Cao. Cần mở rộng `PackPhase` trong `src/app.rs` (thêm trạng thái flipping/animation ticks) và tách layout riêng cho `Summary` trong `pack_opening.rs`.

---

### 2.7. Màn hình Bộ Sưu Tập — `MyCollection` (`src/ui/collection_view.rs`)
- **Vấn đề hiện tại:**
  - Thanh filter (`Length(4)`) nhồi quá nhiều thông tin và nối chuỗi progress của mọi set trên 1 dòng → dễ bị cắt chữ.
  - Chế độ List dùng `List` căn lề chuỗi thủ công, thiếu `Scrollbar`.
  - Chế độ Grid cố định cứng `2×2`: thừa chỗ trên màn hình rất lớn và quá chật trên terminal nhỏ; badge `xN` đặt đè lên viền dưới thủ công.
  - Thiếu **Tìm kiếm theo tên** và **Sắp xếp**.
- **Giải pháp đề xuất:**
  - Dùng `Tabs` cho filter theo Set (hoặc pill bar tự wrap) và bổ sung thanh tìm kiếm kích hoạt bằng phím `/` dùng **`tui-input`**, cùng phím sắp xếp (`Newest`, `Rarity ↓`, `Quantity ↓`, `Card #`).
  - Chuyển chế độ List sang **`Table`** với các cột chuẩn (`Rarity`, `Name`, `Set #`, `Qty`, `Price`) + `Scrollbar`.
  - Làm **Grid Responsive** tự chọn `2×1` / `2×2` / `3×2` dựa theo kích thước `area`, chuyển badge `xN` vào `Block::title_bottom(...)`.
- **Widget sử dụng:** Built-in `Tabs`, `Table`, `Scrollbar`, `LineGauge` + crate **`tui-input`**.
- **Mức độ ưu tiên:** **High**
- **Chi phí & Ảnh hưởng kỹ thuật:** Trung bình. Thêm `search_query`, `search_active`, `sort_mode` vào `App`; mở rộng `image_slots` hỗ trợ tối đa `grid-0..grid-7` cho terminal rộng.

---

### 2.8. Màn hình Chi Tiết Thẻ — `CardDetails` (`src/ui/card_details.rs`)
- **Vấn đề hiện tại:**
  - Panel thông tin bên phải dùng `Paragraph` không cuộn được → thẻ có nhiều Ability/Attack sẽ bị cắt nội dung ở terminal thấp.
  - Hiển thị `Cached File: /home/...` mang tính debug nội bộ; chưa phân biệt màu cho hệ/tags (`Fire`, `Water`...).
- **Giải pháp đề xuất:**
  - Thêm cuộn dọc (`detail_scroll: u16` với `↑/↓`, `PgUp/PgDn`) kết hợp `Scrollbar`.
  - Phân nhóm thông tin thành 3 khối trực quan: **Header & Badges** (tô màu tag theo hệ), **Combat & Stats** (căn cột rõ ràng), **Collector Info & Lore** (flavor text in nghiêng, ẩn path debug).
- **Widget sử dụng:** Built-in `Scrollbar`, `Paragraph::scroll(...)`, `Block`.
- **Mức độ ưu tiên:** **Medium**
- **Chi phí & Ảnh hưởng kỹ thuật:** Thấp. Thêm `detail_scroll: u16` vào `App` và reset về `0` mỗi khi đổi thẻ.

---

### 2.9. Màn hình Cài Đặt — `Settings` (`src/ui/settings.rs`)
- **Vấn đề hiện tại:**
  - Bấm `←` / `→` / `Space` / `Enter` trên dòng `Clear Local Image Cache` đều xóa cache ngay mà không hỏi xác nhận (`src/app.rs`).
- **Giải pháp đề xuất:**
  - Phân tách rõ dòng **Option** (đổi bằng `←/→`) và dòng **Action** (chỉ chạy khi bấm `Enter` và mở **Confirm Modal**).
  - Thêm khối preview kiểm tra nhanh màu TrueColor/Protocol ở panel `Graphics Diagnostics`.
- **Widget sử dụng:** Built-in `List`, `Clear` + Confirm Modal Popup.
- **Mức độ ưu tiên:** **High** (khắc phục rủi ro mất cache ngoài ý muốn)
- **Chi phí & Ảnh hưởng kỹ thuật:** Rất thấp. Sửa `handle_settings_key` trong `src/app.rs`.

---

## 3. Danh sách Custom Widget chuyên biệt nên xây dựng

Chỉ xây dựng Custom Widget cho những phần đặc thù của game thẻ bài mà widget chuẩn của Ratatui không đáp ứng tốt:

| # | Custom Widget | Vị trí áp dụng | Chức năng & Cơ chế hoạt động | Giá trị mang lại |
|---|---|---|---|---|
| 1 | **`BoosterPackWidget`** | `PackSelect`, `PackOpening` (`Ripping`) | Vẽ trực tiếp lên `Buffer` bao bì gói bài có răng cưa foil, nhãn set, màu sắc phân cấp theo tier pack và hiệu ứng vết rách tiến triển + tia sáng `✦ ✧`. | Tạo cảm giác cầm và xé gói bài thật, tách biệt khỏi danh sách text thông thường. |
| 2 | **`PackStripWidget`** | `PackOpening` (`Revealing`) | Thanh slot 1 dòng cuộn ngang theo `cursor`, hiển thị trạng thái úp/mở, màu rarity, đánh dấu `NEW!` và mũi tên tràn `◀ / ▶`. | Giải quyết triệt để lỗi wrap vỡ layout cho pack 10+ thẻ trên mọi kích thước terminal. |
| 3 | **`CardFlipTransition` & `SparkleBorderOverlay`** | `PackOpening` (`Revealing`) | Co/giãn chiều ngang khung thẻ khi lật (4–5 tick) + hiệu ứng hạt sáng/shimmer chạy dọc **viền ngoài và góc khung** cho thẻ `is_hit()` (không đè pixel ảnh `StatefulImage`). | Tạo cao trào thị giác khi mở thẻ hiếm mà vẫn an toàn 100% với Kitty/Sixel. |
| 4 | **`CardMiniFrame` (`StatefulWidget`)** | `src/ui/widgets.rs`, `MyCollection` Grid, `PackOpening` | Đóng gói `render_card_artwork_frame` thành widget chuẩn có `title_bottom` cho badge `xN`/`NEW!`, và giao diện fallback offline chia ngăn giống mặt thẻ TCG thu nhỏ. | Loại bỏ code đặt tọa độ badge thủ công và nâng chất lượng hiển thị khi offline/đang tải ảnh. |

---

## 4. Hệ thống thiết kế & Tính nhất quán (Design System)

Cần chuẩn hóa tại `src/ui/theme.rs`:

1. **Color Tokens theo ngữ nghĩa:** Bổ sung màu badge (`BG_BADGE_NEW`, `ACCENT_PURPLE` cho tier Collector/Holo) và bảng màu hệ/nguyên tố `tag_color(tag)` (Fire, Water, Grass, Lightning, Psychic, Fighting, Darkness, Metal, Dragon).
2. **Quy ước viền Focus:** Panel không focus dùng `BORDER_DIM`, panel đang thao tác dùng `BORDER_FOCUS`, thẻ đang chọn/lật dùng `rarity_color`.
3. **Typography & Pill Badges:** Thống nhất định dạng badge có padding 1 space hai đầu (`[ ● C ]`, `[ ★ NEW! ]`, `[ x3 ]`), bỏ text kỹ thuật dành cho dev khỏi màn hình người chơi.
4. **Spacing & Padding:** Dùng `Padding::horizontal(1)` trên `Block` thay vì chèn `"  "` thủ công ở đầu mỗi `Line`, đảm bảo text khi `Wrap` xuống dòng luôn thẳng lề.
5. **Quy ước phím tắt chuẩn:**
   - Điều hướng: `↑/↓/←/→` hoặc `h/j/k/l`.
   - Chuyển vùng / Filter: `Tab` / `Shift+Tab`.
   - Tìm kiếm: `/` (trong `MyCollection`).
   - Trợ giúp: `?` (mọi màn hình).
   - Quay lại / Đóng popup: `Esc` / `Backspace`.
6. **Ngưỡng Responsive:**
   - Rộng `< 95` cột: rút gọn Header/Footer, tự giảm số cột Grid hoặc gợi ý List.
   - Cao `< 26` dòng: kích hoạt `Scrollbar` cho các panel chi tiết/tỷ lệ pack.

---

## 5. Phân biệt phạm vi & Roadmap triển khai

### 5.1. Cải tiến làm ngay (Near-term — tương thích 100% kiến trúc hiện tại)
- Sửa bug `Settings` (`Clear Cache` cần Modal xác nhận) + thêm Modal dùng chung (`Clear`).
- Footer responsive, Floating Toast Banner, Popup `?` Help, chuẩn hóa `Padding` trong `theme.rs`.
- `MyCollection`: chuyển sang `Table` + `Scrollbar`, thêm Search (`tui-input`) & Sort, Grid responsive.
- `PackOpening`: `PackStripWidget`, `BoosterPackWidget`, animation lật thẻ & viền hit, pre-warm resize ảnh, màn hình `Pack Summary` riêng.
- `CardDetails` có cuộn dọc + `Scrollbar`; `PackSelect` dùng `Table` cho Pack Odds; `CollectionSelect` dùng `BarChart`/`LineGauge`.

### 5.2. Ý tưởng mở rộng tương lai (Future — cần thêm logic/dữ liệu mới)
- **Hỗ trợ Chuột (`EnableMouseCapture`):** Click chọn thẻ, click lật bài, cuộn con lăn trong `Table`/`CardDetails`.
- **Tích hợp `tachyonfx`:** Hiệu ứng fade/dissolve chuyển màn hình và holographic sweep nâng cao.
- **Vòng lặp Game hóa sâu hơn:**
  - Đổi thẻ trùng lặp (`quantity > 1`) lấy điểm/bụi để craft thẻ hoặc mở pack đặc biệt.
  - Biểu đồ `Sparkline` theo dõi tổng giá trị bộ sưu tập từ dữ liệu `pricing` của TCGdex.
  - Mini-game đấu bài nhanh sử dụng `HP` và `attacks` có sẵn trong `CardMetadata`.

### 5.3. Lộ trình triển khai (4 Phases)

| Giai đoạn | Mục tiêu | Hạng mục chính | Ưu tiên |
|---|---|---|---|
| **Phase 1: An toàn & Shell chuẩn** | Sửa lỗi UX, hết cắt chữ & giật layout | Modal xác nhận `Settings`, Footer responsive, Floating Toast, Help `?`, chuẩn `Padding`. | **High** |
| **Phase 2: Trải nghiệm Mở Pack** | Tạo điểm nhấn đặc trưng của game TCG | `PackStripWidget`, `BoosterPackWidget`, `CardFlipTransition`, `SparkleBorderOverlay`, pre-warm ảnh, màn hình `Pack Summary`. | **High** |
| **Phase 3: Kho Thẻ & Tra cứu** | Duyệt và quản lý bộ sưu tập mượt mà | `MyCollection` (`Table`, `tui-input` Search, Sort, Responsive Grid), `CardDetails` scroll, `PackSelect` Odds `Table`, `CollectionSelect` `BarChart`/`LineGauge`. | **Medium – High** |
| **Phase 4: Polish & Mở rộng** | Tăng chiều sâu tương tác dài hạn | Mouse capture, xem lại pack từ `MainMenu` history, `tachyonfx`, Dusting/Vault Value analytics. | **Low – Future** |

