use std::{
    process::Command,
    time::{Duration, Instant},
};

use eframe::egui::{
    Align2,
    Button,
    Color32,
    FontFamily,
    FontId,
    Key,
    Rounding,
    Sense,
    Ui,
    Vec2,
    ViewportCommand,
};

/// Main network widget
pub struct NetworkWidget {
    colors: super::Colors,
    wifi_enabled: bool,
    last_update: Instant,
}

impl NetworkWidget {
    pub fn new(colors: super::Colors) -> Self {
        let mut widget = Self {
            colors,
            wifi_enabled: false,
            last_update: Instant::now(),
        };
        
        widget.update();
        widget
    }

    fn get_wifi_status() -> bool {
        // Check if WiFi is enabled using nmcli
        if let Ok(output) = Command::new("nmcli")
            .args(["radio", "wifi"])
            .output() {
            if let Ok(status) = String::from_utf8(output.stdout) {
                return status.trim() == "enabled";
            }
        }
        false
    }

    fn toggle_wifi(&mut self) {
        let new_state = if self.wifi_enabled { "off" } else { "on" };
        
        // Toggle WiFi using nmcli
        if let Ok(_) = Command::new("nmcli")
            .args(["radio", "wifi", new_state])
            .output() {
            self.wifi_enabled = !self.wifi_enabled;
        }
    }

    pub fn should_update(&self) -> bool {
        self.last_update.elapsed() > Duration::from_millis(500)
    }

    pub fn update(&mut self) {
        self.wifi_enabled = Self::get_wifi_status();
        self.last_update = Instant::now();
    }

    pub fn colors(&self) -> &super::Colors {
        &self.colors
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let mut should_close = false;

        // Create a button that shows WiFi status
        let button = Button::new(if self.wifi_enabled { "WiFi: ON" } else { "WiFi: OFF" })
            .min_size(Vec2::new(120.0, 40.0))
            .fill(if self.wifi_enabled {
                self.colors.surface_container_high
            } else {
                Color32::from_black_alpha(128)
            })
            .rounding(Rounding::same(8.0))
            .stroke((
                2.0,
                if self.wifi_enabled {
                    self.colors.primary_fixed_dim
                } else {
                    Color32::WHITE
                }
            ));

        let response = ui.add(button);

        if response.clicked() {
            self.toggle_wifi();
            should_close = true;
        }

        // Handle closing conditions
        if ui.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Enter)) {
            should_close = true;
        }

        if should_close {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
    }

    pub fn cleanup(&mut self) {
        // No resources to clean up for this widget
    }
} 