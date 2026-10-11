use std::fmt;

/// a keyboard key by its HID usage (page 0x07). the consts cover the usual keys, and `Key(n)` gets you any
/// other one from 0x04 to 0xE7
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key(pub u8);

impl Key {
    pub const A: Key = Key(0x04);
    pub const B: Key = Key(0x05);
    pub const C: Key = Key(0x06);
    pub const D: Key = Key(0x07);
    pub const E: Key = Key(0x08);
    pub const F: Key = Key(0x09);
    pub const G: Key = Key(0x0A);
    pub const H: Key = Key(0x0B);
    pub const I: Key = Key(0x0C);
    pub const J: Key = Key(0x0D);
    pub const K: Key = Key(0x0E);
    pub const L: Key = Key(0x0F);
    pub const M: Key = Key(0x10);
    pub const N: Key = Key(0x11);
    pub const O: Key = Key(0x12);
    pub const P: Key = Key(0x13);
    pub const Q: Key = Key(0x14);
    pub const R: Key = Key(0x15);
    pub const S: Key = Key(0x16);
    pub const T: Key = Key(0x17);
    pub const U: Key = Key(0x18);
    pub const V: Key = Key(0x19);
    pub const W: Key = Key(0x1A);
    pub const X: Key = Key(0x1B);
    pub const Y: Key = Key(0x1C);
    pub const Z: Key = Key(0x1D);
    pub const NUM1: Key = Key(0x1E);
    pub const NUM2: Key = Key(0x1F);
    pub const NUM3: Key = Key(0x20);
    pub const NUM4: Key = Key(0x21);
    pub const NUM5: Key = Key(0x22);
    pub const NUM6: Key = Key(0x23);
    pub const NUM7: Key = Key(0x24);
    pub const NUM8: Key = Key(0x25);
    pub const NUM9: Key = Key(0x26);
    pub const NUM0: Key = Key(0x27);
    pub const ENTER: Key = Key(0x28);
    pub const ESCAPE: Key = Key(0x29);
    pub const BACKSPACE: Key = Key(0x2A);
    pub const TAB: Key = Key(0x2B);
    pub const SPACE: Key = Key(0x2C);
    pub const MINUS: Key = Key(0x2D);
    pub const EQUAL: Key = Key(0x2E);
    pub const LEFT_BRACKET: Key = Key(0x2F);
    pub const RIGHT_BRACKET: Key = Key(0x30);
    pub const BACKSLASH: Key = Key(0x31);
    pub const SEMICOLON: Key = Key(0x33);
    pub const APOSTROPHE: Key = Key(0x34);
    pub const GRAVE: Key = Key(0x35);
    pub const COMMA: Key = Key(0x36);
    pub const PERIOD: Key = Key(0x37);
    pub const SLASH: Key = Key(0x38);
    pub const CAPS_LOCK: Key = Key(0x39);
    pub const F1: Key = Key(0x3A);
    pub const F2: Key = Key(0x3B);
    pub const F3: Key = Key(0x3C);
    pub const F4: Key = Key(0x3D);
    pub const F5: Key = Key(0x3E);
    pub const F6: Key = Key(0x3F);
    pub const F7: Key = Key(0x40);
    pub const F8: Key = Key(0x41);
    pub const F9: Key = Key(0x42);
    pub const F10: Key = Key(0x43);
    pub const F11: Key = Key(0x44);
    pub const F12: Key = Key(0x45);
    pub const PRINT_SCREEN: Key = Key(0x46);
    pub const SCROLL_LOCK: Key = Key(0x47);
    pub const PAUSE: Key = Key(0x48);
    pub const INSERT: Key = Key(0x49);
    pub const HOME: Key = Key(0x4A);
    pub const PAGE_UP: Key = Key(0x4B);
    pub const DELETE: Key = Key(0x4C);
    pub const END: Key = Key(0x4D);
    pub const PAGE_DOWN: Key = Key(0x4E);
    pub const RIGHT: Key = Key(0x4F);
    pub const LEFT: Key = Key(0x50);
    pub const DOWN: Key = Key(0x51);
    pub const UP: Key = Key(0x52);
    pub const NUM_LOCK: Key = Key(0x53);
    pub const KEYPAD_DIVIDE: Key = Key(0x54);
    pub const KEYPAD_MULTIPLY: Key = Key(0x55);
    pub const KEYPAD_MINUS: Key = Key(0x56);
    pub const KEYPAD_PLUS: Key = Key(0x57);
    pub const KEYPAD_ENTER: Key = Key(0x58);
    pub const KEYPAD1: Key = Key(0x59);
    pub const KEYPAD2: Key = Key(0x5A);
    pub const KEYPAD3: Key = Key(0x5B);
    pub const KEYPAD4: Key = Key(0x5C);
    pub const KEYPAD5: Key = Key(0x5D);
    pub const KEYPAD6: Key = Key(0x5E);
    pub const KEYPAD7: Key = Key(0x5F);
    pub const KEYPAD8: Key = Key(0x60);
    pub const KEYPAD9: Key = Key(0x61);
    pub const KEYPAD0: Key = Key(0x62);
    pub const KEYPAD_PERIOD: Key = Key(0x63);
    pub const MENU: Key = Key(0x65);
    pub const LEFT_CTRL: Key = Key(0xE0);
    pub const LEFT_SHIFT: Key = Key(0xE1);
    pub const LEFT_ALT: Key = Key(0xE2);
    pub const LEFT_GUI: Key = Key(0xE3);
    pub const RIGHT_CTRL: Key = Key(0xE4);
    pub const RIGHT_SHIFT: Key = Key(0xE5);
    pub const RIGHT_ALT: Key = Key(0xE6);
    pub const RIGHT_GUI: Key = Key(0xE7);

    /// ctrl, shift, alt and gui on either side. a keyboard report keeps these in a byte of their own
    pub const fn is_modifier(self) -> bool {
        matches!(self.0, 0xE0..=0xE7)
    }

    /// 0x00 to 0x03 are HID error codes, and nothing from 0xE8 up is defined on the keyboard page
    pub(crate) const fn is_valid(self) -> bool {
        matches!(self.0, 0x04..=0xE7)
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "key 0x{:02X}", self.0)
    }
}

/// a media key by its HID consumer usage (page 0x0C)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MediaKey(pub u16);

impl MediaKey {
    pub const PLAY: MediaKey = MediaKey(0xB0);
    pub const PAUSE: MediaKey = MediaKey(0xB1);
    pub const NEXT_TRACK: MediaKey = MediaKey(0xB5);
    pub const PREVIOUS_TRACK: MediaKey = MediaKey(0xB6);
    pub const STOP: MediaKey = MediaKey(0xB7);
    pub const PLAY_PAUSE: MediaKey = MediaKey(0xCD);
    pub const MUTE: MediaKey = MediaKey(0xE2);
    pub const VOLUME_UP: MediaKey = MediaKey(0xE9);
    pub const VOLUME_DOWN: MediaKey = MediaKey(0xEA);
}

impl fmt::Display for MediaKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "media key 0x{:04X}", self.0)
    }
}
