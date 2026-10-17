// a manual run through C++ on a real box: it connects, prints what it found, moves the cursor right and back,
// clicks the left button where the cursor is, then waits up to 20 s for you to press any button on the mouse
// behind the box and closes. on a Medius it also reads the polling rate and reconnects through device.medius(), and
// once anybox is closed it goes through medius' own ways in. pass a port to open that one, otherwise it takes the
// first box it finds

#include "anybox.hpp"

#include <chrono>
#include <cstdio>
#include <functional>
#include <utility>

namespace {

// the port is free again once anybox is closed. the finds take the first box that fits, which can be another one
void through_medius(const std::string& port) {
    namespace medius = anybox::medius;
    for (const auto& each : medius::Device::find_ports()) {
        std::printf("medius sees port %s, serial %s\n", each.path.c_str(), each.serial.value_or("none").c_str());
    }
    std::string id;
    for (const auto& each : medius::Device::list()) {
        std::printf("medius lists %s on %s, id %s, serial %s\n", each.name().c_str(), each.port.path.c_str(),
                    each.id().c_str(), each.serial().value_or("none").c_str());
        if (each.port.path == port) {
            id = each.id();
        }
    }
    std::printf("opened by port: %s\n", medius::to_string(medius::Device::open(port).query_version()).c_str());
    std::printf("opened by id: %s\n", medius::to_string(medius::Device::open_by_id(id).query_version()).c_str());
    const auto on_port = [&](const medius::BoxInfo& box) { return box.port.path == port; };
    std::printf("opened by a match on the port: %s\n",
                medius::to_string(medius::Device::find_where(on_port).query_version()).c_str());
    const std::pair<const char*, std::function<medius::Device()>> finds[] = {
        {"find", medius::Device::find},
        {"find_keyboard_box", medius::Device::find_keyboard_box},
        {"find_mouse_box", medius::Device::find_mouse_box},
    };
    for (const auto& [name, find] : finds) {
        try {
            std::printf("%s: %s\n", name, medius::to_string(find().query_version()).c_str());
        } catch (const medius::Error& error) {
            std::printf("%s: %s\n", name, error.what());
        }
    }
}

}  // namespace

int main(int argc, char** argv) {
    using namespace std::chrono_literals;
    try {
        try {
            anybox::Connect::port("NOPE99").open();
        } catch (const anybox::NotFoundError& error) {
            std::printf("a port with nothing on it is a NotFoundError, %zu port tried\n", error.tried().size());
        }
        auto connect = argc > 1 ? anybox::Connect::port(argv[1]) : anybox::Connect();
        const auto device = connect.can(anybox::Action::Move).open();
        const auto info = device.snapshot().info;
        std::printf("family %u on %s, model %s, firmware %s\n", static_cast<unsigned>(info.family), info.port.c_str(),
                    info.model.value_or("none").c_str(), info.firmware ? anybox::to_string(*info.firmware).c_str() : "none");
        const auto events = device.events(anybox::EventKinds::BUTTONS);
        for (int i = 0; i < 20; i++) {
            device.move_by(10, 0);
        }
        for (int i = 0; i < 20; i++) {
            device.move_by(-10, 0);
        }
        device.click(anybox::Button::Left);
        bool on_medius = false;
        if (const auto medius = device.medius()) {
            on_medius = true;
            const auto rate = medius->query_rate();
            std::printf("polling rate through medius: native every %u us, polled every %u us, %s\n",
                        rate.native_period_us, rate.poll_period_us, rate.confident ? "confident" : "not confident");
        }
        std::printf("moved and clicked, now press a button on the mouse\n");
        const auto deadline = std::chrono::steady_clock::now() + 20s;
        bool pressed = false;
        while (!pressed && std::chrono::steady_clock::now() < deadline) {
            const auto left = std::chrono::duration_cast<std::chrono::microseconds>(deadline - std::chrono::steady_clock::now());
            const auto next = events.recv(left < 0us ? 0us : left);
            if (const auto* button = next ? std::get_if<anybox::ButtonEvent>(&*next) : nullptr; button && button->pressed) {
                std::printf("got button %u down, read %llu us ago on the monotonic clock\n", button->button.value,
                            static_cast<unsigned long long>((anybox::monotonic_ns() - button->at.received_ns) / 1000));
                pressed = true;
            }
        }
        if (!pressed) {
            std::printf("no button came\n");
        }
        if (const auto medius = device.medius()) {
            // the rescan finds this box again by its serial and MAC
            const auto before = medius->query_version();
            medius->reconnect();
            std::printf("reconnect through medius: %s before, %s after\n", anybox::medius::to_string(before).c_str(),
                        anybox::medius::to_string(medius->query_version()).c_str());
        }
        device.close();
        std::printf("close: ok\n");
        if (on_medius) {
            through_medius(info.port);
        }
        return 0;
    } catch (const anybox::Error& error) {
        std::printf("failed: %s\n", error.what());
        return 1;
    }
}
