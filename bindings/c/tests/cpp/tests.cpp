// the C++ wrapper's own tests, for what the conformance suite doesn't reach: its values and refusals, the
// conversions on C structs made up to be wrong, the connect options it builds, and with the mock the lifetimes of
// devices and receivers, the clock and the examples. built with and without ANYBOX_FEATURE_MOCK

#include "anybox.h"
#include "anybox.hpp"

#include <atomic>
#include <chrono>
#include <cstdio>
#include <cstring>
#include <functional>
#include <future>
#include <limits>
#include <string>
#include <thread>
#include <vector>

std::string from_the_second_unit();

namespace {

using namespace std::chrono_literals;
using namespace anybox;

int failures = 0;

void check(bool ok, const std::string& what) {
    if (!ok) {
        std::printf("  FAILED: %s\n", what.c_str());
        failures++;
    }
}

template <class Thrown, class Call>
void throws(Call call, const std::string& what, const char* text = nullptr) {
    try {
        call();
        check(false, what + " didn't throw");
    } catch (const Thrown& error) {
        if (text) {
            check(std::string(error.what()).find(text) != std::string::npos,
                  what + " threw \"" + error.what() + "\" without \"" + text + "\"");
        }
    }
}

void values() {
    check(Button::extra(7).value == ANYBOX_BUTTON_EXTRA_BASE + 7, "extra 7");
    throws<std::invalid_argument>([] { Button::extra(256); }, "extra 256");
    throws<std::invalid_argument>([] { Button::extra(-1); }, "extra -1");
    check(Key::LEFT_CTRL.is_modifier() && Key::RIGHT_GUI.is_modifier() && !Key::A.is_modifier(), "modifiers");
    check(Key::DELETE.value == ANYBOX_KEY_DELETE, "Key::DELETE");
    check(wraps_after(Clock::MediusDeviceChip) == std::chrono::microseconds(4294967296LL), "wraps after 2^32 us");
    check(wraps_after(Clock::MediusHostChip) == std::chrono::microseconds(4294967296LL), "the host chip too");
    check((EventKinds::BUTTONS | EventKinds::KEYS) == static_cast<EventKinds>(ANYBOX_EVENTS_BUTTONS | ANYBOX_EVENTS_KEYS),
          "event kinds combine");
    check(Capabilities().get(Action::Move).execution == Execution::Unsupported &&
              Capabilities()[Action::Move].reason == std::optional<std::string>("this box doesn't have it"),
          "an action the box doesn't list is Rust's missing one");
    check(to_string(StableId::mac({0x01, 0x23, 0x45, 0x67, 0x89, 0xab})) == "MAC 01:23:45:67:89:ab", "a MAC as text");
    check(to_string(StableId::usb_serial("ABC")) == "USB serial ABC", "a USB serial as text");
    Firmware firmware;
    firmware.major = 3;
    firmware.minor = 4;
    firmware.patch = 6;
    check(to_string(firmware) == "3.4.6", "a version as text");
    check(from_the_second_unit() == "4294967296 4 0.0.0 0", "the second unit got " + from_the_second_unit());
}

void durations() {
    throws<std::invalid_argument>([] { ClickTiming::fixed(-1us); }, "a negative fixed click timing", "can't be negative");
    throws<std::invalid_argument>([] { Click::hold(-1us); }, "a negative hold", "can't be negative");
    check(Click::hold(5ms).hold() == std::optional<std::chrono::microseconds>(5000us), "a hold");
    check(!Click().hold() && !Click().is_native_only() && Click().native_only().is_native_only(), "the default click");
    check(ClickTiming::fixed(7ms).hold() == std::optional<std::chrono::microseconds>(7000us), "a fixed timing's hold");
    check(!ClickTiming::makcu().hold() && ClickTiming() == ClickTiming::makcu(), "the default click timing");
}

void abi_mismatches() {
    try {
        detail::check_abi(1, 2);
        check(false, "a mismatched ABI didn't throw");
    } catch (const Error& error) {
        check(!error.status(), "an ABI mismatch has a status");
        check(std::string(error.what()).find("speaks C ABI 1 and this header needs 2") != std::string::npos,
              error.what());
    }
    throws<MediusAbiError>([] { detail::check_medius_abi(9, 10); }, "a mismatched medius ABI",
                           "speaks C ABI 9 and medius.h needs 10");
}

void options() {
    {
        const detail::Options none{Connect()};
        const auto& raw = *none.get();
        check(!raw.port && raw.family == ANYBOX_FAMILY_ANY && raw.ferrum_api == ANYBOX_FERRUM_API_NONE && !raw.can &&
                  raw.can_len == 0 && raw.id_kind == ANYBOX_ID_NONE && raw.click_timing.kind == ANYBOX_CLICK_TIMING_MAKCU,
              "the default options");
    }
    {
        const detail::Options all{Connect::port("COM9")
                                      .family(Family::Ferrum)
                                      .ferrum_api(FerrumApi::Legacy)
                                      .can(Action::Move)
                                      .can(Action::Click)
                                      .id(StableId::mac({1, 2, 3, 4, 5, 0xab}))
                                      .click_timing(ClickTiming::fixed(7ms))};
        const auto& raw = *all.get();
        check(raw.port && std::string(raw.port) == "COM9", "the port");
        check(raw.family == ANYBOX_FAMILY_FERRUM && raw.ferrum_api == ANYBOX_FERRUM_API_LEGACY, "the family and API");
        check(raw.can_len == 2 && raw.can[0] == ANYBOX_ACTION_MOVE && raw.can[1] == ANYBOX_ACTION_CLICK, "the actions");
        const std::uint8_t mac[6] = {1, 2, 3, 4, 5, 0xab};
        check(raw.id_kind == ANYBOX_ID_MAC && std::memcmp(raw.id_mac, mac, 6) == 0 && !raw.id_serial, "the MAC");
        check(raw.click_timing.kind == ANYBOX_CLICK_TIMING_FIXED && raw.click_timing.fixed_us == 7000,
              "the click timing");
    }
    {
        const detail::Options serial{Connect().id(StableId::usb_serial("ABC"))};
        const auto& raw = *serial.get();
        check(raw.id_kind == ANYBOX_ID_USB_SERIAL && raw.id_serial && std::string(raw.id_serial) == "ABC",
              "the USB serial");
    }
    throws<std::invalid_argument>([] { detail::Options nul{Connect::port(std::string("CO\0M", 4))}; },
                                  "a NUL in the port", "NUL");
}

void conversions() {
    AnyboxEvent raw{};
    raw.kind = 99;
    throws<Error>([&] { detail::event(raw); }, "an unknown event kind", "an event this header doesn't know (99)");
    raw = AnyboxEvent{};
    raw.kind = ANYBOX_EVENT_DROPPED;
    raw.source = 7;
    throws<Error>([&] { detail::event(raw); }, "an unknown drop source", "a drop source");
    AnyboxQuirk quirk{};
    quirk.kind = 99;
    throws<Error>([&] { detail::quirk(quirk); }, "an unknown quirk", "a quirk");
    // the mock Medius has none of these shapes, so no scenario shows them
    quirk = AnyboxQuirk{};
    quirk.kind = ANYBOX_QUIRK_FORCED_UP_ON_RELEASE;
    quirk.min_us = 1000;
    quirk.max_us = 2000;
    const auto forced = detail::quirk(quirk);
    check(forced.min == std::optional<std::chrono::microseconds>(1000us) &&
              forced.max == std::optional<std::chrono::microseconds>(2000us) && !forced.count && !forced.click_timing,
          "a forced up quirk's min and max");
    quirk.kind = ANYBOX_QUIRK_MAX_HOLD;
    const auto hold = detail::quirk(quirk);
    check(!hold.min && hold.max == std::optional<std::chrono::microseconds>(2000us), "a max hold quirk's max");
    quirk = AnyboxQuirk{};
    quirk.kind = ANYBOX_QUIRK_MAX_KEYS_HELD;
    quirk.count = 6;
    check(detail::quirk(quirk).count == std::optional<std::uint32_t>(6), "a max keys quirk's count");
    quirk = AnyboxQuirk{};
    quirk.kind = ANYBOX_QUIRK_HOST_CLICK_TIMING;
    quirk.click_timing = AnyboxClickTiming{ANYBOX_CLICK_TIMING_FIXED, 9000};
    check(detail::quirk(quirk).click_timing == std::optional<ClickTiming>(ClickTiming::fixed(9ms)),
          "a host click timing quirk's timing");
    AnyboxInfo info{};
    info.family = ANYBOX_FAMILY_MEDIUS;
    info.firmware_kind = 9;
    throws<Error>([&] { detail::info(info); }, "an unknown firmware kind", "a firmware kind");
    info = AnyboxInfo{};
    info.family = 99;
    throws<Error>([&] { detail::info(info); }, "an unknown family", "a family");

    raw = AnyboxEvent{};
    raw.kind = ANYBOX_EVENT_MOTION;
    raw.since_open_ns = (std::numeric_limits<std::uint64_t>::max)();
    raw.arrived_ns = (std::numeric_limits<std::uint64_t>::max)();
    const auto motion = std::get<MotionEvent>(detail::event(raw));
    check(motion.at.since_open == (std::chrono::nanoseconds::max)(), "a stamp past 292 years clamps");
#if defined(_MSC_VER) || (defined(__GLIBCXX__) && defined(__linux__))
    check(motion.at.arrived().time_since_epoch() ==
              std::chrono::duration_cast<std::chrono::steady_clock::duration>((std::chrono::nanoseconds::max)()),
          "arrived() clamps the same way");
#endif

    AnyboxErrorDetail detail_raw{};
    detail_raw.kind = ANYBOX_STATUS_WRONG_BOX;
    detail_raw.port = "COM3";
    detail_raw.expected_id_kind = ANYBOX_ID_MAC;
    const std::uint8_t mac[6] = {0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff};
    std::memcpy(detail_raw.expected_id_mac, mac, 6);
    detail_raw.found_id_kind = ANYBOX_ID_USB_SERIAL;
    detail_raw.found_id_serial = "XYZ";
    const auto wrong = detail::make_error(ANYBOX_STATUS_WRONG_BOX, "text", detail::fields(detail_raw), {}, {});
    const auto* box = dynamic_cast<const WrongBoxError*>(wrong.get());
    check(box && box->port() == "COM3" && box->expected() == StableId::mac({0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff}) &&
              box->found() == StableId::usb_serial("XYZ"),
          "a wrong box's ids");
    detail_raw = AnyboxErrorDetail{};
    detail_raw.kind = ANYBOX_STATUS_FIRMWARE_TOO_OLD;
    detail_raw.family = ANYBOX_FAMILY_YSC;
    detail_raw.found_firmware_kind = ANYBOX_FIRMWARE_TEXT;
    detail_raw.found_firmware_text = "v1";
    detail_raw.needed_firmware_kind = ANYBOX_FIRMWARE_BUILD;
    detail_raw.needed_firmware_build = 42;
    const auto old = detail::make_error(ANYBOX_STATUS_FIRMWARE_TOO_OLD, "text", detail::fields(detail_raw), {}, {});
    const auto* too_old = dynamic_cast<const FirmwareTooOldError*>(old.get());
    check(too_old && too_old->family() == Family::Ysc && too_old->found() && too_old->found()->text == "v1" &&
              too_old->needed() && too_old->needed()->build == 42,
          "a too old firmware's fields");
    detail_raw = AnyboxErrorDetail{};
    detail_raw.kind = ANYBOX_STATUS_IO;
    detail_raw.io_kind = ANYBOX_IO_KIND_TIMED_OUT;
    detail_raw.has_os_error = true;
    detail_raw.os_error = 121;
    const auto io = detail::make_error(ANYBOX_STATUS_IO, "text", detail::fields(detail_raw), {}, {});
    const auto* io_error = dynamic_cast<const IoError*>(io.get());
    check(io_error && io_error->kind() == IoKind::TimedOut && io_error->os_error() == 121, "an I/O error's fields");
    throws<IoError>([&] { io->raise(); }, "raise() of an I/O error");
}

void no_box() {
    check(!version().empty(), "the version");
#if defined(ANYBOX_FEATURE_MOCK)
    check(has_mock(), "the library has the mock");
#endif
    try {
        Connect::port("NOPE99").open();
        check(false, "a port with nothing on it opened");
    } catch (const NotFoundError& error) {
        check(error.tried() == std::vector<std::string>{"NOPE99"}, "what a failed open tried");
    }
    // the entry's error comes from the last error the result call set, its whole one
    const auto scanned = Connect::port("NOPE97").scan();
    check(scanned.size() == 1 && scanned[0].port == "NOPE97" && !scanned[0].found, "the scan of a port with nothing on it");
    if (scanned.size() == 1) {
        const auto* error = dynamic_cast<const NotFoundError*>(scanned[0].error.get());
        check(error && error->tried() == std::vector<std::string>{"NOPE97"}, "the scan entry keeps its whole error");
        throws<NotFoundError>([&] { scanned[0].error->raise(); }, "raise() of a scan entry's error");
    }
}

#if defined(ANYBOX_FEATURE_MOCK)

void wait_subscribed(const Mock& box, EventKinds kinds) {
    const auto start = std::chrono::steady_clock::now();
    while (!box.subscribed(kinds) && std::chrono::steady_clock::now() - start < 5s) {
        std::this_thread::sleep_for(10ms);
    }
    check(box.subscribed(kinds), "the receiver never subscribed");
}

void a_receiver_closed_from_another_thread_ends_its_reader() {
    auto opened = Mock::medius_manual();
    const auto events = opened.first.events();
    std::atomic<bool> closed{false};
    std::promise<void> ended;
    auto done = ended.get_future();
    std::thread reader([&] {
        try {
            events.recv();
        } catch (const ClosedError&) {
            closed = true;
        }
        ended.set_value();
    });
    // a reader that isn't waiting yet just finds it closed, which ends it too
    std::this_thread::sleep_for(50ms);
    events.close();
    // a close that never wakes it fails here instead of hanging the whole program on the join
    if (done.wait_for(3s) != std::future_status::ready) {
        check(false, "the reader never ended");
        reader.detach();
        return;
    }
    reader.join();
    check(closed, "the reader didn't get ClosedError");
    throws<ClosedError>([&] { events.try_recv(); }, "a closed receiver", "get a new one");
    // a second close does nothing
    events.close();
}

void a_closed_device_ends_its_receivers_and_keeps_its_snapshot() {
    auto opened = Mock::medius_manual();
    const auto& device = opened.first;
    const auto events = device.events();
    check(device.medius().has_value() && device.medius()->get() != nullptr, "medius() on the mock Medius");
    device.close();
    int left = 0;
    for (const auto& event : events) {
        (void)event;
        left++;
    }
    check(left < 100, "the receiver ended after close");
    check(device.snapshot().info.family == Family::Medius, "the snapshot after close");
    int after = 0;
    for (const auto& event : device.events()) {
        (void)event;
        after++;
    }
    check(after == 0, "a receiver made after close is already ended");
    check(!device.medius(), "medius() after close");
    throws<ClosedError>([&] { device.move_by(1, 0); }, "a move after close");
    throws<ClosedError>([&] { device.close(); }, "a second close");
}

// like in Rust, and unlike Python and C# whose receivers keep their device because of the garbage collector
void a_receiver_doesnt_keep_its_device_open() {
    std::optional<Events> events;
    {
        auto opened = Mock::medius_manual();
        const auto device = std::move(opened.first);
        events.emplace(device.events());
        wait_subscribed(opened.second, EventKinds::BUTTONS | EventKinds::MOTION);
    }
    bool ended = false;
    for (int read = 0; read < 100 && !ended; read++) {
        try {
            events->recv(100ms);
        } catch (const RecvClosedError&) {
            ended = true;
        }
    }
    check(ended, "a receiver kept its device open after the last Device went");
    Events moved = std::move(*events);
    throws<ClosedError>([&] { events->close(); }, "close() on a moved-from receiver", "moved from");
}

void a_moved_device_is_empty_and_a_copy_shares_the_box() {
    auto opened = Mock::medius_manual();
    Device moved = std::move(opened.first);
    throws<ClosedError>([&] { opened.first.get(); }, "a moved-from device", "moved from");
    std::optional<Device> copy(moved);
    { Device gone = std::move(moved); }
    check(copy->snapshot().info.family == Family::Medius, "the copy still has the box");
    check(copy->medius().has_value(), "the copy's medius");
}

void the_examples() {
    throws<InvalidArgumentError>([] { example_error<Error>(); }, "the example of Error itself");
    throws<InvalidArgumentError>([] { example_error<PanicError>(); }, "the example of a panic");
    const auto rejected = example_error<RejectedError>();
    check(rejected.status() == Status::Rejected && !rejected.detail().empty(), "the rejected example's detail");
    auto opened = Mock::medius_manual(ClickTiming::makcu(), true);
    const auto events = opened.first.events();
    wait_subscribed(opened.second, EventKinds::ALL);
    example_events(opened.first);
    std::vector<std::size_t> kinds;
    // `*it++` like on any input iterator, a for loop only ever uses the prefix one
    auto it = events.begin();
    while (it != events.end() && kinds.size() < std::variant_size<Event>::value) {
        const Event event = *it++;
        kinds.push_back(event.index());
    }
    check(kinds.size() == std::variant_size<Event>::value, "every kind of example event came");
    throws<std::invalid_argument>([&] { opened.first.events(static_cast<EventKinds>(16)); }, "an unknown event kind");
}

void the_clock() {
    const auto first = std::chrono::steady_clock::now();
    const auto now = monotonic_ns();
    const auto second = std::chrono::steady_clock::now();
#if defined(_MSC_VER) || (defined(__GLIBCXX__) && defined(__linux__))
    const auto ns = [](std::chrono::steady_clock::time_point at) {
        return static_cast<std::uint64_t>(
            std::chrono::duration_cast<std::chrono::nanoseconds>(at.time_since_epoch()).count());
    };
    check(ns(first) <= now && now <= ns(second), "monotonic_ns() is steady_clock's clock");
    auto opened = Mock::medius_manual();
    const auto events = opened.first.events(EventKinds::MOTION);
    wait_subscribed(opened.second, EventKinds::MOTION);
    const auto before = std::chrono::steady_clock::now();
    opened.second.push_motion(1, 1000, 5, -5);
    const auto next = events.recv(2s);
    const auto after = std::chrono::steady_clock::now();
    check(next && std::holds_alternative<MotionEvent>(*next), "a motion event came");
    if (next && std::holds_alternative<MotionEvent>(*next)) {
        const auto& at = std::get<MotionEvent>(*next).at;
        check(before <= at.received() && at.received() <= after, "the stamp's received() is on steady_clock");
    }
#else
    (void)first;
    (void)now;
    (void)second;
#endif
}

#endif

}  // namespace

int main() {
    const std::vector<std::pair<const char*, std::function<void()>>> tests = {
        {"values", values},
        {"durations", durations},
        {"abi mismatches", abi_mismatches},
        {"options", options},
        {"conversions", conversions},
        {"no box", no_box},
#if defined(ANYBOX_FEATURE_MOCK)
        {"a receiver closed from another thread ends its reader", a_receiver_closed_from_another_thread_ends_its_reader},
        {"a closed device ends its receivers and keeps its snapshot", a_closed_device_ends_its_receivers_and_keeps_its_snapshot},
        {"a receiver doesn't keep its device open", a_receiver_doesnt_keep_its_device_open},
        {"a moved device is empty and a copy shares the box", a_moved_device_is_empty_and_a_copy_shares_the_box},
        {"the examples", the_examples},
        {"the clock", the_clock},
#endif
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
        std::printf("tests: %d failed\n", failures);
        return 1;
    }
    std::printf("tests: all passed\n");
    return 0;
}
