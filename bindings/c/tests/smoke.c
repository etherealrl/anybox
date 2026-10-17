/* builds as C99, C11 and C++17 against the mock build and drives a mock Medius through the header, so a header
   that doesn't compile somewhere, or a struct the compiler lays out differently from Rust, fails here.
   run it with the path to tests/layout.txt */

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "anybox.h"

static int failures = 0;

static void check(int ok, const char *what) {
    if (!ok) {
        fprintf(stderr, "FAILED %s (last error: %s)\n", what, anybox_last_error_message());
        failures++;
    }
}

/* the layout table, written out the same way the Rust test writes it */

static char layout[16384];
static size_t layout_len = 0;

static void put(const char *format, ...) {
    va_list args;
    va_start(args, format);
    layout_len += (size_t)vsnprintf(layout + layout_len, sizeof layout - layout_len, format, args);
    va_end(args);
}

#define ALIGNED(T) \
    struct align_##T { \
        char first; \
        T value; \
    };
ALIGNED(AnyboxClickTiming)
ALIGNED(AnyboxClick)
ALIGNED(AnyboxConnectOptions)
ALIGNED(AnyboxInfo)
ALIGNED(AnyboxCapability)
ALIGNED(AnyboxQuirk)
ALIGNED(AnyboxEvent)
ALIGNED(AnyboxErrorDetail)

#define TYPE(T) put("%s size %u align %u\n", #T, (unsigned)sizeof(T), (unsigned)offsetof(struct align_##T, value))
#define FIELD(T, f) put("  %s %u\n", #f, (unsigned)offsetof(T, f))

static void write_layout(void) {
    TYPE(AnyboxClickTiming);
    FIELD(AnyboxClickTiming, kind);
    FIELD(AnyboxClickTiming, fixed_us);
    TYPE(AnyboxClick);
    FIELD(AnyboxClick, hold_us);
    FIELD(AnyboxClick, has_hold);
    FIELD(AnyboxClick, native_only);
    TYPE(AnyboxConnectOptions);
    FIELD(AnyboxConnectOptions, port);
    FIELD(AnyboxConnectOptions, family);
    FIELD(AnyboxConnectOptions, ferrum_api);
    FIELD(AnyboxConnectOptions, can);
    FIELD(AnyboxConnectOptions, can_len);
    FIELD(AnyboxConnectOptions, id_kind);
    FIELD(AnyboxConnectOptions, id_mac);
    FIELD(AnyboxConnectOptions, id_serial);
    FIELD(AnyboxConnectOptions, click_timing);
    TYPE(AnyboxInfo);
    FIELD(AnyboxInfo, port);
    FIELD(AnyboxInfo, family);
    FIELD(AnyboxInfo, model);
    FIELD(AnyboxInfo, firmware_kind);
    FIELD(AnyboxInfo, firmware_major);
    FIELD(AnyboxInfo, firmware_minor);
    FIELD(AnyboxInfo, firmware_patch);
    FIELD(AnyboxInfo, firmware_protocol);
    FIELD(AnyboxInfo, firmware_build);
    FIELD(AnyboxInfo, firmware_text);
    FIELD(AnyboxInfo, id_kind);
    FIELD(AnyboxInfo, id_mac);
    FIELD(AnyboxInfo, id_serial);
    FIELD(AnyboxInfo, ferrum_api);
    FIELD(AnyboxInfo, has_peripheral);
    FIELD(AnyboxInfo, peripheral_vid);
    FIELD(AnyboxInfo, peripheral_pid);
    TYPE(AnyboxCapability);
    FIELD(AnyboxCapability, execution);
    FIELD(AnyboxCapability, reason);
    FIELD(AnyboxCapability, quirk_count);
    TYPE(AnyboxQuirk);
    FIELD(AnyboxQuirk, kind);
    FIELD(AnyboxQuirk, min_us);
    FIELD(AnyboxQuirk, max_us);
    FIELD(AnyboxQuirk, count);
    FIELD(AnyboxQuirk, click_timing);
    TYPE(AnyboxEvent);
    FIELD(AnyboxEvent, kind);
    FIELD(AnyboxEvent, button);
    FIELD(AnyboxEvent, key);
    FIELD(AnyboxEvent, pressed);
    FIELD(AnyboxEvent, media);
    FIELD(AnyboxEvent, dx);
    FIELD(AnyboxEvent, dy);
    FIELD(AnyboxEvent, wheel);
    FIELD(AnyboxEvent, pan);
    FIELD(AnyboxEvent, source);
    FIELD(AnyboxEvent, action);
    FIELD(AnyboxEvent, count);
    FIELD(AnyboxEvent, since_open_ns);
    FIELD(AnyboxEvent, received_since_open_ns);
    FIELD(AnyboxEvent, received_qpc);
    FIELD(AnyboxEvent, has_received_qpc);
    FIELD(AnyboxEvent, has_box_time);
    FIELD(AnyboxEvent, box_clock);
    FIELD(AnyboxEvent, box_time_us);
    FIELD(AnyboxEvent, arrived_ns);
    FIELD(AnyboxEvent, received_ns);
    TYPE(AnyboxErrorDetail);
    FIELD(AnyboxErrorDetail, kind);
    FIELD(AnyboxErrorDetail, has_action);
    FIELD(AnyboxErrorDetail, action);
    FIELD(AnyboxErrorDetail, family);
    FIELD(AnyboxErrorDetail, expected_family);
    FIELD(AnyboxErrorDetail, found_family);
    FIELD(AnyboxErrorDetail, port);
    FIELD(AnyboxErrorDetail, text);
    FIELD(AnyboxErrorDetail, expected_id_kind);
    FIELD(AnyboxErrorDetail, expected_id_mac);
    FIELD(AnyboxErrorDetail, expected_id_serial);
    FIELD(AnyboxErrorDetail, found_id_kind);
    FIELD(AnyboxErrorDetail, found_id_mac);
    FIELD(AnyboxErrorDetail, found_id_serial);
    FIELD(AnyboxErrorDetail, found_firmware_kind);
    FIELD(AnyboxErrorDetail, found_firmware_major);
    FIELD(AnyboxErrorDetail, found_firmware_minor);
    FIELD(AnyboxErrorDetail, found_firmware_patch);
    FIELD(AnyboxErrorDetail, found_firmware_protocol);
    FIELD(AnyboxErrorDetail, found_firmware_build);
    FIELD(AnyboxErrorDetail, found_firmware_text);
    FIELD(AnyboxErrorDetail, needed_firmware_kind);
    FIELD(AnyboxErrorDetail, needed_firmware_major);
    FIELD(AnyboxErrorDetail, needed_firmware_minor);
    FIELD(AnyboxErrorDetail, needed_firmware_patch);
    FIELD(AnyboxErrorDetail, needed_firmware_protocol);
    FIELD(AnyboxErrorDetail, needed_firmware_build);
    FIELD(AnyboxErrorDetail, needed_firmware_text);
    FIELD(AnyboxErrorDetail, expected_ferrum_api);
    FIELD(AnyboxErrorDetail, found_ferrum_api);
    FIELD(AnyboxErrorDetail, io_kind);
    FIELD(AnyboxErrorDetail, has_os_error);
    FIELD(AnyboxErrorDetail, os_error);
}

static void check_layout(const char *path) {
    static char committed[16384];
    size_t len = 0;
    int c;
    FILE *file = fopen(path, "rb");
    check(file != NULL, "open the layout table");
    if (file == NULL) {
        return;
    }
    while ((c = fgetc(file)) != EOF && len + 1 < sizeof committed) {
        if (c != '\r') {
            committed[len++] = (char)c;
        }
    }
    committed[len] = '\0';
    fclose(file);
    write_layout();
    if (strcmp(committed, layout) != 0) {
        fprintf(stderr, "the C layout isn't the committed one, C sees:\n%s", layout);
        failures++;
    }
}

/* the next button or motion event, the mock can send connection events and drops in between */
static AnyboxStatus next_input(AnyboxEvents *events, AnyboxEvent *event) {
    AnyboxStatus status;
    do {
        status = anybox_events_recv(events, 2000000, event);
    } while (status == ANYBOX_STATUS_OK && event->kind != ANYBOX_EVENT_MOTION && event->kind != ANYBOX_EVENT_BUTTON);
    return status;
}

int main(int argc, char **argv) {
    AnyboxMock *mock = NULL;
    AnyboxDevice *device = NULL;
    AnyboxEvents *events = NULL;
    AnyboxSnapshot *snapshot = NULL;
    MediusDevice *medius = NULL;
    AnyboxClickTiming timing;
    AnyboxInfo info;
    AnyboxEvent event;
    AnyboxConnectOptions options;
    AnyboxErrorDetail detail;
    size_t buttons = 0;
    int tries;

    if (argc < 2) {
        fprintf(stderr, "pass the path to tests/layout.txt\n");
        return 2;
    }
    check(anybox_abi_version() == ANYBOX_ABI_VERSION, "anybox ABI version");
    check(medius_abi_version() == MEDIUS_ABI_VERSION, "medius ABI version");
    check(anybox_has_mock(), "a mock build");
    check(strlen(anybox_version()) > 0, "a version");
    check_layout(argv[1]);

    memset(&options, 0xAB, sizeof options);
    check(anybox_connect_options_default(&options) == ANYBOX_STATUS_OK, "default options");
    check(options.port == NULL && options.family == ANYBOX_FAMILY_ANY && options.can == NULL && options.can_len == 0, "the defaults are zeros");
    check(anybox_open(&options, NULL) == ANYBOX_STATUS_INVALID_ARGUMENT, "a null out is refused");
    check(strcmp(anybox_last_error_message(), "out is null") == 0, "the error says which argument");

    timing.kind = ANYBOX_CLICK_TIMING_FIXED;
    timing.fixed_us = 5000;
    check(anybox_mock_medius_open(&timing, false, &mock, &device) == ANYBOX_STATUS_OK, "open the mock");
    if (device == NULL) {
        return 1;
    }

    check(anybox_device_snapshot(device, &snapshot) == ANYBOX_STATUS_OK, "snapshot");
    check(anybox_snapshot_info(snapshot, &info) == ANYBOX_STATUS_OK, "snapshot info");
    check(info.family == ANYBOX_FAMILY_MEDIUS && strcmp(info.port, "mock box") == 0, "it's the mock Medius");
    check(anybox_snapshot_buttons(snapshot, NULL, 0, &buttons) == ANYBOX_STATUS_OK && buttons == 5, "five buttons");
    anybox_snapshot_free(snapshot);

    check(anybox_device_move_by(device, 10, -5) == ANYBOX_STATUS_OK, "move");
    check(anybox_device_click(device, ANYBOX_BUTTON_LEFT) == ANYBOX_STATUS_OK, "click");
    check(anybox_device_press_key(device, 0x04) == ANYBOX_STATUS_UNSUPPORTED, "no keyboard on the mock");
    check(anybox_last_error_detail(&detail) == ANYBOX_STATUS_OK, "the error's fields");
    check(detail.kind == ANYBOX_STATUS_UNSUPPORTED && detail.has_action && detail.action == ANYBOX_ACTION_KEY_PRESS,
          "the error names its action");
    check(detail.family == ANYBOX_FAMILY_MEDIUS && detail.text != NULL, "and the family and the reason");

    check(anybox_device_events(device, ANYBOX_EVENTS_ALL, &events) == ANYBOX_STATUS_OK, "a receiver");
    /* about 5 s at most, so a mock that never subscribes fails instead of hanging */
    for (tries = 0; !anybox_mock_subscribed(mock, ANYBOX_EVENTS_MOTION) && tries < 500; tries++) {
        anybox_events_recv(events, 10000, &event);
    }
    check(anybox_mock_subscribed(mock, ANYBOX_EVENTS_MOTION), "the receiver subscribed");
    check(anybox_mock_push_motion(mock, 2, 1000, 7, -3, 0, 0) == ANYBOX_STATUS_OK, "push motion");
    check(next_input(events, &event) == ANYBOX_STATUS_OK, "receive the motion");
    check(event.kind == ANYBOX_EVENT_MOTION && event.dx == 7 && event.dy == -3, "the motion it pushed");
    check(event.received_ns <= anybox_monotonic_ns(), "the stamp is on the monotonic clock");
    check(anybox_mock_push_buttons(mock, 3, 2000, true, 1u << ANYBOX_BUTTON_RIGHT) == ANYBOX_STATUS_OK, "push a press");
    check(next_input(events, &event) == ANYBOX_STATUS_OK, "receive the press");
    check(event.kind == ANYBOX_EVENT_BUTTON && event.button == ANYBOX_BUTTON_RIGHT && event.pressed, "the press");

    check(anybox_device_medius(device, &medius) == ANYBOX_STATUS_OK, "the Medius handle");
    check(medius_device_led(medius, MEDIUS_LED_TARGET_BOTH, MEDIUS_LED_MODE_BLINK, 0) == MEDIUS_STATUS_OK, "a Medius-only call");
    medius_device_free(medius);
    check(anybox_device_move_by(device, 1, 0) == ANYBOX_STATUS_OK, "anybox still works after the Medius handle is gone");

    check(anybox_device_close(device) == ANYBOX_STATUS_OK, "close");
    while (anybox_events_recv(events, 0, &event) == ANYBOX_STATUS_OK) {
    }
    check(anybox_events_recv(events, -1, &event) == ANYBOX_STATUS_RECV_CLOSED, "the receiver ends after close");
    check(anybox_device_move_by(device, 1, 0) == ANYBOX_STATUS_CLOSED, "closed");

    anybox_events_free(events);
    anybox_device_free(device);
    anybox_mock_free(mock);

    if (failures == 0) {
        printf("ok\n");
    }
    return failures == 0 ? 0 : 1;
}
