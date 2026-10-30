use super::Key;

#[test]
fn every_printable_ascii_character_has_a_key_and_nothing_else_does() {
    for byte in 0..=u8::MAX {
        assert_eq!(
            Key::for_ascii(byte).is_some(),
            (0x20..=0x7E).contains(&byte),
            "byte 0x{byte:02X}"
        );
    }
}

#[test]
fn the_us_layout_keys_and_shift() {
    assert_eq!(Key::for_ascii(b'a'), Some((Key::A, false)));
    assert_eq!(Key::for_ascii(b'Z'), Some((Key::Z, true)));
    assert_eq!(Key::for_ascii(b'0'), Some((Key::NUM0, false)));
    assert_eq!(Key::for_ascii(b'!'), Some((Key::NUM1, true)));
    assert_eq!(Key::for_ascii(b'"'), Some((Key::APOSTROPHE, true)));
    assert_eq!(Key::for_ascii(b'\\'), Some((Key::BACKSLASH, false)));
    assert_eq!(Key::for_ascii(b'~'), Some((Key::GRAVE, true)));
    assert_eq!(Key::for_ascii(b' '), Some((Key::SPACE, false)));
}

#[test]
fn no_two_characters_share_a_key_and_shift() {
    let mut seen = std::collections::HashSet::new();
    for byte in 0x20..=0x7E {
        assert!(seen.insert(Key::for_ascii(byte)), "byte 0x{byte:02X}");
    }
}
