use std::collections::HashSet;

pub struct Key {
    pub id: &'static str,
    pub label: &'static str,
    pub width: f32,
}

const fn wide(id: &'static str, label: &'static str, width: f32) -> Key {
    Key { id, label, width }
}

const fn key(id: &'static str, label: &'static str) -> Key {
    wide(id, label, 1.0)
}

const fn gap(width: f32) -> Key {
    wide("", "", width)
}

pub const ROWS: &[&[Key]] = &[
    &[
        key("escape", "Esc"),
        gap(0.75),
        key("f1", "F1"),
        key("f2", "F2"),
        key("f3", "F3"),
        key("f4", "F4"),
        gap(0.625),
        key("f5", "F5"),
        key("f6", "F6"),
        key("f7", "F7"),
        key("f8", "F8"),
        gap(0.625),
        key("f9", "F9"),
        key("f10", "F10"),
        key("f11", "F11"),
        key("f12", "F12"),
        gap(0.35),
        key("print", "PrtSc"),
        key("scroll_lock", "ScrLk"),
        key("pause", "Pause"),
    ],
    &[
        key("`", "`"),
        key("1", "1"),
        key("2", "2"),
        key("3", "3"),
        key("4", "4"),
        key("5", "5"),
        key("6", "6"),
        key("7", "7"),
        key("8", "8"),
        key("9", "9"),
        key("0", "0"),
        key("-", "-"),
        key("=", "="),
        wide("backspace", "Backspace", 2.0),
        gap(0.35),
        key("insert", "Ins"),
        key("home", "Home"),
        key("pageup", "PgUp"),
    ],
    &[
        wide("tab", "Tab", 1.5),
        key("q", "Q"),
        key("w", "W"),
        key("e", "E"),
        key("r", "R"),
        key("t", "T"),
        key("y", "Y"),
        key("u", "U"),
        key("i", "I"),
        key("o", "O"),
        key("p", "P"),
        key("[", "["),
        key("]", "]"),
        wide("\\", "\\", 1.5),
        gap(0.35),
        key("delete", "Del"),
        key("end", "End"),
        key("pagedown", "PgDn"),
    ],
    &[
        wide("caps_lock", "Caps Lock", 1.75),
        key("a", "A"),
        key("s", "S"),
        key("d", "D"),
        key("f", "F"),
        key("g", "G"),
        key("h", "H"),
        key("j", "J"),
        key("k", "K"),
        key("l", "L"),
        key(";", ";"),
        key("'", "'"),
        wide("enter", "Enter", 2.25),
    ],
    &[
        wide("lshift", "L Shift", 2.25),
        key("z", "Z"),
        key("x", "X"),
        key("c", "C"),
        key("v", "V"),
        key("b", "B"),
        key("n", "N"),
        key("m", "M"),
        key(",", ","),
        key(".", "."),
        key("/", "/"),
        wide("rshift", "R Shift", 2.75),
        gap(1.35),
        key("up", "↑"),
    ],
    &[
        wide("lctrl", "L Ctrl", 1.25),
        wide("lsuper", "L Super", 1.25),
        wide("lalt", "L Alt", 1.25),
        wide("space", "Space", 6.25),
        wide("ralt", "R Alt", 1.25),
        wide("rsuper", "R Super", 1.25),
        wide("menu", "Menu", 1.25),
        wide("rctrl", "R Ctrl", 1.25),
        gap(0.35),
        key("left", "←"),
        key("down", "↓"),
        key("right", "→"),
    ],
];

pub fn key_id(key: &str) -> &str {
    match key {
        "~" => "`",
        "!" => "1",
        "@" => "2",
        "#" => "3",
        "$" => "4",
        "%" => "5",
        "^" => "6",
        "&" => "7",
        "*" => "8",
        "(" => "9",
        ")" => "0",
        "_" => "-",
        "+" => "=",
        "{" => "[",
        "}" => "]",
        "|" => "\\",
        ":" => ";",
        "\"" => "'",
        "<" => ",",
        ">" => ".",
        "?" => "/",
        "sys_req" => "print",
        "break" => "pause",
        _ => key,
    }
}

pub fn layout_keys() -> HashSet<&'static str> {
    ROWS.iter()
        .flat_map(|row| *row)
        .map(|key| key.id)
        .filter(|id| !id.is_empty())
        .collect()
}

#[derive(Default)]
pub struct Tester {
    pub held: HashSet<String>,
    pub tested: HashSet<String>,
    pub last: Option<String>,
}

impl Tester {
    pub fn press(&mut self, key: &str, repeat: bool) {
        if !repeat {
            let id = key_id(key);
            self.held.insert(id.into());
            self.tested.insert(id.into());
        }
        self.last = Some(format!("{}  {key}", if repeat { "REPEAT" } else { "DOWN" }));
    }

    pub fn release(&mut self, key: &str) {
        self.held.remove(key_id(key));
        self.last = Some(format!("UP  {key}"));
    }

    pub fn set_caps_lock(&mut self, active: bool) {
        self.tested.insert("caps_lock".into());
        self.last = Some(format!("CAPS LOCK  {}", if active { "ON" } else { "OFF" }));
    }

    pub fn clear_held(&mut self) {
        self.held.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tkl_layout_counts_each_key_separately() {
        assert_eq!(layout_keys().len(), 87);
        let width = |row: &[Key]| row.iter().map(|key| key.width).sum::<f32>();
        assert!((width(ROWS[0]) - width(ROWS[5])).abs() < 0.01);
    }

    #[test]
    fn shifted_symbols_share_their_normal_key() {
        let mut tester = Tester::default();
        tester.press("!", false);
        tester.press("a", false);
        tester.release("1");
        assert_eq!(tester.held, HashSet::from(["a".into()]));
        assert_eq!(tester.tested, HashSet::from(["1".into(), "a".into()]));
    }

    #[test]
    fn repeat_does_not_reactivate_cleared_keys() {
        let mut tester = Tester::default();
        tester.press("f1", false);
        tester.clear_held();
        tester.press("f1", true);
        assert!(tester.held.is_empty());
        assert!(tester.tested.contains("f1"));
        assert_eq!(tester.last.as_deref(), Some("REPEAT  f1"));
    }

    #[test]
    fn releasing_one_ctrl_leaves_the_other_held() {
        let mut tester = Tester::default();
        tester.press("lctrl", false);
        tester.press("rctrl", false);
        tester.release("lctrl");
        assert_eq!(tester.held, HashSet::from(["rctrl".into()]));
        assert_eq!(
            tester.tested,
            HashSet::from(["lctrl".into(), "rctrl".into()])
        );
    }

    #[test]
    fn caps_lock_toggle_does_not_create_a_held_key() {
        let mut tester = Tester::default();
        tester.set_caps_lock(true);
        assert!(tester.held.is_empty());
        assert!(tester.tested.contains("caps_lock"));
    }
}
