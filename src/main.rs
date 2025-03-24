use eframe::egui::{CentralPanel, Context, ViewportBuilder, Frame, Color32, Margin, Rounding, Key, ViewportCommand, Vec2};
use clap::Parser;
use std::fs;
use shellexpand;

mod workspace_switcher;
use workspace_switcher::WorkspaceSwitcher;

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
}

impl HyprWidgets {
    fn new(args: Args) -> Self {
        Self {
            workspace_switcher: if args.workspaces {
                Some(WorkspaceSwitcher::new(Colors::new()))
            } else {
                None
            },
        }
    }
}

impl eframe::App for HyprWidgets {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if let Some(switcher) = &mut self.workspace_switcher {
            if switcher.should_update() {
                switcher.update();
                ctx.request_repaint();
            }

            CentralPanel::default()
                .frame(Frame::none())
                .show(ctx, |ui| {
                    ui.set_min_size(Vec2::new(0.0, 92.0));
                    
                    let frame = Frame::none()
                        .fill(switcher.colors().surface_container_low)
                        .rounding(Rounding::same(15.0))
                        .inner_margin(Margin::same(6.0));

                    frame.show(ui, |ui| {
                        ui.spacing_mut().button_padding = Vec2::ZERO;
                        ui.spacing_mut().item_spacing = Vec2::new(10.0, 0.0);
                        
                        switcher.show(ui);
                        
                        let rect = ui.min_rect();
                        ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(
                            rect.width() + 12.0,
                            92.0,
                        )));
                    });
                });
        }

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            // Clean up resources before sending close command
            if let Some(switcher) = &mut self.workspace_switcher {
                switcher.cleanup();
            }
            self.workspace_switcher = None;
            // Now send the close command
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Ensure cleanup happens if we haven't already cleaned up
        if let Some(switcher) = &mut self.workspace_switcher {
            switcher.cleanup();
        }
        self.workspace_switcher = None;
    }
}

fn main() -> eframe::Result<()> {
    let args = Args::parse();
    
    if !args.workspaces {
        eprintln!("No widget specified. Use --workspaces to show the workspace switcher.");
        std::process::exit(1);
    }

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_app_id(APP_ID.to_string())
            .with_inner_size([400.0, 92.0])
            .with_resizable(false),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        APP_ID,
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
            Box::new(HyprWidgets::new(args))
        })
    )
}
