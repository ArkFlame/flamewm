#ifndef FLAMEWM_API_SETTINGS_H
#define FLAMEWM_API_SETTINGS_H

#include "errors.h"
#include "ids.h"

#include <cstdint>
#include <map>
#include <string>
#include <vector>

namespace flamewm {
namespace api {

struct SettingsSnapshot {
    uint64_t revision;
    std::map<std::string, std::string> values;

    SettingsSnapshot() : revision(0), values() {}
    explicit SettingsSnapshot(uint64_t rev) : revision(rev), values() {}
    SettingsSnapshot(uint64_t rev, const std::map<std::string, std::string>& v)
        : revision(rev), values(v) {}
    SettingsSnapshot(uint64_t rev, std::map<std::string, std::string>&& v)
        : revision(rev), values(std::move(v)) {}

    std::string get(const std::string& key, const std::string& def = "") const {
        std::map<std::string, std::string>::const_iterator it = values.find(key);
        if (it == values.end()) return def;
        return it->second;
    }

    bool has(const std::string& key) const {
        return values.find(key) != values.end();
    }
};

struct SettingsChange {
    std::string key;
    std::string value; // empty value means reset to default / remove key

    SettingsChange() {}
    SettingsChange(const std::string& k, const std::string& v) : key(k), value(v) {}
};

struct SettingsTransaction {
    uint64_t expectedRevision;
    std::vector<SettingsChange> changes;
    std::string resetSection; // if non-empty, means ResetSection (clear all keys with prefix "section.")

    SettingsTransaction() : expectedRevision(0), changes(), resetSection() {}
    explicit SettingsTransaction(uint64_t rev) : expectedRevision(rev), changes(), resetSection() {}
    SettingsTransaction(uint64_t rev, const std::vector<SettingsChange>& c, const std::string& rs)
        : expectedRevision(rev), changes(c), resetSection(rs) {}
    SettingsTransaction(uint64_t rev, std::vector<SettingsChange>&& c, std::string&& rs)
        : expectedRevision(rev), changes(std::move(c)), resetSection(std::move(rs)) {}

    bool hasResetSection() const { return !resetSection.empty(); }
    bool empty() const { return changes.empty() && resetSection.empty(); }
};

} // namespace api
} // namespace flamewm

#endif // FLAMEWM_API_SETTINGS_H
