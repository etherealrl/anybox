// every plain call of the bindings, one entry each. a call here gets its C function, its Python and C# raw
// bindings and methods and its conformance dispatch from this, with this one doc. `{name}` in a doc is one of the
// call's arguments or another call, each language writes it its own way, and `{click}` is what the `Click` argument
// does in that language. the argument names are the C header's, the C# tests hold the bindings to them

/// what a call is on, each one a handle in C and a class in the bindings. a box's extension (`device.ferrum()` and
/// the like) or a session gets its own, with its regions in its own hand-written class. the matches on it have no
/// wildcard, so a new one doesn't build until it has its handle here and its places in main.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum On {
    Device,
}

pub struct Handle {
    /// the C functions' prefix, the handle is their first argument
    pub c_prefix: &'static str,
    /// the handle argument's name in C and C#, and what the recorder calls the Rust value it has
    pub name: &'static str,
    pub c_type: &'static str,
    /// the C ABI's function that turns the pointer into the Rust value
    pub c_read: &'static str,
    pub csharp_type: &'static str,
}

impl On {
    pub fn handle(self) -> Handle {
        match self {
            On::Device => Handle {
                c_prefix: "anybox_device_",
                name: "device",
                c_type: "AnyboxDevice",
                c_read: "device_ref",
                csharp_type: "DeviceHandle",
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ty {
    I16,
    Button,
    Key,
    MediaKey,
    Axis,
    Direction,
    /// the hold and whether only the box's own timing will do, keywords in Python and optional parameters in C#
    Click,
}

pub struct Arg {
    pub name: &'static str,
    pub ty: Ty,
    /// the value Python and C# give it when it's left out, by its anybox.h name (`BOTH` is `Direction.BOTH` and
    /// `Direction.Both`)
    pub default: Option<&'static str>,
}

pub struct Call {
    pub on: On,
    /// the Rust method's name, and the C function's after the handle's prefix
    pub name: &'static str,
    pub args: &'static [Arg],
    pub doc: &'static str,
    /// the method Python and C# fold this call into, with its `Click` argument as keywords and optional parameters
    pub folds_into: Option<&'static str>,
    /// Python and C# leave this one out, the folded call's method does what it does with no hold
    pub only_in_c: bool,
}

const fn arg(name: &'static str, ty: Ty) -> Arg {
    Arg {
        name,
        ty,
        default: None,
    }
}

const fn both_ways(name: &'static str) -> Arg {
    Arg {
        name,
        ty: Ty::Direction,
        default: Some("BOTH"),
    }
}

const fn call(name: &'static str, args: &'static [Arg], doc: &'static str) -> Call {
    Call {
        on: On::Device,
        name,
        args,
        doc,
        folds_into: None,
        only_in_c: false,
    }
}

const fn folded(
    name: &'static str,
    into: &'static str,
    args: &'static [Arg],
    doc: &'static str,
) -> Call {
    Call {
        folds_into: Some(into),
        ..call(name, args, doc)
    }
}

const fn only_in_c(name: &'static str, args: &'static [Arg], doc: &'static str) -> Call {
    Call {
        only_in_c: true,
        ..call(name, args, doc)
    }
}

pub const CALLS: &[Call] = &[
    call(
        "move_by",
        &[arg("dx", Ty::I16), arg("dy", Ty::I16)],
        "moves by counts, going through whatever smoothing the box has on",
    ),
    call(
        "move_now",
        &[arg("dx", Ty::I16), arg("dy", Ty::I16)],
        "moves by counts on the box's next report, skipping its smoothing for this move",
    ),
    call(
        "press",
        &[arg("button", Ty::Button)],
        "holds a button down until {release}",
    ),
    call(
        "release",
        &[arg("button", Ty::Button)],
        "lets go of a button",
    ),
    only_in_c(
        "click",
        &[arg("button", Ty::Button)],
        "the box's own click, or a click timed on the PC with the device's click timing if the box doesn't have one",
    ),
    folded(
        "click_with",
        "click",
        &[arg("button", Ty::Button), arg("click", Ty::Click)],
        "the box's own click, or one timed on the PC with the device's click timing if the box has none, which \
         blocks the thread for the hold. {click}",
    ),
    call(
        "press_key",
        &[arg("key", Ty::Key)],
        "holds a key down until {release_key}. a key is its HID usage, from 0x04 to 0xE7",
    ),
    call("release_key", &[arg("key", Ty::Key)], "lets go of a key"),
    only_in_c(
        "tap_key",
        &[arg("key", Ty::Key)],
        "presses and lets go, timed by the box, or on the PC with the device's click timing if the box can't",
    ),
    folded(
        "tap_key_with",
        "tap_key",
        &[arg("key", Ty::Key), arg("tap", Ty::Click)],
        "presses and lets go, timed by the box, or on the PC with the device's click timing if the box can't, which \
         blocks the thread for the hold. {click}",
    ),
    call(
        "press_media",
        &[arg("key", Ty::MediaKey)],
        "holds a media key down until {release_media}. a media key is its HID consumer usage",
    ),
    call(
        "release_media",
        &[arg("key", Ty::MediaKey)],
        "lets go of a media key",
    ),
    only_in_c(
        "tap_media",
        &[arg("key", Ty::MediaKey)],
        "presses and lets go of a media key, timed like a key's tap",
    ),
    folded(
        "tap_media_with",
        "tap_media",
        &[arg("key", Ty::MediaKey), arg("tap", Ty::Click)],
        "presses and lets go of a media key, timed like a key's tap, which blocks the thread for the hold. {click}",
    ),
    call(
        "lock_button",
        &[arg("button", Ty::Button)],
        "keeps the physical button from reaching the PC until {unlock_button}. your own presses still go through and \
         its events still come, so you can take the button over",
    ),
    call(
        "unlock_button",
        &[arg("button", Ty::Button)],
        "lets the physical button through again",
    ),
    call(
        "lock_key",
        &[arg("key", Ty::Key)],
        "keeps the physical key from reaching the PC until {unlock_key}",
    ),
    call(
        "unlock_key",
        &[arg("key", Ty::Key)],
        "lets the physical key through again",
    ),
    call(
        "lock_axis",
        &[arg("axis", Ty::Axis), both_ways("direction")],
        "keeps physical motion one way or both ways along an axis from reaching the PC until {unlock_axis}. your own \
         moves still go through",
    ),
    call(
        "unlock_axis",
        &[arg("axis", Ty::Axis), both_ways("direction")],
        "unlocks that way along the axis and leaves the other way as it was. it also undoes a scale there",
    ),
    call(
        "scale_axis",
        &[
            arg("axis", Ty::Axis),
            arg("direction", Ty::Direction),
            arg("percent", Ty::I16),
        ],
        "keeps {percent} of the physical motion one way or both ways along an axis, from -255 to 255. 100 leaves it \
         alone, 0 blocks it and -100 inverts it. {unlock_axis} puts it back to 100",
    ),
    call("wheel", &[arg("notches", Ty::I16)], "positive scrolls up"),
    call(
        "pan",
        &[arg("notches", Ty::I16)],
        "horizontal scroll, positive goes right",
    ),
    call(
        "discard_pending_moves",
        &[],
        "throws away the moves the box took but hasn't sent yet",
    ),
];
