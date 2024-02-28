/// Maximum number of windows that can be open at once.
pub const MAX_WINDOWS: usize = 8;

pub const MAX_WINDOW_TITLE_LENGTH: usize = 255;

pub const MIN_WINDOW_DIMENSION: i16 = 100;

pub const MAX_WINDOW_DIMENSION: u16 = u16::MAX;

pub const MAX_ITEMS_PER_DRAW_LIST: usize = u32::MAX as _;

/// Enforces the maximum number of items of each kind in a draw list and returns
/// the value as a `u32`.
///
/// ## Panics
///
/// Panics if `value` is greater than [`MAX_ITEMS_PER_DRAW_LIST`].
#[must_use]
pub fn enforce_draw_list_max_commands_u32(value: usize) -> u32 {
    const _: () = assert!(u32::MAX as usize <= MAX_ITEMS_PER_DRAW_LIST);

    assert!(value <= MAX_ITEMS_PER_DRAW_LIST);
    u32::try_from(value).unwrap()
}
