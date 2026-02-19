# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] - 2026-02-19

### Added
- Prevent monitor positions from going negative when moving or during collision resolution

### Changed
- Renamed `rects` to `monitor_rects` for clarity in layout rendering
- Renamed collision variables `sel_new`/`other_new` to `new_pos_selected`/`new_pos_other`
- Improved error handling by replacing `unwrap()` with `unwrap_or(0)` in bounding box calculations

### Fixed
- Fixed misleading error message when terminal panel is too small (was "No monitors", now "Panel too small")

## [0.1.6] - 2026-02-19

### Added
- Badges (version, downloads, license) to README
- crates.io metadata for `xwlm-cfg` crate (description, license, repository)

## [0.1.5] - 2026-02-19

### Added
- Auto-detect existing monitor config on first-time setup instead of requiring manual path entry
- Recursive config extraction: walks compositor config trees following source/include directives to find monitor and workspace definitions
- Workspace line extraction alongside monitor lines for Hyprland and Sway
- Manual fallback (`m` key) in setup wizard when auto-detection is not available

### Changed
- Renamed `xwlm-compositor` crate to `xwlm-cfg`
- Replaced `unwrap()` with proper error handling in config extractors

## [0.1.4] - 2026-02-18

### Added
- Workspace panel for assigning workspaces to monitors
- Workspace config parsing, formatting, and compositor reload on save
- Contextual keybinding hints in panel titles when focused
- Compositor support table and `wlx_monitors` library reference in README

### Changed
- Scale step size reduced from 0.25 to 0.1 for finer granularity
- Removed duplicate keybinding bar from monitor layout panel

### Fixed
- Workspace changes now save immediately instead of on next event loop
- River config now generates valid `wlr-randr` commands instead of nonexistent `riverctl` commands
- River config now includes monitor positions and disabled monitors

## [0.1.3] - 2026-02-18

### Added
- Warning modal before disabling the last enabled monitor
- Curl install script and GitHub Pages deployment

## [0.1.2] - 2026-02-18

### Changed
- Updated toml dependency to v1.0.2 (TOML 1.1.0 spec compliant)
- Fixed config directory path from `wlx_monitor_tui` to `xwlm`

## [0.1.1] - 2025-02-17

### Changed
- Renamed package from `wlx_monitor_tui` to `xwlm`
- Updated repository URL to https://github.com/x34-dzt/xwlm

## [0.1.0] - 2025-02-16

### Added
- Initial release
- TUI for managing Wayland monitor configurations
- Support for Hyprland, Sway, and River compositors
- Monitor arrange, resize, rotate, and toggle functionality
- Auto-detection of compositor
- Native config file generation
