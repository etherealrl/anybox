// the conformance suite's C++ runner: every scenario in bindings/conformance/scenarios.json through anybox.hpp and
// the mock, held to what the Rust crate itself did with it, which the capi crate's tests write to
// target/conformance/expected.json. bindings/c/src/conformance.rs says what a scenario is. it runs from the repo's
// root, test-c.sh starts it there

#include "anybox.hpp"
#include "json.hpp"

#include <algorithm>
#include <chrono>
#include <cstdio>
#include <fstream>
#include <map>
#include <sstream>
#include <string>
#include <thread>
#include <vector>

namespace {

using json::Array;
using json::Object;
using json::Value;
using namespace std::chrono_literals;

// what comes in after the events a step waits for, so an extra one shows up instead of being left in the queue
constexpr auto quiet = 150ms;

// what medius' own keepalive sends every 500 ms while something's held, a stats query and the subscriptions again,
// which no call sends, so the recorder and every runner leave them out
bool keepalive(const anybox::Frame& frame) {
    return (frame.type == 0x05 && frame.payload == std::vector<std::uint8_t>{0x05}) || frame.type == 0x0B;
}

struct Failure : std::runtime_error {
    using std::runtime_error::runtime_error;
};

void expect(bool ok, const std::string& what) {
    if (!ok) {
        throw Failure(what);
    }
}

std::string read(const std::string& path) {
    std::ifstream file(path, std::ios::binary);
    expect(file.good(), "there's no " + path + ", run the capi crate's tests first, they record it");
    std::stringstream text;
    text << file.rdbuf();
    return text.str();
}

// every `#define ANYBOX_NAME number` of anybox.h, read from its text like the other runners do, so a value compares
// by its name there and not by this header's own numbers
std::map<std::string, std::uint64_t> read_header() {
    std::map<std::string, std::uint64_t> out;
    std::istringstream lines(read("bindings/c/include/anybox.h"));
    std::string line;
    const std::string prefix = "#define ANYBOX_";
    while (std::getline(lines, line)) {
        if (!line.empty() && line.back() == '\r') {
            line.pop_back();
        }
        if (line.compare(0, prefix.size(), prefix) != 0) {
            continue;
        }
        const auto space = line.find(' ', prefix.size());
        if (space == std::string::npos) {
            continue;
        }
        const auto number = line.substr(space + 1);
        if (number.empty() || number.find_first_not_of("0123456789") != std::string::npos) {
            continue;
        }
        // a 64 bit number, ANYBOX_BOX_CLOCK_WRAP_US doesn't fit in a long on Windows
        out[line.substr(prefix.size(), space - prefix.size())] = std::stoull(number);
    }
    return out;
}

const std::map<std::string, std::uint64_t>& header() {
    static const auto values = read_header();
    return values;
}

std::uint64_t header_value(const std::string& name) {
    const auto found = header().find(name);
    expect(found != header().end(), "anybox.h has no ANYBOX_" + name);
    return found->second;
}

// the header name of a value, checked to be the only one with that number under its prefix
Value name(const std::string& prefix, std::uint64_t number) {
    std::vector<std::string> names;
    for (const auto& entry : header()) {
        if (entry.first.compare(0, prefix.size() + 1, prefix + "_") == 0 && entry.second == number) {
            names.push_back(entry.first.substr(prefix.size() + 1));
        }
    }
    expect(names.size() == 1, "anybox.h has " + std::to_string(names.size()) + " names for " + prefix + " " +
                                  std::to_string(number));
    return Value(names[0]);
}

template <class E>
Value name_of(const std::string& prefix, E value) {
    return name(prefix, static_cast<std::uint64_t>(value));
}

template <class E>
Value optional_name(const std::string& prefix, const std::optional<E>& value) {
    return value ? name_of(prefix, *value) : Value(nullptr);
}

Value button_name(anybox::Button button) {
    const auto extra = header_value("BUTTON_EXTRA_BASE");
    if (button.value >= extra) {
        return Value(Object{{"Extra", Value(button.value - extra)}});
    }
    return name("BUTTON", button.value);
}

std::uint64_t number(const std::string& prefix, const Value& value) {
    if (value.is_string()) {
        return header_value(prefix + "_" + value.as_string());
    }
    if (value.is_object()) {
        return header_value("BUTTON_EXTRA_BASE") + static_cast<std::uint64_t>(value["Extra"].as_int());
    }
    return static_cast<std::uint64_t>(value.as_int());
}

void call(const anybox::Device& device, const std::string& name, const Array& args) {
    const auto arg = [&](std::size_t index, const char* kind) -> const Value& {
        expect(index < args.size(), name + " needs argument " + std::to_string(index));
        const auto* value = args[index].find(kind);
        expect(value != nullptr, "argument " + std::to_string(index) + " isn't a " + kind + ": " + args[index].dump());
        return *value;
    };
    const auto i16 = [&](std::size_t index) { return static_cast<std::int16_t>(arg(index, "i16").as_int()); };
    const auto button = [&](std::size_t index) {
        return anybox::Button{static_cast<std::uint32_t>(number("BUTTON", arg(index, "Button")))};
    };
    const auto key = [&](std::size_t index) {
        return anybox::Key{static_cast<std::uint8_t>(number("KEY", arg(index, "Key")))};
    };
    const auto text = [&](std::size_t index) { return arg(index, "Text").as_string(); };
    const auto keys = [&](std::size_t index) {
        std::vector<anybox::Key> out;
        for (const auto& key : arg(index, "Keys").as_array()) {
            out.push_back(anybox::Key{static_cast<std::uint8_t>(number("KEY", key))});
        }
        return out;
    };
    const auto media = [&](std::size_t index) {
        return anybox::MediaKey{static_cast<std::uint16_t>(number("MEDIA", arg(index, "MediaKey")))};
    };
    const auto axis = [&](std::size_t index) {
        return static_cast<anybox::Axis>(number("AXIS", arg(index, "Axis")));
    };
    const auto direction = [&](std::size_t index) {
        return static_cast<anybox::Direction>(number("DIRECTION", arg(index, "Direction")));
    };
    const auto click = [&](std::size_t index) {
        const auto& made = arg(index, "Click");
        anybox::Click out;
        if (const auto* hold = made.find("hold_us")) {
            out = anybox::Click::hold(std::chrono::microseconds(hold->as_int()));
        }
        if (const auto* only = made.find("native_only"); only && only->as_bool()) {
            out = out.native_only();
        }
        return out;
    };
    // made by bindings/gen from its call table (dispatch), change the table and run cargo run -p anybox-gen
    if (name == "move_by") return device.move_by(i16(0), i16(1));
    if (name == "move_now") return device.move_now(i16(0), i16(1));
    if (name == "press") return device.press(button(0));
    if (name == "release") return device.release(button(0));
    if (name == "click") return device.click(button(0));
    if (name == "click_with") return device.click(button(0), click(1));
    if (name == "press_key") return device.press_key(key(0));
    if (name == "release_key") return device.release_key(key(0));
    if (name == "tap_key") return device.tap_key(key(0));
    if (name == "tap_key_with") return device.tap_key(key(0), click(1));
    if (name == "type_text") return device.type_text(text(0));
    if (name == "press_keys") return device.press_keys(keys(0));
    if (name == "release_keys") return device.release_keys(keys(0));
    if (name == "tap_keys") return device.tap_keys(keys(0));
    if (name == "press_media") return device.press_media(media(0));
    if (name == "release_media") return device.release_media(media(0));
    if (name == "tap_media") return device.tap_media(media(0));
    if (name == "tap_media_with") return device.tap_media(media(0), click(1));
    if (name == "lock_button") return device.lock_button(button(0));
    if (name == "unlock_button") return device.unlock_button(button(0));
    if (name == "force_release_key") return device.force_release_key(key(0));
    if (name == "restore_key") return device.restore_key(key(0));
    if (name == "lock_key") return device.lock_key(key(0));
    if (name == "unlock_key") return device.unlock_key(key(0));
    if (name == "lock_axis") return device.lock_axis(axis(0), direction(1));
    if (name == "unlock_axis") return device.unlock_axis(axis(0), direction(1));
    if (name == "scale_axis") return device.scale_axis(axis(0), direction(1), i16(2));
    if (name == "wheel") return device.wheel(i16(0));
    if (name == "pan") return device.pan(i16(0));
    if (name == "discard_pending_moves") return device.discard_pending_moves();
    // the end of what bindings/gen makes (dispatch)
    throw Failure("the runner doesn't know the call " + name);
}

Value firmware(const anybox::Firmware& firmware) {
    switch (firmware.kind) {
    case anybox::FirmwareKind::Version:
        return Value(Object{{"version", Value(Array{firmware.major, firmware.minor, firmware.patch})}});
    case anybox::FirmwareKind::Build:
        return Value(Object{{"build", firmware.build}});
    case anybox::FirmwareKind::Protocol:
        return Value(Object{{"protocol", firmware.protocol}});
    default:
        return Value(Object{{"text", firmware.text}});
    }
}

Value optional_firmware(const std::optional<anybox::Firmware>& value) {
    return value ? firmware(*value) : Value(nullptr);
}

std::string hex(const std::uint8_t* bytes, std::size_t size) {
    static const char digits[] = "0123456789abcdef";
    std::string out;
    for (std::size_t index = 0; index < size; index++) {
        out += digits[bytes[index] >> 4];
        out += digits[bytes[index] & 0xF];
    }
    return out;
}

Value id(const std::optional<anybox::StableId>& value) {
    if (!value) {
        return Value(nullptr);
    }
    if (const auto& mac = value->mac()) {
        return Value(Object{{"mac", hex(mac->data(), mac->size())}});
    }
    return Value(Object{{"usb_serial", *value->usb_serial()}});
}

Value optional_text(const std::optional<std::string>& value) { return value ? Value(*value) : Value(nullptr); }

Value actions(const std::set<anybox::Action>& missing) {
    std::vector<std::string> names;
    for (const auto action : missing) {
        names.push_back(name_of("ACTION", action).as_string());
    }
    std::sort(names.begin(), names.end());
    Array out;
    for (auto& one : names) {
        out.push_back(Value(one));
    }
    return Value(std::move(out));
}

// the same fields per kind as the recorder writes, each read from the class the status makes, so a binding that
// built the wrong class or lost a field doesn't match
Value error_fields(const anybox::Error& error) {
    using S = anybox::Status;
    const auto status = error.status().value_or(S::Ok);
    const auto as = [&](auto* typed) {
        expect(typed != nullptr, std::string("an error of status ") + name_of("STATUS", status).as_string() +
                                     " isn't its class: " + error.what());
        return typed;
    };
    switch (status) {
    case S::NotFound: {
        Array tried;
        for (const auto& port : as(dynamic_cast<const anybox::NotFoundError*>(&error))->tried()) {
            tried.push_back(Value(port));
        }
        return Value(Object{{"tried", Value(std::move(tried))}});
    }
    case S::PortBusy:
        return Value(Object{{"port", as(dynamic_cast<const anybox::PortBusyError*>(&error))->port()}});
    case S::PermissionDenied:
        return Value(Object{{"port", as(dynamic_cast<const anybox::PermissionDeniedError*>(&error))->port()}});
    case S::NoBoxCan: {
        Array boxes;
        for (const auto& box : as(dynamic_cast<const anybox::NoBoxCanError*>(&error))->boxes()) {
            boxes.push_back(Value(Object{{"port", box.box.info.port}, {"missing", actions(box.missing)}}));
        }
        return Value(Object{{"boxes", Value(std::move(boxes))}});
    }
    case S::WrongFamily: {
        const auto* typed = as(dynamic_cast<const anybox::WrongFamilyError*>(&error));
        return Value(Object{{"port", typed->port()},
                            {"expected", name_of("FAMILY", typed->expected())},
                            {"found", optional_name("FAMILY", typed->found())}});
    }
    case S::WrongBox: {
        const auto* typed = as(dynamic_cast<const anybox::WrongBoxError*>(&error));
        return Value(Object{{"port", typed->port()}, {"expected", id(typed->expected())}, {"found", id(typed->found())}});
    }
    case S::WrongFerrumApi: {
        const auto* typed = as(dynamic_cast<const anybox::WrongFerrumApiError*>(&error));
        return Value(Object{{"port", typed->port()},
                            {"expected", name_of("FERRUM_API", typed->expected())},
                            {"found", name_of("FERRUM_API", typed->found())}});
    }
    case S::FirmwareTooOld: {
        const auto* typed = as(dynamic_cast<const anybox::FirmwareTooOldError*>(&error));
        return Value(Object{{"family", name_of("FAMILY", typed->family())},
                            {"found", optional_firmware(typed->found())},
                            {"needed", optional_firmware(typed->needed())}});
    }
    case S::LibraryTooOld: {
        const auto* typed = as(dynamic_cast<const anybox::LibraryTooOldError*>(&error));
        return Value(
            Object{{"family", name_of("FAMILY", typed->family())}, {"found", optional_firmware(typed->found())}});
    }
    case S::Unsupported: {
        const auto* typed = as(dynamic_cast<const anybox::UnsupportedError*>(&error));
        return Value(Object{{"action", optional_name("ACTION", typed->action())},
                            {"family", optional_name("FAMILY", typed->family())},
                            {"reason", typed->reason()}});
    }
    case S::InvalidArgument: {
        const auto* typed = as(dynamic_cast<const anybox::InvalidArgumentError*>(&error));
        return Value(Object{{"action", optional_name("ACTION", typed->action())}, {"detail", typed->detail()}});
    }
    case S::Rejected: {
        const auto* typed = as(dynamic_cast<const anybox::RejectedError*>(&error));
        return Value(Object{{"action", name_of("ACTION", typed->action())}, {"detail", typed->detail()}});
    }
    case S::Timeout:
        return Value(
            Object{{"action", name_of("ACTION", as(dynamic_cast<const anybox::BoxTimeoutError*>(&error))->action())}});
    case S::Interrupted:
        return Value(Object{
            {"action", name_of("ACTION", as(dynamic_cast<const anybox::BoxInterruptedError*>(&error))->action())}});
    case S::Io: {
        const auto* typed = as(dynamic_cast<const anybox::IoError*>(&error));
        return Value(Object{{"kind", name_of("IO_KIND", typed->kind())},
                            {"os_error", typed->os_error() ? Value(*typed->os_error()) : Value(nullptr)}});
    }
    case S::Closed:
        as(dynamic_cast<const anybox::ClosedError*>(&error));
        return Value(Object{});
    case S::Disconnected:
        as(dynamic_cast<const anybox::DisconnectedError*>(&error));
        return Value(Object{});
    default:
        return Value(Object{});
    }
}

Value error_json(const anybox::Error& error) {
    expect(error.status().has_value(), std::string("an error from the library has no status: ") + error.what());
    return Value(Object{{"status", name_of("STATUS", *error.status())},
                        {"text", std::string(error.what())},
                        {"fields", error_fields(error)}});
}

Value at(const anybox::Stamp& stamp) {
    if (!stamp.box_time) {
        return Value(Object{{"box_time", nullptr}});
    }
    return Value(Object{{"box_time", Value(Object{{"micros", stamp.box_time->micros},
                                                  {"clock", name_of("CLOCK", stamp.box_time->clock)}})}});
}

struct EventJson {
    Value operator()(const anybox::ButtonEvent& event) const {
        return make("Button", Object{{"button", button_name(event.button)}, {"pressed", event.pressed}, {"at", at(event.at)}});
    }
    Value operator()(const anybox::KeyEvent& event) const {
        return make("Key", Object{{"key", event.key.value}, {"pressed", event.pressed}, {"at", at(event.at)}});
    }
    Value operator()(const anybox::MediaEvent& event) const {
        return make("Media", Object{{"key", event.key.value}, {"pressed", event.pressed}, {"at", at(event.at)}});
    }
    Value operator()(const anybox::MotionEvent& event) const {
        return make("Motion", Object{{"dx", event.dx},
                                     {"dy", event.dy},
                                     {"wheel", event.wheel},
                                     {"pan", event.pan},
                                     {"at", at(event.at)}});
    }
    Value operator()(const anybox::Dropped& event) const {
        return make("Dropped", Object{{"count", event.count}, {"source", name_of("DROP_SOURCE", event.source)}});
    }
    Value operator()(const anybox::SessionReset&) const { return make("SessionReset", {}); }
    Value operator()(const anybox::Rejected& event) const {
        return make("Rejected", Object{{"action", name_of("ACTION", event.action)}, {"count", event.count}});
    }
    Value operator()(const anybox::Disconnected&) const { return make("Disconnected", {}); }
    Value operator()(const anybox::Reconnected&) const { return make("Reconnected", {}); }
    Value operator()(const anybox::Changed&) const { return make("Changed", {}); }

    static Value make(const char* event, Object fields) {
        return Value(Object{{"event", event}, {"fields", Value(std::move(fields))}});
    }
};

std::uint64_t us(const std::optional<std::chrono::microseconds>& value) {
    return value ? static_cast<std::uint64_t>(value->count()) : 0;
}

Value snapshot_json(const anybox::Snapshot& snapshot, const Value& expected) {
    const auto& info = snapshot.info;
    Array capabilities;
    for (const auto& want : expected["snapshot"]["capabilities"].as_array()) {
        const auto& action = want["action"].as_string();
        const auto& capability =
            snapshot.capabilities[static_cast<anybox::Action>(header_value("ACTION_" + action))];
        Array quirks;
        for (const auto& quirk : capability.quirks) {
            quirks.push_back(Value(Object{
                {"kind", name_of("QUIRK", quirk.kind)},
                {"min_us", us(quirk.min)},
                {"max_us", us(quirk.max)},
                {"count", quirk.count.value_or(0)},
                {"click_timing",
                 Value(Object{{"kind", quirk.click_timing ? name_of("CLICK_TIMING", quirk.click_timing->kind())
                                                          : Value(nullptr)},
                              {"fixed_us", quirk.click_timing ? us(quirk.click_timing->hold()) : 0}})},
            }));
        }
        capabilities.push_back(Value(Object{{"action", action},
                                            {"execution", name_of("EXECUTION", capability.execution)},
                                            {"reason", optional_text(capability.reason)},
                                            {"quirks", Value(std::move(quirks))}}));
    }
    Array buttons;
    for (const auto button : snapshot.capabilities.buttons()) {
        buttons.push_back(button_name(button));
    }
    Array axes;
    for (const auto axis : snapshot.capabilities.axes()) {
        axes.push_back(name_of("AXIS", axis));
    }
    const auto peripheral =
        info.peripheral ? Value(Object{{"vid", info.peripheral->vid}, {"pid", info.peripheral->pid}}) : Value(nullptr);
    return Value(Object{{"snapshot",
                         Value(Object{{"info", Value(Object{{"port", info.port},
                                                            {"family", name_of("FAMILY", info.family)},
                                                            {"model", optional_text(info.model)},
                                                            {"firmware", optional_firmware(info.firmware)},
                                                            {"id", id(info.id)},
                                                            {"ferrum_api", optional_name("FERRUM_API", info.ferrum_api)},
                                                            {"peripheral", peripheral}})},
                                      {"capabilities", Value(std::move(capabilities))},
                                      {"buttons", Value(std::move(buttons))},
                                      {"axes", Value(std::move(axes))}})}});
}

void push(const anybox::Mock& box, const Value& step) {
    const auto field = [&](const char* key) -> std::int64_t {
        const auto* value = step.find(key);
        return value ? value->as_int() : 0;
    };
    const auto* pressed_value = step.find("pressed");
    const bool pressed = pressed_value && pressed_value->as_bool();
    const auto seq = static_cast<std::uint8_t>(field("seq"));
    const auto ts = static_cast<std::uint32_t>(field("ts_us"));
    const auto& kind = step["push"].as_string();
    if (kind == "buttons") {
        box.push_buttons(seq, ts, pressed, static_cast<std::uint32_t>(field("held")));
    } else if (kind == "keys") {
        std::vector<anybox::Key> held;
        for (const auto& key : step["held"].as_array()) {
            held.push_back(anybox::Key{static_cast<std::uint8_t>(number("KEY", key))});
        }
        box.push_keys(seq, ts, pressed, held);
    } else if (kind == "media") {
        std::vector<anybox::MediaKey> held;
        for (const auto& key : step["held"].as_array()) {
            held.push_back(anybox::MediaKey{static_cast<std::uint16_t>(number("MEDIA", key))});
        }
        box.push_media(seq, ts, pressed, held);
    } else if (kind == "motion") {
        box.push_motion(seq, ts, static_cast<std::int16_t>(field("dx")), static_cast<std::int16_t>(field("dy")),
                        static_cast<std::int16_t>(field("wheel")), static_cast<std::int16_t>(field("pan")));
    } else if (kind == "link_lost") {
        box.link_lost();
    } else if (kind == "restart") {
        box.restart();
    } else {
        throw Failure("the mock can't push " + kind);
    }
}

const char* kind_name(const anybox::Event& event) {
    static const char* names[] = {"ButtonEvent", "KeyEvent",     "MediaEvent",   "MotionEvent", "Dropped",
                                  "SessionReset", "Rejected",    "Disconnected", "Reconnected", "Changed"};
    return names[event.index()];
}

Value recv(const anybox::Events& events, const anybox::Mock& box, const Value& expected, bool tick) {
    const auto wanted = expected["events"].as_array().size();
    Array got;
    const auto start = std::chrono::steady_clock::now();
    std::optional<std::chrono::steady_clock::time_point> quiet_from;
    for (;;) {
        if (got.size() >= wanted) {
            if (!quiet_from) {
                quiet_from = std::chrono::steady_clock::now();
            }
            if (std::chrono::steady_clock::now() - *quiet_from >= quiet) {
                break;
            }
        }
        expect(std::chrono::steady_clock::now() - start < 10s,
               "only " + std::to_string(got.size()) + " of " + std::to_string(wanted) + " events came");
        // no ticks while it waits for extras, so only what the scenario set off can come
        if (tick && got.size() < wanted) {
            box.tick();
        }
        if (auto next = events.recv(50ms)) {
            // one Rust didn't send is only its kind, it has nothing to read its fields by
            got.push_back(got.size() < wanted ? std::visit(EventJson{}, *next)
                                              : Value(Object{{"extra", kind_name(*next)}}));
        }
    }
    return Value(Object{{"events", Value(std::move(got))}});
}

template <class T>
Value example() {
    return Value(Object{{"error", error_json(anybox::example_error<T>())}});
}

// the text of a value given like a call's argument, or of a firmware or an id given the way a snapshot writes them
std::string text_of(const Value& value) {
    const auto& fields = value.as_object();
    expect(fields.size() == 1, "a text step names one value: " + value.dump());
    const auto& kind = fields[0].first;
    const auto& raw = fields[0].second;
    if (kind == "Action") {
        return anybox::to_string(static_cast<anybox::Action>(number("ACTION", raw)));
    }
    if (kind == "Family") {
        return anybox::to_string(static_cast<anybox::Family>(number("FAMILY", raw)));
    }
    if (kind == "FerrumApi") {
        return anybox::to_string(static_cast<anybox::FerrumApi>(number("FERRUM_API", raw)));
    }
    if (kind == "Axis") {
        return anybox::to_string(static_cast<anybox::Axis>(number("AXIS", raw)));
    }
    if (kind == "Button") {
        return anybox::to_string(anybox::Button{static_cast<std::uint32_t>(number("BUTTON", raw))});
    }
    if (kind == "Key") {
        return anybox::to_string(anybox::Key{static_cast<std::uint8_t>(number("KEY", raw))});
    }
    if (kind == "MediaKey") {
        return anybox::to_string(anybox::MediaKey{static_cast<std::uint16_t>(number("MEDIA", raw))});
    }
    if (kind == "Firmware") {
        const auto& field = raw.as_object().at(0);
        anybox::Firmware firmware;
        if (field.first == "version") {
            const auto& parts = field.second.as_array();
            firmware.kind = anybox::FirmwareKind::Version;
            firmware.major = static_cast<std::uint16_t>(parts.at(0).as_int());
            firmware.minor = static_cast<std::uint16_t>(parts.at(1).as_int());
            firmware.patch = static_cast<std::uint16_t>(parts.at(2).as_int());
        } else if (field.first == "build") {
            firmware.kind = anybox::FirmwareKind::Build;
            firmware.build = static_cast<std::uint32_t>(field.second.as_int());
        } else if (field.first == "protocol") {
            firmware.kind = anybox::FirmwareKind::Protocol;
            firmware.protocol = static_cast<std::uint8_t>(field.second.as_int());
        } else {
            firmware.kind = anybox::FirmwareKind::Text;
            firmware.text = field.second.as_string();
        }
        return anybox::to_string(firmware);
    }
    if (kind == "StableId") {
        if (const auto* mac = raw.find("mac")) {
            std::array<std::uint8_t, 6> bytes{};
            for (std::size_t at = 0; at < bytes.size(); at++) {
                bytes[at] = static_cast<std::uint8_t>(std::stoul(mac->as_string().substr(at * 2, 2), nullptr, 16));
            }
            return anybox::to_string(anybox::StableId::mac(bytes));
        }
        return anybox::to_string(anybox::StableId::usb_serial(raw["usb_serial"].as_string()));
    }
    throw Failure("the runner has no text for " + value.dump());
}

Value example_of(const std::string& status) {
    switch (static_cast<anybox::Status>(header_value("STATUS_" + status))) {
    case anybox::Status::NotFound:
        return example<anybox::NotFoundError>();
    case anybox::Status::Closed:
        return example<anybox::ClosedError>();
    case anybox::Status::PortBusy:
        return example<anybox::PortBusyError>();
    case anybox::Status::PermissionDenied:
        return example<anybox::PermissionDeniedError>();
    case anybox::Status::NoBoxCan:
        return example<anybox::NoBoxCanError>();
    case anybox::Status::WrongFamily:
        return example<anybox::WrongFamilyError>();
    case anybox::Status::WrongBox:
        return example<anybox::WrongBoxError>();
    case anybox::Status::WrongFerrumApi:
        return example<anybox::WrongFerrumApiError>();
    case anybox::Status::FirmwareTooOld:
        return example<anybox::FirmwareTooOldError>();
    case anybox::Status::LibraryTooOld:
        return example<anybox::LibraryTooOldError>();
    case anybox::Status::Unsupported:
        return example<anybox::UnsupportedError>();
    case anybox::Status::InvalidArgument:
        return example<anybox::InvalidArgumentError>();
    case anybox::Status::Rejected:
        return example<anybox::RejectedError>();
    case anybox::Status::Timeout:
        return example<anybox::BoxTimeoutError>();
    case anybox::Status::Disconnected:
        return example<anybox::DisconnectedError>();
    case anybox::Status::Interrupted:
        return example<anybox::BoxInterruptedError>();
    case anybox::Status::Io:
        return example<anybox::IoError>();
    default:
        throw Failure("the runner has no example of " + status);
    }
}

anybox::EventKinds kinds(const std::string& kinds_name) {
    return static_cast<anybox::EventKinds>(header_value("EVENTS_" + kinds_name));
}

template <class D>
Value optional_count(const std::optional<D>& value) {
    return value ? Value(static_cast<std::int64_t>(value->count())) : Value(nullptr);
}

Value interpolation_json(const anybox::Interpolation& value) {
    return Value(Object{{"Interpolation", value.is_automatic ? Value("AUTO") : Value(value.percentage)}});
}

Value ysc_json(const anybox::YscInterpolation& value) {
    return Value(Object{{"enabled", Value(value.enabled)},
                        {"profile", Value(static_cast<std::uint32_t>(value.profile))},
                        {"window_ms", optional_count(value.window)},
                        {"max_window_ms", optional_count(value.max_window)},
                        {"measured_interval_us", optional_count(value.measured_interval)}});
}

Value ms_json(const std::optional<std::chrono::milliseconds>& value) { return optional_count(value); }

Value serial_json(const anybox::YscSerial& serial) {
    return Value(Object{{"custom", optional_text(serial.custom)},
                        {"physical", optional_text(serial.physical)},
                        {"active", optional_text(serial.active)}});
}

Value xy_json(const std::pair<std::int32_t, std::int32_t>& xy) { return Value(Array{xy.first, xy.second}); }

// the handle's call in the suite's form, {"none": null} when the device isn't that family
template <class Handle, class Call>
Value on(const std::optional<Handle>& handle, Call call) {
    if (!handle) {
        return Value(Object{{"none", nullptr}});
    }
    return Value(Object{{"ok", call(*handle)}});
}

// one call only one family has, `makcu.interpolation` and the like
Value extension(const anybox::Device& device, const Value& step) {
    const auto& name = step["extension"].as_string();
    std::vector<Value> args;
    for (const auto& arg : step["args"].as_array()) {
        args.push_back(arg.as_object().front().second);
    }
    const auto interpolation = [&](std::size_t index) {
        return args[index].is_string() ? anybox::Interpolation::automatic()
                                       : anybox::Interpolation::percent(static_cast<std::uint8_t>(args[index].as_int()));
    };
    const auto ysc_settings = [&](std::size_t index) {
        const auto& settings = args[index];
        return anybox::YscInterpolationSettings{settings["enabled"].as_bool(),
                                                static_cast<anybox::YscProfile>(settings["profile"].as_int()),
                                                std::chrono::milliseconds(settings["window_ms"].as_int()),
                                                std::chrono::milliseconds(settings["max_window_ms"].as_int())};
    };
    const auto ms_or_off = [&](std::size_t index) {
        return args[index].is_null() ? std::optional<std::chrono::milliseconds>()
                                     : std::chrono::milliseconds(args[index].as_int());
    };
    const auto ms = [&](std::size_t index) { return ms_or_off(index).value(); };
    const auto flag = [&](std::size_t index) { return args[index].as_bool(); };
    try {
        // made by bindings/gen from its call table (makcu_dispatch), change the table and run cargo run -p anybox-gen
        if (name == "makcu.interpolation") return on(device.makcu(), [&](const anybox::Makcu& makcu) { return interpolation_json(makcu.interpolation()); });
        if (name == "makcu.set_interpolation") return on(device.makcu(), [&](const anybox::Makcu& makcu) { makcu.set_interpolation(interpolation(0)); return Value(); });
        if (name == "makcu.moving") return on(device.makcu(), [&](const anybox::Makcu& makcu) { return Value(makcu.moving()); });
        if (name == "makcu.autorelease") return on(device.makcu(), [&](const anybox::Makcu& makcu) { return ms_json(makcu.autorelease()); });
        if (name == "makcu.set_autorelease") return on(device.makcu(), [&](const anybox::Makcu& makcu) { makcu.set_autorelease(ms_or_off(0)); return Value(); });
        // the end of what bindings/gen makes (makcu_dispatch)
        // made by bindings/gen from its call table (ysc_dispatch), change the table and run cargo run -p anybox-gen
        if (name == "ysc.interpolation") return on(device.ysc(), [&](const anybox::Ysc& ysc) { return ysc_json(ysc.interpolation()); });
        if (name == "ysc.set_interpolation") return on(device.ysc(), [&](const anybox::Ysc& ysc) { ysc.set_interpolation(ysc_settings(0)); return Value(); });
        if (name == "ysc.reset_interpolation") return on(device.ysc(), [&](const anybox::Ysc& ysc) { return ysc_json(ysc.reset_interpolation()); });
        if (name == "ysc.serial") return on(device.ysc(), [&](const anybox::Ysc& ysc) { return serial_json(ysc.serial()); });
        // the end of what bindings/gen makes (ysc_dispatch)
        // made by bindings/gen from its call table (ferrum_dispatch), change the table and run cargo run -p anybox-gen
        if (name == "ferrum.catch_xy") return on(device.ferrum(), [&](const anybox::Ferrum& ferrum) { return xy_json(ferrum.catch_xy(ms(0), flag(1))); });
        // the end of what bindings/gen makes (ferrum_dispatch)
        throw Failure("there's no extension call " + name);
    } catch (const anybox::Error& error) {
        const auto* expected = step.find("outcome");
        const bool wants_error = expected && expected->is_object() && expected->find("error");
        return wants_error ? Value(Object{{"error", error_json(error)}})
                           : Value(Object{{"error", std::string(error.what())}});
    }
}

Array run_family(const Value& scenario) {
    const auto& family = scenario["box"].as_string();
    const auto timing = anybox::ClickTiming::fixed(5ms);
    const auto device = family == "makcu" ? anybox::Mock::makcu(timing)
                        : family == "ysc" ? anybox::Mock::ysc(timing)
                        : family == "ferrum"
                            ? anybox::Mock::ferrum(timing)
                            : throw Failure("there's no mock " + family);
    Array steps;
    for (const auto& step : scenario["steps"].as_array()) {
        auto out = step;
        out.set("outcome", extension(device, step));
        steps.push_back(std::move(out));
    }
    device.close();
    return steps;
}

Array run(const Value& scenario) {
    if (scenario.find("box")) {
        return run_family(scenario);
    }
    auto opened = anybox::Mock::medius_manual(anybox::ClickTiming::fixed(5ms), scenario["keyboard"].as_bool());
    auto& device = opened.first;
    auto& box = opened.second;
    std::optional<anybox::Events> events;
    if (const auto* wanted = scenario.find("events")) {
        events.emplace(device.events(kinds(wanted->as_string())));
        auto streamed = kinds(wanted->as_string());
        // a mock with no keyboard has no keys or media to subscribe
        if (!scenario["keyboard"].as_bool()) {
            streamed = streamed & (anybox::EventKinds::BUTTONS | anybox::EventKinds::MOTION);
        }
        const auto start = std::chrono::steady_clock::now();
        while (!box.subscribed(streamed)) {
            if (std::chrono::steady_clock::now() - start > 5s) {
                expect(false, "the receiver never subscribed");
                break;
            }
            std::this_thread::sleep_for(10ms);
        }
    }
    Array steps;
    for (const auto& step : scenario["steps"].as_array()) {
        const auto* expected = step.find("outcome");
        auto out = step;
        out.erase("outcome");
        out.erase("frames");
        Value outcome;
        if (const auto* name = step.find("call")) {
            box.clear_recorded();
            const auto started = std::chrono::steady_clock::now();
            try {
                call(device, name->as_string(), step["args"].as_array());
                outcome = Value(Object{{"ok", nullptr}});
            } catch (const anybox::Error& error) {
                const bool wants_error = expected && expected->is_object() && expected->find("error");
                outcome = wants_error ? Value(Object{{"error", error_json(error)}})
                                      : Value(Object{{"error", std::string(error.what())}});
            }
            // a hold on a box that can't time it is a wait on the PC, which no frame shows
            if (const auto* at_least = step.find("takes_at_least_us")) {
                const auto took = std::chrono::steady_clock::now() - started;
                expect(took >= std::chrono::microseconds(at_least->as_int()), step.dump() + " didn't take its hold");
            }
            Array frames;
            for (const auto& frame : box.recorded()) {
                if (!keepalive(frame)) {
                    frames.push_back(Value(Array{frame.type, hex(frame.payload.data(), frame.payload.size())}));
                }
            }
            out.set("frames", Value(std::move(frames)));
        } else if (step.find("extension")) {
            outcome = extension(device, step);
        } else if (step.find("snapshot")) {
            outcome = snapshot_json(device.snapshot(), *expected);
        } else if (step.find("push")) {
            push(box, step);
        } else if (const auto* wait = step.find("wait_us")) {
            std::this_thread::sleep_for(std::chrono::microseconds(wait->as_int()));
        } else if (step.find("recv")) {
            expect(events.has_value(), "a scenario that receives says which events");
            outcome = recv(*events, box, *expected, false);
        } else if (step.find("tick_until")) {
            expect(events.has_value(), "a scenario that receives says which events");
            outcome = recv(*events, box, *expected, true);
        } else if (step.find("example_events")) {
            anybox::example_events(device);
        } else if (const auto* value = step.find("text")) {
            outcome = Value(Object{{"text", Value(text_of(*value))}});
        } else if (const auto* status = step.find("example")) {
            outcome = example_of(status->as_string());
        } else {
            throw Failure("the runner doesn't know the step " + step.dump());
        }
        out.set("outcome", std::move(outcome));
        steps.push_back(std::move(out));
    }
    return steps;
}

}  // namespace

int main() {
    std::vector<std::string> failed;
    try {
        const auto expected = Value::parse(read("target/conformance/expected.json"));
        const auto scenarios = Value::parse(read("bindings/conformance/scenarios.json"));
        for (const auto& scenario : scenarios.as_array()) {
            const auto& name = scenario["name"].as_string();
            try {
                const Value* recorded = nullptr;
                for (const auto& one : expected.as_array()) {
                    if (one["name"].as_string() == name) {
                        recorded = &one;
                    }
                }
                expect(recorded != nullptr, "expected.json has no scenario called " + name);
                const auto& want = (*recorded)["steps"].as_array();
                const auto got = run(*recorded);
                expect(want.size() == got.size(), "the step count differs");
                for (std::size_t index = 0; index < want.size(); index++) {
                    expect(want[index] == got[index], "step " + std::to_string(index) + " differs from Rust\nwant " +
                                                          want[index].dump() + "\ngot  " + got[index].dump());
                }
                std::printf("ok: %s\n", name.c_str());
            } catch (const std::exception& error) {
                std::printf("FAILED: %s\n%s\n", name.c_str(), error.what());
                failed.push_back(name);
            }
        }
    } catch (const std::exception& error) {
        std::printf("conformance: %s\n", error.what());
        return 1;
    }
    if (!failed.empty()) {
        std::printf("conformance: %zu scenarios differ from Rust\n", failed.size());
        return 1;
    }
    std::printf("conformance: every scenario does what Rust does\n");
    return 0;
}
