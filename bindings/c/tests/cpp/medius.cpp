// the C++ Medius API's tests, the same things the C# Medius tests check (bindings/csharp/tests/Anybox.Tests/Medius,
// each test named like its C# one) over the medius mock in the same library. the payload bytes are the wire format
// from the firmware's ctrl_proto.h, so they pin what each call actually sends, not just that it returned. built with
// the mock of both libraries, from the repo root, since a few tests read medius.h

#include "anybox.hpp"

#include <algorithm>
#include <array>
#include <atomic>
#include <cctype>
#include <chrono>
#include <cstdio>
#include <fstream>
#include <functional>
#include <future>
#include <map>
#include <set>
#include <stdexcept>
#include <string>
#include <thread>
#include <tuple>
#include <typeinfo>
#include <utility>
#include <vector>

std::string medius_from_the_second_unit();

namespace {

using namespace std::chrono_literals;
using namespace anybox::medius;
using Bytes = std::vector<std::uint8_t>;

constexpr std::chrono::seconds wait{2};

int failures = 0;

void check(bool ok, const std::string& what) {
    if (!ok) {
        std::printf("  FAILED: %s\n", what.c_str());
        failures++;
    }
}

template <class Thrown, class Call>
void throws(Call call, const std::string& what, const std::string& text = "") {
    try {
        call();
        check(false, what + " didn't throw");
    } catch (const Thrown& error) {
        if (!text.empty()) {
            check(std::string(error.what()).find(text) != std::string::npos,
                  what + " threw \"" + error.what() + "\" without \"" + text + "\"");
        }
    }
}

// C# asserts these aren't null before it goes on, here the test stops and says what didn't come
template <class T>
T must(std::optional<T> value, const std::string& what) {
    if (!value) {
        throw std::runtime_error(what + " didn't come");
    }
    return std::move(*value);
}

void until(const std::function<bool()>& condition, const std::string& what) {
    const auto start = std::chrono::steady_clock::now();
    while (!condition()) {
        if (std::chrono::steady_clock::now() - start > 3s) {
            check(false, what);
            return;
        }
        std::this_thread::sleep_for(5ms);
    }
}

std::chrono::milliseconds since(std::chrono::steady_clock::time_point start) {
    return std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - start);
}

// waits for every reader to count itself out, so a close that never wakes one fails here instead of hanging the
// whole program on the join
bool ended_in_time(std::vector<std::thread>& readers, const std::atomic<int>& ended) {
    const auto start = std::chrono::steady_clock::now();
    while (ended < static_cast<int>(readers.size()) && since(start) < 3s) {
        std::this_thread::sleep_for(5ms);
    }
    const auto all = ended == static_cast<int>(readers.size());
    for (auto& reader : readers) {
        if (all) {
            reader.join();
        } else {
            reader.detach();
        }
    }
    return all;
}

std::vector<Bytes> frames(const MockBox& mock, FrameType type) {
    std::vector<Bytes> out;
    for (std::size_t index = 0; index < mock.recorded(); index++) {
        const auto frame = mock.recorded_frame(index);
        if (frame && frame->type == type) {
            out.push_back(frame->payload);
        }
    }
    return out;
}

Bytes hex(const std::string& text) {
    Bytes out;
    for (std::size_t at = 0; at < text.size(); at++) {
        if (text[at] != ' ') {
            out.push_back(static_cast<std::uint8_t>(std::stoi(text.substr(at, 2), nullptr, 16)));
            at++;
        }
    }
    return out;
}

Bytes range(std::size_t count) {
    Bytes out;
    for (std::size_t value = 0; value < count; value++) {
        out.push_back(static_cast<std::uint8_t>(value));
    }
    return out;
}

Bytes join(Bytes first, const Bytes& second) {
    first.insert(first.end(), second.begin(), second.end());
    return first;
}

std::vector<std::string> header_lines() {
    std::ifstream file("bindings/c/include/medius.h");
    if (!file) {
        throw std::runtime_error("bindings/c/include/medius.h isn't there, run this from the repo root");
    }
    std::vector<std::string> lines;
    for (std::string line; std::getline(file, line);) {
        if (!line.empty() && line.back() == '\r') {
            line.pop_back();
        }
        lines.push_back(line);
    }
    return lines;
}

const ImperfectStatus allowed{true, false, false};

TrafficEvent traffic(CatchClass catch_class, std::uint16_t id, Direction direction, std::uint8_t flags = 0,
                     std::uint16_t true_len = 0, Bytes bytes = {}) {
    return TrafficEvent{catch_class, id, direction, flags, true_len, std::move(bytes)};
}

RewriteRule rewrite(RewriteClass rewrite_class, std::uint16_t id, Direction direction, RewriteAction action,
                    std::uint16_t offset = 0, Bytes match = {}, Bytes mask = {}, Bytes payload = {}) {
    return RewriteRule{rewrite_class, id, direction, action, offset, std::move(match), std::move(mask), std::move(payload)};
}

ClipTrigger input_trigger(const Usage& on, Edge edge, ClipAction action, bool consume = false) {
    return ClipTrigger{on, edge, action, consume};
}

ClipPacketTrigger packet(TrafficClass traffic_class, std::uint16_t id, Direction direction, ClipAction action,
                         Bytes match = {}, Bytes mask = {}, bool consume = false, bool once_per_run = false,
                         std::uint8_t selector_len = 0, std::uint16_t hits = 0) {
    return ClipPacketTrigger{traffic_class,   id,           direction,    action, std::move(match),
                             std::move(mask), consume,      once_per_run, selector_len, hits};
}

struct RawReport {
    std::uint8_t ep;
    Direction direction;
    Bytes bytes;
};

struct Transfer {
    std::uint8_t ep;
    Setup setup;
    Bytes data;
};

// C#'s ClipBuilder.Frame, which sets the motion, wheel and pan every time
ClipFrame make_frame(std::int16_t dx = 0, std::int16_t dy = 0, std::int16_t wheel = 0, std::int16_t pan = 0,
                     const std::vector<std::pair<Usage, Action>>& edges = {}, const std::vector<RawReport>& raw = {},
                     const std::vector<Transfer>& transfers = {}) {
    ClipFrame frame;
    frame.move(dx, dy).wheel(wheel).pan(pan);
    for (const auto& edge : edges) {
        frame.edge(edge.first, edge.second);
    }
    for (const auto& report : raw) {
        frame.raw(report.ep, report.direction, report.bytes);
    }
    for (const auto& transfer : transfers) {
        frame.transfer(transfer.ep, transfer.setup, transfer.data);
    }
    return frame;
}

void the_mock_is_there() {
    const MockBox mock;
    check(mock.get() != nullptr, "the mock");
}

void the_meta_functions_answer() {
    check(abi_version() == 10u, "the medius ABI is " + std::to_string(abi_version()));
    check(!version().empty(), "the medius version");
    check(default_query_timeout_ms() > 0 && default_keepalive_cadence_ms() > 0 && default_transfer_timeout_ms() > 0,
          "the defaults");
}

void the_abi_version_is_the_headers() {
    const std::string define = "#define MEDIUS_ABI_VERSION ";
    std::optional<unsigned long> declared;
    for (const auto& line : header_lines()) {
        if (line.rfind(define, 0) == 0) {
            declared = std::stoul(line.substr(define.size()));
        }
    }
    check(declared == abi_version(), "medius.h's ABI version");
}

void a_library_with_another_medius_abi_is_refused() {
    const auto speaks = abi_version();
    anybox::detail::check_medius_abi(speaks, AbiVersion);
    for (const std::uint32_t other : {speaks - 1, speaks + 1}) {
        throws<anybox::MediusAbiError>([&] { anybox::detail::check_medius_abi(speaks, other); },
                                       "another medius ABI",
                                       "speaks C ABI " + std::to_string(speaks) + " and medius.h needs " +
                                           std::to_string(other));
    }
}

void a_set_version_comes_back_on_open() {
    const MockBox mock;
    const auto proto = mock.with_device().query_version().proto_ver;
    const std::array<std::uint8_t, 6> mac{0x5A, 0x4E, 0x00, 0x11, 0x1E, 0x28};
    mock.set_version(Version{proto, 9, 8, 7, mac, "Left PC"});
    const auto version = mock.open().query_version();
    check(version.proto_ver == proto && version.fw_major == 9 && version.fw_minor == 8 && version.fw_patch == 7 &&
              version.name == "Left PC",
          "the version");
    check(version.mac == mac, "the MAC");
    check(version.mac_hex() == "5a4e00111e28", "the MAC as hex is " + version.mac_hex());
}

void a_name_can_be_set_and_cleared() {
    const MockBox mock;
    const auto device = mock.open();
    // nothing is checked on the host, like the other setters, the box cleans it up
    device.set_name("Left PC");
    device.set_name(std::string(40, 'x'));
    device.clear_name();
}

void a_silent_mock_fails_the_open() {
    const MockBox mock;
    mock.silent();
    throws<Error>([&] { Device::open_mock(mock); }, "opening a silent mock");
}

void a_bad_protocol_version_says_its_status_and_protocol() {
    const MockBox mock;
    mock.set_version(Version{99, 1, 0, 0, {}, ""});
    try {
        Device::open_mock(mock);
        check(false, "a box on protocol 99 opened");
    } catch (const BadProtoVerError& error) {
        check(error.medius_status() == Status::ErrBadProtoVer, "the status");
        check(error.proto_ver() == 99, "the protocol is " + std::to_string(error.proto_ver()));
        check(!error.status(), "a medius error has an anybox status");
    }
}

void a_listed_box_on_another_protocol_has_no_device() {
    MediusBoxInfo raw{};
    raw.version.proto_ver = 7;
    const std::uint8_t mac[6] = {0x5A, 0x4E, 0x00, 0x00, 0x00, 0x01};
    std::copy(mac, mac + 6, raw.version.mac);
    const auto old = BoxInfo::from_c(raw);
    check(!old.device, "a box on another protocol has a device");
    check(old.version.proto_ver == 7 && old.id() == "5a4e00000001", "the old box's version and id");
    raw.version.proto_ver = 8;
    raw.device.vid = 0x046D;
    raw.device.kind = static_cast<std::uint8_t>(MEDIUS_DEVICE_KIND_MOUSE);
    raw.has_device = 1;
    const auto current = BoxInfo::from_c(raw);
    check(current.device && current.device->kind == DeviceKind::Mouse && current.device->vid == 0x046D,
          "the current box's device");
}

void a_recorded_frames_payload_can_be_read() {
    const MockBox mock;
    const auto device = mock.with_device();
    device.move_rel(1, 2);
    const auto frame = mock.recorded_frame(0);
    check(frame && frame->type == FrameType::Move && !frame->payload.empty(), "the first frame");
    check(!mock.recorded_frame(99), "a frame past the end");
    check(!mock.recorded_frame(mock.recorded()), "the frame just past the end");
}

void move_frames_carry_their_riding_flags() {
    const MockBox mock;
    {
        const auto device = mock.with_device();
        device.move_rel(7, -2);
        device.move_rel_now(7, -2);
        device.wheel_now(3);
        device.flush_motion();
        device.discard_motion();
        device.move_axis(Motion::cursor(5, 5), MoveTiming::Now, PendingMotion::Flush);
    }
    const std::vector<Bytes> wanted = {
        {0, 7, 0, 0xFE, 0xFF, 0x00}, {0, 7, 0, 0xFE, 0xFF, 0x01}, {1, 3, 0, 0x01},
        {0, 0, 0, 0, 0, 0x02},       {0, 0, 0, 0, 0, 0x04},       {0, 5, 0, 5, 0, 0x03},
    };
    check(frames(mock, FrameType::Move) == wanted, "the move frames");
}

void the_other_motion_calls_send_their_frames() {
    const MockBox mock;
    {
        const auto device = mock.with_device();
        device.wheel(-2);
        device.pan(4);
        device.pan_now(-5);
        device.move_axis(Motion::wheel(6));
        device.move_axis(Motion::pan(-7));
    }
    const auto moves = frames(mock, FrameType::Move);
    check(moves.size() == 5, std::to_string(moves.size()) + " move frames");
    if (moves.size() == 5) {
        // the axis byte comes first, the delta after it in little endian, like the cursor's
        check(Bytes(moves[0].begin(), moves[0].begin() + 3) == Bytes{1, 0xFE, 0xFF}, "the wheel frame");
        check(Bytes(moves[3].begin(), moves[3].begin() + 3) == Bytes{1, 6, 0}, "the wheel motion frame");
        check(moves[1] != moves[2], "pan and pan_now sent the same");
    }
}

void caps_come_back_as_they_were_set() {
    const Caps caps{MouseCaps{5, true, true, true, true, false, 2}, KbdCaps{6, false, true, false, true}, false, true};
    const MockBox mock;
    mock.set_caps(caps);
    const auto got = mock.with_device().caps();
    check(got == caps, "the caps");
    check(got.has_mouse() && got.has_keyboard() && got.is_composite(), "what the caps say");
}

void the_rate_comes_back_with_its_native_hz() {
    const MockBox mock;
    mock.set_rate(Rate{1000, 1000, true, false});
    const auto got = mock.with_device().query_rate();
    check(got.native_period_us == 1000 && got.poll_period_us == 1000 && got.confident, "the rate");
    const auto hz = got.native_hz();
    check(hz && *hz >= 999.5f && *hz <= 1000.5f, "the native Hz");
    check(!Rate{0, 1000, false, true}.native_hz(), "an unknown native rate has a Hz");
}

void locks_come_back_and_say_whats_locked() {
    const LockTarget x = Axis::X;
    const MockBox mock;
    mock.set_locks(Locks{{LockEntry{x, false, Direction::Positive, 0}, LockEntry{x, false, Direction::Negative, 0}}});
    const auto locks = mock.with_device().query_locks();
    check(locks.entries.size() == 2, "the lock entries");
    check(locks.is_locked(x, Direction::Both), "X isn't locked");
    check(!locks.is_locked(Axis::Y, Direction::Both), "Y is locked");
}

void locks_carry_a_scale_not_just_a_lock() {
    const LockTarget x = Axis::X;
    const MockBox mock;
    mock.set_locks(Locks{{LockEntry{x, false, Direction::Against, 40}, LockEntry{x, false, Direction::With, 130}}});
    const auto locks = mock.with_device().query_locks();
    check(locks.scale_of(x, Direction::Against) == 40 && locks.scale_of(x, Direction::With) == 130, "the scales");
    check(locks.scale_of(x, Direction::Positive) == LockScalePass, "an unscaled direction");
    check(!locks.is_locked(x, Direction::Against), "a weighed direction is locked");
    check(!locks.entries.at(0).is_block(), "a weighed entry is a block");
}

void scale_and_bearing_reach_the_device() {
    const LockTarget x = Axis::X;
    const MockBox mock;
    {
        const auto device = mock.with_device();
        device.scale(x, Direction::Against, 40);
        device.scale_all(Blanket::Aim, Direction::With, 130);
        device.lock(x, Direction::Positive);
        device.unlock(x, Direction::Positive);
        device.set_bearing(35ms, BearingMode::Vector);
        device.set_bearing(std::nullopt, BearingMode::PerAxis);
    }
    // LOCK is [class][id u16 LE][direction][scale i16 LE] and OPTION(BEARING) is [4][window u16 LE][mode]
    const std::vector<Bytes> locks = {
        {3, 0, 0, 4, 40, 0}, {3, 0, 0, 3, 130, 0}, {3, 1, 0, 3, 130, 0}, {3, 0, 0, 1, 0, 0}, {3, 0, 0, 1, 100, 0},
    };
    check(frames(mock, FrameType::Lock) == locks, "the lock frames");
    check(frames(mock, FrameType::Option) == std::vector<Bytes>{{4, 35, 0, 1}, {4, 0, 0, 0}}, "the bearing frames");
}

void the_boxs_lock_table_answers_query_locks() {
    const LockTarget x = Axis::X;
    const LockTarget y = Axis::Y;
    const MockBox mock;
    const auto device = mock.with_device();
    device.scale(x, Direction::Both, 50);
    auto locks = device.query_locks();
    // both is only the fixed pair, the relative pair keeps passing
    check(locks.entries.size() == 2, "both is " + std::to_string(locks.entries.size()) + " entries");
    check(locks.scale_of(x, Direction::Positive) == 50 && locks.scale_of(x, Direction::Negative) == 50, "the fixed pair");
    check(locks.scale_of(x, Direction::With) == LockScalePass && locks.scale_of(x, Direction::Against) == LockScalePass,
          "the relative pair");
    // in vector mode one relative scale, the lower one, governs both axes
    device.unlock(x, Direction::Both);
    device.scale(x, Direction::With, 130);
    device.scale(y, Direction::With, 60);
    device.set_bearing(20ms, BearingMode::PerAxis);
    locks = device.query_locks();
    check(locks.scale_of(x, Direction::With) == 130 && locks.scale_of(y, Direction::With) == 60, "per axis");
    device.set_bearing(20ms, BearingMode::Vector);
    locks = device.query_locks();
    check(locks.scale_of(x, Direction::With) == 60 && locks.scale_of(y, Direction::With) == 60, "vector");
}

void a_key_blanket_reports_the_edges_it_blocks() {
    const MockBox mock;
    const auto device = mock.with_device();
    device.lock_all(Blanket::Keys, Direction::Positive);
    auto entries = device.query_locks().entries;
    check(entries.size() == 1 && entries[0].is_blanket && entries[0].direction == Direction::Positive &&
              entries[0].scale == 0,
          "the press blanket");
    device.lock_all(Blanket::Keys, Direction::Negative);
    entries = device.query_locks().entries;
    check(entries.size() == 2 && entries[0].direction == Direction::Positive &&
              entries[1].direction == Direction::Negative,
          "both blankets");
}

void a_one_bit_class_stores_the_block_or_pass_it_amounts_to() {
    const LockTarget left = Button::LEFT;
    const MockBox mock;
    const auto device = mock.with_device();
    device.scale(left, Direction::Positive, 50);
    check(device.query_locks().scale_of(left, Direction::Positive) == LockScaleBlock, "50 on a button");
    device.scale(left, Direction::Positive, 150);
    check(device.query_locks().entries.empty(), "150 on a button");
}

void a_media_lock_is_sent_and_reported_as_both() {
    const auto mute = LockTarget::media(MediaKey{0xE2});
    const MockBox mock;
    std::vector<LockEntry> entries;
    {
        const auto device = mock.with_device();
        device.lock(mute, Direction::Press);
        entries = device.query_locks().entries;
    }
    check(frames(mock, FrameType::Lock) == std::vector<Bytes>{{2, 0xE2, 0x00, 0, 0, 0}}, "the media lock frame");
    check(entries.size() == 1 && entries[0].direction == Direction::Both, "the media lock entry");
}

void a_relative_direction_needs_a_bearing_and_only_an_axis_has_one() {
    const MockBox mock;
    {
        const auto device = mock.with_device();
        for (const auto direction : {Direction::With, Direction::Against}) {
            throws<RelativeDirectionError>([&] { device.lock(Button::LEFT, direction); }, "a relative button lock");
            throws<RelativeDirectionError>([&] { device.scale(Key::A, direction, 40); }, "a relative key scale");
            throws<RelativeDirectionError>([&] { device.lock_all(Blanket::Keys, direction); }, "a relative key blanket");
            throws<RelativeDirectionError>([&] { device.lock_all(Blanket::Media, direction); },
                                           "a relative media blanket");
            device.scale(Axis::X, direction, 130);
        }
    }
    check(frames(mock, FrameType::Lock).size() == 2, "the relative axis scales");
}

void values_the_enums_dont_name_are_refused_before_they_cross() {
    const LockTarget x = Axis::X;
    const MockBox mock;
    {
        const auto device = mock.with_device();
        throws<std::invalid_argument>([&] { device.scale(x, static_cast<Direction>(40), 50); }, "direction 40");
        throws<std::invalid_argument>([&] { device.lock_all(static_cast<Blanket>(99), Direction::Both); }, "blanket 99");
        throws<std::invalid_argument>([&] { device.set_bearing(20ms, static_cast<BearingMode>(7)); }, "bearing mode 7");
        throws<std::invalid_argument>([&] { device.set_render(static_cast<RenderMode>(4), false); }, "render mode 4");
        // the window saturates instead of wrapping, like the Rust API, 70000 would be 4464 through a u16
        device.set_bearing(70000ms, BearingMode::Vector);
        // and a zero one is 1 ms like there, 0 on the wire would turn it off
        device.set_bearing(0ms, BearingMode::Vector);
    }
    check(frames(mock, FrameType::Option) == std::vector<Bytes>{{4, 0xFF, 0xFF, 1}, {4, 1, 0, 1}},
          "the saturated and the zero bearing");
    check(frames(mock, FrameType::Lock).empty(), "a refused lock went out");
}

void the_mocks_locks_are_checked_entry_by_entry() {
    const LockTarget x = Axis::X;
    const MockBox mock;
    throws<std::invalid_argument>(
        [&] { mock.set_locks(Locks{{LockEntry{x, false, static_cast<Direction>(40), 0}}}); }, "a mock lock on 40");
    const Locks good{{LockEntry{x, false, Direction::Positive, 0}}};
    throws<std::invalid_argument>([&] { good.is_locked(x, static_cast<Direction>(40)); }, "is_locked on 40");
    throws<std::invalid_argument>([&] { good.scale_of(x, static_cast<Direction>(40)); }, "scale_of on 40");
    mock.set_locks(good);
    check(mock.with_device().query_locks().is_locked(x, Direction::Positive), "the good lock");
}

void a_catch_filters_values_are_checked() {
    const auto filter = CatchFilter::watch(Button::LEFT);
    throws<std::invalid_argument>([&] { filter.with_direction(static_cast<Direction>(40)); }, "direction 40");
    throws<std::invalid_argument>([] { CatchFilter::watch_axis(static_cast<Axis>(9)); }, "axis 9");
    throws<std::invalid_argument>([] { CatchFilter::watch_class(static_cast<Class>(9)); }, "class 9");
    throws<std::invalid_argument>([] { CatchFilter::traffic_of(static_cast<TrafficClass>(99)); }, "traffic class 99");
    check(filter.with_direction(Direction::Press).direction() == Direction::Positive, "press is positive");
    check(filter.with_capture(16).capture() == 16, "the capture");
}

void the_mocks_bearing_answers_query_bearing() {
    const MockBox mock;
    const auto device = mock.with_device();
    check(device.query_bearing() ==
              Bearing{static_cast<std::uint16_t>(BearingWindowDefaultMs), BearingMode::PerAxis},
          "the default bearing");
    mock.set_bearing(Bearing{std::uint16_t{250}, BearingMode::Vector});
    check(device.query_bearing() == Bearing{std::uint16_t{250}, BearingMode::Vector}, "the set bearing");
    mock.set_bearing(Bearing{std::nullopt, BearingMode::PerAxis});
    const auto off = device.query_bearing();
    check(!off.window_ms && !off.is_live(), "the bearing that's off");
}

// the medius crate's own Default, read in ref/medius
void the_defaults_are_the_crates_own() {
    const RewriteRule rule;
    check(rule.class_ == RewriteClass::Emit && rule.direction == Direction::Both && rule.action == RewriteAction::Pass,
          "the rewrite rule's default");
    const Bearing bearing;
    check(bearing.window_ms == static_cast<std::uint16_t>(BearingWindowDefaultMs) &&
              bearing.mode == BearingMode::PerAxis,
          "the bearing's default");
    check(RenderStatus{}.mode == RenderMode::Despiked, "the render mode's default");
    check(Patch{}.section == PatchSection::Device, "the patch section's default");
    check(DeviceInfo{}.kind == DeviceKind::Unknown, "the device kind's default");
    check(ClipStatus{}.state == ClipState::Idle, "the clip state's default");
}

void relative_directions_say_so() {
    check(is_relative(Direction::With) && is_relative(Direction::Against), "the relative directions");
    check(!is_relative(Direction::Both) && !is_relative(Direction::Positive), "the fixed directions");
    check(static_cast<int>(Direction::With) == 3 && static_cast<int>(Direction::Against) == 4, "their numbers");
    check(LockScaleBlock == 0 && LockScalePass == 100 && LockScaleMax == 255 && LockScaleMin == -255, "the scales");
}

void health_comes_back() {
    const Health health{true, false, true, false, true, false, true, false, false, false, false};
    const MockBox mock;
    mock.set_health(health);
    check(mock.with_device().query_health() == health, "the health");
}

void device_info_comes_back() {
    const DeviceInfo info{0x046D, 0xC08B, 0x0111, 0x0200, true, false, DeviceKind::Mouse, "Logitech G502"};
    const MockBox mock;
    mock.set_device_info(info);
    const auto got = mock.with_device().device_info();
    check(got == info, "the device info");
    check(got.is_cloned(), "a cloned mouse isn't cloned");
}

void a_clone_with_no_hid_interface_is_cloned() {
    const DeviceInfo pad{0x045E, 0x028E, 0x0114, 0x0200, true, false, DeviceKind::Unknown, "Controller"};
    const MockBox mock;
    mock.set_device_info(pad);
    const auto got = mock.with_device().device_info();
    check(got.kind == DeviceKind::Unknown && got.is_cloned(), "the controller");
    check(!DeviceInfo{0, 0, 0, 0, false, false, DeviceKind::Unknown, ""}.is_cloned(), "no device is cloned");
}

void stats_come_back() {
    const Stats stats{123456, 12, 34, 7, 2, 900, 3, 4, 0xDEADBEEF, 0, 0x0A0B0C0D, 0xBEEF};
    const MockBox mock;
    mock.set_stats(stats);
    check(mock.with_device().query_stats() == stats, "the stats");
}

CatchState query_catch(const CatchState& state) {
    const MockBox mock;
    mock.set_catch_state(state);
    return mock.with_device().query_catch();
}

void the_catch_state_comes_back() {
    const CatchState state{true, 42, ClockEstimate{-1234, 57, 90, 1500u},
                           {CatchEntry{CatchFilter::everything().with_capture(16), 3},
                            CatchEntry{CatchFilter::traffic(TrafficClass::VendorBulk, 3).with_direction(Direction::Positive),
                                       7}}};
    const auto got = query_catch(state);
    check(got.table_full == state.table_full && got.dropped == state.dropped && got.clock == state.clock,
          "the table and clock");
    check(got.entries == state.entries, "the entries");
    check(got.clock.error_bound_us() == 45, "the error bound");
    check(got.entries.at(1).filter.class_() == CatchClass::VendorBulk && got.entries.at(1).filter.id() == 3,
          "the bulk entry");
}

void a_never_measured_clock_isnt_a_zero_age() {
    const auto never = query_catch(CatchState{false, 0, ClockEstimate{500, 0, 0, std::nullopt}, {}});
    const auto fresh = query_catch(CatchState{false, 0, ClockEstimate{500, 0, 0, 0u}, {}});
    check(!never.clock.age_ms && fresh.clock.age_ms == 0u, "the ages");
    check(never.clock != fresh.clock, "never measured is a zero age");
    check(!never.clock.to_host_domain(1000), "a never measured clock moved a stamp");
    check(fresh.clock.to_host_domain(1000) == 1500, "the moved stamp");
}

void catch_filter_wildcards_survive_the_round_trip() {
    const auto every = CatchFilter::everything();
    const auto blanket = CatchFilter::traffic_of(TrafficClass::HidIn);
    const auto exact = CatchFilter::traffic(TrafficClass::Control, 0);
    check(!every.class_() && !every.id(), "everything");
    check(blanket.class_() == CatchClass::HidIn && !blanket.id(), "the blanket");
    check(exact.class_() == CatchClass::Control && exact.id() == 0, "the exact one");
    const auto state =
        query_catch(CatchState{false, 0, ClockEstimate{}, {CatchEntry{every, 0}, CatchEntry{blanket, 0}, CatchEntry{exact, 0}}});
    std::vector<CatchFilter> filters;
    for (const auto& entry : state.entries) {
        filters.push_back(entry.filter);
    }
    check(filters == std::vector<CatchFilter>{every, blanket, exact}, "the filters read back");
}

void imperfect_status_comes_back() {
    const ImperfectStatus status{true, true, false};
    const MockBox mock;
    mock.set_imperfect_status(status);
    check(mock.with_device().query_imperfect() == status, "the imperfect status");
}

void movement_riding_comes_back() {
    const MockBox mock;
    mock.set_movement_riding(8ms);
    check(mock.with_device().query_movement_riding() == 8ms, "the riding window");
    mock.set_movement_riding(std::nullopt);
    check(!mock.with_device().query_movement_riding(), "riding that's off");
}

void the_emit_pace_comes_back() {
    const MockBox mock;
    mock.set_emit_pace(EmitPace::fixed(500));
    {
        const auto status = mock.with_device().query_emit_pace();
        check(status.mode == EmitPace::fixed(500) && status.resolved_hz == 500, "the fixed pace");
    }
    mock.set_emit_pace(EmitPace::learned());
    const auto learned = mock.with_device().query_emit_pace();
    check(learned.mode == EmitPace::learned() && learned.resolved_hz == 0, "the learned pace");
    // armed, the rendered stream paces itself every millisecond, and disarming puts it back
    mock.set_render(RenderMode::Despiked, false, true);
    check(mock.with_device().query_emit_pace().resolved_hz == 1000, "the armed pace");
    mock.set_render(RenderMode::Despiked, false, false);
    check(mock.with_device().query_emit_pace().resolved_hz == 0, "the disarmed pace");
    check(!learned.force_hz && !learned.force_active && learned.advertised_hz == 0, "nothing forced");
}

void the_spread_comes_back() {
    const MockBox mock;
    const auto device = mock.with_device();
    check(device.query_spread() == SpreadStatus{100, 0}, "the default spread");
    mock.set_spread_learned(8000);
    check(device.query_spread() == SpreadStatus{100, 8000}, "the learned spread");
    device.set_spread(50);
    check(device.query_spread() == SpreadStatus{50, 4000}, "half the spread");
    // above 100 overlaps instead of being clamped, and it's past a byte
    device.set_spread(1000);
    check(device.query_spread() == SpreadStatus{1000, 80000}, "ten times the spread");
    device.set_spread(0);
    check(device.query_spread() == SpreadStatus{0, 0}, "no spread");
}

void the_render_mode_comes_back() {
    {
        const MockBox mock;
        const auto device = mock.with_device();
        check(device.query_render() == RenderStatus{RenderMode::Despiked, false, false}, "the default render");
        for (const auto mode : {RenderMode::Off, RenderMode::Stock, RenderMode::Despiked, RenderMode::Unsmoothed}) {
            for (const bool full : {false, true}) {
                device.set_render(mode, full);
                const auto status = device.query_render();
                check(status.mode == mode && status.full == full, "a render mode");
            }
        }
        device.set_render(RenderMode::Stock, false);
        device.set_emit_pace(EmitPace::learned());
        check(device.query_emit_pace().resolved_hz == 0, "stock and learned");
        device.set_emit_pace(EmitPace::fixed(250));
        check(device.query_emit_pace().resolved_hz == 250, "stock and fixed");
    }
    const MockBox mock;
    const auto device = mock.with_device();
    mock.set_render(RenderMode::Stock, true, true);
    check(device.query_render() == RenderStatus{RenderMode::Stock, true, true}, "the mock's render");
    device.set_emit_pace(EmitPace::learned());
    check(device.query_emit_pace().resolved_hz == 1000, "the armed render's pace");
}

void the_status_codes_are_the_headers() {
    const std::string prefix = "    MEDIUS_STATUS_";
    int declared = 0;
    for (const auto& line : header_lines()) {
        const auto equals = line.find(" = ");
        if (line.rfind(prefix, 0) != 0 || equals == std::string::npos || line.back() != ',') {
            continue;
        }
        declared++;
        std::string name;
        for (const char letter : line.substr(prefix.size(), equals - prefix.size())) {
            if (letter != '_') {
                name += static_cast<char>(std::tolower(static_cast<unsigned char>(letter)));
            }
        }
        const auto value = std::stoi(line.substr(equals + 3));
        // with no text an error says its status' name, so this holds every name to the header
        const auto error = detail::make_error(static_cast<MediusStatus>(value), "", 0, std::nullopt);
        std::string said = error->what();
        std::transform(said.begin(), said.end(), said.begin(),
                       [](unsigned char letter) { return static_cast<char>(std::tolower(letter)); });
        check(error->medius_status() == static_cast<Status>(value) && said == name,
              line + " is " + error->what() + " here");
    }
    check(declared == 36, std::to_string(declared) + " statuses in medius.h");
    check(static_cast<int>(Status::ErrClipPacketTrigger) == 33, "ErrClipPacketTrigger");
}

void every_status_has_its_exception() {
    const std::vector<std::pair<Status, const std::type_info*>> classes = {
        {Status::ErrIo, &typeid(IoError)},
        {Status::ErrNotFound, &typeid(NotFoundError)},
        {Status::ErrNoReply, &typeid(NoReplyError)},
        {Status::ErrBadProtoVer, &typeid(BadProtoVerError)},
        {Status::ErrQueryTimeout, &typeid(QueryTimeoutError)},
        {Status::ErrDisconnected, &typeid(DisconnectedError)},
        {Status::ErrFrameTooLong, &typeid(FrameTooLongError)},
        {Status::ErrUpdate, &typeid(UpdateError)},
        {Status::ErrInvalidArg, &typeid(InvalidArgError)},
        {Status::ErrPanic, &typeid(PanicError)},
        {Status::ErrCatchTableFull, &typeid(CatchTableFullError)},
        {Status::ErrEmptySubscription, &typeid(EmptySubscriptionError)},
        {Status::ErrCaptureNotApplicable, &typeid(CaptureNotApplicableError)},
        {Status::ErrNotAnInputFilter, &typeid(NotAnInputFilterError)},
        {Status::ErrWildcardNotInput, &typeid(WildcardNotInputError)},
        {Status::ErrHalfEdgeInputFilter, &typeid(HalfEdgeInputFilterError)},
        {Status::ErrReservedId, &typeid(ReservedIdError)},
        {Status::ErrRelativeDirection, &typeid(RelativeDirectionError)},
        {Status::ErrLockScaleRange, &typeid(LockScaleRangeError)},
        {Status::ErrLockScaleUsage, &typeid(LockScaleUsageError)},
        {Status::ErrImperfectRequired, &typeid(ImperfectRequiredError)},
        {Status::ErrRewriteMaskLength, &typeid(RewriteMaskLengthError)},
        {Status::ErrRewriteActionClass, &typeid(RewriteActionClassError)},
        {Status::ErrRewritePayloadTooLarge, &typeid(RewritePayloadTooLargeError)},
        {Status::ErrRewriteTableFull, &typeid(RewriteTableFullError)},
        {Status::ErrTransformOpFields, &typeid(TransformOpFieldsError)},
        {Status::ErrTransformTableFull, &typeid(TransformTableFullError)},
        {Status::ErrRawDirection, &typeid(RawDirectionError)},
        {Status::ErrClipFrameCount, &typeid(ClipFrameCountError)},
        {Status::ErrClipFrameTooLong, &typeid(ClipFrameTooLongError)},
        {Status::ErrClipTransferData, &typeid(ClipTransferDataError)},
        {Status::ErrClipPacketTrigger, &typeid(ClipPacketTriggerError)},
        {Status::ErrRewriteMatchTooLong, &typeid(RewriteMatchTooLongError)},
        {Status::ErrRewritePoolFull, &typeid(RewritePoolFullError)},
    };
    check(classes.size() == 34, "every status but ok and unknown");
    for (const auto& entry : classes) {
        const auto status = static_cast<MediusStatus>(entry.first);
        const auto error = detail::make_error(status, "text", 3, std::nullopt);
        check(typeid(*error) == *entry.second, std::string("status ") + std::to_string(status) + " is " +
                                                   typeid(*error).name());
        check(error->medius_status() == entry.first && error->proto_ver() == 3 && std::string(error->what()) == "text",
              "status " + std::to_string(status) + "'s fields");
        check(dynamic_cast<const anybox::Error*>(error.get()) != nullptr, "an error that isn't an anybox one");
    }
    const auto unknown = detail::make_error(MEDIUS_STATUS_ERR_UNKNOWN, "", 0, std::nullopt);
    check(typeid(*unknown) == typeid(Error), std::string("unknown is ") + typeid(*unknown).name());
    check(std::string(unknown->what()) == "ErrUnknown", std::string("unknown says ") + unknown->what());
    check(typeid(*detail::make_error(static_cast<MediusStatus>(99), "x", 0, std::nullopt)) == typeid(Error),
          "status 99");
}

void input_events_turn_snapshots_into_edges() {
    const Usage escape = Key::ESCAPE;
    const Usage a = Key::A;
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.input_events(CatchFilter::all_input());
    mock.push_usages(1, 1000, UsageSnapshot{Class::Key, Direction::Press, {escape}});
    mock.push_usages(2, 2000, UsageSnapshot{Class::Key, Direction::Press, {escape, a}});
    mock.push_usages(3, 3000, UsageSnapshot{Class::Key, Direction::Release, {a}});
    mock.push_motion(4, 4000, MotionEvent{3, -4, 0, 0});
    const auto first = must(stream.recv(wait), "the first press");
    check(first.kind == InputKind::Press && first.usage == escape && first.ts_us == 1000u, "the first press");
    check(first.dx == 0 && first.dy == 0 && first.dz == 0, "a press has motion");
    check(must(stream.recv(wait), "the second press").usage == a, "the second press");
    const auto release = must(stream.recv(wait), "the release");
    check(release.kind == InputKind::Release && release.usage == escape, "the release");
    check(stream.held(Class::Key) == std::vector<Usage>{a}, "the held keys");
    const auto motion = must(stream.recv(wait), "the motion");
    check(motion.kind == InputKind::Motion && motion.dx == 3 && motion.dy == -4 && motion.dz == 0, "the motion");
    check(!motion.usage, "motion has a usage");
    check(!stream.try_recv(), "something more came");
}

void input_events_refuse_what_they_cant_decode() {
    const MockBox mock;
    const auto device = mock.with_device();
    throws<NotAnInputFilterError>([&] { device.input_events({CatchFilter::traffic_of(TrafficClass::VendorBulk)}); },
                                  "a traffic filter");
    throws<WildcardNotInputError>([&] { device.input_events({CatchFilter::everything()}); }, "everything");
    throws<HalfEdgeInputFilterError>([&] { device.input_events({CatchFilter::watch(Key::A).on_press()}); },
                                     "a half edge");
    throws<CaptureNotApplicableError>(
        [&] { device.catch_events({CatchFilter::watch_class(Class::Key).with_capture(8)}); }, "a key capture");
    // across this ABI a media usage of 0xFFFF is the class blanket
    check(!CatchFilter::watch(Usage::media(MediaKey{0xFFFF})).id(), "media 0xFFFF has an id");
    check(CatchFilter::watch_class(Class::Media) == CatchFilter::watch(Usage::media(MediaKey{0xFFFF})),
          "media 0xFFFF");
    const auto stream = device.input_events(CatchFilter::all_input());
    check(stream.dropped() == 0, "a new stream dropped");
}

void the_filter_factories_address_inputs_like_lock_does() {
    check(CatchFilter::watch(Usage::key(Key::A)) == CatchFilter::watch(Key::A), "a key");
    check(CatchFilter::watch(Usage::button(0)) == CatchFilter::watch(Button::LEFT), "a button");
    check(CatchFilter::watch(Usage::media(MediaKey::VOLUME_UP)) == CatchFilter::watch(MediaKey::VOLUME_UP), "media");
    const auto key = CatchFilter::watch(Key::A);
    check(key.class_() == CatchClass::Key && key.id() == Key::A.value, "the key's address");
    const auto left = CatchFilter::watch(Button::LEFT);
    check(left.class_() == CatchClass::Button && left.id() == 0, "the button's address");
    check(CatchFilter::watch_axis(Axis::Wheel).id() == static_cast<std::uint16_t>(Axis::Wheel), "the wheel's id");
    check(!CatchFilter::watch_axes().id(), "every axis has an id");
    std::vector<std::optional<CatchClass>> classes;
    for (const auto& filter : CatchFilter::all_input()) {
        classes.push_back(filter.class_());
    }
    check(classes == std::vector<std::optional<CatchClass>>{CatchClass::Button, CatchClass::Key, CatchClass::Media,
                                                            CatchClass::Axis},
          "all input");
    const auto bulk = CatchFilter::traffic(TrafficClass::VendorBulk, 3);
    check(bulk.same_address(bulk.with_capture(16)), "a capture moved the address");
    check(!bulk.same_address(bulk.outbound()), "outbound is the same address");
    check(bulk != bulk.with_capture(16), "a capture is the same filter");
    check(CatchFilter::everything().capture() == Capture::Whole, "everything's capture");
    check(is_input(CatchClass::Key) && is_traffic(CatchClass::VendorBulk), "the class kinds");
}

void an_unknown_control_status_doesnt_throw() {
    const auto unknown = traffic(CatchClass::Control, 0, Direction::In, 0x03, 8, Bytes(8));
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::traffic_of(TrafficClass::Control)});
    mock.push_traffic(1, 7000, ClockDomain::DeviceChip, unknown);
    const auto event = must(stream.recv(wait), "the control event");
    check(event.traffic() && event.traffic()->control_status() == ControlStatus::Other &&
              event.traffic()->flags == 0x03,
          "the unknown control status");
}

void a_timeline_unwraps_the_rollover_onto_your_clock() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::watch_axes()});
    Timeline timeline;
    mock.push_motion(1, 0xFFFFFE0C, MotionEvent{1, 0, 0, 0});
    mock.push_motion(2, 500, MotionEvent{1, 0, 0, 0});
    const auto a = timeline.observe(must(stream.recv(wait), "the first motion"), 50000000);
    const auto b = timeline.observe(must(stream.recv(wait), "the second motion"), 51001000);
    check(a.box_us == 0xFFFFFE0Cull && b.box_us == (1ull << 32) + 500, "the box stamps");
    check(b.host_ns - a.host_ns == 1000000u && b.excess_ns == 1000u, "the host stamps");
    const Timeline& reading = timeline;
    check(reading.samples(ClockDomain::HostChip) == 2, "the samples");
    timeline.reset(ClockDomain::HostChip);
    check(timeline.samples(ClockDomain::HostChip) == 0, "the samples after a reset");
}

void a_timeline_places_input_events_too() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.input_events({CatchFilter::watch_axes()});
    Timeline timeline;
    mock.push_motion(1, 1000, MotionEvent{1, 0, 0, 0});
    const auto stamped = timeline.observe(must(stream.recv(wait), "the motion"));
    check(stamped.box_us == 1000 && timeline.samples(ClockDomain::HostChip) == 1, "the input event's stamp");
}

void every_enum_going_in_is_checked_before_it_crosses() {
    const MockBox mock;
    {
        const auto device = mock.with_device();
        throws<std::invalid_argument>([&] { device.led(LedTarget::Device, static_cast<LedMode>(77), 5); }, "LED 77");
        throws<std::invalid_argument>([&] { device.led(static_cast<LedTarget>(200), LedMode::Solid, 55); },
                                      "LED target 200");
        throws<std::invalid_argument>([&] { device.reboot(static_cast<RebootTarget>(200)); }, "reboot 200");
        throws<std::invalid_argument>([&] { device.inject(Button::LEFT, static_cast<Action>(200)); }, "action 200");
        // an EmitPace only comes from its three makers, so there's no unnamed mode to send
        throws<std::invalid_argument>([&] { device.move_axis(Motion::cursor(1, 2), static_cast<MoveTiming>(9)); },
                                      "timing 9");
        throws<std::invalid_argument>(
            [&] { device.move_axis(Motion::cursor(1, 2), MoveTiming::Ride, static_cast<PendingMotion>(9)); },
            "pending 9");
        check(mock.recorded() == 0, "a refused value went out");
        device.led(LedTarget::Both, LedMode::Blink, 128);
    }
    check(frames(mock, FrameType::Led) == std::vector<Bytes>{{2, 3, 128}}, "the LED frame");
}

void the_mock_and_stream_enums_are_checked() {
    const MockBox mock;
    const auto device = mock.with_device();
    throws<std::invalid_argument>([&] { mock.push_log(static_cast<LogLevel>(200), "hi"); }, "log level 200");
    throws<std::invalid_argument>([&] { mock.saw(static_cast<FrameType>(200)); }, "frame type 200");
    throws<std::invalid_argument>(
        [&] { mock.push_traffic(0, 0, static_cast<ClockDomain>(200), traffic(CatchClass::HidIn, 1, Direction::In)); },
        "clock 200");
    throws<std::invalid_argument>(
        [&] { mock.set_device_info(DeviceInfo{1, 2, 3, 4, false, false, static_cast<DeviceKind>(200), ""}); },
        "device kind 200");
    throws<std::invalid_argument>(
        [&] { mock.set_clip_status(ClipStatus{static_cast<ClipState>(200), 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, {}}); },
        "clip state 200");
    throws<std::invalid_argument>(
        [&] { mock.push_usages(0, 0, UsageSnapshot{static_cast<Class>(200), Direction::Positive, {}}); }, "class 200");
    {
        Timeline timeline;
        throws<std::invalid_argument>([&] { timeline.reset(static_cast<ClockDomain>(200)); }, "a reset on 200");
        throws<std::invalid_argument>([&] { timeline.samples(static_cast<ClockDomain>(200)); }, "samples of 200");
    }
    const auto stream = device.input_events(CatchFilter::all_input());
    throws<std::invalid_argument>([&] { stream.held(static_cast<Class>(200)); }, "held of 200");
}

void the_new_health_bits_come_back() {
    const Health health{true, false, false, false, false, false, false, false, true, false, true};
    const MockBox mock;
    mock.set_health(health);
    check(mock.with_device().query_health() == health, "the health");
}

void the_advanced_frames_carry_their_type() {
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    {
        const auto device = mock.with_device();
        device.raw(1, Direction::In, {0, 1, 2, 3});
        device.set_rewrite(rewrite(RewriteClass::Emit, 1, Direction::In, RewriteAction::Drop));
        device.set_patch(Patch{PatchSection::Device, 0, 0, 8, {0x34, 0x12}});
        device.transfer(0, Setup{0x80, 0x06, 0x0100, 0, 18});
    }
    check(mock.saw(FrameType::Raw) && mock.saw(FrameType::Rewrite) && mock.saw(FrameType::Patch) &&
              mock.saw(FrameType::Transfer),
          "the advanced frames");
}

void raw_goes_out_as_it_is() {
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    mock.with_device().raw(1, Direction::In, {0x00, 0x01, 0x00, 0x00});
    // [ep][dir][bytes]
    check(frames(mock, FrameType::Raw) == std::vector<Bytes>{{0x01, 0x01, 0x00, 0x01, 0x00, 0x00}}, "the raw frame");
}

void the_gated_calls_need_the_opt_in() {
    const MockBox mock;
    const auto device = mock.with_device();
    throws<ImperfectRequiredError>(
        [&] { device.set_rewrite(rewrite(RewriteClass::Emit, 1, Direction::In, RewriteAction::Drop)); }, "a rewrite");
    throws<ImperfectRequiredError>([&] { device.apply_patch(); }, "an apply");
    // raw doesn't read the opt in, the box drops it there
    device.raw(1, Direction::In, {0x00, 0x01});
    check(mock.saw(FrameType::Raw), "the raw frame");
}

void raw_refuses_a_direction_that_isnt_a_flow() {
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    const auto device = mock.with_device();
    throws<RawDirectionError>([&] { device.raw(1, Direction::Both, {0}); }, "raw both ways");
    throws<RelativeDirectionError>([&] { device.raw(1, Direction::With, {0}); }, "raw with the bearing");
}

void a_transfer_brings_back_its_answer() {
    const Bytes reply{0x12, 0x01, 0x10, 0x02};
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    mock.set_transfer_reply(TransferStatus::Ok, reply);
    const auto device = mock.with_device();
    const auto outcome = device.transfer(0, Setup{0x80, 0x06, 0x0100, 0, 18});
    check(outcome == TransferOutcome{TransferStatus::Ok, reply} && outcome.is_ok(), "the reply");
    const auto timed = device.transfer(0, Setup{0x80, 0x06, 0x0100, 0, 2}, {},
                                       std::chrono::milliseconds(default_transfer_timeout_ms()));
    check(timed.status == TransferStatus::Ok, "the timed reply");
    check(default_transfer_timeout_ms() >= 800, "the default transfer timeout");
}

void a_transfer_is_refused_without_the_opt_in() {
    const MockBox mock;
    const auto outcome = mock.with_device().transfer(0, Setup{0x80, 0x06, 0x0100, 0, 18});
    check(outcome.status == TransferStatus::Refused && !outcome.is_ok() && outcome.data.empty(), "the refusal");
}

void a_transfer_that_isnt_ok_carries_no_data() {
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    mock.set_transfer_reply(TransferStatus::Stall, {0x12, 0x01, 0x00, 0x02});
    const auto outcome = mock.with_device().transfer(0, Setup{0x80, 0x06, 0x0100, 0, 18});
    check(outcome.status == TransferStatus::Stall && outcome.data.empty(), "the stall");
}

void a_rewrite_survives_the_query() {
    const auto rule = rewrite(RewriteClass::Control, 0, Direction::Both, RewriteAction::ReplyPatch, 258, {0x80, 0x06},
                              {0xFF, 0xFF}, Bytes(40, 0xAB));
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    const auto device = mock.with_device();
    device.set_rewrite(rule);
    const auto entries = device.query_rewrite().entries;
    check(entries.size() == 1, std::to_string(entries.size()) + " rewrite entries");
    if (entries.size() == 1) {
        check(entries[0].class_ == RewriteClass::Control && entries[0].action == RewriteAction::ReplyPatch &&
                  entries[0].offset == 258 && entries[0].payload_len == 40,
              "the entry");
    }
    check(device.query_rewrite_entry(0) == rule, "the rule read back");
    device.clear_rewrite();
    check(device.query_rewrite().entries.empty(), "the cleared table");
}

void a_rewrites_refusals_have_their_own_exception() {
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    const auto device = mock.with_device();
    throws<RewriteMaskLengthError>(
        [&] { device.set_rewrite(rewrite(RewriteClass::Emit, 1, Direction::In, RewriteAction::Drop, 0, {1, 2}, {0xFF})); },
        "a short mask");
    throws<RewriteActionClassError>(
        [&] { device.set_rewrite(rewrite(RewriteClass::Control, 0, Direction::Both, RewriteAction::Drop)); },
        "a drop on control");
    throws<std::invalid_argument>(
        [&] { device.set_rewrite(rewrite(RewriteClass::HidIn, 2, Direction::In, static_cast<RewriteAction>(9))); },
        "action 9");
    throws<RelativeDirectionError>(
        [&] { device.set_rewrite(rewrite(RewriteClass::Emit, 1, Direction::With, RewriteAction::Drop)); },
        "a relative rewrite");
    throws<RewritePayloadTooLargeError>(
        [&] { device.set_rewrite(rewrite(RewriteClass::Emit, 1, Direction::In, RewriteAction::Replace, 0, {}, {}, Bytes(100))); },
        "a long replace");
    // the struct holds 512 payload bytes, a longer one is refused instead of cut
    throws<std::invalid_argument>(
        [&] { device.set_rewrite(rewrite(RewriteClass::Control, 0, Direction::Both, RewriteAction::Answer, 0, {}, {}, Bytes(513))); },
        "a 513 byte answer");
    throws<std::invalid_argument>([&] { device.set_patch(Patch{PatchSection::Device, 0, 0, 0, Bytes(513)}); },
                                  "a 513 byte patch");
    // a removal only reads the key, so its payload can be anything, like in Rust
    device.remove_rewrite(rewrite(RewriteClass::Control, 0, Direction::Both, RewriteAction::Answer, 0, {}, {}, Bytes(513)));
}

RewriteRule match_rule(std::size_t match, std::size_t mask) {
    return rewrite(RewriteClass::HidIn, 2, Direction::In, RewriteAction::Pass, 0, Bytes(match, 0x11), Bytes(mask, 0xFF));
}

void a_rewrite_match_past_the_limit_has_its_own_exception() {
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    const auto device = mock.with_device();
    for (const std::size_t length : {17, 40, 0xFFFF}) {
        try {
            device.set_rewrite(match_rule(length, length));
            check(false, "a match of " + std::to_string(length) + " went in");
        } catch (const RewriteMatchTooLongError& error) {
            check(error.medius_status() == Status::ErrRewriteMatchTooLong, "the status");
        }
        throws<RewriteMatchTooLongError>([&] { device.remove_rewrite(match_rule(length, length)); },
                                         "removing a match of " + std::to_string(length));
    }
    throws<RewriteMaskLengthError>([&] { device.set_rewrite(match_rule(17, 16)); }, "17 and 16");
    throws<RewriteMaskLengthError>([&] { device.set_rewrite(match_rule(16, 17)); }, "16 and 17");
    check(device.query_rewrite().entries.empty(), "a refused rule is in the table");
    device.set_rewrite(match_rule(16, 16));
    check(device.query_rewrite_entry(0) == match_rule(16, 16), "the longest match");
}

void a_rewrite_past_the_payload_pool_has_its_own_exception() {
    const auto answer = [](std::uint16_t id, std::size_t length) {
        return rewrite(RewriteClass::Control, id, Direction::Both, RewriteAction::Answer, 0, {}, {}, Bytes(length, 0x5A));
    };
    const MockBox mock;
    const auto device = mock.with_device();
    mock.set_imperfect_status(allowed);
    for (std::uint16_t id = 0; id < 4; id++) {
        device.set_rewrite(answer(id, 501));
    }
    throws<RewritePoolFullError>([&] { device.set_rewrite(answer(4, 45)); }, "a payload past the pool");
    device.set_rewrite(answer(4, 44));
    check(device.query_rewrite().entries.size() == 5, "the pool");
}

void a_patch_survives_the_query() {
    const Bytes data(40, 0xCD);
    const MockBox mock;
    const auto device = mock.with_device();
    device.set_patch(Patch{PatchSection::Report, 1, 2, 258, data});
    const auto entries = device.query_patches().entries;
    check(entries == std::vector<PatchEntry>{PatchEntry{PatchSection::Report, 1, 2, 258, 40}}, "the patch entry");
    check(device.query_patch_entry(0) == Patch{PatchSection::Report, 1, 2, 258, data}, "the patch read back");
}

void apply_and_clear_patch_reach_the_box() {
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    const auto device = mock.with_device();
    device.set_patch(Patch{PatchSection::Device, 0, 0, 8, {0x34, 0x12}});
    check(device.query_patches().pending, "the patch isn't pending");
    device.apply_patch();
    check(device.query_patches().applied, "the patch isn't applied");
    device.clear_patch();
    check(device.query_patches().entries.empty(), "the cleared patches");
}

void an_empty_patch_removes_the_stored_one() {
    const MockBox mock;
    const auto device = mock.with_device();
    device.set_patch(Patch{PatchSection::Device, 0, 0, 8, {0x34, 0x12}});
    check(device.query_patches().entries.size() == 1, "the stored patch");
    device.set_patch(Patch{PatchSection::Device, 0, 0, 8, {}});
    check(device.query_patches().entries.empty(), "the removed patch");
}

void pan_frames_carry_the_motion_tag() {
    const MockBox mock;
    {
        const auto device = mock.with_device();
        device.pan(3);
        device.pan_now(-2);
        device.move_axis(Motion::pan(4), MoveTiming::Now, PendingMotion::Keep);
    }
    // [motion=2][dpan i16 LE][flags], ride and keep is 0, now is 1
    check(frames(mock, FrameType::Move) == std::vector<Bytes>{{2, 3, 0, 0x00}, {2, 0xFE, 0xFF, 0x01}, {2, 4, 0, 0x01}},
          "the pan frames");
}

void buttons_past_five_go_by_their_id() {
    const auto usage = Usage::button(7);
    check(usage.kind == Class::Button && usage.id == 7, "button 7");
    const MockBox mock;
    {
        const auto device = mock.with_device();
        device.press(Usage::button(7));
        device.lock(LockTarget::button(9), Direction::Press);
    }
    // [class=0][id u16 LE][action=press]
    const auto inject = mock.recorded_frame(0);
    check(inject && inject->type == FrameType::Inject && inject->payload == Bytes{0, 7, 0, 1}, "the inject frame");
}

void the_transform_calls_reach_the_box_without_the_opt_in() {
    const MockBox mock;
    {
        const auto device = mock.with_device();
        device.transform_swap(Axis::X, Axis::Y);
        device.transform_remap(Axis::X, Axis::Wheel);
        device.transform(Transform::remap(Axis::Wheel, Axis::Y));
        device.untransform(Transform::remap(Axis::Wheel, Axis::Y));
        device.clear_transforms();
    }
    check(frames(mock, FrameType::Transform).size() == 5, "the transform frames");
}

void a_transform_survives_the_query() {
    const MockBox mock;
    const auto device = mock.with_device();
    device.transform_swap(Axis::X, Axis::Y);
    device.transform_remap(Axis::Wheel, Axis::Y);
    device.transform_remap(Axis::Y, Axis::X);
    const auto transforms = device.query_transforms();
    check(!transforms.table_full, "the transform table is full");
    std::vector<std::tuple<TransformOp, LockTargetKind, LockTargetKind>> entries;
    for (const auto& entry : transforms.entries) {
        entries.emplace_back(entry.op, entry.source.kind(), entry.dest.kind());
    }
    const std::vector<std::tuple<TransformOp, LockTargetKind, LockTargetKind>> wanted = {
        {TransformOp::Swap, LockTargetKind::X, LockTargetKind::Y},
        {TransformOp::Remap, LockTargetKind::Wheel, LockTargetKind::Y},
        {TransformOp::Remap, LockTargetKind::Y, LockTargetKind::X},
    };
    check(entries == wanted, "the transforms");
    check(static_cast<int>(TransformOp::Remap) == 0 && static_cast<int>(TransformOp::Swap) == 1, "the ops' numbers");
}

void a_pan_axis_transform_works_like_any_other() {
    const MockBox mock;
    mock.set_mouse_caps(MouseCaps{5, true, true, true, true, false, 1});
    const auto device = mock.with_device();
    check(device.caps().mouse.pan, "the mouse has no pan");
    device.transform_remap(Axis::Wheel, Axis::Pan);
    const auto entries = device.query_transforms().entries;
    check(entries.size() == 1 && entries[0].op == TransformOp::Remap && entries[0].dest.kind() == LockTargetKind::Pan,
          "the pan transform");
}

void a_transforms_refusals_have_their_own_exception() {
    const MockBox mock;
    const auto device = mock.with_device();
    throws<TransformOpFieldsError>(
        [&] { device.transform(Transform{TransformOp::Remap, Axis::X, LockTarget::button(0)}); }, "an axis to a button");
    throws<TransformOpFieldsError>([&] { device.transform(Transform{TransformOp::Swap, Axis::Y, Axis::Y}); },
                                   "a swap with itself");
    throws<TransformOpFieldsError>(
        [&] { device.transform(Transform{TransformOp::Remap, LockTarget::button(0), LockTarget::button(0)}); },
        "a button to itself");
}

void a_negative_scale_reverses_and_is_refused_where_it_cant() {
    const LockTarget x = Axis::X;
    const MockBox mock;
    const auto device = mock.with_device();
    device.scale(x, Direction::Both, -100);
    const auto locks = device.query_locks();
    check(locks.scale_of(x, Direction::Positive) == -100 && locks.scale_of(x, Direction::Negative) == -100,
          "the reversed scales");
    check(!locks.is_locked(x, Direction::Both), "a reversed axis is locked");
    const auto before = mock.recorded();
    throws<LockScaleUsageError>([&] { device.scale(LockTarget::button(0), Direction::Positive, -100); },
                                "a reversed button");
    throws<LockScaleRangeError>(
        [&] { device.scale(x, Direction::Both, static_cast<std::int16_t>(LockScaleMin - 1)); }, "under the range");
    throws<LockScaleRangeError>(
        [&] { device.scale(x, Direction::Both, static_cast<std::int16_t>(LockScaleMax + 1)); }, "over the range");
    check(mock.recorded() == before, "a refused scale went out");
}

void an_input_event_carries_pan() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.input_events(CatchFilter::all_input());
    mock.push_motion(1, 4000, MotionEvent{3, -4, 0, 5});
    const auto motion = must(stream.recv(wait), "the motion");
    check(motion.kind == InputKind::Motion && motion.dx == 3 && motion.dy == -4 && motion.dz == 0 && motion.pan == 5,
          "the motion with pan");
}

void a_mock_restart_is_recovered_and_the_clip_says_it_was_lost() {
    const MockBox mock;
    const auto device = mock.open();
    const auto clip = device.clip();
    device.press(Button::SIDE1);
    clip.set_retain(true);
    clip.append(ClipBuilder().move(1, 0));
    check(!clip.lost(), "a new clip is lost");
    mock.clear_recorded();
    mock.restart();
    until([&] { return device.counters().restarts >= 1; }, "the restart was never seen");
    check(clip.lost(), "the restart didn't lose the clip");
    // the held press and the retain setting went back to the restarted box
    check(frames(mock, FrameType::Inject) == std::vector<Bytes>{{0, 3, 0, 1}}, "the held press");
    check(frames(mock, FrameType::ClipSet) == std::vector<Bytes>{{2, 1}}, "the retain setting");
    clip.append(ClipBuilder().move(1, 0));
    check(!clip.lost(), "an append after the restart is lost");
}

void a_clone_presented_again_gets_back_what_the_library_held() {
    const LockTarget x = Axis::X;
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    const auto device = mock.open();
    device.set_patch(Patch{PatchSection::Device, 0, 0, 12, {0x00, 0x01}});
    device.scale(x, Direction::Both, 40);
    mock.clear_recorded();
    device.apply_patch();
    until([&] { return !frames(mock, FrameType::Lock).empty(); }, "the scale never went back");
    check(device.query_patches().applied, "the patch isn't applied");
    check(device.query_locks().scale_of(x, Direction::Positive) == 40, "the scale");
    check(device.counters().restarts == 0, "a presentation counted as a restart");
}

void a_release_the_box_counts_is_recovered() {
    const LockTarget x = Axis::X;
    const MockBox mock;
    const auto device = mock.open();
    device.scale(x, Direction::Both, 40);
    mock.link_lost();
    check(device.query_stats().session == 1, "the session");
    until([&] { return device.query_locks().scale_of(x, Direction::Positive) == 40; }, "the scale never went back");
    mock.detach(false);
    mock.attach();
    until([&] { return device.query_locks().scale_of(x, Direction::Positive) == 40; },
          "the scale never went back after a replug");
    check(device.counters().restarts == 0, "a release counted as a restart");
}

CatchEvent push_and_recv(const MockBox& mock, const EventStream& stream, const TrafficEvent& sent,
                         ClockDomain clock = ClockDomain::DeviceChip) {
    mock.push_traffic(1, 7000, clock, sent);
    auto event = must(stream.recv(wait), "the traffic event");
    check(event.kind == CatchEventKind::Traffic && event.traffic(), "a traffic event that isn't one");
    return event;
}

void a_forced_rate_comes_back() {
    const MockBox mock;
    mock.set_advertised_hz(125);
    mock.set_imperfect_status(ImperfectStatus{true, false, false});
    mock.set_emit_pace(EmitPace::fixed(500), std::uint16_t{400});
    {
        const auto status = mock.with_device().query_emit_pace();
        check(status.mode == EmitPace::fixed(500), "the pace");
        check(status.resolved_hz == 500 && status.force_hz == 400 && status.advertised_hz == 500 && status.force_active,
              "the forced rate");
    }
    mock.set_emit_pace(EmitPace::fixed(500));
    const auto status = mock.with_device().query_emit_pace();
    check(!status.force_hz && !status.force_active && status.advertised_hz == 125, "the unforced rate");
}

void a_forced_rate_needs_the_imperfect_opt_in() {
    const MockBox mock;
    mock.set_advertised_hz(125);
    const auto device = mock.with_device();
    device.set_emit_pace(EmitPace::learned(), std::uint16_t{1000});
    auto status = device.query_emit_pace();
    check(status.force_hz == 1000 && !status.force_active && status.advertised_hz == 125, "without the opt in");
    device.allow_imperfect_clones(true);
    status = device.query_emit_pace();
    check(status.force_active && status.advertised_hz == 1000, "with the opt in");
}

void a_forced_rate_goes_through_the_setter() {
    const MockBox mock;
    const auto device = mock.with_device();
    device.allow_imperfect_clones(true);
    device.set_emit_pace(EmitPace::fixed(250), std::uint16_t{125});
    const auto status = device.query_emit_pace();
    check(status.mode == EmitPace::fixed(250), "the pace");
    check(status.force_hz == 125 && status.advertised_hz == 125 && status.force_active, "the forced rate");
    device.set_emit_pace(EmitPace::fixed(250), std::nullopt);
    check(!device.query_emit_pace().force_hz, "the rate that's no longer forced");
}

void the_counters_can_be_read() {
    const MockBox mock;
    const auto device = mock.with_device();
    device.move_rel(1, 0);
    check(device.counters().frames_tx >= 1, "nothing was sent");
}

void a_catch_delivers_motion() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::everything()});
    mock.push_motion(1, 7000, MotionEvent{12, -34, 1, 2});
    const auto event = must(stream.recv(wait), "the motion");
    check(event.kind == CatchEventKind::Motion && event.ts_us == 7000u, "the event");
    check(event.motion() && *event.motion() == MotionEvent{12, -34, 1, 2}, "the motion");
}

void a_catch_delivers_a_key() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::watch_class(Class::Key)});
    mock.push_usages(1, 7000, UsageSnapshot{Class::Key, Direction::Both, {Key::ESCAPE}});
    const auto event = must(stream.recv(wait), "the key");
    check(event.kind == CatchEventKind::Usages && event.usages(), "the event");
    check(event.usages() && event.usages()->is_held(Key::ESCAPE) && !event.usages()->is_held(Key::A), "the held keys");
}

void a_catch_delivers_media() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::everything()});
    mock.push_usages(1, 7000, UsageSnapshot{Class::Media, Direction::Both, {MediaKey::VOLUME_UP}});
    const auto event = must(stream.recv(wait), "the media key");
    check(event.usages() && event.usages()->is_held(MediaKey::VOLUME_UP), "the held media key");
}

void a_catch_delivers_traffic() {
    const auto sent = traffic(CatchClass::HidIn, 2, Direction::Positive, 0, 6, {1, 2, 3, 4, 5, 6});
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::traffic_of(TrafficClass::HidIn)});
    const auto event = push_and_recv(mock, stream, sent, ClockDomain::HostChip);
    check(event.ts_us == 7000u && event.clock == ClockDomain::HostChip, "the stamp");
    check(!event.motion() && !event.usages(), "traffic that's something else too");
    check(event.traffic() && *event.traffic() == sent, "the traffic");
    check(event.traffic() && !event.traffic()->truncated() && event.traffic()->data() == sent.bytes, "the data");
}

void a_true_len_above_the_capture_is_a_truncation() {
    const auto cut = traffic(CatchClass::VendorBulk, 3, Direction::Positive, 0x03, 512, range(16));
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::traffic(TrafficClass::VendorBulk, 3).with_capture(16)});
    const auto event = push_and_recv(mock, stream, cut);
    if (const auto* got = event.traffic()) {
        check(got->true_len == 512 && got->bytes.size() == 16 && got->truncated(), "the truncation");
        check(got->bulk_end_of_transfer() && got->zlp(), "the flags");
    }
    check(!traffic(CatchClass::VendorBulk, 3, Direction::Positive, 0, 16, Bytes(16)).truncated(), "a whole packet");
}

void a_control_events_accessors_read_its_bytes() {
    const Bytes setup{0x80, 0x06, 0x00, 0x01, 0x00, 0x00, 0x12, 0x00};
    const std::array<std::uint8_t, 8> setup_array{0x80, 0x06, 0x00, 0x01, 0x00, 0x00, 0x12, 0x00};
    const auto stalled = traffic(CatchClass::Control, 0, Direction::Positive, 0x81, 8, setup);
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::traffic_of(TrafficClass::Control)});
    const auto event = push_and_recv(mock, stream, stalled);
    if (const auto* got = event.traffic()) {
        check(got->setup_bytes() == setup_array && got->data().empty(), "the setup");
        check(got->control_status() == ControlStatus::Stalled && got->rule_acted() && !got->bus_event(), "the stall");
    }
    const auto answered = traffic(CatchClass::Control, 0, Direction::Positive, 0, 10, join(setup, {0x12, 0x01}));
    check(answered.control_status() == ControlStatus::Ok && !answered.rule_acted(), "the answer");
    check(answered.data() == Bytes{0x12, 0x01}, "the answer's data");
    check(traffic(CatchClass::Control, 0, Direction::Positive, 0x02, 8, setup).control_status() == ControlStatus::Naked,
          "a NAK");
}

void a_clip_transfers_accessors_read_its_bytes() {
    const Bytes setup{0xA1, 0x01, 0x00, 0x03, 0x00, 0x00, 0x02, 0x00};
    const std::array<std::uint8_t, 8> setup_array{0xA1, 0x01, 0x00, 0x03, 0x00, 0x00, 0x02, 0x00};
    const auto on = [&](std::uint8_t status) {
        return traffic(CatchClass::ClipTransfer, 0, Direction::In, status, 8, setup);
    };
    const auto answered = traffic(CatchClass::ClipTransfer, 0, Direction::In,
                                  static_cast<std::uint8_t>(TransferStatus::Ok), 10, join(setup, {4, 1}));
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::traffic_of(TrafficClass::ClipTransfer)});
    const auto event = push_and_recv(mock, stream, answered);
    if (const auto* got = event.traffic()) {
        check(*got == answered && is_traffic(got->class_), "the clip transfer");
        check(got->setup_bytes() == setup_array && got->data() == Bytes{4, 1}, "the bytes");
        check(got->transfer_status() == TransferStatus::Ok && !got->control_status(), "the status");
    }
    check(on(0xFD).transfer_status() == TransferStatus::Stall && !on(0xFD).rule_acted() && on(0xFD).data().empty(),
          "the stall");
    check(on(0xFE).transfer_status() == TransferStatus::Nak, "the NAK");
    // a status nothing names stays its byte
    check(on(0x42).transfer_status() == static_cast<TransferStatus>(0x42), "status 0x42");
    check(!traffic(CatchClass::Control, 0, Direction::In, 0, 8, setup).transfer_status(), "control has a transfer status");
}

void a_bus_event_is_decoded() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::traffic_of(TrafficClass::Bus)});
    const auto event = push_and_recv(mock, stream, traffic(CatchClass::Bus, 0, Direction::Both, 5, 2, {3, 1}));
    if (const auto* got = event.traffic()) {
        const auto bus = got->bus_event();
        check(bus && bus->kind == BusEventKind::SetInterface && bus->interface == 3 && bus->alt == 1, "the bus event");
        check(!got->control_status(), "a bus event has a control status");
    }
}

void traffic_comes_on_every_way_of_receiving() {
    const auto sent = traffic(CatchClass::Emit, 1, Direction::Positive, 0, 3, {1, 2, 3});
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::traffic_of(TrafficClass::Emit)});
    mock.push_traffic(1, 10, ClockDomain::DeviceChip, sent);
    const auto blocking = stream.recv();
    mock.push_traffic(2, 20, ClockDomain::DeviceChip, sent);
    std::optional<CatchEvent> polled;
    const auto start = std::chrono::steady_clock::now();
    while (!polled && std::chrono::steady_clock::now() - start < wait) {
        polled = stream.try_recv();
    }
    mock.push_traffic(3, 30, ClockDomain::DeviceChip, sent);
    const auto timed = stream.recv(wait);
    for (const auto& event : {std::optional<CatchEvent>(blocking), polled, timed}) {
        check(event && event->kind == CatchEventKind::Traffic && event->traffic() && *event->traffic() == sent,
              "a way of receiving");
    }
}

void catch_events_needs_a_filter() {
    const MockBox mock;
    const auto device = mock.with_device();
    throws<EmptySubscriptionError>([&] { device.catch_events({}); }, "catch_events with no filter");
    throws<EmptySubscriptionError>([&] { device.input_events({}); }, "input_events with no filter");
}

void try_recv_gives_null_when_empty() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::everything()});
    check(!stream.try_recv() && stream.dropped() == 0, "an empty stream");
}

void a_log_line_comes() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto logs = device.logs();
    mock.push_log(LogLevel::Warn, "hello world");
    check(logs.recv(wait) == LogLine{LogLevel::Warn, "hello world"}, "the log line");
}

void a_clone_shares_the_connection() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto clone = device.clone();
    device.move_rel(1, 0);
    clone.move_rel(2, 0);
    check(mock.clone().recorded() == 2, "the clones' frames");
}

void a_stream_clone_shares_the_subscription() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::everything()});
    const auto clone = stream.clone();
    mock.push_motion(1, 7000, MotionEvent{9, 0, 0, 0});
    const auto event = clone.recv(wait);
    check(event && event->motion() && event->motion()->dx == 9, "the clone's motion");
}

// a moved from handle is what a disposed one is in C#
void disposing_twice_is_fine() {
    MockBox mock;
    Device device = mock.with_device();
    { Device taken = std::move(device); }
    { Device again = std::move(device); }
    { MockBox taken = std::move(mock); }
    { MockBox again = std::move(mock); }
    throws<anybox::ClosedError>([&] { device.move_rel(1, 1); }, "a moved from device", "moved from");
    throws<anybox::ClosedError>([&] { mock.silent(); }, "a moved from mock", "moved from");

    // every other handle says so too, instead of handing medius.h a null that some calls answer with a quiet 0
    const MockBox other;
    const auto opened = other.with_device();
    auto clip = opened.clip();
    { Clip taken = std::move(clip); }
    throws<anybox::ClosedError>([&] { clip.lost(); }, "a moved from clip", "moved from");
    ClipFrame frame;
    { ClipFrame taken = std::move(frame); }
    throws<anybox::ClosedError>([&] { frame.byte_len(); }, "a moved from frame", "moved from");
    ClipBuilder builder;
    { ClipBuilder taken = std::move(builder); }
    throws<anybox::ClosedError>([&] { builder.byte_len(); }, "a moved from builder", "moved from");
    Timeline timeline;
    { Timeline taken = std::move(timeline); }
    throws<anybox::ClosedError>([&] { timeline.samples(ClockDomain::HostChip); }, "a moved from timeline", "moved from");
    auto events = opened.catch_events({CatchFilter::everything()});
    { EventStream taken = std::move(events); }
    throws<anybox::ClosedError>([&] { events.try_recv(); }, "a moved from event stream", "moved from");
    throws<anybox::ClosedError>([&] { events.close(); }, "closing a moved from event stream", "moved from");
    auto input = opened.input_events(CatchFilter::all_input());
    { InputStream taken = std::move(input); }
    throws<anybox::ClosedError>([&] { input.is_connected(); }, "a moved from input stream", "moved from");
    auto logs = opened.logs();
    { LogStream taken = std::move(logs); }
    throws<anybox::ClosedError>([&] { logs.is_connected(); }, "a moved from log stream", "moved from");
}

void a_snapshots_is_held_takes_every_class() {
    const UsageSnapshot snapshot{Class::Button, Direction::Both, {Button::RIGHT, Key::LEFT_CTRL, Key::A}};
    check(snapshot.is_held(Button::RIGHT) && snapshot.is_held(Key::LEFT_CTRL) && snapshot.is_held(Key::A), "held");
    check(!snapshot.is_held(Button::LEFT) && !snapshot.is_held(Key::B), "not held");
}

// the protocol's own example: HID in interface 2, in, start, consume and once per run, selector 1, match 07 20 under
// mask FF 20
ClipPacketTrigger held_trigger() {
    return packet(TrafficClass::HidIn, 2, Direction::In, ClipAction::Start, hex("07 20"), hex("FF 20"), true, true, 1);
}

// eight packet triggers that differ in every field and each of them one a packet can match
std::vector<ClipPacketTrigger> packet_rows() {
    const std::uint8_t masks[8] = {0xFF, 0xF0, 0x0F, 0x20, 0x81, 0x7E, 0xC3, 0x01};
    struct Row {
        TrafficClass class_;
        std::uint16_t id;
        Direction direction;
        ClipAction action;
        std::size_t length;
        bool consume;
        int selector;
        std::uint16_t hits;
    };
    const Row rows[] = {
        {TrafficClass::HidIn, 0x0102, Direction::In, ClipAction::Start, 1, true, -1, 0},
        {TrafficClass::HidOut, 0x0001, Direction::Out, ClipAction::Stop, 2, false, 1, 1},
        {TrafficClass::VendorInterrupt, 0x0083, Direction::In, ClipAction::Pause, 3, true, 2, 0xFFFF},
        {TrafficClass::VendorBulk, CatchIdAny, Direction::Both, ClipAction::Resume, 4, false, -1, 0x1234},
        {TrafficClass::Control, 0, Direction::Both, ClipAction::Restart, 5, false, -1, 0x00FF},
        {TrafficClass::Emit, 0x0081, Direction::In, ClipAction::Toggle, 0, false, -1, 0xFF00},
        {TrafficClass::VendorBulk, 0x0003, Direction::Out, ClipAction::Start, 7, true, 3, 65534},
        {TrafficClass::Emit, 0x0082, Direction::In, ClipAction::Stop, MaxPktMatch, false, -1, 9},
    };
    std::vector<ClipPacketTrigger> out;
    for (std::size_t i = 0; i < 8; i++) {
        const auto& row = rows[i];
        Bytes match;
        Bytes mask;
        for (std::size_t j = 0; j < row.length; j++) {
            match.push_back(static_cast<std::uint8_t>((0x35 + 0x1B * i + 0x47 * j) & masks[(i + j) % 8]));
            mask.push_back(masks[(i + j) % 8]);
        }
        out.push_back(packet(row.class_, row.id, row.direction, row.action, match, mask, row.consume, row.selector >= 0,
                             static_cast<std::uint8_t>(row.selector >= 0 ? row.selector : 0), row.hits));
    }
    return out;
}

std::vector<ClipPacketTrigger> leading(const std::vector<ClipPacketTrigger>& rows, std::size_t count) {
    return std::vector<ClipPacketTrigger>(rows.begin(), rows.begin() + static_cast<std::ptrdiff_t>(count));
}

void clip_control_frames_are_the_protocols_bytes() {
    const MockBox mock;
    {
        const auto device = mock.with_device();
        const auto clip = device.clip();
        clip.set_retain(true);
        clip.set_autolock({Blanket::Aim, Blanket::Buttons});
        clip.set_loop(true);
        clip.start();
        clip.pause();
        clip.resume();
        clip.restart();
        clip.toggle();
        clip.stop();
        clip.clear();
        clip.finalize();
        clip.bind(input_trigger(Key{0x3A}, Edge::Press, ClipAction::Start));
        clip.bind(input_trigger(Button::RIGHT, Edge::Release, ClipAction::Toggle, true));
        clip.unbind(Key{0x3A}, Edge::Press);
        clip.clear_triggers();
    }
    check(frames(mock, FrameType::ClipSet) == std::vector<Bytes>{{2, 1}, {0, 0x05}, {1, 1}}, "the clip settings");
    check(frames(mock, FrameType::ClipCtrl) == std::vector<Bytes>{{0}, {2}, {3}, {4}, {5}, {1}, {6}, {7}},
          "the clip verbs");
    const std::vector<Bytes> triggers = {
        {1, 0x3A, 0x00, 1, 0, 1}, {0, 0x01, 0x00, 2, 5, 3}, {1, 0x3A, 0x00, 1, 0, 0}, {0xFF, 0xFF, 0xFF, 0, 0, 0}};
    check(frames(mock, FrameType::ClipTrigger) == triggers, "the triggers");
}

void an_append_encodes_and_chunks() {
    const MockBox mock;
    {
        const auto device = mock.with_device();
        ClipBuilder builder;
        const auto clip = device.clip();
        for (int round = 0; round < 150; round++) {
            builder.move(3, -2);
        }
        builder.press(Button::LEFT).gap(4).release(Button::LEFT);
        clip.append(builder);
    }
    const auto appends = frames(mock, FrameType::ClipAppend);
    check(appends.size() >= 2, "a clip past 512 bytes has to be split");
    Bytes joined;
    for (const auto& part : appends) {
        joined = join(joined, part);
        check(part.size() <= 512, "a part of " + std::to_string(part.size()) + " bytes");
    }
    check(joined.size() > 20 && Bytes(joined.begin(), joined.begin() + 5) == Bytes{0x01, 0x03, 0x00, 0xFE, 0xFF},
          "the first move");
    check(joined.size() > 20 &&
              Bytes(joined.end() - 15, joined.end()) ==
                  Bytes{0x04, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00},
          "the press, gap and release");
}

void a_frames_edges_are_encoded() {
    const MockBox mock;
    {
        const auto device = mock.with_device();
        ClipBuilder builder;
        builder.frame(make_frame(1, 2, -1, 0, {{Button::LEFT, Action::Press}, {Key{0x04}, Action::Press}}));
        device.clip().append(builder);
    }
    const auto appends = frames(mock, FrameType::ClipAppend);
    check(!appends.empty() && appends[0] == Bytes{0x07, 0x01, 0x00, 0x02, 0x00, 0xFF, 0xFF, 0x02, 0x00, 0x00, 0x00,
                                                  0x01, 0x01, 0x04, 0x00, 0x01},
          "the frame");
}

void a_frame_carries_pan_raw_reports_and_transfers() {
    const Setup set_report{0x21, 0x09, 0x0300, 0, 2};
    const Setup get_report{0xA1, 0x01, 0x0300, 0, 8};
    const MockBox mock;
    {
        const auto device = mock.with_device();
        ClipBuilder builder;
        builder.frame(make_frame(4, -2, 1, -3, {{Button::LEFT, Action::Press}},
                                 {{2, Direction::Out, {0x10, 0xFF, 0x05}}, {1, Direction::In, {}}},
                                 {{3, set_report, {0x04, 0x01}}, {4, get_report, {}}}));
        device.clip().append(builder);
    }
    const std::vector<Bytes> wanted = {{
        0x3F, 0x04, 0x00, 0xFE, 0xFF, 0x01, 0x00, 0xFD, 0xFF, 0x01, 0x00, 0x00, 0x00, 0x01, 0x02, 0x02,
        0x02, 0x03, 0x00, 0x10, 0xFF, 0x05, 0x01, 0x01, 0x00, 0x00, 0x02, 0x03, 0x21, 0x09, 0x00, 0x03,
        0x00, 0x00, 0x02, 0x00, 0x04, 0x01, 0x04, 0xA1, 0x01, 0x00, 0x03, 0x00, 0x00, 0x08, 0x00,
    }};
    check(frames(mock, FrameType::ClipAppend) == wanted, "the frame");
}

void one_field_pan_raw_and_transfer_frames() {
    const MockBox mock;
    {
        const auto device = mock.with_device();
        ClipBuilder builder;
        builder.pan(7).raw(2, Direction::Out, {0xAA}).transfer(5, Setup{0x21, 0x09, 0x0300, 0, 1}, {0x05});
        builder.transfer(6, Setup{0xA1, 0x01, 0x0300, 0, 8});
        check(builder.byte_len() == 3 + 7 + 12 + 11, "the byte length is " + std::to_string(builder.byte_len()));
        device.clip().append(builder);
    }
    Bytes joined;
    for (const auto& part : frames(mock, FrameType::ClipAppend)) {
        joined = join(joined, part);
    }
    check(joined == hex("08 07 00" "10 01 02 02 01 00 AA" "20 01 05 21 09 00 03 00 00 01 00 05"
                        "20 01 06 A1 01 00 03 00 00 08 00"),
          "the frames");
}

void byte_len_is_what_an_append_puts_in_the_ring() {
    const MockBox mock;
    std::size_t expected = 0;
    {
        const auto device = mock.with_device();
        ClipBuilder builder;
        check(builder.byte_len() == 0, "a new builder's length");
        builder.frame(make_frame());
        check(builder.byte_len() == 5, "an empty frame's length");
        builder.gap(4).wheel(1).press(Button::LEFT);
        check(builder.byte_len() == 5 + 3 + 3 + 6, "the length is " + std::to_string(builder.byte_len()));
        for (int round = 0; round < 150; round++) {
            builder.move(3, -2);
        }
        builder.frame(make_frame(1, 0, 0, 0, {}, {{2, Direction::Out, {0x10, 0xFF}}},
                                 {{3, Setup{0xA1, 0x01, 0x0300, 0, 8}, {}}}));
        expected = builder.byte_len();
        device.clip().append(builder);
        builder.clear();
        check(builder.byte_len() == 0, "a cleared builder's length");
    }
    const auto appends = frames(mock, FrameType::ClipAppend);
    std::size_t sent = 0;
    for (const auto& part : appends) {
        sent += part.size();
    }
    check(appends.size() >= 2 && sent == expected, std::to_string(sent) + " bytes went out for " +
                                                       std::to_string(expected));
}

template <class Thrown>
void refused_on_append(const Clip& clip, const std::function<void(ClipBuilder&)>& build, const std::string& what) {
    ClipBuilder builder;
    builder.move(1, 1);
    build(builder);
    throws<Thrown>([&] { clip.append(builder); }, what);
}

void a_refused_frame_throws_its_own_exception_and_sends_nothing() {
    const Setup set_report{0x21, 0x09, 0x0300, 0, 2};
    const std::vector<std::pair<Usage, Action>> too_many_edges(static_cast<std::size_t>(ClipEdgesMax) + 1,
                                                                {Button::LEFT, Action::Press});
    const std::vector<RawReport> too_many_raw(static_cast<std::size_t>(ClipRawMax) + 1, {1, Direction::Out, Bytes(1)});
    const MockBox mock;
    const auto device = mock.with_device();
    const auto clip = device.clip();
    refused_on_append<ClipFrameCountError>(
        clip, [&](ClipBuilder& builder) { builder.frame(make_frame(0, 0, 0, 0, too_many_edges)); }, "too many edges");
    refused_on_append<ClipFrameCountError>(
        clip, [&](ClipBuilder& builder) { builder.frame(make_frame(0, 0, 0, 0, {}, too_many_raw)); },
        "too many raw reports");
    refused_on_append<ClipFrameTooLongError>(
        clip,
        [&](ClipBuilder& builder) {
            builder.frame(make_frame(0, 0, 0, 0, {}, {{1, Direction::Out, Bytes(static_cast<std::size_t>(ClipEntryMax))}}));
        },
        "a raw report past the entry");
    refused_on_append<ClipTransferDataError>(
        clip, [&](ClipBuilder& builder) { builder.frame(make_frame(0, 0, 0, 0, {}, {}, {{0, set_report, {4}}})); },
        "OUT data that isn't its length");
    refused_on_append<ClipTransferDataError>(
        clip,
        [&](ClipBuilder& builder) {
            builder.frame(make_frame(0, 0, 0, 0, {}, {}, {{0, Setup{0xA1, 0x01, 0x0300, 0, 8}, {4}}}));
        },
        "data on an IN transfer");
    refused_on_append<RawDirectionError>(
        clip, [&](ClipBuilder& builder) { builder.frame(make_frame(0, 0, 0, 0, {}, {{1, Direction::Both, Bytes(1)}})); },
        "raw both ways");
    refused_on_append<RelativeDirectionError>(
        clip, [&](ClipBuilder& builder) { builder.frame(make_frame(0, 0, 0, 0, {}, {{1, Direction::With, Bytes(1)}})); },
        "raw with the bearing");
    check(frames(mock, FrameType::ClipAppend).empty(), "a refused frame went out");
    {
        ClipBuilder builder;
        builder.frame(make_frame(0, 0, 0, 0,
                                 std::vector<std::pair<Usage, Action>>(static_cast<std::size_t>(ClipEdgesMax),
                                                                       {Button::LEFT, Action::Press}),
                                 std::vector<RawReport>(static_cast<std::size_t>(ClipRawMax), {1, Direction::Out, Bytes(1)})));
        clip.append(builder);
    }
    check(frames(mock, FrameType::ClipAppend).size() == 1, "the fullest frame");
}

void a_bad_frame_field_leaves_the_builder_as_it_was() {
    const MockBox mock;
    {
        const auto device = mock.with_device();
        ClipBuilder builder;
        builder.move(1, 1);
        throws<std::invalid_argument>(
            [&] { builder.frame(make_frame(2, 0, 0, 0, {}, {{1, static_cast<Direction>(200), Bytes(1)}})); },
            "a frame's raw report on 200");
        throws<std::invalid_argument>([&] { builder.raw(1, static_cast<Direction>(200), {0}); }, "raw on 200");
        device.clip().append(builder);
    }
    check(frames(mock, FrameType::ClipAppend) == std::vector<Bytes>{{0x01, 0x01, 0x00, 0x01, 0x00}}, "the builder");
}

void a_gap_of_zero_does_nothing() {
    const MockBox mock;
    {
        const auto device = mock.with_device();
        ClipBuilder builder;
        builder.gap(0);
        device.clip().append(builder);
    }
    check(frames(mock, FrameType::ClipAppend).empty(), "a gap of 0 went out");
}

void the_clip_status_and_config_come_back() {
    const ClipStatus status{ClipState::Playing, 512, 40, 8, 99, 2, 0, 1, 7, 3, 5, {Button::SIDE1, Key::A}};
    ClipSettings settings;
    settings.autolock = {Blanket::Aim, Blanket::Keys};
    settings.loop = true;
    settings.retain = true;
    settings.ride = true;
    settings.triggers = {input_trigger(Button::RIGHT, Edge::Both, ClipAction::Toggle),
                         input_trigger(Key{0x3A}, Edge::Release, ClipAction::Stop, true)};
    settings.packet_triggers = leading(packet_rows(), 3);
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    mock.set_clip_status(status);
    mock.set_clip_settings(settings);
    const auto device = mock.with_device();
    const auto clip = device.clip();
    auto got = clip.query_status();
    const auto config = clip.query_config();
    check(got.held == status.held, "the held usages");
    check(got.is_held(Button::SIDE1) && got.is_held(Key::A) && !got.is_held(Button::LEFT), "what's held");
    got.held = status.held;
    check(got == status, "the status");
    check(config.autolock == settings.autolock, "the autolock");
    check(config.loop == settings.loop && config.retain == settings.retain && config.finalized == settings.finalized &&
              config.ride == settings.ride,
          "the flags");
    check(config.triggers == settings.triggers, "the triggers");
    check(config.packet_triggers == settings.packet_triggers, "the packet triggers");
}

// the settings carry the box's CLIP_LOCK bits (the crate's protocol/opcode.rs), which the header maps itself, so each
// one is pinned against the bit the library puts on the wire
void the_autolock_bits_are_the_boxs() {
    const std::vector<std::pair<std::vector<Blanket>, std::uint8_t>> cases = {
        {{Blanket::Aim}, 0x01},
        {{Blanket::Wheel}, 0x02},
        {{Blanket::Buttons}, 0x04},
        {{Blanket::Keys}, 0x08},
        {{Blanket::Media}, 0x10},
        {{Blanket::Aim, Blanket::Wheel, Blanket::Buttons, Blanket::Keys, Blanket::Media}, 0x1F},
    };
    for (const auto& entry : cases) {
        const auto& blankets = entry.first;
        const auto bits = entry.second;
        const MockBox mock;
        {
            const auto device = mock.with_device();
            device.clip().set_autolock(blankets);
        }
        check(frames(mock, FrameType::ClipSet) == std::vector<Bytes>{{0, bits}}, "the bits " + std::to_string(bits));
        ClipSettings settings;
        settings.autolock = blankets;
        check(settings.to_c().autolock_bits == bits, "the bits going in");
        MediusClipSettings raw{};
        raw.autolock_bits = bits;
        check(ClipSettings::from_c(raw).autolock == blankets, "the bits coming out");
    }
}

void a_packet_trigger_goes_out_as_the_bytes_the_protocol_names() {
    const auto held = held_trigger();
    // a removal sends the key alone, whatever else is in it
    auto stale = held;
    stale.action = static_cast<ClipAction>(5);
    stale.consume = true;
    stale.once_per_run = true;
    stale.selector_len = 9;
    stale.hits = 77;
    const MockBox mock;
    {
        const auto device = mock.with_device();
        const auto clip = device.clip();
        clip.bind_packet(held);
        clip.unbind_packet(stale);
        clip.bind_packet(packet(TrafficClass::VendorBulk, CatchIdAny, Direction::Both, ClipAction::Pause, {}, {}, true));
        clip.bind_packet(packet(TrafficClass::HidOut, 1, Direction::Out, ClipAction::Resume, hex("01 02 03"),
                                hex("FF FF 0F"), false, true, 2));
        clip.bind_packet(packet(TrafficClass::Control, 0, Direction::Both, ClipAction::Toggle, range(16),
                                Bytes(16, 0xFF), false, false, 0, 500));
        clip.bind_packet(packet(TrafficClass::Emit, 0x0181, Direction::In, ClipAction::Stop, {0x80}, {0xF0}));
        clip.clear_triggers();
    }
    // [class][id u16][dir][action][flags: 1 present, 2 consume, 4 once per run][slen][mlen][match][mask]
    const std::vector<Bytes> wanted = {
        hex("04 02 00 01 00 07 01 02 07 20 FF 20"),
        hex("04 02 00 01 00 00 00 02 07 20 FF 20"),
        hex("07 FF FF 00 02 03 00 00"),
        hex("05 01 00 02 03 05 02 03 01 02 03 FF FF 0F"),
        join(join(hex("08 00 00 00 05 01 00 10"), range(16)), Bytes(16, 0xFF)),
        hex("09 81 01 01 01 01 00 01 80 F0"),
        hex("FF FF FF 00 00 00"),
    };
    check(frames(mock, FrameType::ClipTrigger) == wanted, "the packet trigger frames");
}

void packet_triggers_read_back_field_for_field() {
    const auto rows = packet_rows();
    check(rows[2].hits == 0xFFFF, "row 2's hits");
    const std::vector<ClipTrigger> inputs = {input_trigger(Button::RIGHT, Edge::Press, ClipAction::Start)};
    for (const std::size_t count : {0, 1, 8}) {
        const MockBox mock;
        mock.set_imperfect_status(allowed);
        ClipSettings settings;
        settings.triggers = inputs;
        settings.packet_triggers = leading(rows, count);
        mock.set_clip_settings(settings);
        const auto config = mock.with_device().clip().query_config();
        check(config.packet_triggers == leading(rows, count), std::to_string(count) + " packet triggers");
        check(config.triggers == inputs, "the input triggers with " + std::to_string(count));
    }
    const auto spec = packet(TrafficClass::HidIn, 0x0102, Direction::In, ClipAction::Toggle, hex("07 20"), hex("FF 20"),
                             true, true, 1, 0xFFFF);
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    ClipSettings settings;
    settings.packet_triggers = {spec};
    mock.set_clip_settings(settings);
    const auto device = mock.with_device();
    const auto clip = device.clip();
    const auto read = clip.query_config().packet_triggers;
    check(read == std::vector<ClipPacketTrigger>{spec}, "the spec");
    // a read trigger replays as a bind, its hits left behind
    if (!read.empty()) {
        clip.bind_packet(read[0]);
    }
    check(frames(mock, FrameType::ClipTrigger) == std::vector<Bytes>{hex("04 02 01 01 05 07 01 02 07 20 FF 20")},
          "the replayed bind");
}

void each_length_of_a_read_back_trigger_is_its_own() {
    MediusClipPacketTrigger raw{};
    raw.class_ = static_cast<std::uint8_t>(TrafficClass::HidIn);
    raw.id = 2;
    raw.direction = static_cast<std::uint8_t>(Direction::In);
    raw.action = static_cast<std::uint8_t>(ClipAction::Start);
    for (std::size_t index = 0; index < static_cast<std::size_t>(MaxPktMatch); index++) {
        raw.match_bytes[index] = static_cast<std::uint8_t>(index + 1);
        raw.mask[index] = static_cast<std::uint8_t>(0xF0 + index);
    }
    raw.match_len = 3;
    raw.mask_len = 1;
    auto read = ClipPacketTrigger::from_c(raw);
    check(read.match_bytes == hex("01 02 03") && read.mask == hex("F0"), "the lengths");
    // a length past the array reads the array
    raw.match_len = 1;
    raw.mask_len = 40;
    read = ClipPacketTrigger::from_c(raw);
    Bytes masks;
    for (std::uint8_t value = 0xF0; value != 0; value++) {
        masks.push_back(value);
    }
    check(read.match_bytes == hex("01") && read.mask == masks, "a length past the array");
}

void a_consuming_trigger_scripted_with_the_opt_in_off_is_left_out() {
    auto consuming = held_trigger();
    consuming.hits = 7;
    const auto watching =
        packet(TrafficClass::HidIn, 3, Direction::In, ClipAction::Start, hex("07 20"), hex("FF 20"), false, true, 1, 5);
    const std::vector<ClipTrigger> inputs = {input_trigger(Button::RIGHT, Edge::Press, ClipAction::Start, true)};
    ClipSettings settings;
    settings.triggers = inputs;
    settings.packet_triggers = {consuming, watching};
    const MockBox mock;
    const auto device = mock.with_device();
    const auto clip = device.clip();
    mock.set_clip_settings(settings);
    auto config = clip.query_config();
    check(config.triggers == inputs, "the input triggers");
    check(config.packet_triggers == std::vector<ClipPacketTrigger>{watching}, "without the opt in");
    mock.set_imperfect_status(allowed);
    mock.set_clip_settings(settings);
    check(clip.query_config().packet_triggers == std::vector<ClipPacketTrigger>{consuming, watching},
          "with the opt in");
    mock.set_imperfect_status(ImperfectStatus{false, false, false});
    check(clip.query_config().packet_triggers == std::vector<ClipPacketTrigger>{watching}, "with the opt in turned off");
}

void a_bound_packet_trigger_reads_back_as_it_was_bound() {
    const auto rows = packet_rows();
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    const auto device = mock.with_device();
    const auto clip = device.clip();
    for (const auto& row : rows) {
        clip.bind_packet(row);
    }
    const auto config = clip.query_config();
    check(config.packet_triggers.size() == 8, std::to_string(config.packet_triggers.size()) + " bound");
    if (config.packet_triggers.size() == 8) {
        check(config.packet_triggers[0].consume && !config.packet_triggers[0].once_per_run &&
                  !config.packet_triggers[1].consume && config.packet_triggers[1].once_per_run,
              "the first two rows' flags");
    }
    auto bound = rows;
    for (auto& row : bound) {
        // a bind starts its count at zero
        row.hits = 0;
    }
    check(config.packet_triggers == bound, "the bound rows");
    clip.unbind_packet(rows[2]);
    clip.unbind_packet(rows[7]);
    std::vector<ClipPacketTrigger> kept = {bound[0], bound[1], bound[3], bound[4], bound[5], bound[6]};
    check(clip.query_config().packet_triggers == kept, "after two unbinds");
}

// a list too long going in is refused, since the library would cut it without a word
void settings_past_the_arrays_are_refused() {
    std::vector<ClipTrigger> inputs;
    std::vector<ClipPacketTrigger> packets;
    for (std::size_t index = 0; index <= static_cast<std::size_t>(ClipPktTrigMax); index++) {
        inputs.push_back(input_trigger(Key{static_cast<std::uint8_t>(0x04 + index)}, Edge::Press, ClipAction::Start));
        packets.push_back(packet(TrafficClass::HidIn, static_cast<std::uint16_t>(index), Direction::In,
                                 ClipAction::Start, {static_cast<std::uint8_t>(index)}, {0xFF}, false, false, 0,
                                 static_cast<std::uint16_t>(index)));
    }
    const MockBox mock;
    ClipSettings too_many_inputs;
    too_many_inputs.triggers = inputs;
    throws<std::invalid_argument>([&] { mock.set_clip_settings(too_many_inputs); }, "9 input triggers");
    ClipSettings too_many_packets;
    too_many_packets.packet_triggers = packets;
    throws<std::invalid_argument>([&] { mock.set_clip_settings(too_many_packets); }, "9 packet triggers");
    inputs.pop_back();
    packets.pop_back();
    ClipSettings full;
    full.triggers = inputs;
    full.packet_triggers = packets;
    mock.set_clip_settings(full);
    const auto config = mock.with_device().clip().query_config();
    check(config.triggers == inputs && config.packet_triggers == packets, "the full arrays");
}

void clear_triggers_removes_both_kinds() {
    const auto rows = packet_rows();
    ClipSettings settings;
    settings.triggers = {input_trigger(Button::RIGHT, Edge::Press, ClipAction::Start)};
    settings.packet_triggers = {rows[1]};
    const MockBox mock;
    mock.set_clip_settings(settings);
    const auto device = mock.with_device();
    const auto clip = device.clip();
    clip.bind_packet(rows[5]);
    const auto held = clip.query_config();
    check(held.triggers.size() == 1 && held.packet_triggers.size() == 2, "the held triggers");
    clip.clear_triggers();
    const auto cleared = clip.query_config();
    check(cleared.triggers.empty() && cleared.packet_triggers.empty(), "the cleared triggers");
}

ClipPacketTrigger base() {
    return packet(TrafficClass::HidIn, 2, Direction::In, ClipAction::Start, hex("07 20"), hex("FF 20"));
}

void refused_for(const std::function<void()>& call, const std::string& reason) {
    try {
        call();
        check(false, "a trigger that \"" + reason + "\" went in");
    } catch (const ClipPacketTriggerError& error) {
        const std::string message = error.what();
        check(error.medius_status() == Status::ErrClipPacketTrigger, "the status");
        check(message.rfind("a clip packet trigger ", 0) == 0 && message.find(reason) != std::string::npos,
              "\"" + message + "\" isn't \"" + reason + "\"");
    }
}

void a_packet_trigger_the_box_would_refuse_has_its_own_exception() {
    struct Refused {
        ClipPacketTrigger trigger;
        std::string reason;
        bool key_fault;
    };
    std::vector<Refused> refused;
    const auto with = [&](const std::function<void(ClipPacketTrigger&)>& change, const std::string& reason,
                          bool key_fault) {
        auto trigger = base();
        change(trigger);
        refused.push_back(Refused{trigger, reason, key_fault});
    };
    with([](ClipPacketTrigger& t) { t.class_ = TrafficClass::Bus; }, "names a surface packets cross", true);
    with([](ClipPacketTrigger& t) { t.class_ = TrafficClass::ClipTransfer; }, "names a surface packets cross", true);
    with([](ClipPacketTrigger& t) { t.mask = hex("FF"); }, "a match and a mask of one length", true);
    with([](ClipPacketTrigger& t) { t.match_bytes = hex("07"); }, "a match and a mask of one length", true);
    with([](ClipPacketTrigger& t) { t.match_bytes = t.mask = Bytes(17); }, "at most 16 match bytes", true);
    with([](ClipPacketTrigger& t) { t.match_bytes = t.mask = Bytes(0xFFFF); }, "at most 16 match bytes", true);
    with([](ClipPacketTrigger& t) { t.class_ = TrafficClass::Control; t.consume = true; },
         "takes consume on a report or vendor class", false);
    with([](ClipPacketTrigger& t) { t.selector_len = 1; }, "a selector length only with once_per_run", false);
    with([](ClipPacketTrigger& t) { t.class_ = TrafficClass::Control; t.once_per_run = true; }, "needs one stream",
         false);
    with([](ClipPacketTrigger& t) { t.id = CatchIdAny; t.once_per_run = true; }, "needs one stream", false);
    with([](ClipPacketTrigger& t) { t.direction = Direction::Both; t.once_per_run = true; }, "needs one stream", false);
    with([](ClipPacketTrigger& t) { t.once_per_run = true; t.selector_len = 2; }, "needs match bytes past its selector",
         false);
    with(
        [](ClipPacketTrigger& t) {
            t.match_bytes.clear();
            t.mask.clear();
            t.once_per_run = true;
        },
        "needs match bytes past its selector", false);
    const MockBox mock;
    const auto device = mock.with_device();
    const auto clip = device.clip();
    std::size_t sent_ones = 0;
    for (const auto& row : refused) {
        refused_for([&] { clip.bind_packet(row.trigger); }, row.reason);
        // a removal reads the key alone, so it refuses a bad key and takes anything else
        if (row.key_fault) {
            refused_for([&] { clip.unbind_packet(row.trigger); }, row.reason);
        } else {
            clip.unbind_packet(row.trigger);
            sent_ones++;
        }
    }
    for (const auto relative : {Direction::With, Direction::Against}) {
        auto trigger = base();
        trigger.direction = relative;
        throws<RelativeDirectionError>([&] { clip.bind_packet(trigger); }, "a relative bind");
        throws<RelativeDirectionError>([&] { clip.unbind_packet(trigger); }, "a relative unbind");
    }
    const auto sent = frames(mock, FrameType::ClipTrigger);
    check(sent.size() == sent_ones, std::to_string(sent.size()) + " removals went out");
    for (const auto& frame : sent) {
        check(frame.size() >= 7 && Bytes(frame.begin() + 4, frame.begin() + 7) == Bytes(3), "a removal's key");
    }
    check(clip.query_config().packet_triggers.empty(), "a refused trigger is bound");
    auto full = base();
    full.match_bytes = Bytes(static_cast<std::size_t>(MaxPktMatch), 0x11);
    full.mask = Bytes(static_cast<std::size_t>(MaxPktMatch), 0xFF);
    clip.bind_packet(full);
    check(clip.query_config().packet_triggers == std::vector<ClipPacketTrigger>{full}, "the longest match");
}

void a_trigger_on_a_direction_its_class_never_carries_is_refused() {
    const std::string reason = "names a direction its class never carries: HidIn and Emit flow IN, HidOut flows OUT";
    const auto on = [](TrafficClass traffic_class, Direction direction) {
        return packet(traffic_class, 1, direction, ClipAction::Start, hex("07"), hex("FF"));
    };
    const MockBox mock;
    const auto device = mock.with_device();
    const auto clip = device.clip();
    for (const auto& pair : std::vector<std::pair<TrafficClass, Direction>>{{TrafficClass::HidIn, Direction::Out},
                                                                            {TrafficClass::Emit, Direction::Out},
                                                                            {TrafficClass::HidOut, Direction::In}}) {
        refused_for([&] { clip.bind_packet(on(pair.first, pair.second)); }, reason);
        refused_for([&] { clip.unbind_packet(on(pair.first, pair.second)); }, reason);
    }
    check(frames(mock, FrameType::ClipTrigger).empty(), "a refused trigger went out");
    const std::vector<ClipPacketTrigger> carried = {
        on(TrafficClass::HidIn, Direction::In),           on(TrafficClass::HidIn, Direction::Both),
        on(TrafficClass::HidOut, Direction::Out),         on(TrafficClass::HidOut, Direction::Both),
        on(TrafficClass::VendorInterrupt, Direction::In), on(TrafficClass::VendorInterrupt, Direction::Out),
        on(TrafficClass::VendorBulk, Direction::Both),    on(TrafficClass::Emit, Direction::In),
    };
    for (const auto& trigger : carried) {
        clip.bind_packet(trigger);
    }
    check(clip.query_config().packet_triggers == carried, "the carried directions");
}

void a_trigger_with_a_match_bit_outside_its_mask_is_refused() {
    const std::string reason = "has a match bit outside its mask, which no packet can equal";
    const auto hid_in = [](Bytes match, Bytes mask) {
        return packet(TrafficClass::HidIn, 2, Direction::In, ClipAction::Start, std::move(match), std::move(mask));
    };
    const auto last = static_cast<std::size_t>(MaxPktMatch) - 1;
    const MockBox mock;
    const auto device = mock.with_device();
    const auto clip = device.clip();
    for (const auto& trigger : {hid_in(hex("07 21"), hex("FF 20")), hid_in(hex("01"), hex("00")),
                                hid_in(join(Bytes(last), {0x80}), join(Bytes(last, 0xFF), {0x7F}))}) {
        refused_for([&] { clip.bind_packet(trigger); }, reason);
        refused_for([&] { clip.unbind_packet(trigger); }, reason);
    }
    check(frames(mock, FrameType::ClipTrigger).empty(), "a refused trigger went out");
    const auto held = hid_in(hex("07 20"), hex("FF 20"));
    clip.bind_packet(held);
    check(frames(mock, FrameType::ClipTrigger) == std::vector<Bytes>{hex("04 02 00 01 00 01 00 02 07 20 FF 20")},
          "the bind");
    check(clip.query_config().packet_triggers == std::vector<ClipPacketTrigger>{held}, "the bound trigger");
}

void a_once_per_run_trigger_with_no_masked_bit_past_its_selector_is_refused() {
    const std::string reason = "needs a masked bit past its selector";
    const auto run = [](std::uint8_t selector, const Bytes& match, const Bytes& mask, bool once_per_run = true) {
        return packet(TrafficClass::HidIn, 2, Direction::In, ClipAction::Start, match, mask, false, once_per_run,
                      once_per_run ? selector : std::uint8_t{0});
    };
    const std::vector<std::tuple<std::uint8_t, Bytes, Bytes>> refused = {
        {std::uint8_t{1}, hex("07 00"), hex("FF 00")},
        {std::uint8_t{0}, hex("00 00"), hex("00 00")},
        {std::uint8_t{2}, hex("07 01 00 00"), hex("FF FF 00 00")},
    };
    const MockBox mock;
    const auto device = mock.with_device();
    const auto clip = device.clip();
    for (const auto& row : refused) {
        refused_for([&] { clip.bind_packet(run(std::get<0>(row), std::get<1>(row), std::get<2>(row))); }, reason);
    }
    check(frames(mock, FrameType::ClipTrigger).empty(), "a refused trigger went out");
    std::vector<ClipPacketTrigger> wanted;
    for (const auto& row : refused) {
        clip.unbind_packet(run(std::get<0>(row), std::get<1>(row), std::get<2>(row)));
        clip.bind_packet(run(std::get<0>(row), std::get<1>(row), std::get<2>(row), false));
        wanted.push_back(run(std::get<0>(row), std::get<1>(row), std::get<2>(row), false));
    }
    const auto one_bit = run(1, hex("07 00 00"), hex("FF 00 01"));
    clip.bind_packet(one_bit);
    wanted.push_back(one_bit);
    check(clip.query_config().packet_triggers == wanted, "the bound triggers");
}

void a_packet_that_cant_exist_fires_nothing_in_the_mock() {
    const std::vector<TrafficClass> surfaces = {TrafficClass::HidIn,      TrafficClass::HidOut,  TrafficClass::VendorInterrupt,
                                                TrafficClass::VendorBulk, TrafficClass::Control, TrafficClass::Emit};
    const MockBox mock;
    const auto device = mock.with_device();
    const auto clip = device.clip();
    for (const auto surface : surfaces) {
        clip.bind_packet(packet(surface, CatchIdAny, Direction::Both, ClipAction::Toggle));
    }
    std::vector<std::pair<TrafficClass, Direction>> no_packet = {
        {TrafficClass::HidIn, Direction::Out}, {TrafficClass::Emit, Direction::Out}, {TrafficClass::HidOut, Direction::In},
        {TrafficClass::Bus, Direction::In},    {TrafficClass::ClipTransfer, Direction::In},
    };
    for (const auto surface : surfaces) {
        for (const auto direction : {Direction::Both, Direction::With, Direction::Against}) {
            no_packet.emplace_back(surface, direction);
        }
    }
    const std::pair<std::optional<ClipAction>, bool> nothing{std::nullopt, false};
    for (const auto& pair : no_packet) {
        check(mock.clip_packet(pair.first, 1, pair.second, {0x07}) == nothing, "a packet that can't exist fired");
    }
    for (const auto& row : clip.query_config().packet_triggers) {
        check(row.hits == 0, "a trigger counted a packet that can't exist");
    }
    const std::pair<std::optional<ClipAction>, bool> toggled{ClipAction::Toggle, false};
    for (const auto& pair : std::vector<std::pair<TrafficClass, Direction>>{
             {TrafficClass::HidIn, Direction::In},           {TrafficClass::HidOut, Direction::Out},
             {TrafficClass::VendorInterrupt, Direction::In}, {TrafficClass::VendorInterrupt, Direction::Out},
             {TrafficClass::VendorBulk, Direction::In},      {TrafficClass::VendorBulk, Direction::Out},
             {TrafficClass::Control, Direction::In},         {TrafficClass::Control, Direction::Out},
             {TrafficClass::Emit, Direction::In}}) {
        check(mock.clip_packet(pair.first, 1, pair.second, {0x07}) == toggled, "a packet that can exist");
    }
    std::vector<int> hits;
    for (const auto& row : clip.query_config().packet_triggers) {
        hits.push_back(row.hits);
    }
    check(hits == std::vector<int>{1, 1, 2, 2, 2, 1}, "the hits");
}

void a_consuming_trigger_needs_the_opt_in_and_goes_when_its_turned_off() {
    const auto consuming = held_trigger();
    const auto watching = packet(TrafficClass::HidIn, 3, Direction::In, ClipAction::Start, hex("07 20"), hex("FF 20"));
    const MockBox mock;
    const auto device = mock.with_device();
    const auto clip = device.clip();
    clip.bind_packet(consuming);
    clip.bind_packet(watching);
    check(clip.query_config().packet_triggers == std::vector<ClipPacketTrigger>{watching}, "without the opt in");
    device.allow_imperfect_clones(true);
    clip.bind_packet(consuming);
    check(clip.query_config().packet_triggers == std::vector<ClipPacketTrigger>{watching, consuming}, "with it");
    device.allow_imperfect_clones(false);
    check(clip.query_config().packet_triggers == std::vector<ClipPacketTrigger>{watching}, "with it turned off");
}

void the_mock_runs_a_packet_through_its_triggers() {
    const auto held = held_trigger();
    const auto let_go = packet(TrafficClass::HidIn, 2, Direction::In, ClipAction::Stop, hex("07 00"), hex("FF 20"),
                               false, true, 1);
    const auto wide = packet(TrafficClass::HidIn, CatchIdAny, Direction::Both, ClipAction::Toggle);
    const MockBox mock;
    mock.set_imperfect_status(allowed);
    const auto device = mock.with_device();
    const auto clip = device.clip();
    for (const auto& trigger : {wide, held, let_go}) {
        clip.bind_packet(trigger);
    }
    using Result = std::pair<std::optional<ClipAction>, bool>;
    const auto run = [&](const Bytes& head) { return mock.clip_packet(TrafficClass::HidIn, 2, Direction::In, head); };
    check(run({0x07, 0x20, 0x55}) == Result{ClipAction::Start, true}, "the run starts");
    check(run({0x07, 0x20, 0x56}) == Result{std::nullopt, true}, "the run goes on");
    check(run({0x09, 0x20}) == Result{ClipAction::Toggle, false}, "the wide one");
    check(run({0x07, 0x20}) == Result{std::nullopt, true}, "the run goes on again");
    check(run({0x07, 0x00}) == Result{ClipAction::Stop, false}, "the let go");
    check(run({0x07, 0x20}) == Result{ClipAction::Start, true}, "the next run");
    check(mock.clip_packet(TrafficClass::HidIn, 5, Direction::In, {0x07, 0x20}) == Result{ClipAction::Toggle, false},
          "another interface");
    check(mock.clip_packet(TrafficClass::HidIn, 5, Direction::In, {}) == Result{ClipAction::Toggle, false},
          "an empty packet");
    check(mock.clip_packet(TrafficClass::HidOut, 2, Direction::Out, {0x07, 0x20}) == Result{std::nullopt, false},
          "another class");
    std::vector<int> hits;
    for (const auto& row : clip.query_config().packet_triggers) {
        hits.push_back(row.hits);
    }
    check(hits == std::vector<int>{3, 4, 1}, "the hits");
    throws<std::invalid_argument>([&] { mock.clip_packet(static_cast<TrafficClass>(200), 2, Direction::In, {}); },
                                  "class 200");
    throws<std::invalid_argument>(
        [&] { mock.clip_packet(static_cast<TrafficClass>(CatchClass::Key), 2, Direction::In, {}); }, "the key class");
    throws<std::invalid_argument>(
        [&] { mock.clip_packet(TrafficClass::HidIn, 2, static_cast<Direction>(200), {}); }, "direction 200");
}

void a_packet_triggers_values_are_checked() {
    const std::vector<ClipPacketTrigger> bad = {
        packet(static_cast<TrafficClass>(200), 2, Direction::In, ClipAction::Start),
        packet(static_cast<TrafficClass>(CatchClass::Key), 2, Direction::In, ClipAction::Start),
        packet(TrafficClass::HidIn, 2, static_cast<Direction>(200), ClipAction::Start),
    };
    const MockBox mock;
    const auto device = mock.with_device();
    const auto clip = device.clip();
    for (const auto& trigger : bad) {
        throws<std::invalid_argument>([&] { clip.bind_packet(trigger); }, "a bad bind");
        throws<std::invalid_argument>([&] { clip.unbind_packet(trigger); }, "a bad unbind");
        ClipSettings settings;
        settings.packet_triggers = {trigger};
        throws<std::invalid_argument>([&] { mock.set_clip_settings(settings); }, "a bad mock trigger");
    }
    throws<std::invalid_argument>(
        [&] { clip.bind_packet(packet(TrafficClass::HidIn, 2, Direction::In, static_cast<ClipAction>(200))); },
        "action 200");
    check(frames(mock, FrameType::ClipTrigger).empty(), "a bad trigger went out");
    // an unbind sends the key alone, so a bad action next to it doesn't matter
    clip.unbind_packet(packet(TrafficClass::HidIn, 2, Direction::In, static_cast<ClipAction>(200)));
    check(frames(mock, FrameType::ClipTrigger).size() == 1, "the unbind with a bad action didn't go out");
}

void the_clip_enums_are_checked() {
    const MockBox mock;
    {
        const auto device = mock.with_device();
        const auto clip = device.clip();
        ClipBuilder builder;
        throws<std::invalid_argument>([&] { clip.unbind(Button::LEFT, static_cast<Edge>(200)); }, "unbind on 200");
        throws<std::invalid_argument>(
            [&] { clip.bind(input_trigger(Button::LEFT, static_cast<Edge>(200), ClipAction::Start)); }, "bind on 200");
        throws<std::invalid_argument>(
            [&] { clip.bind(input_trigger(Button::LEFT, Edge::Press, static_cast<ClipAction>(200))); }, "action 200");
        throws<std::invalid_argument>([&] { clip.set_autolock({Blanket::Aim, static_cast<Blanket>(99)}); },
                                      "blanket 99");
        throws<std::invalid_argument>([&] { builder.edge(Button::LEFT, static_cast<Action>(200)); }, "an edge on 200");
        throws<std::invalid_argument>(
            [&] { builder.frame(make_frame(0, 0, 0, 0, {{Button::LEFT, static_cast<Action>(200)}})); },
            "a frame's edge on 200");
    }
    check(frames(mock, FrameType::ClipTrigger).empty() && frames(mock, FrameType::ClipSet).empty(),
          "a refused value went out");
}

void a_medius_gives_its_own_device_that_shares_the_connection() {
    auto opened = anybox::Mock::medius();
    auto& device = opened.first;
    {
        const auto medius = device.medius();
        check(medius.has_value(), "the medius device");
        check(medius && medius->query_version().proto_ver > 0, "the medius version");
    }
    // the Medius device going leaves the anybox one working
    device.move_by(1, 1);
    device.close();
    check(!device.medius(), "medius() after close");
}

void the_medius_device_outlives_the_anybox_one_on_its_handle() {
    auto opened = anybox::Mock::medius();
    std::optional<anybox::Device> device(std::move(opened.first));
    auto medius = must(device->medius(), "the medius device");
    device.reset();
    // it keeps the connection, so it still answers after the anybox device is gone
    check(medius.query_version().proto_ver > 0, "the medius device after the anybox one went");
    { Device gone = std::move(medius); }
    throws<anybox::ClosedError>([&] { medius.query_version(); }, "a moved from medius device");
}

// gen-medius fails on a medius.h function its table doesn't have, and this checks the headers really call each one,
// but the three that are done another way on purpose
void every_header_function_is_bound() {
    std::string header;
    for (const auto& line : header_lines()) {
        header += line.substr(0, line.find("//")) + "\n";
    }
    std::vector<std::string> declared;
    for (std::size_t at = header.find("medius_"); at != std::string::npos; at = header.find("medius_", at + 1)) {
        if (at > 0 && (std::isalnum(static_cast<unsigned char>(header[at - 1])) || header[at - 1] == '_')) {
            continue;
        }
        auto end = at;
        while (end < header.size() && (std::isalnum(static_cast<unsigned char>(header[end])) || header[end] == '_')) {
            end++;
        }
        if (end >= header.size() || header[end] != '(') {
            continue;
        }
        int depth = 0;
        auto close = end;
        for (; close < header.size(); close++) {
            depth += header[close] == '(' ? 1 : header[close] == ')' ? -1 : 0;
            if (depth == 0) {
                break;
            }
        }
        if (close + 1 < header.size() && header[close + 1] == ';') {
            declared.push_back(header.substr(at, end - at));
        }
    }
    std::sort(declared.begin(), declared.end());
    declared.erase(std::unique(declared.begin(), declared.end()), declared.end());
    check(declared.size() == 258, std::to_string(declared.size()) + " functions in medius.h");
    // the code of both headers without their comments, which name functions they don't call
    std::string wrapper;
    for (const auto* path : {"bindings/c/include/anybox_medius.hpp", "bindings/c/include/anybox.hpp"}) {
        std::ifstream file(path);
        for (std::string line; std::getline(file, line);) {
            wrapper += line.substr(0, line.find("//")) + "\n";
        }
    }
    check(wrapper.size() > 100000, "the headers couldn't be read, run it from the repo root");
    const auto in_name = [&](std::size_t at) {
        return at < wrapper.size() && (std::isalnum(static_cast<unsigned char>(wrapper[at])) || wrapper[at] == '_');
    };
    const auto used = [&](const std::string& name) {
        for (auto at = wrapper.find(name); at != std::string::npos; at = wrapper.find(name, at + 1)) {
            if ((at == 0 || !in_name(at - 1)) && !in_name(at + name.size())) {
                return true;
            }
        }
        return false;
    };
    // update_firmware is stage, activate and abort like the crate's own, and the two is_held look through the list
    const std::set<std::string> other_way = {"medius_device_update_firmware", "medius_usage_event_is_held",
                                             "medius_clip_status_is_held"};
    for (const auto& name : declared) {
        const bool done_another_way = other_way.count(name) == 1;
        check(used(name) != done_another_way, name + (done_another_way ? " is used after all" : " isn't used"));
    }
}

// these three have no C# test to port, they hold what the C++ side does on its own
void a_lock_target_has_a_usage_only_on_a_usage() {
    check(!LockTarget(Axis::Wheel).input(), "an axis has a usage");
    check(LockTarget(Key::A).input() == std::optional<Usage>(Usage(Key::A)), "a key's usage");
    check(LockTarget(Axis::Pan).kind() == LockTargetKind::Pan, "pan's kind");
}

void a_payload_as_long_as_its_array_goes_in() {
    check(Patch{PatchSection::Device, 0, 0, 0, Bytes(512)}.to_c().len == 512, "a 512 byte patch");
    ClipSettings settings;
    settings.autolock = {Blanket::Media, static_cast<Blanket>(9)};
    throws<std::invalid_argument>([&] { settings.to_c(); }, "an autolock blanket nothing names");
}

void a_for_loop_ends_when_a_clone_closes_the_stream() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto stream = device.catch_events({CatchFilter::everything()});
    const auto clone = stream.clone();
    stream.close();
    int read = 0;
    for (const auto& event : clone) {
        (void)event;
        read++;
    }
    check(read == 0, "the clone read " + std::to_string(read) + " events after the close");
}

void an_unknown_value_from_the_library_is_an_error() {
    const auto unknown = [](const std::string& name) {
        return "the library sent a " + name +
               " this header doesn't know (200), so build against the anybox.hpp that came with the library";
    };
    const auto refused = [&](const std::string& name, const std::function<void()>& decode) {
        try {
            decode();
            check(false, "an unknown " + name + " came through");
        } catch (const anybox::Error& error) {
            check(error.what() == unknown(name), error.what());
        }
    };
    refused("Class", [] {
        MediusUsage raw{};
        raw.kind = 200;
        Usage::from_c(raw);
    });
    refused("LogLevel", [] {
        MediusLogLine raw{};
        raw.level = static_cast<MediusLogLevel>(200);
        LogLine::from_c(raw);
    });
    refused("ClipState", [] {
        MediusClipStatus raw{};
        raw.state = 200;
        ClipStatus::from_c(raw);
    });
    refused("RenderMode", [] {
        MediusRenderStatus raw{};
        raw.mode = static_cast<MediusRenderMode>(200);
        RenderStatus::from_c(raw);
    });
    refused("CatchEventKind", [] {
        MediusCatchEvent raw{};
        raw.kind = static_cast<MediusCatchEventKind>(200);
        CatchEvent::from_c(raw);
    });
    refused("ClockDomain", [] {
        MediusCatchEvent raw{};
        raw.kind = MEDIUS_CATCH_EVENT_KIND_MOTION;
        raw.clock = static_cast<MediusClockDomain>(200);
        CatchEvent::from_c(raw);
    });
}

void the_image_states_and_update_targets_are_the_headers() {
    std::string states;
    bool chips = false;
    for (const auto& line : header_lines()) {
        const auto at = line.find("// 0 new, 1 pending-verify");
        if (at != std::string::npos) {
            states = line.substr(at + 3, line.rfind('.') - at - 3);
        }
        chips = chips || line.find("(0 = device chip, 1 = host chip)") != std::string::npos;
    }
    const std::map<std::string, ImageState> names = {
        {"new", ImageState::New},         {"pending-verify", ImageState::PendingVerify},
        {"valid", ImageState::Valid},     {"invalid", ImageState::Invalid},
        {"aborted", ImageState::Aborted}, {"unknown", ImageState::Unknown},
    };
    std::size_t named = 0;
    for (std::size_t start = 0; start < states.size();) {
        auto end = states.find(", ", start);
        end = end == std::string::npos ? states.size() : end;
        const auto part = states.substr(start, end - start);
        const auto space = part.find(' ');
        const auto number = std::stoi(part.substr(0, space), nullptr, part.rfind("0x", 0) == 0 ? 16 : 10);
        const auto name = names.find(part.substr(space + 1));
        check(name != names.end() && static_cast<int>(name->second) == number, "the image state " + part);
        named++;
        start = end + 2;
    }
    check(named == names.size(), std::to_string(named) + " image states in medius.h");
    check(chips, "medius.h doesn't say which chip is which");
    check(static_cast<int>(UpdateTarget::Device) == 0 && static_cast<int>(UpdateTarget::Host) == 1, "the targets");
}

void the_firmware_info_reads_both_chips() {
    MediusFirmwareInfo raw{};
    raw.device.major = 1;
    raw.device.minor = 2;
    raw.device.patch = 3;
    raw.device.slot = 1;
    raw.device.state = 1;
    raw.host_present = 1;
    raw.host.major = 4;
    raw.host.minor = 5;
    raw.host.patch = 6;
    raw.host.slot = 0;
    raw.host.state = 0x42;
    raw.slot_size = 0x1F0000;
    raw.host_staged = 1;
    const auto info = FirmwareInfo::from_c(raw);
    check(info == FirmwareInfo{ChipFirmware{1, 2, 3, 1, ImageState::PendingVerify},
                               ChipFirmware{4, 5, 6, 0, static_cast<ImageState>(0x42)}, 0x1F0000, false, true},
          "the firmware info");
    check(info.any_pending(), "nothing pending");
    check(to_string(info.device) == "1.2.3 on ota_1 (pending-verify)", "the device chip is " + to_string(info.device));
    check(to_string(*info.host) == "4.5.6 on ota_0 (unknown(66))", "the host chip is " + to_string(*info.host));
    check(to_string(ChipFirmware{0, 0, 0, 0, ImageState::Unknown}) == "0.0.0 on ota_0 (unknown(255))",
          "a state named unknown");
    auto no_host = raw;
    no_host.host_present = 0;
    check(!FirmwareInfo::from_c(no_host).host, "a host that isn't there");
    auto valid = raw;
    valid.device.state = 2;
    check(!FirmwareInfo::from_c(valid).any_pending(), "a valid chip is pending");
    auto host_pending = valid;
    host_pending.host.state = 1;
    check(FirmwareInfo::from_c(host_pending).any_pending(), "a host chip on probation isn't pending");
}

void the_keys_and_catch_classes_are_the_headers() {
    std::map<std::string, int> defines;
    for (const auto& line : header_lines()) {
        const std::string define = "#define MEDIUS_";
        const auto space = line.find(' ', define.size());
        if (line.rfind(define, 0) != 0 || space == std::string::npos) {
            continue;
        }
        const auto value = line.substr(space + 1);
        if (!value.empty() && std::all_of(value.begin(), value.end(), [](unsigned char c) { return std::isdigit(c); })) {
            defines[line.substr(8, space - 8)] = std::stoi(value);
        }
    }
    check(defines["MEDIUS_KEY_A"] == Key::A.value && defines["MEDIUS_KEY_1"] == Key::NUM1.value, "the keys");
    check(defines["MEDIUS_MEDIA_VOLUME_UP"] == MediaKey::VOLUME_UP.value, "the media keys");
    check(defines["MEDIUS_CATCH_CLASS_CLIP_TRANSFER"] == static_cast<int>(CatchClass::ClipTransfer) &&
              static_cast<int>(CatchClass::ClipTransfer) == static_cast<int>(TrafficClass::ClipTransfer),
          "the clip transfer class");
    check(defines["MEDIUS_CLIP_PKT_TRIG_MAX"] == ClipPktTrigMax && defines["MEDIUS_MAX_PKT_MATCH"] == MaxPktMatch,
          "the clip limits");
}

// the second unit includes windows.h first, so every public name here has to live with its macros
void no_public_name_clashes_with_windows_h_macros() {
    const auto wanted = std::to_string(MaxPath) + " " + std::to_string(Key::A.value);
    check(medius_from_the_second_unit() == wanted, "the second unit got " + medius_from_the_second_unit());
}

void closing_a_stream_from_another_thread_ends_its_reader() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto events = device.catch_events({CatchFilter::everything()});
    const auto inputs = device.input_events(CatchFilter::all_input());
    const auto logs = device.logs();
    const auto timed_events = device.catch_events({CatchFilter::everything()});
    const auto timed_inputs = device.input_events(CatchFilter::all_input());
    const auto timed_logs = device.logs();
    std::atomic<int> closed{0};
    std::atomic<int> ended{0};
    const auto reader = [&](const std::function<void()>& read) {
        return std::thread([&closed, &ended, read] {
            try {
                read();
            } catch (const anybox::ClosedError&) {
                closed++;
            } catch (...) {
            }
            ended++;
        });
    };
    std::vector<std::thread> readers;
    readers.push_back(reader([&] { events.recv(); }));
    readers.push_back(reader([&] { inputs.recv(); }));
    readers.push_back(reader([&] { logs.recv(); }));
    readers.push_back(reader([&] { timed_events.recv(10s); }));
    readers.push_back(reader([&] { timed_inputs.recv(10s); }));
    readers.push_back(reader([&] { timed_logs.recv(10s); }));
    // a reader that isn't waiting yet just finds it closed, which ends it too
    std::this_thread::sleep_for(200ms);
    events.close();
    inputs.close();
    logs.close();
    timed_events.close();
    timed_inputs.close();
    timed_logs.close();
    if (!ended_in_time(readers, ended)) {
        check(false, "a close didn't wake its reader");
        return;
    }
    check(closed == 6, std::to_string(closed) + " readers got ClosedError");
    for (const auto& event : events) {
        (void)event;
        check(false, "a closed stream gave an event");
    }
    events.close();
    logs.close();
}

// like the crate, a close ends the subscription for every clone on it, while a clone that only goes away frees its
// own handle and leaves the others reading
void closing_one_clone_ends_the_others_and_dropping_one_doesnt() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto events = device.catch_events({CatchFilter::everything()});
    const auto logs = device.logs();
    const auto other = device.logs();
    const auto events_clone = events.clone();
    const auto logs_clone = logs.clone();
    std::atomic<int> disconnected{0};
    std::atomic<int> ended{0};
    std::vector<std::thread> readers;
    readers.emplace_back([&] {
        try {
            events_clone.recv();
        } catch (const DisconnectedError&) {
            disconnected++;
        }
        ended++;
    });
    readers.emplace_back([&] {
        try {
            logs_clone.recv();
        } catch (const DisconnectedError&) {
            disconnected++;
        }
        ended++;
    });
    std::this_thread::sleep_for(200ms);
    events.close();
    logs.close();
    if (!ended_in_time(readers, ended)) {
        check(false, "a close didn't wake a clone's reader");
        return;
    }
    check(disconnected == 2, std::to_string(disconnected) + " clones got DisconnectedError");
    check(!events_clone.is_connected() && !logs_clone.is_connected(), "a clone of a closed stream is connected");
    check(!events_clone.try_recv(), "a clone of a closed stream gave an event");
    // well under the 5 s it waits for a line that could still come
    const auto start = std::chrono::steady_clock::now();
    check(!logs_clone.recv(5s), "a clone of a closed log gave a line");
    check(since(start) < 2s, "a closed stream's timed wait took " + std::to_string(since(start).count()) + " ms");
    // another log of the same device keeps going
    check(other.is_connected(), "the other log isn't connected");
    mock.push_log(LogLevel::Info, "still here");
    const auto line = other.recv(2s);
    check(line && line->text == "still here", "the other log's line");

    const auto kept = device.logs();
    { const auto gone = kept.clone(); }
    check(kept.is_connected(), "a clone going away closed its original");
    mock.push_log(LogLevel::Info, "after the clone went");
    const auto after = kept.recv(2s);
    check(after && after->text == "after the clone went", "the line after the clone went");
}

// the library takes the input stream shared, so its other calls go in while a receive is blocked on it
void an_input_stream_answers_while_another_thread_waits_on_it() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto inputs = device.input_events(CatchFilter::all_input());
    std::atomic<bool> closed{false};
    std::thread reader([&] {
        try {
            inputs.recv();
        } catch (const anybox::ClosedError&) {
            closed = true;
        }
    });
    std::this_thread::sleep_for(200ms);
    auto asked = std::async(std::launch::async, [&] {
        return std::make_tuple(inputs.held(Class::Button).size(), inputs.dropped(), inputs.is_connected());
    });
    check(asked.wait_for(2s) == std::future_status::ready, "the calls waited on the receive");
    check(asked.get() == std::make_tuple(std::size_t{0}, std::uint64_t{0}, true), "what the stream said");
    inputs.close();
    reader.join();
    check(closed, "the reader didn't get ClosedError");
}

// the handle is freed too, so every other call says the stream is gone
template <class Stream>
void close_and_check(const Stream& stream, const std::string& kind) {
    stream.close();
    throws<anybox::ClosedError>([&] { stream.is_connected(); }, kind + " is_connected()");
    throws<anybox::ClosedError>([&] { stream.try_recv(); }, kind + " try_recv()");
    throws<anybox::ClosedError>([&] { stream.recv(); }, kind + " recv()");
    throws<anybox::ClosedError>([&] { stream.recv(1s); }, kind + " recv(1s)");
    for (const auto& item : stream) {
        (void)item;
        check(false, "a closed " + kind + " gave something");
    }
}

void a_closed_stream_says_so() {
    const MockBox mock;
    const auto device = mock.with_device();
    const auto events = device.catch_events({CatchFilter::everything()});
    close_and_check(events, "event stream");
    throws<anybox::ClosedError>([&] { events.dropped(); }, "event stream dropped()");
    const auto inputs = device.input_events(CatchFilter::all_input());
    close_and_check(inputs, "input stream");
    throws<anybox::ClosedError>([&] { inputs.dropped(); }, "input stream dropped()");
    close_and_check(device.logs(), "log");
}

// C# watches the device get collected, here the holder has to still work once the last Device is gone
void every_stream_and_clip_keeps_its_device_alive() {
    const MockBox mock;
    const auto motion_comes = [&](const auto& stream, const std::string& kind) {
        mock.push_motion(1, 1000, MotionEvent{1, 0, 0, 0});
        check(stream.recv(wait).has_value(), "the " + kind + " lost its device");
    };
    const auto line_comes = [&](const LogStream& logs, const std::string& kind) {
        mock.push_log(LogLevel::Info, kind);
        const auto line = logs.recv(wait);
        check(line && line->text == kind, "the " + kind + " lost its device");
    };
    {
        const auto events = mock.with_device().catch_events({CatchFilter::everything()});
        motion_comes(events, "events");
    }
    {
        const auto inputs = mock.with_device().input_events(CatchFilter::all_input());
        motion_comes(inputs, "inputs");
    }
    {
        const auto logs = mock.with_device().logs();
        line_comes(logs, "logs");
    }
    {
        const auto clip = mock.with_device().clip();
        mock.clear_recorded();
        clip.start();
        check(frames(mock, FrameType::ClipCtrl).size() == 1, "the clip lost its device");
    }
    {
        std::optional<EventStream> original = mock.with_device().catch_events({CatchFilter::everything()});
        const auto clone = original->clone();
        original.reset();
        motion_comes(clone, "events clone");
    }
    {
        std::optional<LogStream> original = mock.with_device().logs();
        const auto clone = original->clone();
        original.reset();
        line_comes(clone, "logs clone");
    }
}

void a_staged_update_reports_its_progress() {
    const auto image = range(64 * 1024);
    std::vector<UpdateProgress> seen;
    const MockBox mock;
    const auto device = mock.open();
    device.stage_firmware(UpdateTarget::Device, image,
                          [&](const UpdateProgress& progress) { seen.push_back(progress); });
    check(!seen.empty(), "no progress came");
    if (!seen.empty()) {
        check(seen.back().total == image.size() && seen.back().sent == seen.back().total &&
                  seen.back().target == UpdateTarget::Device && seen.back().percent() == 100,
              "the last progress");
    }
    for (std::size_t index = 1; index < seen.size(); index++) {
        check(seen[index - 1].sent <= seen[index].sent, "the progress went back");
    }
    check(device.firmware_info().device_staged, "nothing staged");
    device.abort_update(UpdateTarget::Device);
    // no progress is fine too
    device.stage_firmware(UpdateTarget::Device, image);
    // medius refuses it itself, like every other value it can't take
    throws<InvalidArgError>([&] { device.stage_firmware(UpdateTarget::Device, {}); }, "an empty image");
}

// a box rebooting into its new image doesn't answer for a while, so the wait asks again like the crate's instead of
// failing on the first query that timed out
void waiting_for_the_firmware_rides_out_the_reboot() {
    const auto image = range(4096);
    const MockBox mock;
    const auto device = mock.open();
    check(!device.wait_firmware_confirmed().any_pending(), "a new box is pending");
    mock.reboot_after_activate(1, 2);
    device.update_firmware(UpdateTarget::Device, image);
    const auto before = mock.recorded();
    check(!device.wait_firmware_confirmed().any_pending(), "the box is still pending");
    // one left unanswered by the reboot, two with the chip on probation, then the confirmed one, like the crate
    const std::uint8_t firmware_query = 11;
    int asked = 0;
    for (auto index = before; index < mock.recorded(); index++) {
        const auto frame = mock.recorded_frame(index);
        if (frame && frame->type == FrameType::Query && !frame->payload.empty() && frame->payload[0] == firmware_query) {
            asked++;
        }
    }
    check(asked == 4, std::to_string(asked) + " firmware queries");
}

void a_timeout_goes_to_the_library_rounded_up() {
    const std::vector<std::pair<std::chrono::microseconds, std::uint64_t>> cases = {
        {0us, 0},
        {1us, 1},
        {1000us, 1},
        {1001us, 2},
        {1s, 1000},
        {std::chrono::microseconds::max(), static_cast<std::uint64_t>(std::chrono::microseconds::max().count() / 1000) + 1},
    };
    for (const auto& entry : cases) {
        check(detail::millis(entry.first) == entry.second, std::to_string(entry.first.count()) + " us");
    }
}

void a_negative_timeout_is_refused() {
    throws<std::invalid_argument>([] { detail::millis(-1us); }, "a negative timeout");
}

// after the crate's own update tests
void a_refused_update_says_which_op_and_why() {
    const auto image = range(4096);
    const MockBox mock;
    const auto device = mock.open();
    try {
        device.activate_firmware();
        check(false, "an activate with nothing staged went through");
    } catch (const UpdateError& error) {
        check(error.op() == 4 && error.update_status() == 0x19 && error.arg(), "nothing staged");
    }
    // a refused activate aborts what it staged, so nothing can boot it later
    const std::uint8_t link_down = 0x17;
    mock.refuse_next_activate(link_down);
    try {
        device.update_firmware(UpdateTarget::Host, image);
        check(false, "a refused activate went through");
    } catch (const UpdateError& error) {
        check(error.op() == 4 && error.update_status() == link_down, "the link that's down");
    }
    const auto info = device.firmware_info();
    check(!info.host_staged && !info.device_staged, "the refused image is still staged");
    // only an update error has the detail, and one made without it has none
    throws<InvalidArgError>([&] { device.stage_firmware(UpdateTarget::Device, {}); }, "an empty image");
    const UpdateError bare("text", 0, std::nullopt);
    check(!bare.op() && !bare.update_status() && !bare.arg(), "an update error with no detail");
}

void a_progress_that_throws_aborts_the_update_and_comes_out() {
    const auto image = range(64 * 1024);
    int calls = 0;
    const MockBox mock;
    const auto device = mock.open();
    throws<std::runtime_error>(
        [&] {
            device.stage_firmware(UpdateTarget::Device, image, [&](const UpdateProgress&) {
                calls++;
                throw std::runtime_error("the handler broke");
            });
        },
        "a progress that throws", "the handler broke");
    check(calls == 1, "the progress was called " + std::to_string(calls) + " times");
    check(!device.firmware_info().device_staged, "the update wasn't aborted");
}

}  // namespace

int main() {
    const std::vector<std::pair<const char*, std::function<void()>>> tests = {
        {"the mock is there", the_mock_is_there},
        {"the meta functions answer", the_meta_functions_answer},
        {"the abi version is the headers", the_abi_version_is_the_headers},
        {"a library with another medius abi is refused", a_library_with_another_medius_abi_is_refused},
        {"a set version comes back on open", a_set_version_comes_back_on_open},
        {"a name can be set and cleared", a_name_can_be_set_and_cleared},
        {"a silent mock fails the open", a_silent_mock_fails_the_open},
        {"a bad protocol version says its status and protocol", a_bad_protocol_version_says_its_status_and_protocol},
        {"a listed box on another protocol has no device", a_listed_box_on_another_protocol_has_no_device},
        {"a recorded frames payload can be read", a_recorded_frames_payload_can_be_read},
        {"move frames carry their riding flags", move_frames_carry_their_riding_flags},
        {"the other motion calls send their frames", the_other_motion_calls_send_their_frames},
        {"caps come back as they were set", caps_come_back_as_they_were_set},
        {"the rate comes back with its native hz", the_rate_comes_back_with_its_native_hz},
        {"locks come back and say whats locked", locks_come_back_and_say_whats_locked},
        {"locks carry a scale not just a lock", locks_carry_a_scale_not_just_a_lock},
        {"scale and bearing reach the device", scale_and_bearing_reach_the_device},
        {"the boxs lock table answers query locks", the_boxs_lock_table_answers_query_locks},
        {"a key blanket reports the edges it blocks", a_key_blanket_reports_the_edges_it_blocks},
        {"a one bit class stores the block or pass it amounts to", a_one_bit_class_stores_the_block_or_pass_it_amounts_to},
        {"a media lock is sent and reported as both", a_media_lock_is_sent_and_reported_as_both},
        {"a relative direction needs a bearing and only an axis has one",
         a_relative_direction_needs_a_bearing_and_only_an_axis_has_one},
        {"values the enums dont name are refused before they cross",
         values_the_enums_dont_name_are_refused_before_they_cross},
        {"the mocks locks are checked entry by entry", the_mocks_locks_are_checked_entry_by_entry},
        {"a catch filters values are checked", a_catch_filters_values_are_checked},
        {"the mocks bearing answers query bearing", the_mocks_bearing_answers_query_bearing},
        {"the defaults are the crates own", the_defaults_are_the_crates_own},
        {"relative directions say so", relative_directions_say_so},
        {"health comes back", health_comes_back},
        {"device info comes back", device_info_comes_back},
        {"a clone with no hid interface is cloned", a_clone_with_no_hid_interface_is_cloned},
        {"stats come back", stats_come_back},
        {"the catch state comes back", the_catch_state_comes_back},
        {"a never measured clock isnt a zero age", a_never_measured_clock_isnt_a_zero_age},
        {"catch filter wildcards survive the round trip", catch_filter_wildcards_survive_the_round_trip},
        {"imperfect status comes back", imperfect_status_comes_back},
        {"movement riding comes back", movement_riding_comes_back},
        {"the emit pace comes back", the_emit_pace_comes_back},
        {"the spread comes back", the_spread_comes_back},
        {"the render mode comes back", the_render_mode_comes_back},

        {"the status codes are the headers", the_status_codes_are_the_headers},
        {"every status has its exception", every_status_has_its_exception},
        {"input events turn snapshots into edges", input_events_turn_snapshots_into_edges},
        {"input events refuse what they cant decode", input_events_refuse_what_they_cant_decode},
        {"the filter factories address inputs like lock does", the_filter_factories_address_inputs_like_lock_does},
        {"an unknown control status doesnt throw", an_unknown_control_status_doesnt_throw},
        {"a timeline unwraps the rollover onto your clock", a_timeline_unwraps_the_rollover_onto_your_clock},
        {"a timeline places input events too", a_timeline_places_input_events_too},
        {"every enum going in is checked before it crosses", every_enum_going_in_is_checked_before_it_crosses},
        {"the mock and stream enums are checked", the_mock_and_stream_enums_are_checked},
        {"the new health bits come back", the_new_health_bits_come_back},
        {"the advanced frames carry their type", the_advanced_frames_carry_their_type},
        {"raw goes out as it is", raw_goes_out_as_it_is},
        {"the gated calls need the opt in", the_gated_calls_need_the_opt_in},
        {"raw refuses a direction that isnt a flow", raw_refuses_a_direction_that_isnt_a_flow},
        {"a transfer brings back its answer", a_transfer_brings_back_its_answer},
        {"a transfer is refused without the opt in", a_transfer_is_refused_without_the_opt_in},
        {"a transfer that isnt ok carries no data", a_transfer_that_isnt_ok_carries_no_data},
        {"a rewrite survives the query", a_rewrite_survives_the_query},
        {"a rewrites refusals have their own exception", a_rewrites_refusals_have_their_own_exception},
        {"a rewrite match past the limit has its own exception", a_rewrite_match_past_the_limit_has_its_own_exception},
        {"a rewrite past the payload pool has its own exception", a_rewrite_past_the_payload_pool_has_its_own_exception},
        {"a patch survives the query", a_patch_survives_the_query},
        {"apply and clear patch reach the box", apply_and_clear_patch_reach_the_box},
        {"an empty patch removes the stored one", an_empty_patch_removes_the_stored_one},
        {"pan frames carry the motion tag", pan_frames_carry_the_motion_tag},
        {"buttons past five go by their id", buttons_past_five_go_by_their_id},
        {"the transform calls reach the box without the opt in", the_transform_calls_reach_the_box_without_the_opt_in},
        {"a transform survives the query", a_transform_survives_the_query},
        {"a pan axis transform works like any other", a_pan_axis_transform_works_like_any_other},
        {"a transforms refusals have their own exception", a_transforms_refusals_have_their_own_exception},
        {"a negative scale reverses and is refused where it cant", a_negative_scale_reverses_and_is_refused_where_it_cant},
        {"an input event carries pan", an_input_event_carries_pan},
        {"a mock restart is recovered and the clip says it was lost",
         a_mock_restart_is_recovered_and_the_clip_says_it_was_lost},
        {"a clone presented again gets back what the library held",
         a_clone_presented_again_gets_back_what_the_library_held},
        {"a release the box counts is recovered", a_release_the_box_counts_is_recovered},

        {"a forced rate comes back", a_forced_rate_comes_back},
        {"a forced rate needs the imperfect opt in", a_forced_rate_needs_the_imperfect_opt_in},
        {"a forced rate goes through the setter", a_forced_rate_goes_through_the_setter},
        {"the counters can be read", the_counters_can_be_read},
        {"a catch delivers motion", a_catch_delivers_motion},
        {"a catch delivers a key", a_catch_delivers_a_key},
        {"a catch delivers media", a_catch_delivers_media},
        {"a catch delivers traffic", a_catch_delivers_traffic},
        {"a true len above the capture is a truncation", a_true_len_above_the_capture_is_a_truncation},
        {"a control events accessors read its bytes", a_control_events_accessors_read_its_bytes},
        {"a clip transfers accessors read its bytes", a_clip_transfers_accessors_read_its_bytes},
        {"a bus event is decoded", a_bus_event_is_decoded},
        {"traffic comes on every way of receiving", traffic_comes_on_every_way_of_receiving},
        {"catch events needs a filter", catch_events_needs_a_filter},
        {"try recv gives null when empty", try_recv_gives_null_when_empty},
        {"a log line comes", a_log_line_comes},
        {"a clone shares the connection", a_clone_shares_the_connection},
        {"a stream clone shares the subscription", a_stream_clone_shares_the_subscription},
        {"disposing twice is fine", disposing_twice_is_fine},
        {"a snapshots is held takes every class", a_snapshots_is_held_takes_every_class},

        {"clip control frames are the protocols bytes", clip_control_frames_are_the_protocols_bytes},
        {"an append encodes and chunks", an_append_encodes_and_chunks},
        {"a frames edges are encoded", a_frames_edges_are_encoded},
        {"a frame carries pan raw reports and transfers", a_frame_carries_pan_raw_reports_and_transfers},
        {"one field pan raw and transfer frames", one_field_pan_raw_and_transfer_frames},
        {"byte len is what an append puts in the ring", byte_len_is_what_an_append_puts_in_the_ring},
        {"a refused frame throws its own exception and sends nothing",
         a_refused_frame_throws_its_own_exception_and_sends_nothing},
        {"a bad frame field leaves the builder as it was", a_bad_frame_field_leaves_the_builder_as_it_was},
        {"a gap of zero does nothing", a_gap_of_zero_does_nothing},
        {"the clip status and config come back", the_clip_status_and_config_come_back},
        {"the autolock bits are the boxs", the_autolock_bits_are_the_boxs},
        {"a packet trigger goes out as the bytes the protocol names",
         a_packet_trigger_goes_out_as_the_bytes_the_protocol_names},
        {"packet triggers read back field for field", packet_triggers_read_back_field_for_field},
        {"each length of a read back trigger is its own", each_length_of_a_read_back_trigger_is_its_own},
        {"a consuming trigger scripted with the opt in off is left out",
         a_consuming_trigger_scripted_with_the_opt_in_off_is_left_out},
        {"a bound packet trigger reads back as it was bound", a_bound_packet_trigger_reads_back_as_it_was_bound},
        {"settings past the arrays are refused", settings_past_the_arrays_are_refused},
        {"clear triggers removes both kinds", clear_triggers_removes_both_kinds},
        {"a packet trigger the box would refuse has its own exception",
         a_packet_trigger_the_box_would_refuse_has_its_own_exception},
        {"a trigger on a direction its class never carries is refused",
         a_trigger_on_a_direction_its_class_never_carries_is_refused},
        {"a trigger with a match bit outside its mask is refused", a_trigger_with_a_match_bit_outside_its_mask_is_refused},
        {"a once per run trigger with no masked bit past its selector is refused",
         a_once_per_run_trigger_with_no_masked_bit_past_its_selector_is_refused},
        {"a packet that cant exist fires nothing in the mock", a_packet_that_cant_exist_fires_nothing_in_the_mock},
        {"a consuming trigger needs the opt in and goes when its turned off",
         a_consuming_trigger_needs_the_opt_in_and_goes_when_its_turned_off},
        {"the mock runs a packet through its triggers", the_mock_runs_a_packet_through_its_triggers},
        {"a packet triggers values are checked", a_packet_triggers_values_are_checked},
        {"the clip enums are checked", the_clip_enums_are_checked},

        {"a medius gives its own device that shares the connection",
         a_medius_gives_its_own_device_that_shares_the_connection},
        {"the medius device outlives the anybox one on its handle",
         the_medius_device_outlives_the_anybox_one_on_its_handle},
        {"every header function is bound", every_header_function_is_bound},
        {"a lock target has a usage only on a usage", a_lock_target_has_a_usage_only_on_a_usage},
        {"a payload as long as its array goes in", a_payload_as_long_as_its_array_goes_in},
        {"a for loop ends when a clone closes the stream", a_for_loop_ends_when_a_clone_closes_the_stream},
        {"an unknown value from the library is an error", an_unknown_value_from_the_library_is_an_error},
        {"the image states and update targets are the headers", the_image_states_and_update_targets_are_the_headers},
        {"the firmware info reads both chips", the_firmware_info_reads_both_chips},
        {"the keys and catch classes are the headers", the_keys_and_catch_classes_are_the_headers},
        {"no public name clashes with windows.h's macros", no_public_name_clashes_with_windows_h_macros},
        {"closing a stream from another thread ends its reader", closing_a_stream_from_another_thread_ends_its_reader},
        {"closing one clone ends the others and dropping one doesnt",
         closing_one_clone_ends_the_others_and_dropping_one_doesnt},
        {"an input stream answers while another thread waits on it",
         an_input_stream_answers_while_another_thread_waits_on_it},
        {"a closed stream says so", a_closed_stream_says_so},
        {"every stream and clip keeps its device alive", every_stream_and_clip_keeps_its_device_alive},
        {"a staged update reports its progress", a_staged_update_reports_its_progress},
        {"waiting for the firmware rides out the reboot", waiting_for_the_firmware_rides_out_the_reboot},
        {"a timeout goes to the library rounded up", a_timeout_goes_to_the_library_rounded_up},
        {"a negative timeout is refused", a_negative_timeout_is_refused},
        {"a refused update says which op and why", a_refused_update_says_which_op_and_why},
        {"a progress that throws aborts the update and comes out",
         a_progress_that_throws_aborts_the_update_and_comes_out},
    };
    for (const auto& test : tests) {
        const auto before = failures;
        try {
            test.second();
        } catch (const std::exception& error) {
            check(false, std::string("threw ") + error.what());
        }
        std::printf("%s: %s\n", failures == before ? "ok" : "FAILED", test.first);
    }
    if (failures > 0) {
        std::printf("medius: %d failed\n", failures);
        return 1;
    }
    std::printf("medius: all %zu passed\n", tests.size());
    return 0;
}
