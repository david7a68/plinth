use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{GetKeyState, VK_MENU},
        WindowsAndMessaging::{
            WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN,
            WM_MBUTTONUP, WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP,
        },
    },
};

use crate::{
    geometry::window::WindowPoint,
    system::input::{ButtonState, ModifierKeys, MouseButton, ScrollAxis},
};

pub(crate) fn mouse_button(msg: u32) -> Option<(MouseButton, ButtonState)> {
    match msg {
        WM_LBUTTONDOWN => Some((MouseButton::Left, ButtonState::Pressed)),
        WM_LBUTTONUP => Some((MouseButton::Left, ButtonState::Released)),
        WM_LBUTTONDBLCLK => Some((MouseButton::Left, ButtonState::DoubleTapped)),
        WM_RBUTTONDOWN => Some((MouseButton::Right, ButtonState::Pressed)),
        WM_RBUTTONUP => Some((MouseButton::Right, ButtonState::Released)),
        WM_RBUTTONDBLCLK => Some((MouseButton::Right, ButtonState::DoubleTapped)),
        WM_MBUTTONDOWN => Some((MouseButton::Middle, ButtonState::Pressed)),
        WM_MBUTTONUP => Some((MouseButton::Middle, ButtonState::Released)),
        WM_MBUTTONDBLCLK => Some((MouseButton::Middle, ButtonState::DoubleTapped)),
        _ => unreachable!(),
    }
}

pub(crate) fn wheel_axis(msg: u32) -> Option<ScrollAxis> {
    match msg {
        0x20A => Some(ScrollAxis::Vertical),
        0x20E => Some(ScrollAxis::Horizontal),
        _ => None,
    }
}

pub(crate) fn mouse_coords(lparam: LPARAM) -> WindowPoint {
    let x = (lparam.0 & 0xffff) as i32;
    let y = ((lparam.0 >> 16) & 0xffff) as i32;
    WindowPoint { x, y }
}

pub(crate) fn mouse_modifiers(wparam: WPARAM) -> ModifierKeys {
    const MK_CONTROL: usize = 0x0008;
    const MK_SHIFT: usize = 0x0004;

    let mut modifiers = ModifierKeys::empty();

    if wparam.0 & MK_CONTROL != 0 {
        modifiers |= ModifierKeys::CTRL;
    }

    if wparam.0 & MK_SHIFT != 0 {
        modifiers |= ModifierKeys::SHIFT;
    }

    if unsafe { GetKeyState(i32::from(VK_MENU.0)) } < 0 {
        modifiers |= ModifierKeys::ALT;
    }

    modifiers
}
