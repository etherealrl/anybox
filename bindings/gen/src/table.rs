// every plain call of the bindings, one entry each. a call here gets its C function, its Python and C# raw
// bindings and methods and its conformance dispatch from this, with this one doc. `{name}` in a doc is one of the
// call's arguments or another call, each language writes it its own way, `{click}` is what the `Click` argument
// does in that language, `{off}` is how that language says off (0, None, null, an empty one) and `{rejected}` is
// how it hands back a Rejected error. the argument names are the C header's, the C# tests hold the bindings to them

/// what a call is on, each one a handle in C and a class in the bindings. a box's extension (`device.ferrum()` and
/// the like) or a session gets its own, with its regions in its own hand-written class. the matches on it have no
/// wildcard, so a new one doesn't build until it has its handle here and its places in main.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum On {
    Device,
    Makcu,
    Ysc,
    Ferrum,
}

/// what only one family has, reached through `device.makcu()` and the like. in C the device is still the first
/// argument and a call on another box is unsupported naming the box
#[derive(Clone, Copy)]
pub struct Extension {
    /// the accessor on the device, and what the call identities in the conformance suite start with
    pub accessor: &'static str,
    /// the `Family` variant the C ABI names in its unsupported error
    pub family: &'static str,
    /// the class in Python, C# and C++
    pub class: &'static str,
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
    /// none on the device itself
    pub extension: Option<Extension>,
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
                extension: None,
            },
            On::Makcu => extension("anybox_makcu_", "makcu", "MakcuV4", "Makcu"),
            On::Ysc => extension("anybox_ysc_", "ysc", "Ysc", "Ysc"),
            On::Ferrum => extension("anybox_ferrum_", "ferrum", "Ferrum", "Ferrum"),
        }
    }
}

const fn extension(
    c_prefix: &'static str,
    accessor: &'static str,
    family: &'static str,
    class: &'static str,
) -> Handle {
    Handle {
        c_prefix,
        name: "device",
        c_type: "AnyboxDevice",
        c_read: "device",
        csharp_type: "DeviceHandle",
        extension: Some(Extension {
            accessor,
            family,
            class,
        }),
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
    /// a string, NUL terminated in C, and a NUL inside is refused before C in the bindings
    Text,
    /// a list of keys, the pointer and `len` after it in C, a variadic call in Python and C#
    Keys,
    /// a MAKCU interpolation, a struct by pointer in C
    Interpolation,
    /// what a YSC interpolation gets set to, a struct by pointer in C
    YscSettings,
    /// a duration in whole milliseconds, `{name}_ms` in C, refused before C when it isn't whole or is negative
    Ms,
    /// the same or off, which is 0 in C, so the bindings refuse a 0 asked for instead of turning it off
    MsOrOff,
    Bool,
}

/// what a call gives back. in C each one is an out parameter after the arguments
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ret {
    Unit,
    Bool,
    Interpolation,
    YscInterpolation,
    /// an owned handle in C, read by its getters and freed
    YscSerial,
    /// two `int32_t` outs, `x` and `y`
    Xy,
    /// `ms_out` in C, 0 when it's off
    MsOrOff,
}

pub struct Arg {
    pub name: &'static str,
    pub ty: Ty,
    /// the value Python and C# give it when it's left out, by its anybox.h name (`BOTH` is `Direction.BOTH` and
    /// `Direction.Both`), and `false` for a `Bool`
    pub default: Option<&'static str>,
    /// what a refusal before C calls it, for the duration types
    pub what: &'static str,
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
    pub ret: Ret,
    /// the action a call on another box is unsupported with, as Rust writes it after `Action::`. only the
    /// extensions have one
    pub action: Option<&'static str>,
}

const fn arg(name: &'static str, ty: Ty) -> Arg {
    Arg {
        name,
        ty,
        default: None,
        what: "",
    }
}

const fn ms(name: &'static str, ty: Ty, what: &'static str) -> Arg {
    Arg {
        name,
        ty,
        default: None,
        what,
    }
}

const fn both_ways(name: &'static str) -> Arg {
    Arg {
        name,
        ty: Ty::Direction,
        default: Some("BOTH"),
        what: "",
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
        ret: Ret::Unit,
        action: None,
    }
}

const fn on(
    on: On,
    action: &'static str,
    name: &'static str,
    args: &'static [Arg],
    ret: Ret,
    doc: &'static str,
) -> Call {
    Call {
        on,
        ret,
        action: Some(action),
        ..call(name, args, doc)
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
        "type_text",
        &[arg("text", Ty::Text)],
        "types the text on the box with the box's own timing, as US layout key presses, and returns once the box has \
         typed all of it. it takes printable ASCII only, so tap keys like Enter yourself after it. other calls on the \
         device can wait while it types, and a close stops it after the few characters going out. after an error part \
         of it may already be typed",
    ),
    call(
        "press_keys",
        &[arg("keys", Ty::Keys)],
        "holds the keys down with one command of the box, up to 14 of them, until {release_keys}",
    ),
    call(
        "release_keys",
        &[arg("keys", Ty::Keys)],
        "lets go of the keys with one command of the box, or one at a time like {release_key} on a box that can't, \
         so a key you hold always gets let go. a key you don't hold still has to be one the box can let go of, so it \
         can fail as unsupported after the held ones already went",
    ),
    call(
        "tap_keys",
        &[arg("keys", Ty::Keys)],
        "presses and lets go of the keys with one command of the box, timed by the box",
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
        "force_release_key",
        &[arg("key", Ty::Key)],
        "the PC sees the key up from now on, whatever the finger does, until {restore_key}. key events keep reporting \
         the finger, so you can tell whether it's still held. a key anybox holds or has locked itself, or one a text \
         needs while it types, is refused, since each box would do something else with it",
    ),
    call(
        "restore_key",
        &[arg("key", Ty::Key)],
        "gives the PC the finger's state of the key again, so a key that's still held goes down for it. it also clears \
         a force another program left, except on a key anybox holds or has locked",
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
    on(
        On::Makcu,
        "Makcu(MakcuAction::Interpolation)",
        "interpolation",
        &[],
        Ret::Interpolation,
        "how long the box spreads a move, read fresh from the box. it's global to the box and every program on it \
         shares it",
    ),
    on(
        On::Makcu,
        "Makcu(MakcuAction::Interpolation)",
        "set_interpolation",
        &[arg("value", Ty::Interpolation)],
        Ret::Unit,
        "sets it and reads it back, a box that didn't take it {rejected}. it changes how moves land for every \
         program on the box, and anybox leaves it set when you close",
    ),
    on(
        On::Makcu,
        "Makcu(MakcuAction::Moving)",
        "moving",
        &[],
        Ret::Bool,
        "whether injected motion is still queued or being spread",
    ),
    on(
        On::Makcu,
        "Makcu(MakcuAction::Autorelease)",
        "autorelease",
        &[],
        Ret::MsOrOff,
        "how long{in_seconds} the box waits with nothing coming in before it lets go of injected mouse buttons, {off} when it's \
         off",
    ),
    on(
        On::Makcu,
        "Makcu(MakcuAction::Autorelease)",
        "set_autorelease",
        &[ms("after", Ty::MsOrOff, "autorelease")],
        Ret::Unit,
        "turns autorelease on for 1 ms to 60 s ({whole_ms}), or off with {off}. anybox asks the box \
         something about every 250 ms, so with more than that it only fires once your app dies or hangs, and a \
         button it held gets let go instead of staying down. the box \
         shares the timer with every program on it and forgets it when it loses power, so anybox sends it again \
         after a reconnect. needs V4.108",
    ),
    on(
        On::Ysc,
        "Ysc(YscAction::Interpolation)",
        "interpolation",
        &[],
        Ret::YscInterpolation,
        "how the box spreads moves. it's on by default and kept on the box, so another app may have changed it",
    ),
    on(
        On::Ysc,
        "Ysc(YscAction::Interpolation)",
        "set_interpolation",
        &[arg("settings", Ty::YscSettings)],
        Ret::Unit,
        "sets it and reads it back, a box that didn't take it {rejected}. the box keeps it, anybox leaves it set \
         when you close",
    ),
    on(
        On::Ysc,
        "Ysc(YscAction::Interpolation)",
        "reset_interpolation",
        &[],
        Ret::YscInterpolation,
        "puts it back to the box's default and gives you what that is",
    ),
    on(
        On::Ysc,
        "Ysc(YscAction::Serial)",
        "serial",
        &[],
        Ret::YscSerial,
        "the serial the box gives the mouse behind it",
    ),
    on(
        On::Ferrum,
        "Ferrum(FerrumAction::CatchXy)",
        "catch_xy",
        &[
            ms("window", Ty::Ms, "the window"),
            Arg {
                default: Some("false"),
                ..arg("with_software", Ty::Bool)
            },
        ],
        Ret::Xy,
        "the mouse's own motion summed over the last {window} ({whole_ms}, up to a second), and what software \
         sent too with {with_software}. y is positive down like {move_by} per the vendor's first example, its \
         second says the opposite and nobody checked it on a box yet. what a 0 window does isn't documented",
    ),
];
