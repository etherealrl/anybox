// the wrapper against a few C functions of its own instead of the library's, for what the conformance suite can't
// show. the plain calls write down what they were handed, so a wrong argument shows even where the mock has
// nothing to show it with (it has no pan) and so does a wrong default. the snapshot, scan, candidates, receiver
// and device functions hand out handles or take them back with their frees counted, and an allocation can be made
// to fail, so a wrapper that throws halfway is seen freeing each handle exactly once, and never under a read. the
// three medius streams and the firmware staging get the same, since a free under a read and an exception crossing
// into Rust can't be seen from the real library. this program's own definitions take the place of the library's for
// these (an import library or a shared library is only searched for what's still missing), everything else is the
// real library with the mock

#include "anybox.hpp"

#include <atomic>
#include <condition_variable>
#include <cstdio>
#include <cstdlib>
#include <mutex>
#include <new>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

namespace {

std::vector<std::string> calls;
int failures = 0;

// the next allocation of this program throws std::bad_alloc when this is set
std::atomic<bool> fail_next_allocation{false};

template <class... Values>
AnyboxStatus record(const char* name, Values... values) {
    std::string line = name;
    ((line += ' ', line += std::to_string(values)), ...);
    calls.push_back(line);
    return ANYBOX_STATUS_OK;
}

void expect(bool ok, const std::string& what) {
    if (!ok) {
        std::printf("FAILED: %s\n", what.c_str());
        failures++;
    }
}

int snapshot_frees = 0;
int scan_list_frees = 0;
int string_list_frees = 0;
std::atomic<int> events_frees{0};
std::atomic<int> events_closes{0};
std::atomic<bool> events_closed{false};
std::atomic<int> reading{0};
std::atomic<bool> freed_under_a_read{false};
std::vector<const void*> freed_devices;
std::vector<const void*> freed_mocks;

// addresses that are never a real handle, so a free can tell them apart
AnyboxSnapshot* const fake_snapshot = reinterpret_cast<AnyboxSnapshot*>(&snapshot_frees);
AnyboxScanList* const fake_scan_list = reinterpret_cast<AnyboxScanList*>(&scan_list_frees);
AnyboxStringList* const fake_string_list = reinterpret_cast<AnyboxStringList*>(&string_list_frees);
AnyboxEvents* const fake_events = reinterpret_cast<AnyboxEvents*>(&events_frees);

}  // namespace

void* operator new(std::size_t size) {
    if (fail_next_allocation.exchange(false)) {
        throw std::bad_alloc();
    }
    if (void* memory = std::malloc(size == 0 ? 1 : size)) {
        return memory;
    }
    throw std::bad_alloc();
}

void operator delete(void* memory) noexcept { std::free(memory); }

void operator delete(void* memory, std::size_t) noexcept { std::free(memory); }

// made by bindings/gen from its call table (calls), change the table and run cargo run -p anybox-gen
AnyboxStatus anybox_device_move_by(const AnyboxDevice*, int16_t dx, int16_t dy) {
    return record("move_by", dx, dy);
}

AnyboxStatus anybox_device_move_now(const AnyboxDevice*, int16_t dx, int16_t dy) {
    return record("move_now", dx, dy);
}

AnyboxStatus anybox_device_press(const AnyboxDevice*, uint32_t button) {
    return record("press", button);
}

AnyboxStatus anybox_device_release(const AnyboxDevice*, uint32_t button) {
    return record("release", button);
}

AnyboxStatus anybox_device_click(const AnyboxDevice*, uint32_t button) {
    return record("click", button);
}

AnyboxStatus anybox_device_click_with(const AnyboxDevice*, uint32_t button, const AnyboxClick* click) {
    return record("click_with", button, click->hold_us, click->has_hold, click->native_only);
}

AnyboxStatus anybox_device_press_key(const AnyboxDevice*, uint8_t key) {
    return record("press_key", key);
}

AnyboxStatus anybox_device_release_key(const AnyboxDevice*, uint8_t key) {
    return record("release_key", key);
}

AnyboxStatus anybox_device_tap_key(const AnyboxDevice*, uint8_t key) {
    return record("tap_key", key);
}

AnyboxStatus anybox_device_tap_key_with(const AnyboxDevice*, uint8_t key, const AnyboxClick* tap) {
    return record("tap_key_with", key, tap->hold_us, tap->has_hold, tap->native_only);
}

AnyboxStatus anybox_device_press_media(const AnyboxDevice*, uint16_t key) {
    return record("press_media", key);
}

AnyboxStatus anybox_device_release_media(const AnyboxDevice*, uint16_t key) {
    return record("release_media", key);
}

AnyboxStatus anybox_device_tap_media(const AnyboxDevice*, uint16_t key) {
    return record("tap_media", key);
}

AnyboxStatus anybox_device_tap_media_with(const AnyboxDevice*, uint16_t key, const AnyboxClick* tap) {
    return record("tap_media_with", key, tap->hold_us, tap->has_hold, tap->native_only);
}

AnyboxStatus anybox_device_lock_button(const AnyboxDevice*, uint32_t button) {
    return record("lock_button", button);
}

AnyboxStatus anybox_device_unlock_button(const AnyboxDevice*, uint32_t button) {
    return record("unlock_button", button);
}

AnyboxStatus anybox_device_lock_key(const AnyboxDevice*, uint8_t key) {
    return record("lock_key", key);
}

AnyboxStatus anybox_device_unlock_key(const AnyboxDevice*, uint8_t key) {
    return record("unlock_key", key);
}

AnyboxStatus anybox_device_lock_axis(const AnyboxDevice*, uint32_t axis, uint32_t direction) {
    return record("lock_axis", axis, direction);
}

AnyboxStatus anybox_device_unlock_axis(const AnyboxDevice*, uint32_t axis, uint32_t direction) {
    return record("unlock_axis", axis, direction);
}

AnyboxStatus anybox_device_scale_axis(const AnyboxDevice*, uint32_t axis, uint32_t direction, int16_t percent) {
    return record("scale_axis", axis, direction, percent);
}

AnyboxStatus anybox_device_wheel(const AnyboxDevice*, int16_t notches) {
    return record("wheel", notches);
}

AnyboxStatus anybox_device_pan(const AnyboxDevice*, int16_t notches) {
    return record("pan", notches);
}

AnyboxStatus anybox_device_discard_pending_moves(const AnyboxDevice*) {
    return record("discard_pending_moves");
}

// the end of what bindings/gen makes (calls)

AnyboxStatus anybox_device_snapshot(const AnyboxDevice*, AnyboxSnapshot** out) {
    *out = fake_snapshot;
    return ANYBOX_STATUS_OK;
}

// a family this header doesn't know, so reading the snapshot throws after its handle came back
AnyboxStatus anybox_snapshot_info(const AnyboxSnapshot*, AnyboxInfo* out) {
    *out = AnyboxInfo{};
    out->family = 99;
    return ANYBOX_STATUS_OK;
}

void anybox_snapshot_free(AnyboxSnapshot* snapshot) {
    if (snapshot == fake_snapshot) {
        snapshot_frees++;
    }
}

AnyboxStatus anybox_scan(const AnyboxConnectOptions*, AnyboxScanList** out) {
    *out = fake_scan_list;
    return ANYBOX_STATUS_OK;
}

uintptr_t anybox_scan_list_count(const AnyboxScanList*) { return 1; }

const char* anybox_scan_list_port(const AnyboxScanList*, uintptr_t) { return "FAKE0"; }

AnyboxStatus anybox_scan_list_result(const AnyboxScanList*, uintptr_t, AnyboxSnapshot** out) {
    *out = fake_snapshot;
    return ANYBOX_STATUS_OK;
}

void anybox_scan_list_free(AnyboxScanList* list) {
    if (list == fake_scan_list) {
        scan_list_frees++;
    }
}

AnyboxStatus anybox_candidates(const AnyboxConnectOptions*, AnyboxStringList** out) {
    *out = fake_string_list;
    return ANYBOX_STATUS_OK;
}

uintptr_t anybox_string_list_count(const AnyboxStringList*) { return 1; }

const char* anybox_string_list_get(const AnyboxStringList*, uintptr_t) { return "FAKE1"; }

void anybox_string_list_free(AnyboxStringList* list) {
    if (list == fake_string_list) {
        string_list_frees++;
    }
}

AnyboxStatus anybox_device_events(const AnyboxDevice*, uint32_t, AnyboxEvents** out) {
    *out = fake_events;
    return ANYBOX_STATUS_OK;
}

// a read that waits like the real one until a close wakes it or its timeout runs out (at most 50 ms, so the tests
// stay quick), and gets nothing
AnyboxStatus anybox_events_recv(const AnyboxEvents*, int64_t timeout_us, AnyboxEvent*) {
    reading++;
    const auto until = std::chrono::steady_clock::now() +
                       std::chrono::microseconds(timeout_us < 0 || timeout_us > 50000 ? 50000 : timeout_us);
    while (!events_closed && (timeout_us < 0 || std::chrono::steady_clock::now() < until)) {
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    reading--;
    return events_closed ? ANYBOX_STATUS_CLOSED : ANYBOX_STATUS_RECV_TIMEOUT;
}

AnyboxStatus anybox_events_close(const AnyboxEvents*) {
    events_closes++;
    events_closed = true;
    return ANYBOX_STATUS_OK;
}

void anybox_events_free(AnyboxEvents* events) {
    if (events == fake_events) {
        if (reading > 0) {
            freed_under_a_read = true;
        }
        events_frees++;
    }
}

// the real ones can't be called from here, the name is this program's now, so the devices and mocks of this
// program are never really freed, which only matters until it ends
void anybox_device_free(AnyboxDevice* device) { freed_devices.push_back(device); }

void anybox_mock_free(AnyboxMock* mock) { freed_mocks.push_back(mock); }

namespace {

// one of each medius stream. a read blocks until the stream gets closed and then answers like a box that's gone
struct FakeStream {
    std::mutex lock;
    std::condition_variable woken;
    bool closed = false;
    std::atomic<int> frees{0};
    std::atomic<int> reading{0};
    std::atomic<bool> freed_under_a_read{false};

    void open() {
        std::lock_guard<std::mutex> guard(lock);
        closed = false;
        frees = 0;
        freed_under_a_read = false;
    }

    // at most 3 s, so a wrapper that never closes fails the test instead of hanging it
    MediusStatus recv() {
        reading++;
        {
            std::unique_lock<std::mutex> guard(lock);
            woken.wait_for(guard, std::chrono::seconds(3), [&] { return closed; });
        }
        reading--;
        return MEDIUS_STATUS_ERR_DISCONNECTED;
    }

    void close() {
        {
            std::lock_guard<std::mutex> guard(lock);
            closed = true;
        }
        woken.notify_all();
    }

    void freed() {
        if (reading > 0) {
            freed_under_a_read = true;
        }
        frees++;
    }
};

FakeStream event_stream;
FakeStream input_stream;
FakeStream log_stream;

MediusEventStream* const fake_event_stream = reinterpret_cast<MediusEventStream*>(&event_stream);
MediusInputStream* const fake_input_stream = reinterpret_cast<MediusInputStream*>(&input_stream);
MediusLogStream* const fake_log_stream = reinterpret_cast<MediusLogStream*>(&log_stream);

std::atomic<bool> staging{false};
int progress_calls_made = 0;
int aborts = 0;
bool aborted_while_staging = false;
int aborted_target = -1;
// what the stage, the abort and the activate answer, so a test can make each of them fail
MediusStatus stage_status = MEDIUS_STATUS_OK;
MediusStatus abort_status = MEDIUS_STATUS_OK;
MediusStatus activate_status = MEDIUS_STATUS_OK;

}  // namespace

MediusStatus medius_device_catch_events(struct MediusDevice*, const struct MediusCatchFilter*, uintptr_t,
                                        struct MediusEventStream** out) {
    event_stream.open();
    *out = fake_event_stream;
    return MEDIUS_STATUS_OK;
}

MediusStatus medius_event_stream_recv(struct MediusEventStream*, struct MediusCatchEvent*) {
    return event_stream.recv();
}

void medius_event_stream_close(struct MediusEventStream* stream) {
    if (stream == fake_event_stream) {
        event_stream.close();
    }
}

void medius_event_stream_free(struct MediusEventStream* stream) {
    if (stream == fake_event_stream) {
        event_stream.freed();
    }
}

MediusStatus medius_device_input_events(struct MediusDevice*, const struct MediusCatchFilter*, uintptr_t,
                                        struct MediusInputStream** out) {
    input_stream.open();
    *out = fake_input_stream;
    return MEDIUS_STATUS_OK;
}

MediusStatus medius_input_stream_recv(struct MediusInputStream*, struct MediusInputEvent*) {
    return input_stream.recv();
}

void medius_input_stream_close(struct MediusInputStream* stream) {
    if (stream == fake_input_stream) {
        input_stream.close();
    }
}

void medius_input_stream_free(struct MediusInputStream* stream) {
    if (stream == fake_input_stream) {
        input_stream.freed();
    }
}

MediusStatus medius_device_logs(struct MediusDevice*, struct MediusLogStream** out) {
    log_stream.open();
    *out = fake_log_stream;
    return MEDIUS_STATUS_OK;
}

MediusStatus medius_log_stream_recv(struct MediusLogStream*, struct MediusLogLine*) { return log_stream.recv(); }

void medius_log_stream_close(struct MediusLogStream* stream) {
    if (stream == fake_log_stream) {
        log_stream.close();
    }
}

void medius_log_stream_free(struct MediusLogStream* stream) {
    if (stream == fake_log_stream) {
        log_stream.freed();
    }
}

// three progress calls, each one has to come back here for the next to go out
MediusStatus medius_device_stage_firmware(struct MediusDevice*, uint8_t, const uint8_t*, uintptr_t len,
                                          void (*progress)(void*, uintptr_t, uintptr_t), void* user) {
    staging = true;
    for (uintptr_t part = 1; part <= 3; part++) {
        if (progress) {
            progress(user, len * part / 3, len);
            progress_calls_made++;
        }
    }
    staging = false;
    return stage_status;
}

MediusStatus medius_device_abort_update(struct MediusDevice*, uint8_t target) {
    aborts++;
    aborted_while_staging = staging;
    aborted_target = target;
    return abort_status;
}

MediusStatus medius_device_activate_firmware(struct MediusDevice*) { return activate_status; }

namespace {

// what medius_last_error_message gives, nothing unless a test sets it, since no medius error says this much
std::string error_text;

}  // namespace

uintptr_t medius_last_error_message(char* buf, uintptr_t cap) {
    if (buf && cap > 0) {
        const auto kept = error_text.size() < cap - 1 ? error_text.size() : cap - 1;
        error_text.copy(buf, kept);
        buf[kept] = '\0';
    }
    return error_text.size();
}

namespace {

void every_call_hands_c_what_it_was_given() {
    using namespace std::chrono_literals;
    using namespace anybox;
    auto opened = Mock::medius_manual();
    const auto& device = opened.first;
    device.move_by(3, -4);
    device.move_now(-5, 6);
    device.press(Button::Right);
    device.release(Button::extra(7));
    device.click(Button::Middle);
    device.click(Button::Left, Click::hold(12500us).native_only());
    device.press_key(Key::B);
    device.release_key(Key{0x64});
    device.tap_key(Key::C);
    device.tap_key(Key::C, Click::hold(2ms));
    device.press_media(MediaKey::MUTE);
    device.release_media(MediaKey{402});
    device.tap_media(MediaKey::PLAY);
    device.tap_media(MediaKey::PLAY, Click().native_only());
    device.lock_button(Button::Side1);
    device.unlock_button(Button::Side2);
    device.lock_key(Key::D);
    device.unlock_key(Key::E);
    device.lock_axis(Axis::Y, Direction::Negative);
    device.lock_axis(Axis::X);
    device.unlock_axis(Axis::Wheel);
    device.unlock_axis(Axis::Pan, Direction::Positive);
    device.scale_axis(Axis::Pan, Direction::Positive, -100);
    device.wheel(-2);
    device.pan(9);
    device.discard_pending_moves();
    // the C-only click and taps are never called, their `_with` one with no hold does what they do
    const std::vector<std::string> wanted = {
        "move_by 3 -4",
        "move_now -5 6",
        "press 1",
        "release 263",
        "click_with 2 0 0 0",
        "click_with 0 12500 1 1",
        "press_key 5",
        "release_key 100",
        "tap_key_with 6 0 0 0",
        "tap_key_with 6 2000 1 0",
        "press_media 226",
        "release_media 402",
        "tap_media_with 176 0 0 0",
        "tap_media_with 176 0 0 1",
        "lock_button 3",
        "unlock_button 4",
        "lock_key 7",
        "unlock_key 8",
        "lock_axis 1 2",
        "lock_axis 0 0",
        "unlock_axis 2 0",
        "unlock_axis 3 1",
        "scale_axis 3 1 -100",
        "wheel -2",
        "pan 9",
        "discard_pending_moves",
    };
    expect(calls == wanted, "the calls handed C something else");
    if (calls != wanted) {
        for (const auto& call : calls) {
            std::printf("  got %s\n", call.c_str());
        }
    }
}

void a_snapshot_that_throws_halfway_is_freed_once() {
    auto opened = anybox::Mock::medius_manual();
    try {
        opened.first.snapshot();
        expect(false, "a snapshot of an unknown family didn't throw");
    } catch (const anybox::Error& error) {
        expect(!error.status().has_value(), "the unknown family has a status");
        expect(std::string(error.what()).find("a family this header doesn't know (99)") != std::string::npos,
               std::string("the unknown family's text: ") + error.what());
    }
    expect(snapshot_frees == 1, "the snapshot was freed " + std::to_string(snapshot_frees) + " times");
}

void a_scan_that_throws_halfway_frees_the_list_and_the_snapshot_once() {
    snapshot_frees = 0;
    try {
        anybox::scan();
        expect(false, "a scan with an unknown family didn't throw");
    } catch (const anybox::Error&) {
    }
    expect(scan_list_frees == 1, "the scan list was freed " + std::to_string(scan_list_frees) + " times");
    expect(snapshot_frees == 1, "the scanned snapshot was freed " + std::to_string(snapshot_frees) + " times");
}

void candidates_free_their_list_once_whether_they_throw_or_not() {
    expect(anybox::candidates() == std::vector<std::string>{"FAKE1"}, "the candidates");
    expect(string_list_frees == 1, "the candidates were freed " + std::to_string(string_list_frees) + " times");
    string_list_frees = 0;
    fail_next_allocation = true;
    try {
        anybox::candidates();
        expect(false, "candidates with no memory didn't throw");
    } catch (const std::bad_alloc&) {
    }
    expect(string_list_frees == 1, "the candidates that threw were freed " + std::to_string(string_list_frees) + " times");
}

void a_closed_receiver_is_freed_right_away_and_once() {
    events_frees = 0;
    events_closes = 0;
    events_closed = false;
    auto opened = anybox::Mock::medius_manual();
    {
        const auto events = opened.first.events();
        events.close();
        expect(events_frees == 1, "close() freed the receiver " + std::to_string(events_frees) + " times");
        // a second close does nothing, not even in the library
        events.close();
        expect(events_closes == 1, "the library got " + std::to_string(events_closes) + " closes");
    }
    expect(events_frees == 1, "the closed receiver was freed again when it went");
    events_closed = false;
    {
        const auto events = opened.first.events();
    }
    expect(events_frees == 2, "a receiver that goes away wasn't freed");
}

void a_receiver_closed_during_a_read_is_freed_after_the_read() {
    events_frees = 0;
    events_closed = false;
    auto opened = anybox::Mock::medius_manual();
    const auto events = opened.first.events();
    std::atomic<bool> closed{false};
    std::thread reader([&] {
        try {
            events.recv();
        } catch (const anybox::ClosedError&) {
            closed = true;
        }
    });
    for (int wait = 0; wait < 100 && reading == 0; wait++) {
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }
    events.close();
    reader.join();
    expect(closed, "the reader didn't get ClosedError");
    expect(!freed_under_a_read, "the receiver was freed while a read was in it");
    expect(events_frees == 1, "the receiver closed during a read was freed " + std::to_string(events_frees) + " times");
}

void a_receiver_whose_own_allocation_fails_frees_its_handle() {
    events_frees = 0;
    auto opened = anybox::Mock::medius_manual();
    fail_next_allocation = true;
    try {
        opened.first.events();
        expect(false, "a receiver with no memory didn't throw");
    } catch (const std::bad_alloc&) {
    }
    expect(events_frees == 1, "the receiver that couldn't be made was freed " + std::to_string(events_frees) + " times");
}

void a_mock_whose_device_cant_be_made_frees_both() {
    freed_devices.clear();
    freed_mocks.clear();
    fail_next_allocation = true;
    try {
        anybox::Mock::medius_manual();
        expect(false, "a mock with no memory didn't throw");
    } catch (const std::bad_alloc&) {
    }
    expect(freed_devices.size() == 1 && freed_mocks.size() == 1,
           "the mock open that threw freed " + std::to_string(freed_devices.size()) + " devices and " +
               std::to_string(freed_mocks.size()) + " mocks");
}

template <class Open>
void a_medius_stream_is_freed_once_when_its_last_owner_goes(FakeStream& fake, Open open, const std::string& kind) {
    {
        auto stream = open();
        const auto moved = std::move(stream);
        expect(fake.frees == 0, "the moved " + kind + " was freed " + std::to_string(fake.frees) + " times");
    }
    expect(fake.frees == 1, "the " + kind + " was freed " + std::to_string(fake.frees) + " times when it went");
    {
        const auto stream = open();
        stream.close();
        expect(fake.frees == 1, "close() freed the " + kind + " " + std::to_string(fake.frees) + " times");
    }
    expect(fake.frees == 1, "the closed " + kind + " was freed again when it went");
}

template <class Open>
void a_medius_stream_closed_during_a_read_is_freed_after_the_read(FakeStream& fake, Open open,
                                                                  const std::string& kind) {
    const auto stream = open();
    std::atomic<bool> closed{false};
    std::thread reader([&] {
        try {
            stream.recv();
        } catch (const anybox::ClosedError&) {
            closed = true;
        } catch (...) {
        }
    });
    for (int wait = 0; wait < 100 && fake.reading == 0; wait++) {
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }
    expect(fake.reading == 1, "the " + kind + "'s reader never got into the read");
    stream.close();
    reader.join();
    expect(closed, "the " + kind + "'s reader didn't get ClosedError");
    expect(!fake.freed_under_a_read, "the " + kind + " was freed while a read was in it");
    expect(fake.frees == 1, "the " + kind + " closed during a read was freed " + std::to_string(fake.frees) + " times");
}

void the_medius_streams_are_freed_once_and_never_under_a_read() {
    auto opened = anybox::Mock::medius_manual();
    const auto medius = *opened.first.medius();
    const auto events = [&] { return medius.catch_events({anybox::medius::CatchFilter::everything()}); };
    const auto inputs = [&] { return medius.input_events(anybox::medius::CatchFilter::all_input()); };
    const auto logs = [&] { return medius.logs(); };
    a_medius_stream_is_freed_once_when_its_last_owner_goes(event_stream, events, "event stream");
    a_medius_stream_is_freed_once_when_its_last_owner_goes(input_stream, inputs, "input stream");
    a_medius_stream_is_freed_once_when_its_last_owner_goes(log_stream, logs, "log stream");
    a_medius_stream_closed_during_a_read_is_freed_after_the_read(event_stream, events, "event stream");
    a_medius_stream_closed_during_a_read_is_freed_after_the_read(input_stream, inputs, "input stream");
    a_medius_stream_closed_during_a_read_is_freed_after_the_read(log_stream, logs, "log stream");
}

void a_medius_stream_whose_own_allocation_fails_frees_its_handle() {
    auto opened = anybox::Mock::medius_manual();
    const auto medius = *opened.first.medius();
    // logs() allocates nothing before the C call, so the allocation that fails is the stream's own
    fail_next_allocation = true;
    try {
        medius.logs();
        expect(false, "a log stream with no memory didn't throw");
    } catch (const std::bad_alloc&) {
    }
    expect(log_stream.frees == 1,
           "the log stream that couldn't be made was freed " + std::to_string(log_stream.frees) + " times");
}

// the first read takes 256 bytes, so a longer text has to be read again whole
void a_medius_error_text_past_256_bytes_comes_whole() {
    error_text.assign(400, 'x');
    error_text += "the end";
    const auto error = anybox::medius::detail::last_error(MEDIUS_STATUS_ERR_IO);
    expect(error->what() == error_text, "the error text came as " + std::to_string(std::string(error->what()).size()) +
                                            " bytes out of " + std::to_string(error_text.size()));
    expect(dynamic_cast<const anybox::medius::IoError*>(error.get()) != nullptr, "the long error isn't an IoError");
    error_text.clear();
}

void a_progress_that_throws_comes_out_after_the_stage_and_aborts_once() {
    auto opened = anybox::Mock::medius_manual();
    const auto medius = *opened.first.medius();
    int reported = 0;
    try {
        medius.stage_firmware(anybox::medius::UpdateTarget::Device, std::vector<std::uint8_t>(300, 1),
                              [&](const anybox::medius::UpdateProgress&) {
                                  reported++;
                                  throw std::runtime_error("the handler broke");
                              });
        expect(false, "a progress that throws didn't come out");
    } catch (const std::runtime_error& error) {
        expect(std::string(error.what()) == "the handler broke", std::string("the stage threw ") + error.what());
        expect(!staging, "the exception came out while the stage was still going");
    }
    // the trampoline swallowed it, so the stage went on to its other two calls
    expect(progress_calls_made == 3, "the stage got " + std::to_string(progress_calls_made) + " progress calls back");
    expect(reported == 1, "the progress was called " + std::to_string(reported) + " times");
    expect(aborts == 1 && !aborted_while_staging && aborted_target == 0,
           "the abort went " + std::to_string(aborts) + " times, during the stage " +
               std::to_string(aborted_while_staging) + ", for chip " + std::to_string(aborted_target));
}

// the progress' exception is what you need, a stage that timed out after it and an abort that failed too aren't
void a_progress_that_throws_wins_over_a_stage_and_an_abort_that_fail() {
    auto opened = anybox::Mock::medius_manual();
    const auto medius = *opened.first.medius();
    stage_status = MEDIUS_STATUS_ERR_QUERY_TIMEOUT;
    abort_status = MEDIUS_STATUS_ERR_IO;
    aborts = 0;
    try {
        medius.stage_firmware(anybox::medius::UpdateTarget::Device, std::vector<std::uint8_t>(300, 1),
                              [&](const anybox::medius::UpdateProgress&) { throw std::runtime_error("the handler broke"); });
        expect(false, "a progress that throws didn't come out");
    } catch (const std::runtime_error& error) {
        expect(std::string(error.what()) == "the handler broke", std::string("the stage threw ") + error.what());
    }
    expect(aborts == 1, "the abort went " + std::to_string(aborts) + " times");
    // with nothing thrown from the progress, the stage's own error comes out and nothing gets aborted
    aborts = 0;
    try {
        medius.stage_firmware(anybox::medius::UpdateTarget::Device, std::vector<std::uint8_t>(300, 1),
                              [](const anybox::medius::UpdateProgress&) {});
        expect(false, "a stage that timed out went through");
    } catch (const anybox::medius::QueryTimeoutError&) {
    }
    expect(aborts == 0, "a failed stage got aborted " + std::to_string(aborts) + " times");
    // and a refused activate wins over the abort that failed after it
    stage_status = MEDIUS_STATUS_OK;
    activate_status = MEDIUS_STATUS_ERR_QUERY_TIMEOUT;
    try {
        medius.update_firmware(anybox::medius::UpdateTarget::Device, std::vector<std::uint8_t>(300, 1));
        expect(false, "a refused activate went through");
    } catch (const anybox::medius::QueryTimeoutError&) {
    }
    expect(aborts == 1, "the refused activate got aborted " + std::to_string(aborts) + " times");
    abort_status = MEDIUS_STATUS_OK;
    activate_status = MEDIUS_STATUS_OK;
}

}  // namespace

int main() {
    every_call_hands_c_what_it_was_given();
    a_snapshot_that_throws_halfway_is_freed_once();
    a_scan_that_throws_halfway_frees_the_list_and_the_snapshot_once();
    candidates_free_their_list_once_whether_they_throw_or_not();
    a_closed_receiver_is_freed_right_away_and_once();
    a_receiver_closed_during_a_read_is_freed_after_the_read();
    a_receiver_whose_own_allocation_fails_frees_its_handle();
    a_mock_whose_device_cant_be_made_frees_both();
    the_medius_streams_are_freed_once_and_never_under_a_read();
    a_medius_stream_whose_own_allocation_fails_frees_its_handle();
    a_medius_error_text_past_256_bytes_comes_whole();
    a_progress_that_throws_comes_out_after_the_stage_and_aborts_once();
    a_progress_that_throws_wins_over_a_stage_and_an_abort_that_fail();
    if (failures > 0) {
        std::printf("fake: %d failed\n", failures);
        return 1;
    }
    std::printf("fake: every call forwards what it was given, and every owned handle is freed once\n");
    return 0;
}
