use image::DynamicImage;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::thread::{ResizeRequest, ResizeResponse, ThreadProtocol};
use ratatui_image::{FilterType, Resize, StatefulImage};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use crate::domain::ProtocolMode;

/// Wraps `ratatui_image::picker::Picker` with auto-detection, safe fallback, and runtime mode switching.
#[derive(Clone, Debug)]
pub struct ImageRenderer {
    detected_protocol: ProtocolType,
    active_mode: ProtocolMode,
    picker: Picker,
}

impl ImageRenderer {
    /// Probes terminal stdio for native image protocols (Kitty, Sixel, iTerm2) and font cell dimensions.
    /// Safely falls back to Unicode Half-Blocks if the terminal does not respond or lacks graphics support.
    pub fn detect(mode: ProtocolMode) -> Self {
        use std::io::IsTerminal;

        let should_probe = mode != ProtocolMode::Halfblocks
            && std::io::stdin().is_terminal()
            && std::io::stdout().is_terminal()
            && std::env::var("TERM").as_deref() != Ok("dumb");

        let mut picker = if should_probe {
            Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks())
        } else {
            Picker::halfblocks()
        };
        let detected_protocol = picker.protocol_type();
        Self::apply_mode_to_picker(&mut picker, detected_protocol, mode);

        Self {
            detected_protocol,
            active_mode: mode,
            picker,
        }
    }

    /// Creates a deterministic halfblocks renderer suitable for non-TTY tests.
    pub fn halfblocks_only() -> Self {
        let picker = Picker::halfblocks();
        Self {
            detected_protocol: ProtocolType::Halfblocks,
            active_mode: ProtocolMode::Halfblocks,
            picker,
        }
    }

    pub fn detected_protocol(&self) -> ProtocolType {
        self.detected_protocol
    }

    pub fn active_protocol(&self) -> ProtocolType {
        self.picker.protocol_type()
    }

    pub fn active_mode(&self) -> ProtocolMode {
        self.active_mode
    }

    pub fn font_size(&self) -> (u16, u16) {
        self.picker.font_size()
    }

    pub fn set_mode(&mut self, mode: ProtocolMode) {
        self.active_mode = mode;
        Self::apply_mode_to_picker(&mut self.picker, self.detected_protocol, mode);
    }

    fn apply_mode_to_picker(
        picker: &mut Picker,
        detected: ProtocolType,
        mode: ProtocolMode,
    ) {
        let target = match mode {
            ProtocolMode::Auto => detected,
            ProtocolMode::Halfblocks => ProtocolType::Halfblocks,
            ProtocolMode::Kitty => ProtocolType::Kitty,
            ProtocolMode::Sixel => ProtocolType::Sixel,
            ProtocolMode::Iterm2 => ProtocolType::Iterm2,
        };
        picker.set_protocol_type(target);
    }

    pub fn create_protocol(&self, img: DynamicImage) -> ratatui_image::protocol::StatefulProtocol {
        self.picker.new_resize_protocol(img)
    }

    pub fn protocol_display_name(proto: ProtocolType) -> &'static str {
        match proto {
            ProtocolType::Halfblocks => "Unicode Half-Blocks",
            ProtocolType::Kitty => "Kitty Graphics",
            ProtocolType::Sixel => "Sixel Bitmap",
            ProtocolType::Iterm2 => "iTerm2 Inline",
        }
    }
}

/// Message emitted when a background resize+encode operation finishes for a specific image slot.
pub struct ResizedSlotEvent {
    pub slot_key: String,
    pub response: ResizeResponse,
}

/// Non-blocking image slot backed by `ThreadProtocol` and a dedicated `spawn_blocking` resize worker.
pub struct CardImageSlot {
    pub slot_key: String,
    pub protocol: ThreadProtocol,
    pub assigned_card_id: Option<String>,
    pub loading: bool,
    pub error: Option<String>,
}

impl CardImageSlot {
    pub fn new(
        slot_key: impl Into<String>,
        done_tx: UnboundedSender<ResizedSlotEvent>,
    ) -> Self {
        let slot_key = slot_key.into();
        let (req_tx, mut req_rx) = unbounded_channel::<ResizeRequest>();
        let worker_key = slot_key.clone();

        tokio::spawn(async move {
            while let Some(request) = req_rx.recv().await {
                let key = worker_key.clone();
                let res = tokio::task::spawn_blocking(move || request.resize_encode()).await;
                if let Ok(Ok(response)) = res {
                    let _ = done_tx.send(ResizedSlotEvent {
                        slot_key: key,
                        response,
                    });
                }
            }
        });

        Self {
            slot_key,
            protocol: ThreadProtocol::new(req_tx, None),
            assigned_card_id: None,
            loading: false,
            error: None,
        }
    }

    pub fn begin_loading(&mut self, card_id: &str) {
        self.assigned_card_id = Some(card_id.to_string());
        self.loading = true;
        self.error = None;
        self.protocol.empty_protocol();
    }

    pub fn set_image(&mut self, card_id: &str, renderer: &ImageRenderer, img: DynamicImage) {
        self.assigned_card_id = Some(card_id.to_string());
        self.loading = false;
        self.error = None;
        self.protocol.replace_protocol(renderer.create_protocol(img));
    }

    pub fn set_error(&mut self, card_id: &str, msg: String) {
        if self.assigned_card_id.as_deref() == Some(card_id) {
            self.loading = false;
            self.error = Some(msg);
            self.protocol.empty_protocol();
        }
    }

    pub fn clear(&mut self) {
        self.assigned_card_id = None;
        self.loading = false;
        self.error = None;
        self.protocol.empty_protocol();
    }

    pub fn apply_resize(&mut self, response: ResizeResponse) -> bool {
        self.protocol.update_resized_protocol(response)
    }

    pub fn has_ready_protocol(&self) -> bool {
        self.protocol.protocol_type().is_some()
    }

    /// Renders the card image scaled proportionally to fit `area` while preserving aspect ratio
    /// and centering the resulting image based on the terminal's detected font aspect ratio.
    pub fn render_into(&mut self, frame: &mut Frame<'_>, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let resize = Resize::Scale(Some(FilterType::Triangle));
        let target_area = match self.protocol.size_for(resize.clone(), area) {
            Some(fitted) if fitted.width > 0 && fitted.height > 0 => {
                let x = area.x + area.width.saturating_sub(fitted.width) / 2;
                let y = area.y + area.height.saturating_sub(fitted.height) / 2;
                Rect::new(x, y, fitted.width.min(area.width), fitted.height.min(area.height))
            }
            _ => area,
        };
        let widget = StatefulImage::default().resize(resize);
        frame.render_stateful_widget(widget, target_area, &mut self.protocol);
    }
}
