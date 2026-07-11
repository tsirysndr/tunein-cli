//! Shared color palette so the CLI `--help` output and the interactive
//! TUI share one coherent look.

#![allow(dead_code)]

use ratatui::style::Color;

pub const PRIMARY: Color = Color::Rgb(0, 232, 198);
pub const SECONDARY: Color = Color::Rgb(0, 198, 232);
pub const ACCENT: Color = Color::Rgb(130, 100, 255);
pub const HIGHLIGHT: Color = Color::Rgb(100, 232, 130);
pub const MUTED: Color = Color::Rgb(200, 210, 220);
pub const LINK: Color = Color::Rgb(255, 160, 100);
pub const SKY: Color = Color::Rgb(0, 210, 255);
pub const ERROR: Color = Color::Rgb(255, 100, 100);
