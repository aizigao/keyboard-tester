use std::{collections::HashSet, fs::File, io, path::Path};

use evdev::{KeyCode, raw_stream::RawDevice};

pub struct Keyboard(RawDevice);

impl Keyboard {
    pub fn open(path: &Path) -> io::Result<Self> {
        let device = RawDevice::from_fd(File::open(path)?.into())?;
        if !device
            .supported_keys()
            .is_some_and(|keys| keys.contains(KeyCode::KEY_A))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Not a keyboard device",
            ));
        }
        Ok(Self(device))
    }

    pub fn held(&self) -> io::Result<HashSet<String>> {
        Ok(self
            .0
            .get_key_state()?
            .iter()
            .filter_map(key_id)
            .map(str::to_owned)
            .collect())
    }
}

fn key_id(code: KeyCode) -> Option<&'static str> {
    use KeyCode as K;
    Some(match code {
        K::KEY_ESC => "escape",
        K::KEY_F1 => "f1",
        K::KEY_F2 => "f2",
        K::KEY_F3 => "f3",
        K::KEY_F4 => "f4",
        K::KEY_F5 => "f5",
        K::KEY_F6 => "f6",
        K::KEY_F7 => "f7",
        K::KEY_F8 => "f8",
        K::KEY_F9 => "f9",
        K::KEY_F10 => "f10",
        K::KEY_F11 => "f11",
        K::KEY_F12 => "f12",
        K::KEY_SYSRQ => "print",
        K::KEY_SCROLLLOCK => "scroll_lock",
        K::KEY_PAUSE => "pause",
        K::KEY_GRAVE => "`",
        K::KEY_1 => "1",
        K::KEY_2 => "2",
        K::KEY_3 => "3",
        K::KEY_4 => "4",
        K::KEY_5 => "5",
        K::KEY_6 => "6",
        K::KEY_7 => "7",
        K::KEY_8 => "8",
        K::KEY_9 => "9",
        K::KEY_0 => "0",
        K::KEY_MINUS => "-",
        K::KEY_EQUAL => "=",
        K::KEY_BACKSPACE => "backspace",
        K::KEY_INSERT => "insert",
        K::KEY_HOME => "home",
        K::KEY_PAGEUP => "pageup",
        K::KEY_TAB => "tab",
        K::KEY_Q => "q",
        K::KEY_W => "w",
        K::KEY_E => "e",
        K::KEY_R => "r",
        K::KEY_T => "t",
        K::KEY_Y => "y",
        K::KEY_U => "u",
        K::KEY_I => "i",
        K::KEY_O => "o",
        K::KEY_P => "p",
        K::KEY_LEFTBRACE => "[",
        K::KEY_RIGHTBRACE => "]",
        K::KEY_BACKSLASH => "\\",
        K::KEY_DELETE => "delete",
        K::KEY_END => "end",
        K::KEY_PAGEDOWN => "pagedown",
        K::KEY_CAPSLOCK => "caps_lock",
        K::KEY_A => "a",
        K::KEY_S => "s",
        K::KEY_D => "d",
        K::KEY_F => "f",
        K::KEY_G => "g",
        K::KEY_H => "h",
        K::KEY_J => "j",
        K::KEY_K => "k",
        K::KEY_L => "l",
        K::KEY_SEMICOLON => ";",
        K::KEY_APOSTROPHE => "'",
        K::KEY_ENTER => "enter",
        K::KEY_LEFTSHIFT => "lshift",
        K::KEY_Z => "z",
        K::KEY_X => "x",
        K::KEY_C => "c",
        K::KEY_V => "v",
        K::KEY_B => "b",
        K::KEY_N => "n",
        K::KEY_M => "m",
        K::KEY_COMMA => ",",
        K::KEY_DOT => ".",
        K::KEY_SLASH => "/",
        K::KEY_RIGHTSHIFT => "rshift",
        K::KEY_UP => "up",
        K::KEY_LEFTCTRL => "lctrl",
        K::KEY_LEFTMETA => "lsuper",
        K::KEY_LEFTALT => "lalt",
        K::KEY_SPACE => "space",
        K::KEY_RIGHTALT => "ralt",
        K::KEY_RIGHTMETA => "rsuper",
        K::KEY_COMPOSE | K::KEY_MENU => "menu",
        K::KEY_RIGHTCTRL => "rctrl",
        K::KEY_LEFT => "left",
        K::KEY_DOWN => "down",
        K::KEY_RIGHT => "right",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_mapping_covers_exactly_the_displayed_keys() {
        let mapped: HashSet<_> = (0..=u16::MAX)
            .filter_map(|code| key_id(KeyCode(code)))
            .collect();
        assert_eq!(crate::keyboard::layout_keys(), mapped);
        assert_eq!(key_id(KeyCode::KEY_LEFTCTRL), Some("lctrl"));
        assert_eq!(key_id(KeyCode::KEY_RIGHTCTRL), Some("rctrl"));
        assert_eq!(key_id(KeyCode::BTN_LEFT), None);
    }
}
