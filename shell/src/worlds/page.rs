//! Shared column for every world page: same width, same padding, same census row.
//!
//! Tables and stat widgets inherit this. A phone narrower than MEASURE
//! still gets 100% — the constant is a ceiling, not a floor.

use bevy::prelude::*;
use prysm::theme;

pub const PAD: f32 = theme::G * 3.0;

pub fn column() -> Node {
    Node {
        width: Val::Percent(100.0),
        max_width: Val::Px(theme::MEASURE),
        flex_direction: FlexDirection::Column,
        padding: UiRect::all(Val::Px(PAD)),
        row_gap: Val::Px(theme::G),
        ..default()
    }
}

/// `column()` then scroll — never `overflow: scroll_y, ..column()`, that
/// overwrites overflow back to clip and the page does not scroll.
pub fn scroll_column() -> Node {
    let mut n = column();
    n.flex_grow = 1.0;
    n.height = Val::Percent(100.0);
    n.overflow = Overflow::scroll_y();
    n
}

pub fn census_row() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(theme::G),
        ..default()
    }
}

/// Overlay band (brain HUD): same MEASURE column, centered in the viewport.
pub fn overlay_band(top: bool) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: if top {
            Val::Px(crate::shell::chrome::CHROME_TOP_H + 8.0)
        } else {
            Val::Auto
        },
        bottom: if top {
            Val::Auto
        } else {
            Val::Px(crate::shell::chrome::CHROME_BOTTOM_H + 8.0)
        },
        height: Val::Px(56.0),
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::Center,
        padding: UiRect::horizontal(Val::Px(PAD)),
        ..default()
    }
}

pub fn overlay_inner() -> Node {
    Node {
        width: Val::Percent(100.0),
        max_width: Val::Px(theme::MEASURE),
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(theme::G),
        ..default()
    }
}
