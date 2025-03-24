use std::{
    fs,
    process::Command,
    time::{Duration, Instant},
    collections::HashMap,
    path::Path,
    cell::RefCell,
};

use eframe::egui::{

    Align2,
    Button,
    Color32,
    FontFamily,
    FontId,
    Image,
    Key,
    Rounding,
    Sense,
    TextureHandle,
    Ui,
    Vec2,
    Rect,
    Pos2,
    ViewportCommand,
};

use serde::{Deserialize, Serialize};
use resvg::usvg;
use tiny_skia::Pixmap;
use shellexpand;

/// Path to the colors configuration file
const COLORS_CONFIG_PATH: &str = "~/.config/hypr/hyprland/colors.conf";
/// Default icon size used throughout the application


/// Represents a Hyprland workspace
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Workspace {
    id: i32,
    name: String,
}

/// Represents a window in Hyprland with its properties
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Window {
    workspace: WorkspaceInfo,
    class: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    address: String,
    #[serde(default)]
    mapped: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    at: Vec<i32>,
    #[serde(default)]
    size: Vec<i32>,
    #[serde(default)]
    floating: bool,
    #[serde(default)]
    pseudo: bool,
    #[serde(default)]
    monitor: i32,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    title: String,
    #[serde(rename = "initialClass")]
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    initial_class: String,
    #[serde(rename = "initialTitle")]
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    initial_title: String,
    #[serde(default)]
    pid: i32,
    #[serde(default)]
    xwayland: bool,
    #[serde(default)]
    pinned: bool,
    #[serde(default)]
    fullscreen: i32,
    #[serde(rename = "fullscreenClient")]
    #[serde(default)]
    fullscreen_client: i32,
    #[serde(default)]
    grouped: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    swallowing: String,
    #[serde(rename = "focusHistoryID")]
    #[serde(default)]
    focus_history_id: i32,
    #[serde(rename = "inhibitingIdle")]
    #[serde(default)]
    inhibiting_idle: bool,
}

/// Information about a workspace
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct WorkspaceInfo {
    id: i32,
    name: String,
}

/// Information about a monitor
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Monitor {
    id: i32,
    name: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    #[serde(rename = "activeWorkspace")]
    active_workspace: WorkspaceInfo,
}

/// Cache for storing loaded application icons
struct IconCache {
    cache: RefCell<HashMap<String, Option<TextureHandle>>>,
}

impl IconCache {
    fn new() -> Self {
        Self {
            cache: RefCell::new(HashMap::new()),
        }
    }

    fn get_or_load(&self, ui: &mut Ui, class_name: &str) -> Option<TextureHandle> {
        if let Some(cached_icon) = self.cache.borrow().get(class_name) {
            return cached_icon.clone();
        }

        // Special case mappings for known apps
        let lookup_class = match class_name {
            "Cursor" => "com.cursor.Cursor",
            "discord" => "com.discordapp.Discord",
            // Handle both native and Flatpak Discord
            "Discord" => "com.discordapp.Discord",
            _ => class_name
        };

        // Additional Flatpak-specific paths for Discord
        if lookup_class == "com.discordapp.Discord" {
            let flatpak_paths = [
                "/var/lib/flatpak/app/com.discordapp.Discord/current/active/files/discord/discord.png",
                "/var/lib/flatpak/app/com.discordapp.Discord/current/active/export/share/icons/hicolor/256x256/apps/com.discordapp.Discord.png",
                "~/.local/share/flatpak/app/com.discordapp.Discord/current/active/files/discord/discord.png",
            ];

            for path in &flatpak_paths {
                let expanded_path = shellexpand::tilde(path).to_string();
                if Path::new(&expanded_path).exists() {
                    return self.load_png(&expanded_path, ui);
                }
            }
        }

        // Use the exact reliable command to find desktop files
        let output = Command::new("find")
            .args([
                "/usr/share/applications",
                "~/.local/share/applications",
                "/var/lib/flatpak/exports/share/applications",
                "~/.local/share/flatpak/exports/share/applications",
                "-name",
                "*.desktop"
            ])
            .output()
            .ok()?;

        let desktop_files = String::from_utf8(output.stdout).ok()?;
        let mut icon_path = None;
        let mut found_icon_name = None;

        // First pass: try to find exact class match in desktop files
        'desktop_search: for path in desktop_files.lines() {
            let expanded_path = shellexpand::tilde(path).to_string();
            if let Ok(content) = fs::read_to_string(&expanded_path) {
                // Check if this desktop file is for our app
                let is_match = content.lines().any(|line| {
                    (line.starts_with("Name=") || line.starts_with("Exec=")) && 
                    (line.to_lowercase().contains(&lookup_class.to_lowercase()) ||
                     line.to_lowercase().contains(&class_name.to_lowercase()))
                });

                if !is_match {
                    continue;
                }

                // Found matching desktop file, get icon name
                for line in content.lines() {
                    if line.starts_with("Icon=") {
                        found_icon_name = Some(line.trim_start_matches("Icon=").to_string());
                        break;
                    }
                }
            }
        }

        // If we found an icon name, try all possible paths
        if let Some(icon_name) = found_icon_name.as_ref().or(Some(&lookup_class.to_string())) {
            let icon_theme_paths = [
                // Flatpak-specific paths first
                "/var/lib/flatpak/exports/share/icons/hicolor",
                "~/.local/share/flatpak/exports/share/icons/hicolor",
                // Then system paths
                "/usr/share/icons/hicolor",
                "/usr/share/icons/Papirus",
                "/usr/share/icons/breeze",
                "/usr/share/icons/default",
                "~/.local/share/icons",
            ];

            let sizes = ["256x256", "128x128", "64x64", "48x48", "32x32", "24x24", "16x16", "scalable"];
            let categories = ["apps", "devices", "places", "status"];

            // Try variations of the icon name
            let icon_variations = [
                icon_name.to_string(),
                icon_name.to_lowercase(),
                icon_name.replace('.', "-"),
                icon_name.replace('.', "-").to_lowercase(),
                // Add more variations for Flatpak apps
                format!("com.discordapp.{}", icon_name),  // For Discord specifically
                format!("{}.png", icon_name),  // Some Flatpak apps use direct filenames
            ];

            'icon_search: for theme_path in &icon_theme_paths {
                let expanded_theme_path = shellexpand::tilde(theme_path).to_string();
                for size in &sizes {
                    for category in &categories {
                        for variation in &icon_variations {
                            let possible_paths = [
                                format!("{}/{}/{}/{}.png", expanded_theme_path, size, category, variation),
                                format!("{}/{}/{}/{}.svg", expanded_theme_path, size, category, variation),
                            ];

                            for path in &possible_paths {
                                if Path::new(path).exists() {
                                    icon_path = Some(path.clone());
                                    break 'icon_search;
                                }
                            }
                        }
                    }
                }
            }

            // Try direct paths and pixmaps as last resort
            if icon_path.is_none() {
                let fallback_paths = [
                    format!("/usr/share/pixmaps/{}.png", icon_name),
                    format!("/usr/share/pixmaps/{}.svg", icon_name),
                    format!("/usr/share/pixmaps/{}.xpm", icon_name),
                    icon_name.to_string(), // In case it's a full path
                ];

                for path in &fallback_paths {
                    let expanded_path = shellexpand::tilde(path).to_string();
                    if Path::new(&expanded_path).exists() {
                        icon_path = Some(expanded_path);
                        break;
                    }
                }
            }
        }

        let icon = if let Some(path) = icon_path {
            if path.ends_with(".svg") {
                self.load_svg(&path, ui)
            } else {
                self.load_png(&path, ui)
            }
        } else {
            None
        };

        self.cache.borrow_mut().insert(class_name.to_string(), icon.clone());
        icon
    }

    fn load_svg(&self, path: &str, ui: &mut Ui) -> Option<TextureHandle> {
        let svg_data = fs::read(path).ok()?;
        let opt = usvg::Options::default();
        let rtree = usvg::Tree::from_data(&svg_data, &opt).ok()?;
        
        let size = 24;
        let mut pixmap = Pixmap::new(size, size)?;
        
        // Calculate scale to maintain aspect ratio
        let scale = (size as f32 / rtree.size().width())
            .min(size as f32 / rtree.size().height());
            
        // Center the icon
        let translate_x = (size as f32 - rtree.size().width() * scale) / 2.0;
        let translate_y = (size as f32 - rtree.size().height() * scale) / 2.0;
        
        let transform = tiny_skia::Transform::from_scale(scale, scale)
            .post_translate(translate_x, translate_y);
            
        resvg::render(&rtree, transform, &mut pixmap.as_mut());
        
        Some(ui.ctx().load_texture(
            format!("svg-icon-{}", path),
            eframe::epaint::ColorImage::from_rgba_unmultiplied(
                [size as usize, size as usize],
                pixmap.data()
            ),
            Default::default(),
        ))
    }

    fn load_png(&self, path: &str, ui: &mut Ui) -> Option<TextureHandle> {
        let img = image::open(path).ok()?;
        let size = 24;
        let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
        let rgba = resized.to_rgba8();
        
        Some(ui.ctx().load_texture(
            format!("png-icon-{}", path),
            eframe::epaint::ColorImage::from_rgba_unmultiplied(
                [size as usize, size as usize],
                &rgba.into_raw(),
            ),
            Default::default(),
        ))
    }
}

/// Main workspace switcher widget
pub struct WorkspaceSwitcher {
    colors: super::Colors,
    current_workspace: i32,
    workspaces: Vec<Workspace>,
    last_update: Instant,
    background: Option<TextureHandle>,
    icon_cache: IconCache,
    selected_window: Option<String>,
}

impl WorkspaceSwitcher {
    pub fn new(colors: super::Colors) -> Self {
        let mut switcher = Self {
            colors,
            current_workspace: 1,
            workspaces: Vec::new(),
            last_update: Instant::now(),
            background: None,
            icon_cache: IconCache::new(),
            selected_window: None,
        };
        
        switcher.update();
        switcher
    }

    fn get_background_path() -> Option<String> {
        let config_path = shellexpand::tilde(COLORS_CONFIG_PATH).to_string();
        if let Ok(content) = fs::read_to_string(config_path) {
            for line in content.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim().trim_start_matches('$');
                    let value = value.trim();
                    if key == "image" {
                        return Some(shellexpand::tilde(value.trim_matches('"')).to_string());
                    }
                }
            }
        }
        None
    }

    fn get_workspaces() -> Vec<Workspace> {
        if let Ok(output) = Command::new("hyprctl").args(&["workspaces", "-j"]).output() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if let Ok(mut workspaces) = serde_json::from_str::<Vec<Workspace>>(&stdout) {
                    workspaces.sort_by_key(|w| w.id);
                    return workspaces;
                }
            }
        }
        Vec::new()
    }

    fn get_current_workspace() -> i32 {
        if let Ok(output) = Command::new("hyprctl").args(&["activeworkspace", "-j"]).output() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if let Ok(workspace) = serde_json::from_str::<Workspace>(&stdout) {
                    return workspace.id;
                }
            }
        }
        1
    }

    fn get_windows() -> Vec<Window> {
        let output = match Command::new("hyprctl")
            .args(["clients", "-j"])
            .output() {
                Ok(output) => output,
                Err(_) => return Vec::new(),
            };

        let output_str = match String::from_utf8(output.stdout) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        match serde_json::from_str::<Vec<Window>>(&output_str) {
            Ok(windows) => windows,
            Err(_) => Vec::new(),
        }
    }


    fn switch_to_workspace(&mut self, workspace_id: i32) {
        if let Some(workspace) = self.workspaces.iter().find(|w| w.id == workspace_id) {
            // First switch to the workspace
            Command::new("hyprctl")
                .args(&["dispatch", "workspace", &workspace.name])
                .output()
                .ok();

        }
    }

    pub fn should_update(&self) -> bool {
        self.last_update.elapsed() > Duration::from_millis(500)
    }

    pub fn update(&mut self) {
        self.workspaces = Self::get_workspaces();
        self.current_workspace = Self::get_current_workspace();
        self.last_update = Instant::now();
    }

    pub fn colors(&self) -> &super::Colors {
        &self.colors
    }

    pub fn workspaces(&self) -> &Vec<Workspace> {
        &self.workspaces
    }

    pub fn workspace_count(&self) -> usize {
        self.workspaces.len()
    }

    fn get_app_icon(&self, ui: &mut Ui, class_name: &str) -> Option<TextureHandle> {
        self.icon_cache.get_or_load(ui, class_name)
    }

    pub fn show(&mut self, ui: &mut Ui) {
        // Load background image if not loaded
        if self.background.is_none() {
            if let Some(path) = Self::get_background_path() {
                let _ = image::io::Reader::open(&path)
                    .map_err(|_| ())
                    .and_then(|reader| reader.decode().map_err(|_| ()))
                    .map(|image| {
                        let size = [image.width() as _, image.height() as _];
                        let pixels = image.to_rgba8();
                        self.background = Some(ui.ctx().load_texture(
                            "workspace-bg",
                            eframe::epaint::ColorImage::from_rgba_unmultiplied(
                                size,
                                pixels.as_raw(),
                            ),
                            Default::default(),
                        ));
                    });
            }
        }

        let mut workspace_to_switch = None;
        let mut should_close = false;
        let windows = Self::get_windows();
        let workspaces = self.workspaces.clone();
        let current_workspace = self.current_workspace;
        let colors = &self.colors;

        // Handle arrow key navigation and Tab
        if ui.input(|i| i.key_pressed(Key::ArrowLeft)) {
            if let Some(current_idx) = workspaces.iter().position(|w| w.id == current_workspace) {
                if current_idx > 0 {
                    workspace_to_switch = Some(workspaces[current_idx - 1].id);
                }
            }
        }
        if ui.input(|i| i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::Tab)) {
            if let Some(current_idx) = workspaces.iter().position(|w| w.id == current_workspace) {
                if current_idx < workspaces.len() - 1 {
                    workspace_to_switch = Some(workspaces[current_idx + 1].id);
                }
            }
        }

        // Handle number keys for direct workspace switching
        for key in [
            Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4,
            Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9,
        ] {
            if ui.input(|i| i.key_pressed(key)) {
                let num = match key {
                    Key::Num0 => 10,
                    Key::Num1 => 1,
                    Key::Num2 => 2,
                    Key::Num3 => 3,
                    Key::Num4 => 4,
                    Key::Num5 => 5,
                    Key::Num6 => 6,
                    Key::Num7 => 7,
                    Key::Num8 => 8,
                    Key::Num9 => 9,
                    _ => continue,
                };
                
                // Find workspace with this number
                if let Some(workspace) = workspaces.iter().find(|w| w.id == num) {
                    workspace_to_switch = Some(workspace.id);
                    should_close = true;
                }
            }
        }

        // Handle closing conditions
        if ui.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Enter)) {
            should_close = true;
        }

        ui.horizontal(|ui| {
            for workspace in workspaces {
                let is_current = workspace.id == current_workspace;
                
                let height = 80.0;
                let width = (height * 16.0) / 9.0;
                let rounding = Rounding::same(15);
                
                let button = Button::new("")
                    .min_size(Vec2::new(width, height))
                    .fill(if is_current { colors.surface_container_high } else { Color32::from_black_alpha(128) })
                    .rounding(rounding)
                    .stroke((
                        if is_current { 2.0 } else { 0.0 },
                        colors.primary_fixed_dim
                    ))
                    .frame(false);
                
                let response = ui.add(button);

                // Draw background image if available
                if let Some(bg) = &self.background {
                    // Create a slightly smaller rect for the background
                    let inner_rect = response.rect.shrink(2.0);
                    
                    // First draw the background image
                    Image::new(bg)
                        .rounding(Rounding::same(15))
                        .fit_to_exact_size(inner_rect.size())
                        .paint_at(ui, inner_rect);

                    // Add multiple layers for a better blur/dim effect
                    ui.painter().rect_filled(
                        inner_rect,
                        Rounding::same(15),
                        Color32::from_black_alpha(120), // First layer of dimming
                    );
                    
                    // Add a subtle colored overlay
                    ui.painter().rect_filled(
                        inner_rect,
                        Rounding::same(15),
                        colors.surface.gamma_multiply(0.3), // Second layer with surface color
                    );
                    
                    // Add extra overlay for current workspace
                    if is_current {
                        ui.painter().rect_filled(
                            inner_rect,
                            Rounding::same(15),
                            Color32::from_black_alpha(80),
                        );
                    }
                }

                // Draw workspace number (bottom left)
                let workspace_pos = response.rect.left_bottom() + Vec2::new(8.0, -8.0);
                ui.painter().text(
                    workspace_pos,
                    Align2::LEFT_BOTTOM,
                    &workspace.name,
                    FontId::new(14.0, FontFamily::Proportional),
                    if is_current {
                        colors.primary_fixed_dim
                    } else {
                        colors.on_surface_variant
                    },
                );

                // Draw app icons (top left)
                let workspace_windows: Vec<String> = windows.iter()
                    .filter(|w| w.workspace.id == workspace.id && w.class != "hypowertools")
                    .map(|w| w.class.clone())
                    .collect::<Vec<String>>();

                let unique_windows: Vec<&String> = workspace_windows.iter()
                    .enumerate()
                    .filter(|(i, app)| workspace_windows[..*i].iter().find(|&x| x == *app).is_none())
                    .map(|(_, app)| app)
                    .collect();

                if !workspace_windows.is_empty() {
                    let icon_size = 26.0; // Reduced from 32.0 to 26.0
                    let icon_spacing = 4.0; // Reduced spacing
                    let icon_margin = 8.0;
                    let icon_area_width = (icon_size + icon_spacing) * 3.0 - icon_spacing;
                    
                    // Create a container for icons at the top of the workspace button
                    let icon_area = Rect::from_min_size(
                        Pos2::new(
                            response.rect.left() + icon_margin,
                            response.rect.top() + icon_margin
                        ),
                        Vec2::new(icon_area_width, icon_size)
                    );

                    for (idx, app_class) in unique_windows.iter().take(3).enumerate() {
                        // Special handling for Cursor
                        let lookup_name = if **app_class == "Cursor" {
                            "cursor"  // Try lowercase
                        } else {
                            app_class
                        };
                        
                        if let Some(icon) = self.get_app_icon(ui, lookup_name) {
                            let icon_rect = Rect::from_min_size(
                                Pos2::new(
                                    icon_area.left() + (icon_size + icon_spacing) * idx as f32,
                                    icon_area.top()
                                ),
                                Vec2::new(icon_size, icon_size)
                            );
                            
                            Image::new(&icon)
                                .fit_to_exact_size(Vec2::new(icon_size, icon_size))
                                .paint_at(ui, icon_rect);
                        }
                    }

                    if unique_windows.len() > 3 {
                        let text_pos = Pos2::new(
                            icon_area.right() + 6.0,
                            icon_area.center().y
                        );
                        ui.painter().text(
                            text_pos,
                            Align2::LEFT_CENTER,
                            &format!("+{}", unique_windows.len() - 3),
                            FontId::new(11.0, FontFamily::Proportional),
                            if is_current { colors.primary_fixed_dim } else { colors.on_surface_variant },
                        );
                    }
                }
                
                if response.clicked() {
                    workspace_to_switch = Some(workspace.id);
                }
            }
        });

        // Handle actions after UI
        if let Some(workspace_id) = workspace_to_switch {
            self.switch_to_workspace(workspace_id);
            self.update();
        }
        if should_close {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
    }

    pub fn cleanup(&mut self) {
        // Drop all cached textures to ensure proper cleanup
        self.icon_cache.cache.borrow_mut().clear();
        // Drop background texture if it exists
        self.background = None;
    }

} 