//! Design tokens for the standalone command manager.
//!
//! Two skins share one structure: the original Nord dark palette and a paper
//! light palette. Every color used by the custom views comes from [`Palette`];
//! `views.rs` must not contain raw color literals.
use gpui::{App, Global, px, rgba};
use gpui_component::{Theme, ThemeMode};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------- dimensions

pub const PANEL_WIDTH: f32 = 520.0;
/// The panel fills the whole window: GPUI windows are opaque, so any outer
/// padding would show up as a grey halo around the content.
pub const WINDOW_PAD: f32 = 0.0;
pub const WINDOW_WIDTH: f32 = PANEL_WIDTH + WINDOW_PAD * 2.0;
pub const SEARCH_BAR_HEIGHT: f32 = 40.0;
/// The search field floats inside a padded block that also acts as drag handle.
pub const SEARCH_BLOCK_PAD_TOP: f32 = 10.0;
pub const SEARCH_BLOCK_PAD_BOTTOM: f32 = 8.0;
pub const SEARCH_BLOCK_HEIGHT: f32 =
    SEARCH_BLOCK_PAD_TOP + SEARCH_BAR_HEIGHT + SEARCH_BLOCK_PAD_BOTTOM;
pub const TOOLBAR_HEIGHT: f32 = 40.0;
pub const FOLDER_HEIGHT: f32 = 60.0;
/// Folders and commands share one height so the list reads as a grid.
pub const ROW_HEIGHT: f32 = 60.0;
pub const ROW_GAP: f32 = 2.0;
/// Break between the folder card and the command list.
pub const BLOCK_GAP: f32 = 6.0;
/// Inner padding of the scrolling list; it belongs to the chrome, not the rows.
pub const LIST_PAD: f32 = 8.0;
pub const FOOTER_HEIGHT: f32 = 46.0;
/// Height of the empty-state block, used to size the window when nothing matches.
pub const EMPTY_STATE_HEIGHT: f32 = 168.0;
pub const RADIUS_PANEL: f32 = 12.0;
pub const RADIUS_CONTROL: f32 = 7.0;
pub const RADIUS_CHIP: f32 = 5.0;
/// The open tag filter panel has a fixed height so the window maths stay exact.
pub const TAGS_PANEL_HEIGHT: f32 = 94.0;
/// Scrollable chip area inside that panel.
pub const TAG_CHIPS_MAX_HEIGHT: f32 = 48.0;

// -------------------------------------------------------------------- theme

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    #[default]
    Dark,
    Light,
}

impl ThemeChoice {
    pub fn mode(self) -> ThemeMode {
        match self {
            Self::Dark => ThemeMode::Dark,
            Self::Light => ThemeMode::Light,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "深色",
            Self::Light => "浅色",
        }
    }
}

/// Semantic colors. Every value is `0xRRGGBBAA` so a single [`rgba`] call
/// renders it, whether or not it carries alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// Bezel behind the panel; also the fallback for the drop shadow.
    pub window: u32,
    /// Panel surface.
    pub panel: u32,
    /// Inputs and code blocks.
    pub surface: u32,
    /// Folder icons and cards that sit on the panel.
    pub raised: u32,
    pub border: u32,
    pub border_strong: u32,
    pub fg: u32,
    pub fg_muted: u32,
    pub fg_subtle: u32,
    pub accent: u32,
    pub accent_soft: u32,
    pub accent_fg: u32,
    pub success: u32,
    pub danger: u32,
    pub warning: u32,
    pub hover: u32,
    pub selected: u32,
    pub copied: u32,
    pub tag_bg: u32,
    pub tag_fg: u32,
    pub shadow: u32,
}

impl Palette {
    pub const DARK: Self = Self {
        window: 0x23272FFF,
        panel: 0x262C36FF,
        surface: 0x323947FF,
        raised: 0x39414FFF,
        border: 0xFFFFFF12,
        border_strong: 0x454E60FF,
        fg: 0xE7EBF3FF,
        fg_muted: 0xAEB7C6FF,
        fg_subtle: 0x7C8798FF,
        accent: 0x88C0D0FF,
        accent_soft: 0x88C0D024,
        accent_fg: 0x21262EFF,
        success: 0xA3BE8CFF,
        danger: 0xBF616AFF,
        warning: 0xEBCB8BFF,
        hover: 0xFFFFFF09,
        selected: 0x88C0D01F,
        copied: 0xA3BE8C1F,
        tag_bg: 0x88C0D016,
        tag_fg: 0x9FD2DEFF,
        shadow: 0x00000040,
    };

    pub const LIGHT: Self = Self {
        window: 0xE6E8ECFF,
        panel: 0xFFFFFFFF,
        surface: 0xF1F3F6FF,
        raised: 0xE8EBF0FF,
        border: 0x0000000F,
        border_strong: 0xD6DAE0FF,
        fg: 0x1B2027FF,
        fg_muted: 0x59616FFF,
        fg_subtle: 0x8A929FFF,
        accent: 0x2E7A8CFF,
        accent_soft: 0x2E7A8C14,
        accent_fg: 0xFFFFFFFF,
        success: 0x3C7A4BFF,
        danger: 0xB23B45FF,
        warning: 0x9A6B12FF,
        hover: 0x00000006,
        selected: 0x2E7A8C12,
        copied: 0x3C7A4B12,
        tag_bg: 0x2E7A8C12,
        tag_fg: 0x26697AFF,
        shadow: 0x00000014,
    };

    pub fn of(choice: ThemeChoice) -> Self {
        match choice {
            ThemeChoice::Dark => Self::DARK,
            ThemeChoice::Light => Self::LIGHT,
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::DARK
    }
}

impl Global for Palette {}

/// The active palette. [`init`] always installs one before any view renders.
pub fn palette(cx: &App) -> &Palette {
    cx.global::<Palette>()
}

// ------------------------------------------------------------------- startup

pub fn init(cx: &mut App, choice: ThemeChoice) {
    gpui_component::init(cx);
    if let Err(error) = cx.text_system().add_fonts(vec![std::borrow::Cow::Borrowed(
        include_bytes!("../assets/GeistVF.ttf").as_slice(),
    )]) {
        eprintln!("Could not register the Geist font: {error}");
    }
    apply(choice, cx);
}

/// Installs the palette globally and mirrors it into the component theme so
/// built-in widgets (buttons, inputs, menus, scrollbars) follow the skin.
pub fn apply(choice: ThemeChoice, cx: &mut App) {
    let palette = Palette::of(choice);
    cx.set_global(palette);
    Theme::change(choice.mode(), None, cx);

    let theme = Theme::global_mut(cx);
    theme.background = rgba(palette.panel).into();
    theme.foreground = rgba(palette.fg).into();
    theme.muted = rgba(palette.surface).into();
    theme.muted_foreground = rgba(palette.fg_muted).into();
    theme.popover = rgba(palette.panel).into();
    theme.popover_foreground = rgba(palette.fg).into();
    theme.border = rgba(palette.border).into();
    theme.input = rgba(palette.border).into();
    theme.list_hover = rgba(palette.hover).into();
    theme.list_active = rgba(palette.selected).into();
    theme.list_active_border = rgba(palette.accent).into();
    theme.accent = rgba(palette.accent_soft).into();
    theme.accent_foreground = rgba(palette.accent).into();
    theme.primary = rgba(palette.accent).into();
    theme.primary_foreground = rgba(palette.accent_fg).into();
    theme.primary_hover = rgba(palette.accent).into();
    theme.primary_active = rgba(palette.accent).into();
    theme.ring = rgba(palette.accent_soft).into();
    theme.caret = rgba(palette.accent).into();
    theme.selection = rgba(palette.accent_soft).into();
    theme.secondary = rgba(palette.surface).into();
    theme.secondary_foreground = rgba(palette.fg).into();
    theme.secondary_hover = rgba(palette.raised).into();
    theme.secondary_active = rgba(palette.selected).into();
    theme.button = rgba(palette.surface).into();
    theme.button_foreground = rgba(palette.fg).into();
    theme.button_hover = rgba(palette.raised).into();
    theme.button_active = rgba(palette.selected).into();
    theme.danger = rgba(palette.danger).into();
    theme.danger_foreground = rgba(palette.accent_fg).into();
    theme.success = rgba(palette.success).into();
    theme.warning = rgba(palette.warning).into();
    theme.info = rgba(palette.accent).into();
    theme.link = rgba(palette.accent).into();
    theme.overlay = rgba(palette.shadow).into();
    theme.group_box = rgba(palette.surface).into();
    theme.scrollbar = rgba(0x00000000).into();
    theme.scrollbar_thumb = rgba(match choice {
        ThemeChoice::Dark => 0x7C879873,
        ThemeChoice::Light => 0x8A929F66,
    })
    .into();
    theme.scrollbar_thumb_hover = rgba(palette.fg_subtle).into();
    theme.title_bar = rgba(palette.panel).into();
    theme.status_bar = rgba(palette.panel).into();
    theme.status_bar_border = rgba(palette.border).into();
    theme.window_border = rgba(palette.border_strong).into();
    theme.font_family = "Geist".into();
    theme.mono_font_family = "Consolas".into();
    theme.font_size = px(14.0);
    theme.mono_font_size = px(13.0);
    theme.radius = px(RADIUS_CONTROL);
    theme.radius_lg = px(RADIUS_PANEL);
    theme.shadow = true;
    theme.tokens = (&theme.colors).into();
}

/// Small helper so views read like prose: `tint(p.hover)`.
pub fn tint(value: u32) -> gpui::Hsla {
    rgba(value).into()
}
