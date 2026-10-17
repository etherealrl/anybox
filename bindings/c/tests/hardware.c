/* a manual run through C on a real box: it connects, prints what it found, moves the cursor right and back,
   clicks the left button where the cursor is, then waits up to 20 s for you to press any button on the mouse
   behind the box and closes. pass a port to open that one, otherwise it takes the first box it finds */

#include <stdio.h>

#include "anybox.h"

static int failed(const char *what) {
    printf("couldn't %s: %s\n", what, anybox_last_error_message());
    return 1;
}

int main(int argc, char **argv) {
    AnyboxConnectOptions options;
    AnyboxDevice *device = NULL;
    AnyboxSnapshot *snapshot = NULL;
    AnyboxEvents *events = NULL;
    AnyboxInfo info;
    AnyboxEvent event;
    AnyboxStatus status;
    AnyboxErrorDetail detail;
    AnyboxStringList *tried = NULL;
    const uint32_t can[1] = {ANYBOX_ACTION_MOVE};
    int i;

    if (anybox_abi_version() != ANYBOX_ABI_VERSION) {
        printf("this library doesn't go with this anybox.h\n");
        return 1;
    }
    anybox_connect_options_default(&options);
    options.port = "NOPE99";
    if (anybox_open(&options, &device) != ANYBOX_STATUS_OK && anybox_last_error_detail(&detail) == ANYBOX_STATUS_OK &&
        anybox_last_error_tried(&tried) == ANYBOX_STATUS_OK) {
        printf("a port with nothing on it is status %u, %u port tried\n", detail.kind,
               (unsigned)anybox_string_list_count(tried));
        anybox_string_list_free(tried);
    }
    options.port = argc > 1 ? argv[1] : NULL;
    options.can = can;
    options.can_len = 1;
    if (anybox_open(&options, &device) != ANYBOX_STATUS_OK) {
        return failed("connect");
    }
    if (anybox_device_snapshot(device, &snapshot) != ANYBOX_STATUS_OK ||
        anybox_snapshot_info(snapshot, &info) != ANYBOX_STATUS_OK) {
        return failed("read the snapshot");
    }
    printf("family %u on %s, model %s, ", info.family, info.port, info.model ? info.model : "none");
    switch (info.firmware_kind) {
    case ANYBOX_FIRMWARE_VERSION:
        printf("firmware %u.%u.%u\n", info.firmware_major, info.firmware_minor, info.firmware_patch);
        break;
    case ANYBOX_FIRMWARE_BUILD:
        printf("firmware build %u\n", info.firmware_build);
        break;
    case ANYBOX_FIRMWARE_TEXT:
        printf("firmware %s\n", info.firmware_text);
        break;
    default:
        printf("no firmware reported\n");
    }
    anybox_snapshot_free(snapshot);

    if (anybox_device_events(device, ANYBOX_EVENTS_BUTTONS, &events) != ANYBOX_STATUS_OK) {
        return failed("listen for buttons");
    }
    for (i = 0; i < 20; i++) {
        if (anybox_device_move_by(device, 10, 0) != ANYBOX_STATUS_OK) {
            return failed("move");
        }
    }
    for (i = 0; i < 20; i++) {
        anybox_device_move_by(device, -10, 0);
    }
    if (anybox_device_click(device, ANYBOX_BUTTON_LEFT) != ANYBOX_STATUS_OK) {
        return failed("click");
    }
    printf("moved and clicked, now press a button on the mouse\n");
    do {
        status = anybox_events_recv(events, 20000000, &event);
    } while (status == ANYBOX_STATUS_OK && !(event.kind == ANYBOX_EVENT_BUTTON && event.pressed));
    if (status == ANYBOX_STATUS_OK) {
        printf("got button %u down, read %llu us ago on the monotonic clock\n", event.button,
               (unsigned long long)((anybox_monotonic_ns() - event.received_ns) / 1000));
    } else {
        printf("no button came: %s\n", anybox_last_error_message());
    }

    status = anybox_device_close(device);
    printf("close: %s\n", status == ANYBOX_STATUS_OK ? "ok" : anybox_last_error_message());
    anybox_events_free(events);
    anybox_device_free(device);
    return 0;
}
