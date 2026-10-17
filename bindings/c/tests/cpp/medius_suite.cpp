// the Medius suite's C++ runner: every scenario in bindings/conformance/medius.json through anybox_medius.hpp and
// medius' mock, held to what the medius crate itself did with it, which the capi crate's tests write to
// target/conformance/medius-expected.json. bindings/c/src/medius_suite.rs says what a scenario is. it runs from the
// repo's root, test-c.sh starts it there

#include "anybox.hpp"
#include "json.hpp"

#include <array>
#include <chrono>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <functional>
#include <map>
#include <optional>
#include <set>
#include <sstream>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

namespace suite {

using json::Array;
using json::Object;
using json::Value;
using namespace anybox::medius;

struct Failure : std::runtime_error {
    using std::runtime_error::runtime_error;
};

void expect(bool ok, const std::string& what) {
    if (!ok) {
        throw Failure(what);
    }
}

std::string hex(const std::uint8_t* bytes, std::size_t length) {
    static const char digits[] = "0123456789abcdef";
    std::string out;
    for (std::size_t at = 0; at < length; at++) {
        out += digits[bytes[at] >> 4];
        out += digits[bytes[at] & 0xF];
    }
    return out;
}

// a refusal the runner stands in for, where C++'s types can't even hold the value, like a MAC that isn't 6 bytes
struct Refused : std::invalid_argument {
    using std::invalid_argument::invalid_argument;
};

std::vector<std::uint8_t> unhex(const Value& value) {
    const auto& text = value.as_string();
    const auto digit = [&](char c) {
        if (c >= '0' && c <= '9') {
            return c - '0';
        }
        if (c >= 'a' && c <= 'f') {
            return c - 'a' + 10;
        }
        throw std::runtime_error("the scenario has bytes that aren't lowercase hex: " + text);
    };
    if (text.size() % 2 != 0) {
        throw std::runtime_error("the scenario has an odd number of hex digits: " + text);
    }
    std::vector<std::uint8_t> out;
    for (std::size_t at = 0; at < text.size(); at += 2) {
        out.push_back(static_cast<std::uint8_t>(digit(text[at]) * 16 + digit(text[at + 1])));
    }
    return out;
}

// the plain values, declared before the generated converters and the templates that call them
template <class T, std::enable_if_t<std::is_integral<T>::value && !std::is_same<T, bool>::value, int> = 0>
Value write(T number) {
    return Value(number);
}
inline Value write(bool flag) { return Value(flag); }
inline Value write(const std::string& text) { return Value(text); }
inline Value write(const std::vector<std::uint8_t>& bytes) { return Value(hex(bytes.data(), bytes.size())); }
template <std::size_t N>
Value write(const std::array<std::uint8_t, N>& bytes) {
    return Value(hex(bytes.data(), N));
}
template <class T>
Value write(const std::vector<T>& items);
template <class T>
Value write(const std::optional<T>& item);
template <class A, class B>
Value write(const std::pair<A, B>& pair);

template <class T, std::enable_if_t<std::is_integral<T>::value && !std::is_same<T, bool>::value, int> = 0>
void read(const Value& value, T& out) {
    out = static_cast<T>(value.as_int());
}
inline void read(const Value& value, bool& out) { out = value.as_bool(); }
inline void read(const Value& value, std::string& out) { out = value.as_string(); }
inline void read(const Value& value, std::vector<std::uint8_t>& out) { out = unhex(value); }
template <std::size_t N>
void read(const Value& value, std::array<std::uint8_t, N>& out) {
    const auto bytes = unhex(value);
    // a MAC that isn't the array's length is refused before medius, like the other bindings refuse it
    if (bytes.size() != N) {
        throw Refused("the bytes are " + std::to_string(bytes.size()) + " long, they have to be " +
                      std::to_string(N));
    }
    std::copy(bytes.begin(), bytes.end(), out.begin());
}
template <class T>
void read(const Value& value, std::vector<T>& out);
template <class T>
void read(const Value& value, std::optional<T>& out);

// the records the generated ones hold that the bindings write by hand
Value write(const Usage& usage);
void read(const Value& value, Usage& out);
Value write(const LockTarget& target);
void read(const Value& value, LockTarget& out);
Value write(const CatchFilter& filter);
void read(const Value& value, CatchFilter& out);
Value write(const EmitPaceStatus& status);
Value write(const ClipSettings& settings);
void read(const Value& value, ClipSettings& out);
Value write(const CatchEvent& event);
Value write(const InputEvent& event);
Value write(DirectionMeaning meaning);

// a value of a type with no default, made from its JSON. it's defined after the generated readers, because a
// template only sees the overloads declared before it and an enum's read isn't found through its namespace
template <class T>
T parse(const Value& value);
template <>
Motion parse<Motion>(const Value& value);

template <class T>
T arg(const Array& args, std::size_t at) {
    return parse<T>(args.at(at));
}

// the handles a scenario works on, made as it needs them
struct Context {
    MockBox mock_;
    std::optional<Device> device_;
    std::optional<Clip> clip_;
    std::optional<ClipBuilder> builder_{ClipBuilder()};
    std::optional<ClipFrame> frame_{ClipFrame()};
    std::optional<Timeline> timeline_{Timeline()};
    std::optional<EventStream> events_;
    std::optional<InputStream> inputs_;
    std::optional<LogStream> logs_;

    MockBox& mock() { return mock_; }
    Device& device() { return *device_; }
    Clip& clip() { return *clip_; }
    ClipBuilder& builder() { return *builder_; }
    ClipFrame& frame() { return *frame_; }
    Timeline& timeline() { return *timeline_; }
};

#include "medius_suite.hpp"

template <class T>
T parse(const Value& value) {
    T out{};
    read(value, out);
    return out;
}

template <class T>
Value write(const std::vector<T>& items) {
    Array out;
    for (const auto& item : items) {
        out.push_back(write(item));
    }
    return Value(std::move(out));
}

template <class T>
Value write(const std::optional<T>& item) {
    return item ? write(*item) : Value();
}

template <class A, class B>
Value write(const std::pair<A, B>& pair) {
    return Value(Array{write(pair.first), write(pair.second)});
}

template <class T>
void read(const Value& value, std::vector<T>& out) {
    out.clear();
    for (const auto& item : value.as_array()) {
        out.push_back(parse<T>(item));
    }
}

template <class T>
void read(const Value& value, std::optional<T>& out) {
    if (value.is_null()) {
        out.reset();
    } else {
        out = parse<T>(value);
    }
}

Value write(const Usage& usage) { return Value(Object{{"kind", write(usage.kind)}, {"id", write(usage.id)}}); }

void read(const Value& value, Usage& out) {
    out = Usage(parse<Class>(value["kind"]), static_cast<std::uint16_t>(value["id"].as_int()));
}

Value write(const LockTarget& target) {
    return Value(Object{{"kind", write(target.kind())}, {"usage", write(target.input())}});
}

void read(const Value& value, LockTarget& out) {
    const auto kind = parse<LockTargetKind>(value["kind"]);
    switch (kind) {
    case LockTargetKind::X:
        out = LockTarget::axis(Axis::X);
        break;
    case LockTargetKind::Y:
        out = LockTarget::axis(Axis::Y);
        break;
    case LockTargetKind::Wheel:
        out = LockTarget::axis(Axis::Wheel);
        break;
    case LockTargetKind::Pan:
        out = LockTarget::axis(Axis::Pan);
        break;
    default:
        out = LockTarget::usage(parse<Usage>(value["usage"]));
    }
}

template <>
Motion parse<Motion>(const Value& value) {
    const auto& kind = value["kind"].as_string();
    if (kind == "Cursor") {
        return Motion::cursor(static_cast<std::int16_t>(value["dx"].as_int()), static_cast<std::int16_t>(value["dy"].as_int()));
    }
    if (kind == "Wheel") {
        return Motion::wheel(static_cast<std::int16_t>(value["delta"].as_int()));
    }
    return Motion::pan(static_cast<std::int16_t>(value["delta"].as_int()));
}

Value write(const CatchFilter& filter) {
    return Value(Object{{"class_", write(filter.class_())},
                        {"id", write(filter.id())},
                        {"direction", write(filter.direction())},
                        {"capture", write(filter.capture())}});
}

// a filter argument is the calls that build it, a constructor and then the modifiers, and a filter the box lists is
// its fields, which only the C struct takes
void read(const Value& value, CatchFilter& out) {
    if (value.is_object()) {
        MediusCatchFilter raw{};
        raw.class_ = value["class_"].is_null() ? MEDIUS_CATCH_CLASS_ANY : detail::write(parse<CatchClass>(value["class_"]));
        raw.id = value["id"].is_null() ? MEDIUS_CATCH_ID_ANY : static_cast<std::uint16_t>(value["id"].as_int());
        raw.direction = detail::write(parse<Direction>(value["direction"]));
        raw.capture = static_cast<std::uint8_t>(value["capture"].as_int());
        out = CatchFilter::from_c(raw);
        return;
    }
    std::optional<CatchFilter> filter;
    for (const auto& call : value.as_array()) {
        const auto& args = call.as_array();
        const auto& name = args.at(0).as_string();
        if (name == "watch") {
            filter = CatchFilter::watch(parse<Usage>(args.at(1)));
        } else if (name == "watch_axis") {
            filter = CatchFilter::watch_axis(parse<Axis>(args.at(1)));
        } else if (name == "watch_class") {
            filter = CatchFilter::watch_class(parse<Class>(args.at(1)));
        } else if (name == "watch_axes") {
            filter = CatchFilter::watch_axes();
        } else if (name == "traffic") {
            filter = CatchFilter::traffic(parse<TrafficClass>(args.at(1)), static_cast<std::uint16_t>(args.at(2).as_int()));
        } else if (name == "traffic_of") {
            filter = CatchFilter::traffic_of(parse<TrafficClass>(args.at(1)));
        } else if (name == "everything") {
            filter = CatchFilter::everything();
        } else if (name == "with_direction") {
            filter = filter->with_direction(parse<Direction>(args.at(1)));
        } else if (name == "with_capture") {
            filter = filter->with_capture(static_cast<std::uint8_t>(args.at(1).as_int()));
        } else if (name == "on_press") {
            filter = filter->on_press();
        } else if (name == "on_release") {
            filter = filter->on_release();
        } else if (name == "inbound") {
            filter = filter->inbound();
        } else if (name == "outbound") {
            filter = filter->outbound();
        } else {
            throw std::runtime_error("a filter can't take " + name);
        }
    }
    out = *filter;
}

Value pace(const EmitPace& value) { return Value(Object{{"mode", write(value.mode())}, {"hz", write(value.hz())}}); }

EmitPace pace(const Value& value) {
    switch (parse<EmitMode>(value["mode"])) {
    case EmitMode::Learned:
        return EmitPace::learned();
    case EmitMode::Interval:
        return EmitPace::interval();
    default:
        return EmitPace::fixed(static_cast<std::uint16_t>(value["hz"].as_int()));
    }
}

Value write(const EmitPaceStatus& status) {
    return Value(Object{{"mode", pace(status.mode)},
                        {"resolved_hz", write(status.resolved_hz)},
                        {"force_hz", write(status.force_hz)},
                        {"advertised_hz", write(status.advertised_hz)},
                        {"force_active", write(status.force_active)}});
}

Value write(const ClipSettings& settings) {
    return Value(Object{{"autolock", write(settings.autolock)},
                        {"loop", write(settings.loop)},
                        {"retain", write(settings.retain)},
                        {"finalized", write(settings.finalized)},
                        {"ride", write(settings.ride)},
                        {"triggers", write(settings.triggers)},
                        {"packet_triggers", write(settings.packet_triggers)}});
}

void read(const Value& value, ClipSettings& out) {
    read(value["autolock"], out.autolock);
    read(value["loop"], out.loop);
    read(value["retain"], out.retain);
    read(value["finalized"], out.finalized);
    read(value["ride"], out.ride);
    read(value["triggers"], out.triggers);
    read(value["packet_triggers"], out.packet_triggers);
}

Value write(const CatchEvent& event) {
    Value data;
    if (const auto* motion = event.motion()) {
        data = write(*motion);
    } else if (const auto* usages = event.usages()) {
        data = write(*usages);
    } else if (const auto* traffic = event.traffic()) {
        data = write(*traffic);
    }
    return Value(Object{{"kind", write(event.kind)}, {"ts_us", write(event.ts_us)}, {"clock", write(event.clock)}, {"data", data}});
}

CatchEvent catch_event(const Value& value) {
    CatchEvent event;
    event.kind = parse<CatchEventKind>(value["kind"]);
    event.ts_us = static_cast<std::uint32_t>(value["ts_us"].as_int());
    event.clock = parse<ClockDomain>(value["clock"]);
    switch (event.kind) {
    case CatchEventKind::Motion:
        event.data = parse<MotionEvent>(value["data"]);
        break;
    case CatchEventKind::Usages:
        event.data = parse<UsageSnapshot>(value["data"]);
        break;
    case CatchEventKind::Traffic:
        event.data = parse<TrafficEvent>(value["data"]);
        break;
    }
    return event;
}

Value write(const InputEvent& event) {
    return Value(Object{{"kind", write(event.kind)},
                        {"ts_us", write(event.ts_us)},
                        {"clock", write(event.clock)},
                        {"usage", write(event.usage)},
                        {"dx", write(event.dx)},
                        {"dy", write(event.dy)},
                        {"dz", write(event.dz)},
                        {"pan", write(event.pan)}});
}

InputEvent input_event(const Value& value) {
    InputEvent event;
    event.kind = parse<InputKind>(value["kind"]);
    read(value["usage"], event.usage);
    read(value["dx"], event.dx);
    read(value["dy"], event.dy);
    read(value["dz"], event.dz);
    read(value["pan"], event.pan);
    return event;
}

Value write(DirectionMeaning meaning) {
    switch (meaning) {
    case DirectionMeaning::Edge:
        return Value("Edge");
    case DirectionMeaning::Sign:
        return Value("Sign");
    default:
        return Value("Flow");
    }
}

std::optional<std::chrono::milliseconds> window(const Value& value) {
    if (value.is_null()) {
        return std::nullopt;
    }
    return std::chrono::milliseconds(value.as_int());
}

std::optional<std::uint16_t> force(const Value& value) {
    if (value.is_null()) {
        return std::nullopt;
    }
    return static_cast<std::uint16_t>(value.as_int());
}

// a function the binding wraps by hand, by its C++ class and method
Value by_hand(Context& ctx, const std::string& cls, const std::string& method, const Array& args) {
    const auto call = cls + "::" + method;
    if (call == "Device::clip") {
        ctx.clip_.emplace(ctx.device().clip());
    } else if (call == "Device::catch_events") {
        ctx.events_.emplace(ctx.device().catch_events(arg<std::vector<CatchFilter>>(args, 0)));
    } else if (call == "Device::input_events") {
        ctx.inputs_.emplace(ctx.device().input_events(arg<std::vector<CatchFilter>>(args, 0)));
    } else if (call == "Device::logs") {
        ctx.logs_.emplace(ctx.device().logs());
    } else if (call == "Device::transfer") {
        return write(ctx.device().transfer(arg<std::uint8_t>(args, 0), arg<Setup>(args, 1),
                                           arg<std::vector<std::uint8_t>>(args, 2), window(args.at(3))));
    } else if (call == "Device::remove_rewrite") {
        ctx.device().remove_rewrite(arg<RewriteRule>(args, 0));
    } else if (call == "Device::set_movement_riding") {
        ctx.device().set_movement_riding(window(args.at(0)));
    } else if (call == "Device::query_movement_riding") {
        const auto riding = ctx.device().query_movement_riding();
        return riding ? Value(static_cast<std::int64_t>(riding->count())) : Value();
    } else if (call == "Device::set_emit_pace") {
        ctx.device().set_emit_pace(pace(args.at(0)), force(args.at(1)));
    } else if (call == "Device::set_bearing") {
        ctx.device().set_bearing(window(args.at(0)), arg<BearingMode>(args, 1));
    } else if (call == "Device::stage_firmware") {
        ctx.device().stage_firmware(arg<UpdateTarget>(args, 0), arg<std::vector<std::uint8_t>>(args, 1));
    } else if (call == "Device::update_firmware") {
        ctx.device().update_firmware(arg<UpdateTarget>(args, 0), arg<std::vector<std::uint8_t>>(args, 1));
    } else if (call == "Clip::set_autolock") {
        ctx.clip().set_autolock(arg<std::vector<Blanket>>(args, 0));
    } else if (call == "Clip::unbind_packet") {
        ctx.clip().unbind_packet(arg<ClipPacketTrigger>(args, 0));
    } else if (call == "MockBox::set_movement_riding") {
        ctx.mock().set_movement_riding(window(args.at(0)));
    } else if (call == "MockBox::set_bearing") {
        ctx.mock().set_bearing(arg<Bearing>(args, 0));
    } else if (call == "MockBox::set_emit_pace") {
        ctx.mock().set_emit_pace(pace(args.at(0)), force(args.at(1)));
    } else if (call == "MockBox::clip_packet") {
        const auto fired = ctx.mock().clip_packet(arg<TrafficClass>(args, 0), arg<std::uint16_t>(args, 1),
                                                  arg<Direction>(args, 2), arg<std::vector<std::uint8_t>>(args, 3));
        return Value(Object{{"action", write(fired.first)}, {"consumed", write(fired.second)}});
    } else if (call == "MockBox::push_raw") {
        ctx.mock().push_raw(arg<std::vector<std::uint8_t>>(args, 0));
    } else if (call == "EventStream::recv") {
        return write(ctx.events_->recv(std::chrono::milliseconds(args.at(0).as_int())));
    } else if (call == "EventStream::try_recv") {
        return write(ctx.events_->try_recv());
    } else if (call == "EventStream::dropped") {
        return write(ctx.events_->dropped());
    } else if (call == "EventStream::is_connected") {
        return write(ctx.events_->is_connected());
    } else if (call == "InputStream::recv") {
        return write(ctx.inputs_->recv(std::chrono::milliseconds(args.at(0).as_int())));
    } else if (call == "InputStream::try_recv") {
        return write(ctx.inputs_->try_recv());
    } else if (call == "InputStream::dropped") {
        return write(ctx.inputs_->dropped());
    } else if (call == "InputStream::is_connected") {
        return write(ctx.inputs_->is_connected());
    } else if (call == "InputStream::held") {
        return write(ctx.inputs_->held(arg<Class>(args, 0)));
    } else if (call == "LogStream::recv") {
        return write(ctx.logs_->recv(std::chrono::milliseconds(args.at(0).as_int())));
    } else if (call == "LogStream::try_recv") {
        return write(ctx.logs_->try_recv());
    } else if (call == "LogStream::is_connected") {
        return write(ctx.logs_->is_connected());
    } else if (call == "Timeline::observe") {
        CatchEvent event;
        event.ts_us = static_cast<std::uint32_t>(args.at(0)["ts_us"].as_int());
        event.clock = parse<ClockDomain>(args.at(0)["clock"]);
        return write(ctx.timeline().observe(event, static_cast<std::uint64_t>(args.at(1).as_int())));
    } else if (call == "Timeline::observe_stamp") {
        return write(ctx.timeline().observe_stamp(arg<std::uint32_t>(args, 0), arg<ClockDomain>(args, 1),
                                                  static_cast<std::uint64_t>(args.at(2).as_int())));
    } else if (call == "ClipBuilder::new") {
        ctx.builder_.emplace();
    } else if (call == "ClipFrame::new") {
        ctx.frame_.emplace();
    } else if (call == "Timeline::new") {
        ctx.timeline_.emplace();
    } else {
        throw std::runtime_error("the runner has no " + call);
    }
    return Value();
}

Value key(const PatchKey& key) {
    return Value(Object{{"section", write(key.section)}, {"cfg", write(key.cfg)}, {"index", write(key.index)}, {"offset", write(key.offset)}});
}

Value axis_or_null(const std::optional<Axis>& axis) { return write(axis); }

// a helper by its Rust name, with what the binding has for it
Value helper(const std::string& name, const Array& args) {
    using Helper = std::function<Value(const Array&)>;
    static const std::map<std::string, Helper> helpers = {
        {"Bearing::is_live", [](const Array& a) { return write(arg<Bearing>(a, 0).is_live()); }},
        {"Caps::has_keyboard", [](const Array& a) { return write(arg<Caps>(a, 0).has_keyboard()); }},
        {"Caps::has_mouse", [](const Array& a) { return write(arg<Caps>(a, 0).has_mouse()); }},
        {"Caps::is_composite", [](const Array& a) { return write(arg<Caps>(a, 0).is_composite()); }},
        {"Capture::bytes", [](const Array& a) { return write(Capture::bytes(arg<std::uint8_t>(a, 0))); }},
        {"Capture::widest", [](const Array& a) { return write(Capture::widest(arg<std::uint8_t>(a, 0), arg<std::uint8_t>(a, 1))); }},
        {"CatchClass: From<Class>", [](const Array& a) { return write(to_catch_class(arg<Class>(a, 0))); }},
        {"CatchClass: From<TrafficClass>", [](const Array& a) { return write(to_catch_class(arg<TrafficClass>(a, 0))); }},
        {"CatchClass::direction_meaning", [](const Array& a) { return write(direction_meaning(arg<CatchClass>(a, 0))); }},
        {"CatchClass::is_input", [](const Array& a) { return write(is_input(arg<CatchClass>(a, 0))); }},
        {"CatchClass::is_traffic", [](const Array& a) { return write(is_traffic(arg<CatchClass>(a, 0))); }},
        {"CatchEvent::bytes", [](const Array& a) { return write(catch_event(a.at(0)).bytes()); }},
        {"CatchEvent::class", [](const Array& a) { return write(catch_event(a.at(0)).class_()); }},
        {"CatchEvent::clock", [](const Array& a) { return write(catch_event(a.at(0)).clock); }},
        {"CatchEvent::direction", [](const Array& a) { return write(catch_event(a.at(0)).direction()); }},
        {"CatchEvent::id", [](const Array& a) { return write(catch_event(a.at(0)).id()); }},
        {"CatchEvent::ts_us", [](const Array& a) { return write(catch_event(a.at(0)).ts_us); }},
        {"CatchFilter::all_input", [](const Array&) { return write(CatchFilter::all_input()); }},
        {"CatchFilter::capture", [](const Array& a) { return write(arg<CatchFilter>(a, 0).capture()); }},
        {"CatchFilter::class", [](const Array& a) { return write(arg<CatchFilter>(a, 0).class_()); }},
        {"CatchFilter::direction", [](const Array& a) { return write(arg<CatchFilter>(a, 0).direction()); }},
        {"CatchFilter::id", [](const Array& a) { return write(arg<CatchFilter>(a, 0).id()); }},
        {"CatchFilter::same_address", [](const Array& a) { return write(arg<CatchFilter>(a, 0).same_address(arg<CatchFilter>(a, 1))); }},
        {"CatchFilter::everything", [](const Array&) { return write(CatchFilter::everything()); }},
        {"CatchFilter::watch", [](const Array& a) { return write(CatchFilter::watch(arg<Usage>(a, 0))); }},
        {"CatchFilter::watch_axes", [](const Array&) { return write(CatchFilter::watch_axes()); }},
        {"CatchFilter::watch_axis", [](const Array& a) { return write(CatchFilter::watch_axis(arg<Axis>(a, 0))); }},
        {"CatchFilter::watch_class", [](const Array& a) { return write(CatchFilter::watch_class(arg<Class>(a, 0))); }},
        {"CatchFilter::traffic", [](const Array& a) { return write(CatchFilter::traffic(arg<TrafficClass>(a, 0), arg<std::uint16_t>(a, 1))); }},
        {"CatchFilter::traffic_class", [](const Array& a) { return write(CatchFilter::traffic_of(arg<TrafficClass>(a, 0))); }},
        {"CatchFilter::inbound", [](const Array& a) { return write(arg<CatchFilter>(a, 0).inbound()); }},
        {"CatchFilter::outbound", [](const Array& a) { return write(arg<CatchFilter>(a, 0).outbound()); }},
        {"CatchFilter::on_press", [](const Array& a) { return write(arg<CatchFilter>(a, 0).on_press()); }},
        {"CatchFilter::on_release", [](const Array& a) { return write(arg<CatchFilter>(a, 0).on_release()); }},
        {"CatchFilter::with_capture", [](const Array& a) { return write(arg<CatchFilter>(a, 0).with_capture(arg<std::uint8_t>(a, 1))); }},
        {"CatchFilter::with_direction", [](const Array& a) { return write(arg<CatchFilter>(a, 0).with_direction(arg<Direction>(a, 1))); }},
        {"CatchState::is_empty", [](const Array& a) { return write(arg<CatchState>(a, 0).is_empty()); }},
        {"ClipStatus::is_held", [](const Array& a) { return write(arg<ClipStatus>(a, 0).is_held(arg<Usage>(a, 1))); }},
        {"ClockEstimate::drift_us_over", [](const Array& a) {
             return write(arg<ClockEstimate>(a, 0).drift_us_over(std::chrono::microseconds(a.at(1).as_int())));
         }},
        {"ClockEstimate::error_bound_us", [](const Array& a) { return write(arg<ClockEstimate>(a, 0).error_bound_us()); }},
        {"ClockEstimate::to_host_domain", [](const Array& a) { return write(arg<ClockEstimate>(a, 0).to_host_domain(a.at(1).as_int())); }},
        {"DeviceInfo::is_cloned", [](const Array& a) { return write(arg<DeviceInfo>(a, 0).is_cloned()); }},
        {"Direction: From<Edge>", [](const Array& a) { return write(to_direction(arg<Edge>(a, 0))); }},
        {"Direction::admits", [](const Array& a) { return write(admits(arg<Direction>(a, 0), arg<Direction>(a, 1))); }},
        {"Direction::is_relative", [](const Array& a) { return write(is_relative(arg<Direction>(a, 0))); }},
        {"Direction::of_delta", [](const Array& a) { return write(of_delta(arg<std::int16_t>(a, 0))); }},
        {"FirmwareInfo::any_pending", [](const Array& a) { return write(arg<FirmwareInfo>(a, 0).any_pending()); }},
        {"ImageState::is_pending", [](const Array& a) { return write(is_pending(arg<ImageState>(a, 0))); }},
        {"Input::axes", [](const Array& a) { return write(input_event(a.at(0)).axes()); }},
        {"Input::direction", [](const Array& a) { return write(input_event(a.at(0)).direction()); }},
        {"Input::is_press", [](const Array& a) { return write(input_event(a.at(0)).is_press()); }},
        {"Input::is_release", [](const Array& a) { return write(input_event(a.at(0)).is_release()); }},
        {"Input::usage", [](const Array& a) { return write(input_event(a.at(0)).usage); }},
        {"Key::is_modifier", [](const Array& a) { return write(is_modifier(arg<Key>(a, 0))); }},
        {"LockEntry::is_block", [](const Array& a) { return write(arg<LockEntry>(a, 0).is_block()); }},
        {"LockTarget::as_axis", [](const Array& a) { return axis_or_null(arg<LockTarget>(a, 0).as_axis()); }},
        {"Locks::is_locked", [](const Array& a) { return write(arg<Locks>(a, 0).is_locked(arg<LockTarget>(a, 1), arg<Direction>(a, 2))); }},
        {"Locks::scale_of", [](const Array& a) { return write(arg<Locks>(a, 0).scale_of(arg<LockTarget>(a, 1), arg<Direction>(a, 2))); }},
        {"MotionEvent::all_axes", [](const Array& a) { return write(arg<MotionEvent>(a, 0).all_axes()); }},
        {"MotionEvent::axes", [](const Array& a) { return write(arg<MotionEvent>(a, 0).axes()); }},
        {"MouseCaps::is_composite", [](const Array& a) { return write(arg<MouseCaps>(a, 0).is_composite()); }},
        {"Patch::in_config", [](const Array& a) {
             return write(Patch::in_config(arg<std::uint8_t>(a, 0), arg<std::uint16_t>(a, 1), arg<std::vector<std::uint8_t>>(a, 2)));
         }},
        {"Patch::in_interface", [](const Array& a) {
             return write(Patch::in_interface(arg<std::uint8_t>(a, 0), arg<std::uint8_t>(a, 1), arg<std::uint16_t>(a, 2),
                                              arg<std::vector<std::uint8_t>>(a, 3)));
         }},
        {"Patch::in_string", [](const Array& a) { return write(Patch::in_string(arg<std::uint8_t>(a, 0), arg<std::vector<std::uint8_t>>(a, 1))); }},
        {"Patch::key", [](const Array& a) { return key(arg<Patch>(a, 0).key()); }},
        {"Rate::native_hz", [](const Array& a) {
             // the bits, since the JSON has no floats and the bits compare exactly
             const auto hz = arg<Rate>(a, 0).native_hz();
             if (!hz) {
                 return Value();
             }
             std::uint32_t bits = 0;
             std::memcpy(&bits, &*hz, sizeof bits);
             return write(bits);
         }},
        {"RewriteAction::carries_payload", [](const Array& a) { return write(carries_payload(arg<RewriteAction>(a, 0))); }},
        {"RewriteAction::is_valid_for", [](const Array& a) { return write(is_valid_for(arg<RewriteAction>(a, 0), arg<RewriteClass>(a, 1))); }},
        {"RewriteClass::is_control", [](const Array& a) { return write(is_control(arg<RewriteClass>(a, 0))); }},
        {"RewriteRule::key", [](const Array& a) {
             const auto key = arg<RewriteRule>(a, 0).key();
             return Value(Object{{"class_", write(key.class_)},
                                 {"id", write(key.id)},
                                 {"direction", write(key.direction)},
                                 {"match_bytes", write(key.match_bytes)},
                                 {"mask", write(key.mask)}});
         }},
        {"Setup::is_in", [](const Array& a) { return write(arg<Setup>(a, 0).is_in()); }},
        {"Setup::to_bytes", [](const Array& a) { return write(arg<Setup>(a, 0).to_bytes()); }},
        {"TrafficClass: TryFrom<CatchClass>", [](const Array& a) { return write(to_traffic_class(arg<CatchClass>(a, 0))); }},
        {"TrafficEvent::bulk_end_of_transfer", [](const Array& a) { return write(arg<TrafficEvent>(a, 0).bulk_end_of_transfer()); }},
        {"TrafficEvent::bus_event", [](const Array& a) { return write(arg<TrafficEvent>(a, 0).bus_event()); }},
        {"TrafficEvent::control_status", [](const Array& a) { return write(arg<TrafficEvent>(a, 0).control_status()); }},
        {"TrafficEvent::data", [](const Array& a) { return write(arg<TrafficEvent>(a, 0).data()); }},
        {"TrafficEvent::rule_acted", [](const Array& a) { return write(arg<TrafficEvent>(a, 0).rule_acted()); }},
        {"TrafficEvent::setup", [](const Array& a) { return write(arg<TrafficEvent>(a, 0).setup_bytes()); }},
        {"TrafficEvent::transfer_status", [](const Array& a) { return write(arg<TrafficEvent>(a, 0).transfer_status()); }},
        {"TrafficEvent::truncated", [](const Array& a) { return write(arg<TrafficEvent>(a, 0).truncated()); }},
        {"TrafficEvent::zlp", [](const Array& a) { return write(arg<TrafficEvent>(a, 0).zlp()); }},
        {"TransferOutcome::is_ok", [](const Array& a) { return write(arg<TransferOutcome>(a, 0).is_ok()); }},
        {"TransferOutcome::ok_data", [](const Array& a) { return write(arg<TransferOutcome>(a, 0).ok_data()); }},
        {"TransferStatus::is_ok", [](const Array& a) { return write(is_ok(arg<TransferStatus>(a, 0))); }},
        {"Transform::key", [](const Array& a) {
             const auto key = arg<Transform>(a, 0).key();
             return Value(Object{{"source", write(key.source)}, {"dest", write(key.dest)}});
         }},
        {"Transform::remap", [](const Array& a) { return write(Transform::remap(arg<LockTarget>(a, 0), arg<LockTarget>(a, 1))); }},
        {"Transform::swap", [](const Array& a) { return write(Transform::swap(arg<Axis>(a, 0), arg<Axis>(a, 1))); }},
        {"TransformOp::admits", [](const Array& a) {
             return write(admits(arg<TransformOp>(a, 0), arg<LockTarget>(a, 1), arg<LockTarget>(a, 2)));
         }},
        {"UpdateProgress::percent", [](const Array& a) {
             const auto& value = a.at(0);
             const UpdateProgress progress{parse<UpdateTarget>(value["target"]), static_cast<std::size_t>(value["sent"].as_int()),
                                           static_cast<std::size_t>(value["total"].as_int())};
             return write(progress.percent());
         }},
        {"UsageSnapshot::is_held", [](const Array& a) { return write(arg<UsageSnapshot>(a, 0).is_held(arg<Usage>(a, 1))); }},
        {"Version::mac_hex", [](const Array& a) { return write(arg<Version>(a, 0).mac_hex()); }},
    };
    const auto found = helpers.find(name);
    expect(found != helpers.end(), "the runner has no helper " + name);
    return found->second(args);
}

Value text(const std::string& name, const Value& value) {
    if (name == "ChipFirmware") {
        return Value(to_string(parse<ChipFirmware>(value)));
    }
    if (name == "DeviceInfo") {
        return Value(to_string(parse<DeviceInfo>(value)));
    }
    if (name == "DeviceKind") {
        return Value(to_string(parse<DeviceKind>(value)));
    }
    if (name == "ImageState") {
        return Value(to_string(parse<ImageState>(value)));
    }
    if (name == "UpdateTarget") {
        return Value(to_string(parse<UpdateTarget>(value)));
    }
    if (name == "Version") {
        return Value(to_string(parse<Version>(value)));
    }
    throw std::runtime_error("the runner has no text for " + name);
}

// how a step ended, a value, medius' error, or a refusal before medius
Value outcome(const std::function<Value()>& step) {
    try {
        return Value(Object{{"value", step()}});
    } catch (const UpdateError& error) {
        Object out{{"status", write(error.medius_status())}, {"text", Value(error.what())}, {"proto_ver", write(error.proto_ver())}};
        if (error.op()) {
            out.push_back({"update", Value(Object{{"op", write(*error.op())}, {"status", write(*error.update_status())}, {"arg", write(*error.arg())}})});
        }
        return Value(Object{{"error", Value(std::move(out))}});
    } catch (const Error& error) {
        return Value(Object{{"error", Value(Object{{"status", write(error.medius_status())},
                                                     {"text", Value(error.what())},
                                                     {"proto_ver", write(error.proto_ver())}})}});
    } catch (const std::invalid_argument&) {
        return Value(Object{{"refused", Value(true)}});
    }
}

// the runner's own mistake has to stop it, only the binding or a value C++ can't hold counts as a refusal
void runner_mistakes_stop_it() {
    expect(outcome([]() -> Value { throw Refused("a MAC of 5 bytes"); }) == Value(Object{{"refused", Value(true)}}),
           "a refusal the runner stands in for didn't come out refused");
    bool stopped = false;
    try {
        outcome([]() -> Value {
            unhex(Value(std::string("abc")));
            return Value();
        });
    } catch (const std::runtime_error&) {
        stopped = true;
    }
    expect(stopped, "bytes the runner can't read passed as an outcome");
}

// the frames the mock got since `seen`, without the keepalive's and nothing after the box restarts, the same way the
// recorder leaves them out
Value frames(Context& ctx, std::size_t& seen, std::vector<Value>& sent, bool& restarted) {
    Array out;
    const auto total = ctx.mock().recorded();
    // clear_recorded empties the list, so the count starts over
    if (total < seen) {
        seen = 0;
    }
    for (; seen < total && !restarted; seen++) {
        const auto frame = *ctx.mock().recorded_frame(seen);
        const Value value(Object{{"type", write(frame.type)}, {"payload", write(frame.payload)}});
        const bool keepalive =
            (frame.type == FrameType::Query && frame.payload.size() == 1 && (frame.payload[0] == 5 || frame.payload[0] == 10)) ||
            ((frame.type == FrameType::Catch || frame.type == FrameType::Rewrite || frame.type == FrameType::Transform) &&
             std::find(sent.begin(), sent.end(), value) != sent.end());
        if (!keepalive) {
            sent.push_back(value);
            out.push_back(value);
        }
        // the chip boots on a RESET with the NVS flag (0x01) and on a reboot into the device chip's firmware
        restarted = !frame.payload.empty() &&
                    ((frame.type == FrameType::Reset && (frame.payload[0] & 0x01) != 0) ||
                     (frame.type == FrameType::RebootDl &&
                      frame.payload[0] == static_cast<std::uint8_t>(RebootTarget::DeviceRun)));
    }
    return Value(std::move(out));
}

// a type's default the way C++ gives it, a value made with nothing set, or the record field that holds it
Value default_of(const std::string& name) {
    static const std::map<std::string, std::function<Value()>> defaults{
        {"Bearing", [] { return write(Bearing{}); }},
        {"Caps", [] { return write(Caps{}); }},
        {"CountersSnapshot", [] { return write(CountersSnapshot{}); }},
        {"DeviceInfo", [] { return write(DeviceInfo{}); }},
        {"EmitPaceStatus", [] { return write(EmitPaceStatus{}); }},
        {"ImperfectStatus", [] { return write(ImperfectStatus{}); }},
        {"KbdCaps", [] { return write(KbdCaps{}); }},
        {"MouseCaps", [] { return write(MouseCaps{}); }},
        {"Patch", [] { return write(Patch{}); }},
        {"PatchSet", [] { return write(PatchSet{}); }},
        {"RenderStatus", [] { return write(RenderStatus{}); }},
        {"RewriteRule", [] { return write(RewriteRule{}); }},
        {"RewriteTable", [] { return write(RewriteTable{}); }},
        {"SpreadStatus", [] { return write(SpreadStatus{}); }},
        {"Transforms", [] { return write(Transforms{}); }},
        {"ClipStatus", [] { return write(ClipStatus{}); }},
        {"Setup", [] { return write(Setup{}); }},
        {"ClipSettings", [] { return write(ClipSettings{}); }},
        {"EmitPace", [] { return pace(EmitPace{}); }},
        {"ClipBuilder", [] { return write(ClipBuilder().byte_len()); }},
        {"ClipFrame", [] { return write(ClipFrame().byte_len()); }},
        {"Capture", [] { return write(Capture::Whole); }},
        {"BearingMode", [] { return write(BearingMode{}); }},
        {"ClipState", [] { return write(ClipState{}); }},
        {"DeviceKind", [] { return write(DeviceKind{}); }},
        {"Direction", [] { return write(Direction{}); }},
        {"MoveTiming", [] { return write(MoveTiming{}); }},
        {"PatchSection", [] { return write(PatchSection{}); }},
        {"PendingMotion", [] { return write(PendingMotion{}); }},
        {"RenderMode", [] { return write(RenderStatus{}.mode); }},
        {"RewriteAction", [] { return write(RewriteAction{}); }},
        {"RewriteClass", [] { return write(RewriteRule{}.class_); }},
    };
    const auto found = defaults.find(name);
    if (found == defaults.end()) {
        throw std::runtime_error("the runner has no default for " + name);
    }
    return found->second();
}

std::string read_file(const std::string& path) {
    std::ifstream file(path, std::ios::binary);
    expect(file.good(), "there's no " + path + ", run the capi crate's tests first, they record it");
    std::stringstream text;
    text << file.rdbuf();
    return text.str();
}

// the calls the ledger holds on hardware, by their C++ name. a mock device's reconnect still rescans the real serial
// ports and takes the first box that opens, so none of them runs here, like in the recorder
const std::set<std::string>& on_hardware() {
    static const std::set<std::string> calls = [] {
        std::set<std::string> out;
        std::istringstream lines(read_file("bindings/conformance/medius-crate.txt"));
        for (std::string line; std::getline(lines, line);) {
            std::vector<std::string> fields;
            std::istringstream row(line);
            for (std::string field; std::getline(row, field, '|');) {
                const auto first = field.find_first_not_of(" \r");
                fields.push_back(first == std::string::npos
                                     ? std::string()
                                     : field.substr(first, field.find_last_not_of(" \r") - first + 1));
            }
            if (fields.size() >= 4 && fields[0].rfind('#', 0) != 0 && fields[1] == "hardware") {
                out.insert(fields[3]);
            }
        }
        return out;
    }();
    return calls;
}

Value step(Context& ctx, const Value& step) {
    const auto* given = step.find("args");
    const auto args = given ? given->as_array() : Array{};
    if (const auto* name = step.find("helper")) {
        return helper(name->as_string(), args);
    }
    if (const auto* name = step.find("text")) {
        return text(name->as_string(), step["value"]);
    }
    if (const auto* name = step.find("default")) {
        return default_of(name->as_string());
    }
    const auto* on = step.find("on");
    const auto cls = on ? on->as_string() : std::string("Device");
    const auto& method = step["call"].as_string();
    if (on_hardware().count(cls + "::" + method) != 0) {
        throw std::runtime_error(cls + "::" + method +
                                 " only runs on a real box, on the mock it would reach whatever box is plugged in");
    }
    if (auto result = call(ctx, cls, method, args)) {
        return *result;
    }
    return by_hand(ctx, cls, method, args);
}

Value run(const Value& scenario) {
    Context ctx;
    if (const auto* setup = scenario.find("setup")) {
        for (const auto& each : setup->as_array()) {
            const auto* given = each.find("args");
            const auto args = given ? given->as_array() : Array{};
            const auto& method = each["call"].as_string();
            if (!call(ctx, "MockBox", method, args)) {
                by_hand(ctx, "MockBox", method, args);
            }
        }
    }
    std::size_t seen = 0;
    std::vector<Value> sent;
    bool restarted = false;
    const auto* bare = scenario.find("bare");
    const auto opened = outcome([&] {
        ctx.device_.emplace(bare && bare->as_bool() ? Device::with_mock(ctx.mock()) : Device::open_mock(ctx.mock()));
        return Value();
    });
    Object out{{"open", Value(Object{{"outcome", opened}, {"frames", frames(ctx, seen, sent, restarted)}})}};
    Array steps;
    for (const auto& each : scenario["steps"].as_array()) {
        const auto result = outcome([&] { return step(ctx, each); });
        // the mock's own restart boots the chip without a frame
        const auto* on = each.find("on");
        const auto* call = each.find("call");
        if (on && call && on->as_string() == "MockBox" && call->as_string() == "restart") {
            restarted = true;
        }
        steps.push_back(Value(Object{{"outcome", result}, {"frames", frames(ctx, seen, sent, restarted)}}));
    }
    out.push_back({"steps", Value(std::move(steps))});
    return Value(std::move(out));
}

// open stands in for reconnect, so this can't reach a box even with the check gone
void a_call_held_on_hardware_never_reaches_the_mock() {
    expect(on_hardware().count("Device::reconnect") != 0, "the ledger doesn't hold reconnect on hardware");
    Context ctx;
    std::string why;
    try {
        step(ctx, Value(Object{{"call", Value(std::string("open"))}}));
    } catch (const std::runtime_error& error) {
        why = error.what();
    }
    expect(why.find("only runs on a real box") != std::string::npos, "a call held on hardware got past the runner: " + why);
}

// the query and whatever medius sends to recover come after the restart
void nothing_after_a_restart_counts() {
    Context ctx;
    ctx.device_.emplace(Device::open_mock(ctx.mock()));
    std::size_t seen = ctx.mock().recorded();
    std::vector<Value> sent;
    bool restarted = false;
    step(ctx, Value::parse(R"({"call": "factory_reset"})"));
    step(ctx, Value::parse(R"({"call": "query_version"})"));
    const auto got = frames(ctx, seen, sent, restarted);
    expect(got == Value::parse(R"([{"type": "Catch", "payload": "ffffff000000"}, {"type": "Reset", "payload": "01"}])") &&
               restarted,
           "a frame after the restart counted: " + got.dump());
    const auto mock = run(Value::parse(
        R"({"name": "after the mock's restart", "steps": [{"on": "MockBox", "call": "restart"}, {"call": "query_version"}]})"));
    expect(mock["steps"].as_array().at(1)["frames"] == Value(Array{}),
           "a frame after the mock's restart counted: " + mock["steps"].dump());
}

}  // namespace suite

int main() {
    using suite::Value;
    std::vector<std::string> failed;
    try {
        suite::runner_mistakes_stop_it();
        suite::a_call_held_on_hardware_never_reaches_the_mock();
        suite::nothing_after_a_restart_counts();
        const auto expected = Value::parse(suite::read_file("target/conformance/medius-expected.json"));
        const auto scenarios = Value::parse(suite::read_file("bindings/conformance/medius.json"));
        for (const auto& scenario : scenarios.as_array()) {
            const auto& name = scenario["name"].as_string();
            try {
                const Value* recorded = nullptr;
                for (const auto& one : expected.as_array()) {
                    if (one["name"].as_string() == name) {
                        recorded = &one;
                    }
                }
                suite::expect(recorded != nullptr, "medius-expected.json has no scenario called " + name);
                const auto got = suite::run(scenario);
                suite::expect((*recorded)["open"] == got["open"],
                              "the open differs from Rust\nwant " + (*recorded)["open"].dump() + "\ngot  " + got["open"].dump());
                const auto& want = (*recorded)["steps"].as_array();
                const auto& steps = got["steps"].as_array();
                suite::expect(want.size() == steps.size(), "the step count differs");
                for (std::size_t index = 0; index < want.size(); index++) {
                    suite::expect(want[index] == steps[index], "step " + std::to_string(index) + " differs from Rust\nwant " +
                                                                   want[index].dump() + "\ngot  " + steps[index].dump());
                }
                std::printf("ok: %s\n", name.c_str());
            } catch (const std::exception& error) {
                std::printf("FAILED: %s\n%s\n", name.c_str(), error.what());
                failed.push_back(name);
            }
        }
    } catch (const std::exception& error) {
        std::printf("medius suite: %s\n", error.what());
        return 1;
    }
    if (!failed.empty()) {
        std::printf("medius suite: %zu scenarios differ from Rust\n", failed.size());
        return 1;
    }
    std::printf("medius suite: every scenario does what the medius crate does\n");
    return 0;
}
