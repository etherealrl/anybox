// the second translation unit of the C++ tests, so a definition in anybox.hpp that isn't inline fails to link. it
// includes the headers the other way round from tests.cpp, and windows.h first on Windows, whose DELETE, min and
// max macros the header has to live with

#if defined(_WIN32)
#include <windows.h>
#endif

#include "anybox.hpp"
#include "anybox.h"

#include <string>

std::string from_the_second_unit() {
    return std::to_string(anybox::wraps_after(anybox::Clock::MediusHostChip).count()) + " " +
           std::to_string(anybox::Key::A.value) + " " + anybox::to_string(anybox::Firmware{}) + " " +
           std::to_string(anybox::Capabilities().get(anybox::Action::Move).is_supported());
}

// windows.h has a MAX_PATH macro, which is why medius' constants are PascalCase
std::string medius_from_the_second_unit() {
    return std::to_string(anybox::medius::MaxPath) + " " + std::to_string(anybox::medius::Key::A.value);
}
