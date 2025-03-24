# hypowertools

A collection of powerful utilities for Hyprland, designed to enhance your workflow and desktop experience.

## Features

- **Workspace Switcher**: A modern, visually appealing workspace switcher that displays:
  - Current workspace indicator
  - Application icons for each workspace
  - Background image support
  - Smooth animations
  - Keyboard navigation support
  - Intelligent icon handling for both native and Flatpak applications

## Requirements

- Hyprland
- Rust (for building)
- Icon theme (hicolor-icon-theme, or others like Papirus)
- For Flatpak support: flatpak installed and configured

## Installation

1. Clone the repository:

```bash
git clone https://github.com/yourusername/hypowertools.git
cd hypowertools
```

2. Build the project:

```bash
cargo build --release
```

3. Install the binary:

```bash
sudo cp target/release/hypowertools /usr/local/bin/
```

## Configuration

### Workspace Switcher

Create or edit your Hyprland configuration file (typically `~/.config/hypr/hyprland.conf`):

```bash
# Bind the workspace switcher to a key
bind = SUPER, Tab, exec, hypowertools --workspaces

# Optional: Configure colors
exec-once = echo 'image="/path/to/your/background.png"' > ~/.config/hypr/hyprland/colors.conf
```

### Color Configuration

The workspace switcher reads colors from `~/.config/hypr/hyprland/colors.conf`. Example configuration:

```bash
surface_container_low=rgba(1b1b21ff)
surface_container_high=rgba(292a2fff)
on_surface_variant=rgba(c6c5d0ff)
on_primary_fixed=rgba(08164bff)
primary_fixed_dim=rgba(b9c3ffff)
surface=rgba(121318ff)
surface_container=rgba(1f1f25ff)
outline=rgba(90909aff)
```

## Usage

### Workspace Switcher

- Press `Super + Tab` to open the workspace switcher
- Use arrow keys or mouse to navigate between workspaces
- Click or press Enter to switch to the selected workspace
- First 3 application icons are shown for each workspace
- "+N" indicator shows when more than 3 applications are present

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
