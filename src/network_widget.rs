use std::{
    process::Command,
    time::{Duration, Instant},
};

use eframe::egui::{
    Color32,
    Frame,
    RichText,
    Rounding,
    ScrollArea,
    Ui,
    Vec2,
    Layout,
    Align,
    Button,
    ViewportCommand,
};

// ENHANCEMENT: Add icons using egui_nerdfonts
// To replace text with icons, add the following to Cargo.toml:
//   egui_nerdfonts = "0.1.3"
// 
// Then add this to the main.rs setup:
//   let mut fonts = egui::FontDefinitions::default();
//   egui_nerdfonts::add_to_fonts(&mut fonts, egui_nerdfonts::Variant::Regular);
//   ctx.set_fonts(fonts);
//
// Then replace the text labels with icons like:
// - WiFi icons: egui_nerdfonts::regular::NF_DEV_WIFI (or similar based on signal strength)
// - Connect: egui_nerdfonts::regular::NF_FA_PLUG
// - Disconnect: egui_nerdfonts::regular::NF_FA_UNLINK
// - Forget: egui_nerdfonts::regular::NF_FA_TRASH
// - Unknown networks: egui_nerdfonts::regular::NF_FA_QUESTION

#[derive(Debug, Clone)]
struct WifiNetwork {
    ssid: String,
    signal_strength: i32,
    security: String,
    is_known: bool,
}

#[derive(Debug, Clone)]
enum ConnectionState {
    Disconnected,
    Connected(String),
}

/// Main network widget
pub struct NetworkWidget {
    colors: super::Colors,
    connection_state: ConnectionState,
    known_networks: Vec<WifiNetwork>,
    available_networks: Vec<WifiNetwork>,
    last_update: Instant,
    expanded_network: Option<String>,
    size: Vec2,
}

impl NetworkWidget {
    pub fn new(colors: super::Colors) -> Self {
        let mut widget = Self {
            colors,
            connection_state: ConnectionState::Disconnected,
            known_networks: Vec::new(),
            available_networks: Vec::new(),
            last_update: Instant::now(),
            expanded_network: None,
            size: Vec2::new(400.0, 434.0), // Wider default size
        };
        
        widget.update();
        widget
    }

    fn get_current_network() -> Option<String> {
        if let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "ACTIVE,SSID,SIGNAL", "device", "wifi"])
            .output() {
            if let Ok(output) = String::from_utf8(output.stdout) {
                for line in output.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 2 && parts[0] == "yes" {
                        return Some(parts[1].to_string());
                    }
                }
            }
        }
        None
    }

    fn get_networks() -> (Vec<WifiNetwork>, Vec<WifiNetwork>) {
        let mut known = Vec::new();
        let mut available = Vec::new();

        // Get list of known networks
        if let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "NAME,UUID", "connection", "show"])
            .output() {
            if let Ok(output) = String::from_utf8(output.stdout) {
                for line in output.lines() {
                    if let Some(name) = line.split(':').next() {
                        if !name.contains("ethernet") && !name.contains("loopback") {
                            known.push(WifiNetwork {
                                ssid: name.to_string(),
                                signal_strength: 0,
                                security: String::new(),
                                is_known: true,
                            });
                        }
                    }
                }
            }
        }

        // Get list of available networks
        if let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "SSID,SIGNAL,SECURITY,IN-USE", "device", "wifi", "list"])
            .output() {
            if let Ok(output) = String::from_utf8(output.stdout) {
                for line in output.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 4 {
                        let ssid = parts[0].to_string();
                        let signal = parts[1].parse().unwrap_or(0);
                        let security = parts[2].to_string();
                        
                        // Skip empty SSIDs
                        if ssid.is_empty() {
                            continue;
                        }
                        
                        // Check if this network is already known
                        let is_known = known.iter().any(|n| n.ssid == ssid);
                        
                        let network = WifiNetwork {
                            ssid,
                            signal_strength: signal,
                            security,
                            is_known,
                        };

                        if is_known {
                            // Update known network with signal strength and security
                            if let Some(known_net) = known.iter_mut().find(|n| n.ssid == network.ssid) {
                                known_net.signal_strength = network.signal_strength;
                                known_net.security = network.security;
                            }
                        } else {
                            available.push(network);
                        }
                    }
                }
            }
        }

        // Sort networks by signal strength
        known.sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));
        available.sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));

        (known, available)
    }

    pub fn should_update(&self) -> bool {
        self.last_update.elapsed() > Duration::from_millis(1000)
    }

    pub fn update(&mut self) {
        let current = Self::get_current_network();
        let connection_changed = match (&self.connection_state, &current) {
            (ConnectionState::Connected(old), Some(new)) => old != new,
            (ConnectionState::Connected(_), None) => true,
            (ConnectionState::Disconnected, Some(_)) => true,
            _ => false,
        };
        
        // Update connection state
        if let Some(current) = current {
            self.connection_state = ConnectionState::Connected(current);
        } else {
            self.connection_state = ConnectionState::Disconnected;
        }
        
        // Only fetch all networks if connection changed or none are available
        if connection_changed || self.known_networks.is_empty() && self.available_networks.is_empty() {
            let (known, available) = Self::get_networks();
            self.known_networks = known;
            self.available_networks = available;
        }
        self.last_update = Instant::now();
    }

    pub fn colors(&self) -> &super::Colors {
        &self.colors
    }

    fn get_signal_icon(strength: i32) -> &'static str {
        if strength >= 80 { egui_phosphor::regular::WIFI_HIGH }
        else if strength >= 60 { egui_phosphor::regular::WIFI_MEDIUM }
        else if strength >= 40 { egui_phosphor::regular::WIFI_LOW }
        else if strength >= 20 { egui_phosphor::regular::WIFI_SLASH }
        else { egui_phosphor::regular::WIFI_X }
    }
    
    // Helper function to get button text and icon
    fn get_button_config(button_type: &str) -> String {
        match button_type {
            "connect" => egui_phosphor::regular::PLUG.to_string(),
            "disconnect" => egui_phosphor::regular::PLUG_CHARGING.to_string(),
            "forget" => egui_phosphor::regular::TRASH.to_string(),
            _ => egui_phosphor::regular::WARNING.to_string(),
        }
    }

    fn get_unknown_indicator() -> &'static str {
        egui_phosphor::regular::QUESTION
    }

    fn get_security_icon() -> &'static str {
        egui_phosphor::regular::LOCK
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let mut size = self.size;

        // Main panel
        Frame::new()
            .fill(self.colors.surface_container_low)
            .corner_radius(12)
            .inner_margin(8.0)
            .show(ui, |ui| {
                // Set fixed width and height for the main panel
                ui.set_width(400.0); // Wider to accommodate scrollbar
                ui.set_min_height(434.0);

                // Combined networks list
                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .max_height(434.0 - 16.0) // Account for padding
                    .show(ui, |ui| {
                        ui.set_width(384.0); // Wider content area for proper layout
                        
                        // Collect networks to display first
                        let mut networks_to_show = Vec::new();
                        let current_network = if let ConnectionState::Connected(ref current) = self.connection_state {
                            Some(current.clone())
                        } else {
                            None
                        };
                        
                        // Add connected network first
                        if let Some(current) = &current_network {
                            if let Some(network) = self.known_networks.iter()
                                .find(|n| &n.ssid == current && n.signal_strength > 0)
                                .or_else(|| self.available_networks.iter()
                                    .find(|n| &n.ssid == current && n.signal_strength > 0)) {
                                networks_to_show.push((network.clone(), true));
                            }
                        }

                        // Add known networks
                        for network in &self.known_networks {
                            if Some(&network.ssid) != current_network.as_ref() && network.signal_strength > 0 {
                                networks_to_show.push((network.clone(), false));
                            }
                        }

                        // Add available networks
                        for network in &self.available_networks {
                            if Some(&network.ssid) != current_network.as_ref() && network.signal_strength > 0 {
                                networks_to_show.push((network.clone(), false));
                            }
                        }

                        // Now display all networks
                        let total = networks_to_show.len();
                        for (idx, (network, is_connected)) in networks_to_show.into_iter().enumerate() {
                            let text = network.ssid.clone();
                            let is_expanded = self.expanded_network.as_ref().map_or(false, |n| n == &network.ssid);

                            let color = if is_connected {
                                self.colors.primary_fixed_dim
                            } else {
                                self.colors.on_surface_variant
                            };

                            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                                // Main network entry row
                                let response = ui.vertical(|ui| {
                                    // Main network entry
                                    let row_height = 32.0;
                                    let button = Button::new("")
                                        .fill(Color32::TRANSPARENT)
                                        .frame(false)
                                        .min_size(Vec2::new(ui.available_width(), row_height));
                                    
                                    let button_response = ui.add_sized([ui.available_width(), row_height], button);
                                    
                                    // Overlay the content on top of the button
                                    let rect = button_response.rect;
                                    ui.allocate_ui_at_rect(rect, |ui| {
                                        ui.horizontal(|ui| {
                                            // Network name on the left
                                            ui.add_space(8.0);
                                            ui.label(RichText::new(&text).color(color).size(16.0));
                                            
                                            // Push the remaining elements to the right
                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                ui.add_space(8.0);
                                                // Show ? for unknown networks
                                                if !network.is_known {
                                                    ui.label(RichText::new(Self::get_unknown_indicator()).color(self.colors.outline).size(20.0));
                                                    ui.add_space(4.0);
                                                }
                                                // Signal strength indicator
                                                ui.label(RichText::new(Self::get_signal_icon(network.signal_strength))
                                                    .color(if is_expanded { self.colors.primary_fixed_dim } else { color })
                                                    .size(20.0));
                                            });
                                        });
                                    });

                                    // If expanded, show a second row with buttons
                                    if is_expanded {
                                        // Use a fixed height for the button row
                                        let buttons_height = 32.0;
                                        ui.allocate_exact_size(
                                            Vec2::new(ui.available_width(), buttons_height),
                                            eframe::egui::Sense::hover()
                                        );
                                        
                                        // Position the buttons directly
                                        let button_height = 32.0;
                                        let button_width = 36.0;
                                        let spacing = 10.0;
                                        
                                        // Security indicator on the left
                                        if !network.security.is_empty() && network.security != "none" {
                                            let security_rect = eframe::egui::Rect::from_min_size(
                                                eframe::egui::pos2(
                                                    rect.left() + 8.0,  // Add left padding
                                                    rect.max.y + 4.0
                                                ),
                                                eframe::egui::vec2(20.0, button_height)
                                            );
                                            
                                            ui.put(
                                                security_rect,
                                                Button::new(RichText::new(Self::get_security_icon()).color(self.colors.outline).size(18.0))
                                                .fill(Color32::TRANSPARENT)
                                                .frame(false)
                                            );
                                            
                                            // Display security type (WPA, WEP, etc.)
                                            let security_text_rect = eframe::egui::Rect::from_min_size(
                                                eframe::egui::pos2(
                                                    rect.left() + 28.0,  // Adjust for left padding
                                                    rect.max.y + 4.0
                                                ),
                                                eframe::egui::vec2(80.0, button_height)
                                            );
                                            
                                            // Format the security type for display
                                            let security_text = if network.security.contains("WPA2") {
                                                "WPA2"
                                            } else if network.security.contains("WPA3") {
                                                "WPA3"
                                            } else if network.security.contains("WPA") {
                                                "WPA"
                                            } else if network.security.contains("WEP") {
                                                "WEP"
                                            } else {
                                                &network.security
                                            };
                                            
                                            ui.allocate_ui_at_rect(security_text_rect, |ui| {
                                                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                                    ui.label(RichText::new(security_text).color(self.colors.outline).size(14.0));
                                                });
                                            });
                                        }
                                        
                                        // Use the parent rect's width for proper alignment
                                        let right_edge = rect.right() - 8.0;  // Add right padding
                                        
                                        if is_connected {
                                            // Connected network - Disconnect and Forget
                                            
                                            // Calculate positions for right-aligned buttons
                                            let disconnect_rect = eframe::egui::Rect::from_min_size(
                                                eframe::egui::pos2(
                                                    right_edge - button_width,
                                                    rect.max.y + 4.0
                                                ),
                                                eframe::egui::vec2(button_width, button_height)
                                            );
                                            
                                            let forget_rect = eframe::egui::Rect::from_min_size(
                                                eframe::egui::pos2(
                                                    right_edge - (button_width * 2.0) - spacing,
                                                    rect.max.y + 4.0
                                                ),
                                                eframe::egui::vec2(button_width, button_height)
                                            );
                                            
                                            // Styled Disconnect button
                                            if ui.put(
                                                disconnect_rect,
                                                Button::new(RichText::new(Self::get_button_config("disconnect")).color(self.colors.primary_fixed_dim).size(18.0))
                                                .fill(self.colors.surface_container)
                                                .corner_radius(6)
                                                .stroke(eframe::egui::Stroke::new(1.5, self.colors.primary_fixed_dim))
                                            ).clicked() {
                                                Command::new("nmcli")
                                                    .args(["device", "disconnect", "wifi"])
                                                    .spawn()
                                                    .ok();
                                            }
                                            
                                            // Styled Forget button
                                            if ui.put(
                                                forget_rect,
                                                Button::new(RichText::new(Self::get_button_config("forget")).color(self.colors.outline).size(18.0))
                                                .fill(self.colors.surface_container)
                                                .corner_radius(6)
                                                .stroke(eframe::egui::Stroke::new(1.5, self.colors.outline))
                                            ).clicked() {
                                                Command::new("nmcli")
                                                    .args(["connection", "delete", &text])
                                                    .spawn()
                                                    .ok();
                                            }
                                        } else if network.is_known {
                                            // Known network - Connect and Forget
                                            
                                            // Calculate positions for right-aligned buttons
                                            let connect_rect = eframe::egui::Rect::from_min_size(
                                                eframe::egui::pos2(
                                                    right_edge - button_width,
                                                    rect.max.y + 4.0
                                                ),
                                                eframe::egui::vec2(button_width, button_height)
                                            );
                                            
                                            let forget_rect = eframe::egui::Rect::from_min_size(
                                                eframe::egui::pos2(
                                                    right_edge - (button_width * 2.0) - spacing,
                                                    rect.max.y + 4.0
                                                ),
                                                eframe::egui::vec2(button_width, button_height)
                                            );
                                            
                                            // Styled Connect button
                                            if ui.put(
                                                connect_rect,
                                                Button::new(RichText::new(Self::get_button_config("connect")).color(self.colors.primary_fixed_dim).size(18.0))
                                                .fill(self.colors.surface_container)
                                                .corner_radius(6)
                                                .stroke(eframe::egui::Stroke::new(1.5, self.colors.primary_fixed_dim))
                                            ).clicked() {
                                                Command::new("nmcli")
                                                    .args(["connection", "up", &text])
                                                    .spawn()
                                                    .ok();
                                            }
                                            
                                            // Styled Forget button
                                            if ui.put(
                                                forget_rect,
                                                Button::new(RichText::new(Self::get_button_config("forget")).color(self.colors.outline).size(18.0))
                                                .fill(self.colors.surface_container)
                                                .corner_radius(6)
                                                .stroke(eframe::egui::Stroke::new(1.5, self.colors.outline))
                                            ).clicked() {
                                                Command::new("nmcli")
                                                    .args(["connection", "delete", &text])
                                                    .spawn()
                                                    .ok();
                                            }
                                        } else {
                                            // Unknown network - Connect only
                                            
                                            // Calculate position for right-aligned button
                                            let connect_rect = eframe::egui::Rect::from_min_size(
                                                eframe::egui::pos2(
                                                    right_edge - button_width,
                                                    rect.max.y + 4.0
                                                ),
                                                eframe::egui::vec2(button_width, button_height)
                                            );
                                            
                                            // Styled Connect button for unknown networks
                                            if ui.put(
                                                connect_rect,
                                                Button::new(RichText::new(Self::get_button_config("connect")).color(self.colors.primary_fixed_dim).size(18.0))
                                                .fill(self.colors.surface_container)
                                                .corner_radius(6)
                                                .stroke(eframe::egui::Stroke::new(1.5, self.colors.primary_fixed_dim))
                                            ).clicked() {
                                                // For new networks, we need to implement password dialog
                                                // For now, we'll just print a message
                                                eprintln!("Would connect to new network: {}", text);
                                            }
                                        }
                                    }
                                    
                                    button_response
                                });

                                // Allow clicking on any network type
                                if response.inner.clicked() {
                                    if is_expanded {
                                        self.expanded_network = None;
                                    } else {
                                        self.expanded_network = Some(text);
                                    }
                                }
                            });

                            if idx < total - 1 {
                                ui.add_space(4.0);
                            }
                        }

                        // Get the actual size needed for the content
                        size = Vec2::new(400.0, 434.0); // Keep the fixed larger size
                    });
            });
        
        // Update our stored size
        self.size = size;
        
        // Send the size update
        ui.ctx().send_viewport_cmd(ViewportCommand::InnerSize(size));
    }

    // Add a getter for size
    pub fn size(&self) -> Vec2 {
        self.size
    }
}