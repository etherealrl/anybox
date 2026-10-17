// the JSON the C++ conformance runner reads and writes, only what the suite has: objects, arrays, strings, integers,
// booleans and null. a number never goes through a double, the suite has 64 bit ones, and two values compare like
// Python's == and C#'s DeepEquals, an object's keys in any order and an array's in its own

#pragma once

#include <cstdint>
#include <limits>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <utility>
#include <variant>
#include <vector>

namespace json {

struct Value;

using Array = std::vector<Value>;
// a vector and not a map, a map of a type that isn't complete yet isn't allowed
using Object = std::vector<std::pair<std::string, Value>>;

struct Value {
    std::variant<std::nullptr_t, bool, std::int64_t, std::uint64_t, std::string, Array, Object> data;

    Value() : data(nullptr) {}
    Value(std::nullptr_t) : data(nullptr) {}
    Value(bool flag) : data(flag) {}
    Value(std::string text) : data(std::move(text)) {}
    Value(const char* text) : data(std::string(text)) {}
    Value(Array values) : data(std::move(values)) {}
    Value(Object fields) : data(std::move(fields)) {}

    template <class T, std::enable_if_t<std::is_integral<T>::value && !std::is_same<T, bool>::value, int> = 0>
    Value(T number) {
        // one number has one form, so 5 read as signed and 5 made unsigned are the same value
        if (std::is_signed<T>::value || static_cast<std::uint64_t>(number) <=
                                            static_cast<std::uint64_t>((std::numeric_limits<std::int64_t>::max)())) {
            data = static_cast<std::int64_t>(number);
        } else {
            data = static_cast<std::uint64_t>(number);
        }
    }

    bool is_null() const { return std::holds_alternative<std::nullptr_t>(data); }
    bool is_string() const { return std::holds_alternative<std::string>(data); }
    bool is_object() const { return std::holds_alternative<Object>(data); }

    bool as_bool() const { return get<bool>("a boolean"); }
    std::int64_t as_int() const { return get<std::int64_t>("an integer"); }
    const std::string& as_string() const { return get<std::string>("a string"); }
    const Array& as_array() const { return get<Array>("an array"); }
    const Object& as_object() const { return get<Object>("an object"); }

    const Value* find(const std::string& key) const {
        for (const auto& field : as_object()) {
            if (field.first == key) {
                return &field.second;
            }
        }
        return nullptr;
    }

    const Value& operator[](const std::string& key) const {
        if (const auto* found = find(key)) {
            return *found;
        }
        throw std::runtime_error("there's no " + key + " in " + dump());
    }

    // sets the key, in place if it's there and at the end if not
    void set(const std::string& key, Value value) {
        auto& fields = std::get<Object>(data);
        for (auto& field : fields) {
            if (field.first == key) {
                field.second = std::move(value);
                return;
            }
        }
        fields.emplace_back(key, std::move(value));
    }

    void erase(const std::string& key) {
        auto& fields = std::get<Object>(data);
        for (auto field = fields.begin(); field != fields.end(); ++field) {
            if (field->first == key) {
                fields.erase(field);
                return;
            }
        }
    }

    friend bool operator==(const Value& a, const Value& b) {
        if (a.data.index() != b.data.index()) {
            return false;
        }
        if (a.is_object()) {
            const auto& left = a.as_object();
            const auto& right = b.as_object();
            if (left.size() != right.size()) {
                return false;
            }
            for (const auto& field : left) {
                const auto* other = b.find(field.first);
                if (!other || !(*other == field.second)) {
                    return false;
                }
            }
            return true;
        }
        if (std::holds_alternative<Array>(a.data)) {
            const auto& left = a.as_array();
            const auto& right = b.as_array();
            if (left.size() != right.size()) {
                return false;
            }
            for (std::size_t index = 0; index < left.size(); index++) {
                if (!(left[index] == right[index])) {
                    return false;
                }
            }
            return true;
        }
        return a.data == b.data;
    }

    friend bool operator!=(const Value& a, const Value& b) { return !(a == b); }

    std::string dump() const {
        std::string out;
        write(out);
        return out;
    }

    static Value parse(const std::string& text) {
        std::size_t at = 0;
        auto value = parse_value(text, at);
        skip_space(text, at);
        if (at != text.size()) {
            throw std::runtime_error("JSON goes on after its value at byte " + std::to_string(at));
        }
        return value;
    }

private:
    template <class T>
    const T& get(const char* what) const {
        if (const auto* value = std::get_if<T>(&data)) {
            return *value;
        }
        throw std::runtime_error(dump() + " isn't " + what);
    }

    static void write_string(std::string& out, const std::string& text) {
        static const char digits[] = "0123456789abcdef";
        out += '"';
        for (const char c : text) {
            const auto byte = static_cast<unsigned char>(c);
            if (c == '"' || c == '\\') {
                out += '\\';
                out += c;
            } else if (byte < 0x20) {
                out += "\\u00";
                out += digits[byte >> 4];
                out += digits[byte & 0xF];
            } else {
                out += c;
            }
        }
        out += '"';
    }

    void write(std::string& out) const {
        if (is_null()) {
            out += "null";
        } else if (const auto* flag = std::get_if<bool>(&data)) {
            out += *flag ? "true" : "false";
        } else if (const auto* number = std::get_if<std::int64_t>(&data)) {
            out += std::to_string(*number);
        } else if (const auto* big = std::get_if<std::uint64_t>(&data)) {
            out += std::to_string(*big);
        } else if (const auto* text = std::get_if<std::string>(&data)) {
            write_string(out, *text);
        } else if (const auto* values = std::get_if<Array>(&data)) {
            out += '[';
            for (std::size_t index = 0; index < values->size(); index++) {
                if (index > 0) {
                    out += ',';
                }
                (*values)[index].write(out);
            }
            out += ']';
        } else {
            out += '{';
            bool first = true;
            for (const auto& field : as_object()) {
                if (!first) {
                    out += ',';
                }
                first = false;
                write_string(out, field.first);
                out += ':';
                field.second.write(out);
            }
            out += '}';
        }
    }

    static void skip_space(const std::string& text, std::size_t& at) {
        while (at < text.size() && (text[at] == ' ' || text[at] == '\n' || text[at] == '\r' || text[at] == '\t')) {
            at++;
        }
    }

    static char next(const std::string& text, std::size_t& at) {
        if (at >= text.size()) {
            throw std::runtime_error("the JSON ends in the middle of a value");
        }
        return text[at++];
    }

    static void expect(const std::string& text, std::size_t& at, const char* word) {
        for (const char* c = word; *c; c++) {
            if (next(text, at) != *c) {
                throw std::runtime_error(std::string("expected ") + word + " at byte " + std::to_string(at));
            }
        }
    }

    static void append_utf8(std::string& out, std::uint32_t code) {
        if (code < 0x80) {
            out += static_cast<char>(code);
        } else if (code < 0x800) {
            out += static_cast<char>(0xC0 | (code >> 6));
            out += static_cast<char>(0x80 | (code & 0x3F));
        } else if (code < 0x10000) {
            out += static_cast<char>(0xE0 | (code >> 12));
            out += static_cast<char>(0x80 | ((code >> 6) & 0x3F));
            out += static_cast<char>(0x80 | (code & 0x3F));
        } else {
            out += static_cast<char>(0xF0 | (code >> 18));
            out += static_cast<char>(0x80 | ((code >> 12) & 0x3F));
            out += static_cast<char>(0x80 | ((code >> 6) & 0x3F));
            out += static_cast<char>(0x80 | (code & 0x3F));
        }
    }

    static std::uint32_t hex4(const std::string& text, std::size_t& at) {
        std::uint32_t code = 0;
        for (int digit = 0; digit < 4; digit++) {
            const char c = next(text, at);
            code <<= 4;
            if (c >= '0' && c <= '9') {
                code |= static_cast<std::uint32_t>(c - '0');
            } else if (c >= 'a' && c <= 'f') {
                code |= static_cast<std::uint32_t>(c - 'a' + 10);
            } else if (c >= 'A' && c <= 'F') {
                code |= static_cast<std::uint32_t>(c - 'A' + 10);
            } else {
                throw std::runtime_error("a \\u escape needs four hex digits");
            }
        }
        return code;
    }

    static std::string parse_string(const std::string& text, std::size_t& at) {
        std::string out;
        for (;;) {
            const char c = next(text, at);
            if (c == '"') {
                return out;
            }
            if (c != '\\') {
                out += c;
                continue;
            }
            switch (next(text, at)) {
            case '"':
                out += '"';
                break;
            case '\\':
                out += '\\';
                break;
            case '/':
                out += '/';
                break;
            case 'b':
                out += '\b';
                break;
            case 'f':
                out += '\f';
                break;
            case 'n':
                out += '\n';
                break;
            case 'r':
                out += '\r';
                break;
            case 't':
                out += '\t';
                break;
            case 'u': {
                auto code = hex4(text, at);
                // a character past 0xFFFF comes as two escapes, a surrogate pair
                if (code >= 0xD800 && code <= 0xDBFF) {
                    expect(text, at, "\\u");
                    code = 0x10000 + ((code - 0xD800) << 10) + (hex4(text, at) - 0xDC00);
                }
                append_utf8(out, code);
                break;
            }
            default:
                throw std::runtime_error("an unknown escape at byte " + std::to_string(at));
            }
        }
    }

    static Value parse_number(const std::string& text, std::size_t& at) {
        const bool negative = text[at] == '-';
        if (negative) {
            at++;
        }
        std::uint64_t magnitude = 0;
        const auto start = at;
        while (at < text.size() && text[at] >= '0' && text[at] <= '9') {
            magnitude = magnitude * 10 + static_cast<std::uint64_t>(text[at] - '0');
            at++;
        }
        if (at == start || (at < text.size() && (text[at] == '.' || text[at] == 'e' || text[at] == 'E'))) {
            throw std::runtime_error("the suite only has integers, not what's at byte " + std::to_string(start));
        }
        if (negative) {
            return Value(static_cast<std::int64_t>(0 - magnitude));
        }
        return Value(magnitude);
    }

    static Value parse_value(const std::string& text, std::size_t& at) {
        skip_space(text, at);
        if (at >= text.size()) {
            throw std::runtime_error("the JSON ends where a value should be");
        }
        const char c = text[at];
        if (c == '{') {
            at++;
            Object fields;
            skip_space(text, at);
            if (at < text.size() && text[at] == '}') {
                at++;
                return Value(std::move(fields));
            }
            for (;;) {
                skip_space(text, at);
                if (next(text, at) != '"') {
                    throw std::runtime_error("an object key has to be a string, at byte " + std::to_string(at));
                }
                auto key = parse_string(text, at);
                skip_space(text, at);
                if (next(text, at) != ':') {
                    throw std::runtime_error("a key needs a : after it, at byte " + std::to_string(at));
                }
                auto value = parse_value(text, at);
                fields.emplace_back(std::move(key), std::move(value));
                skip_space(text, at);
                const char after = next(text, at);
                if (after == '}') {
                    return Value(std::move(fields));
                }
                if (after != ',') {
                    throw std::runtime_error("expected , or } at byte " + std::to_string(at));
                }
            }
        }
        if (c == '[') {
            at++;
            Array values;
            skip_space(text, at);
            if (at < text.size() && text[at] == ']') {
                at++;
                return Value(std::move(values));
            }
            for (;;) {
                values.push_back(parse_value(text, at));
                skip_space(text, at);
                const char after = next(text, at);
                if (after == ']') {
                    return Value(std::move(values));
                }
                if (after != ',') {
                    throw std::runtime_error("expected , or ] at byte " + std::to_string(at));
                }
            }
        }
        if (c == '"') {
            at++;
            return Value(parse_string(text, at));
        }
        if (c == 't') {
            expect(text, at, "true");
            return Value(true);
        }
        if (c == 'f') {
            expect(text, at, "false");
            return Value(false);
        }
        if (c == 'n') {
            expect(text, at, "null");
            return Value(nullptr);
        }
        return parse_number(text, at);
    }
};

}  // namespace json
