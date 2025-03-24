use eframe::egui::{CentralPanel, Context, ViewportBuilder, Frame, Color32, Margin, Rounding, Key, ViewportCommand, Vec2};
use clap::Parser;
use std::fs;
use shellexpand;
use serde_json;
use std::process::Command;
use std::thread;
use std::time::Duration;

mod workspace_switcher;
mod network_widget;
use workspace_switcher::WorkspaceSwitcher;
use network_widget::NetworkWidget;

/// Application identifier for window manager
const APP_ID: &str = "hypowertools";
/// Path to the colors configuration file
const COLORS_CONFIG_PATH: &str = "~/.config/hypr/hyprland/colors.conf";

/// Command line arguments for the application
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Show workspace switcher widget
    #[arg(long)]
    workspaces: bool,

    /// Show network widget
    #[arg(long)]
    network: bool,

    /// Position of the widget (center, top, top-left, top-right, bottom, bottom-left, bottom-right)
    #[arg(long, default_value = "center")]
    position: Position,

    /// Padding from top edge in pixels
    #[arg(long, default_value = "20")]
    padding_top: i32,

    /// Padding from bottom edge in pixels
    #[arg(long, default_value = "20")]
    padding_bottom: i32,

    /// Padding from left edge in pixels
    #[arg(long, default_value = "20")]
    padding_left: i32,

    /// Padding from right edge in pixels
    #[arg(long, default_value = "20")]
    padding_right: i32,
}

#[derive(Parser, Debug, Clone)]
enum Position {
    Center,
    Top,
    TopLeft,
    TopRight,
    Bottom,
    BottomLeft,
    BottomRight,
}

impl std::str::FromStr for Position {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "center" => Ok(Position::Center),
            "top" => Ok(Position::Top),
            "top-left" => Ok(Position::TopLeft),
            "top-right" => Ok(Position::TopRight),
            "bottom" => Ok(Position::Bottom),
            "bottom-left" => Ok(Position::BottomLeft),
            "bottom-right" => Ok(Position::BottomRight),
            _ => Err(format!("Invalid position: {}", s)),
        }
    }
}

/// Parses an RGBA color string in the format "rgba(rrggbbaa)"
fn parse_rgba_color(rgba_str: &str) -> Option<Color32> {
    if rgba_str.starts_with("rgba(") && rgba_str.ends_with(")") {
        let hex = rgba_str
            .trim_start_matches("rgba(")
            .trim_end_matches(")")
            .trim();

        if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            return Some(Color32::from_rgba_unmultiplied(r, g, b, a));
        }
    }
    None
}

/// Reads color configuration from the config file
fn read_colors_from_config() -> Option<Colors> {
    let config_path = shellexpand::tilde(COLORS_CONFIG_PATH).to_string();
    let content = fs::read_to_string(config_path).ok()?;
    let mut colors = std::collections::HashMap::new();
    
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().trim_start_matches('$');
            let value = value.trim();
            if value.starts_with("rgba(") {
                colors.insert(key.to_string(), value.to_string());
            }
        }
    }
    
    Some(Colors {
        surface_container_low: parse_rgba_color(colors.get("surface_container_low")?)?,
        surface_container_high: parse_rgba_color(colors.get("surface_container_high")?)?,
        on_surface_variant: parse_rgba_color(colors.get("on_surface_variant")?)?,
        on_primary_fixed: parse_rgba_color(colors.get("on_primary_fixed")?)?,
        primary_fixed_dim: parse_rgba_color(colors.get("primary_fixed_dim")?)?,
        surface: parse_rgba_color(colors.get("surface")?)?,
        surface_container: parse_rgba_color(colors.get("surface_container")?)?,
        outline: parse_rgba_color(colors.get("outline")?)?,
    })
}

/// Color configuration for the application
#[derive(Clone)]
pub struct Colors {
    pub surface_container_low: Color32,
    pub surface_container_high: Color32,
    pub on_surface_variant: Color32,
    pub on_primary_fixed: Color32,
    pub primary_fixed_dim: Color32,
    pub surface: Color32,
    pub surface_container: Color32,
    pub outline: Color32,
}

impl Colors {
    fn new() -> Self {
        read_colors_from_config().unwrap_or_else(|| Self {
            surface_container_low: Color32::from_rgba_unmultiplied(27, 27, 33, 255),
            surface_container_high: Color32::from_rgba_unmultiplied(41, 42, 47, 255),
            on_surface_variant: Color32::from_rgba_unmultiplied(198, 197, 208, 255),
            on_primary_fixed: Color32::from_rgba_unmultiplied(8, 22, 75, 255),
            primary_fixed_dim: Color32::from_rgba_unmultiplied(185, 195, 255, 255),
            surface: Color32::from_rgba_unmultiplied(18, 19, 24, 255),
            surface_container: Color32::from_rgba_unmultiplied(31, 31, 37, 255),
            outline: Color32::from_rgba_unmultiplied(144, 144, 154, 255),
        })
    }
}

/// Main application state
struct HyprWidgets {
    workspace_switcher: Option<WorkspaceSwitcher>,
    network_widget: Option<NetworkWidget>,
    position: Position,
    padding_top: i32,
    padding_bottom: i32,
    padding_left: i32,
    padding_right: i32,
}

impl HyprWidgets {
    fn new(args: Args) -> Self {
        let colors = Colors::new();
        Self {
            workspace_switcher: if args.workspaces {
                Some(WorkspaceSwitcher::new(colors.clone()))
            } else {
                None
            },
            network_widget: if args.network {
                Some(NetworkWidget::new(colors))
            } else {
                None
            },
            position: args.position,
            padding_top: args.padding_top,
            padding_bottom: args.padding_bottom,
            padding_left: args.padding_left,
            padding_right: args.padding_right,
        }
    }
}

impl eframe::App for HyprWidgets {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        // First time initialization and positioning
        static mut POSITIONED: bool = false;
        static mut ATTEMPTS: i32 = 0;
        unsafe {
            if !POSITIONED && ATTEMPTS < 5 {
                ATTEMPTS += 1;
                eprintln!("Positioning attempt {}", ATTEMPTS);

                // First find our window
                if let Ok(output) = Command::new("hyprctl")
                    .args(&["clients", "-j"])
                    .output() {
                    if let Ok(output_str) = String::from_utf8(output.stdout) {
                        if let Ok(clients) = serde_json::from_str::<Vec<serde_json::Value>>(&output_str) {
                            // Find our window by class name
                            if let Some(window) = clients.iter().find(|c| {
                                c["class"].as_str().map_or(false, |class| class == APP_ID)
                            }) {
                                if let Some(address) = window["address"].as_str() {
                                    eprintln!("Found our window at address: {}", address);

                                    // Focus our window first
                                    Command::new("hyprctl")
                                        .args(&["dispatch", "focuswindow", APP_ID])
                                        .output()
                                        .ok();

                                    // thread::sleep(Duration::from_millis(100));

                                    // Calculate the actual window size needed based on content
                                    let size = if let Some(ws) = self.workspace_switcher.as_mut() {
                                        // Ensure workspace data is up to date
                                        ws.update();
                                        
                                        // Calculate width based on workspace count
                                        let count = ws.workspace_count();
                                        
                                        // Each workspace button is ~142px wide (80px height * 16/9 aspect ratio + spacing)
                                        // Add padding (12px) and margin (10px spacing between items)
                                        let button_width = 142.0;
                                        let spacing = 10.0;
                                        let padding = 12.0; // 6px on each side
                                        
                                        // Calculate total width including padding and spacing
                                        let width = (count as f32 * button_width) + // Width of all buttons
                                                  ((count.saturating_sub(1)) as f32 * spacing) + // Spacing between buttons
                                                  padding; // Total padding (6px on each side)
                                        
                                        // Keep height fixed at 92px
                                        (width, 92.0)
                                    } else if let Some(nw) = self.network_widget.as_mut() {
                                        // Update network data
                                        nw.update();
                                        
                                        // Use the network widget's size
                                        let size = nw.size();
                                        (size.x, size.y)
                                    } else {
                                        (100.0, 50.0) // Fallback
                                    };

                                    // Calculate position based on the position enum
                                    let (x, y) = match self.position {
                                        Position::Center => (960 - (size.0 / 2.0) as i32, 540 - (size.1 / 2.0) as i32),
                                        Position::Top => (960 - (size.0 / 2.0) as i32, self.padding_top),
                                        Position::TopLeft => (self.padding_left, self.padding_top),
                                        Position::TopRight => (1920 - size.0 as i32 - self.padding_right, self.padding_top),
                                        Position::Bottom => (960 - (size.0 / 2.0) as i32, 1080 - size.1 as i32 - self.padding_bottom),
                                        Position::BottomLeft => (self.padding_left, 1080 - size.1 as i32 - self.padding_bottom),
                                        Position::BottomRight => (1920 - size.0 as i32 - self.padding_right, 1080 - size.1 as i32 - self.padding_bottom),
                                    };

                                    eprintln!("Moving window to position: x={}, y={}", x, y);

                                    // Make window floating and pin it
                                    Command::new("hyprctl")
                                        .args(&["dispatch", "togglefloating", APP_ID])
                                        .output()
                                        .ok();

                                    // thread::sleep(Duration::from_millis(50));

                                    // Move window to position
                                    let move_cmd = format!("hyprctl dispatch movewindowpixel \"exact {} {},address:{}\"", x, y, address);
                                    eprintln!("Running command: {}", move_cmd);
                                    Command::new("sh")
                                        .args(&["-c", &move_cmd])
                                        .output()
                                        .ok();

                                    let resize_cmd = format!("hyprctl dispatch resizewindowpixel \"exact {} {},address:{}\"", size.0, size.1, address);
                                    eprintln!("Running command: {}", resize_cmd);
                                    Command::new("sh")
                                        .args(&["-c", &resize_cmd])
                                        .output()
                                        .ok();
                                    // thread::sleep(Duration::from_millis(50));

                                    let address_arg = format!("address:{}", address);

                                    Command::new("hyprctl")
                                    .args(&["dispatch", "pin", &address_arg])
                                    .output()
                                    .ok();
                                
                         


                                    POSITIONED = true;
                                }
                            }
                        }
                    }
                }

                if !POSITIONED {
                    // Request a repaint to try again
                    ctx.request_repaint();
                }
            }
        }

        if let Some(switcher) = &mut self.workspace_switcher {
            if switcher.should_update() {
                switcher.update();
                ctx.request_repaint();
            }

            let mut size = Vec2::new(400.0, 92.0);
            CentralPanel::default()
                .frame(Frame::none())
                .show(ctx, |ui| {
                    ui.set_min_size(Vec2::new(0.0, 92.0));
                    
                    let frame = Frame::none()
                        .fill(switcher.colors().surface_container_low)
                        .rounding(Rounding::same(15))
                        .inner_margin(Margin::same(6));

                    frame.show(ui, |ui| {
                        ui.spacing_mut().button_padding = Vec2::ZERO;
                        ui.spacing_mut().item_spacing = Vec2::new(10.0, 0.0);
                        
                        switcher.show(ui);
                        
                        let rect = ui.min_rect();
                        size = Vec2::new(rect.width() + 12.0, 92.0);
                    });
                });
            
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
        }

        if let Some(network) = &mut self.network_widget {
            if network.should_update() {
                network.update();
                ctx.request_repaint();
            }

            let mut size = Vec2::new(132.0, 52.0);
            CentralPanel::default()
                .frame(Frame::none())
                .show(ctx, |ui| {
                    let frame = Frame::none()
                        .fill(network.colors().surface_container_low)
                        .rounding(Rounding::same(8))
                        .inner_margin(Margin::same(6));

                    frame.show(ui, |ui| {
                        network.show(ui);
                        
                        // Get the actual size needed for the content
                        let rect = ui.min_rect();
                        size = Vec2::new(rect.width() + 12.0, 52.0);
                    });
                });
            
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
        }

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }
}

fn main() -> eframe::Result<()> {
    let args = Args::parse();
    
    if !args.workspaces && !args.network {
        eprintln!("No widget specified. Use --workspaces for workspace switcher or --network for network widget.");
        std::process::exit(1);
    }

    // Set initial size based on widget type
    let initial_size = if args.workspaces {
        // Start with a reasonable default for one workspace, including padding
        [154.0, 92.0] // 142px (button) + 12px (padding)
    } else {
        [400.0, 434.0] // Keep the network widget's original height
    };

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_app_id(APP_ID.to_string())
            .with_inner_size(initial_size)
            .with_min_inner_size(if args.workspaces {
                [154.0, 92.0] // Minimum size for workspace switcher
            } else {
                [400.0, 434.0] // Fixed size for network widget
            })
            .with_max_inner_size(if args.workspaces {
                [1024.0, 92.0] // Maximum size for workspace switcher
            } else {
                [400.0, 434.0] // Fixed size for network widget
            })
            .with_resizable(args.workspaces), // Only allow resizing for workspace switcher
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        APP_ID,
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
            
            // Initialize Phosphor icons
            let mut fonts = eframe::egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);
            
            Ok(Box::new(HyprWidgets::new(args)))
        })
    )
}