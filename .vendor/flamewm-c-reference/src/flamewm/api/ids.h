#ifndef FLAMEWM_API_IDS_H
#define FLAMEWM_API_IDS_H

#include <cstdint>
#include <string>
#include <functional>

namespace flamewm {
namespace api {

struct WindowRef {
    uint64_t id;
    uint64_t generation;

    WindowRef() : id(0), generation(0) {}
    WindowRef(uint64_t i, uint64_t g) : id(i), generation(g) {}

    bool valid() const { return id != 0; }

    bool operator==(const WindowRef& o) const { return id == o.id && generation == o.generation; }
    bool operator!=(const WindowRef& o) const { return !(*this == o); }
    bool operator<(const WindowRef& o) const {
        if (id != o.id) return id < o.id;
        return generation < o.generation;
    }

    std::string toString() const {
        return std::to_string(id) + ":" + std::to_string(generation);
    }

    struct Hash {
        std::size_t operator()(const WindowRef& v) const {
            // FNV-style combine, C++11 compatible
            std::size_t h1 = std::hash<uint64_t>()(v.id);
            std::size_t h2 = std::hash<uint64_t>()(v.generation);
            return h1 ^ (h2 + 0x9e3779b9u + (h1 << 6) + (h1 >> 2));
        }
    };
};

struct WorkspaceRef {
    int index;
    uint64_t revision;

    WorkspaceRef() : index(-1), revision(0) {}
    WorkspaceRef(int idx, uint64_t rev) : index(idx), revision(rev) {}

    bool valid() const { return index >= 0; }

    bool operator==(const WorkspaceRef& o) const { return index == o.index && revision == o.revision; }
    bool operator!=(const WorkspaceRef& o) const { return !(*this == o); }
    bool operator<(const WorkspaceRef& o) const {
        if (index != o.index) return index < o.index;
        return revision < o.revision;
    }

    std::string toString() const {
        return std::to_string(index) + ":" + std::to_string(revision);
    }

    struct Hash {
        std::size_t operator()(const WorkspaceRef& v) const {
            std::size_t h1 = std::hash<int>()(v.index);
            std::size_t h2 = std::hash<uint64_t>()(v.revision);
            return h1 ^ (h2 + 0x9e3779b9u + (h1 << 6) + (h1 >> 2));
        }
    };
};

struct OutputId {
    std::string key;

    OutputId() {}
    explicit OutputId(const std::string& k) : key(k) {}
    explicit OutputId(std::string&& k) : key(std::move(k)) {}

    bool valid() const { return !key.empty(); }
    bool empty() const { return key.empty(); }

    bool operator==(const OutputId& o) const { return key == o.key; }
    bool operator!=(const OutputId& o) const { return key != o.key; }
    bool operator<(const OutputId& o) const { return key < o.key; }

    std::string toString() const { return key; }

    struct Hash {
        std::size_t operator()(const OutputId& v) const {
            return std::hash<std::string>()(v.key);
        }
    };
};

struct ModeId {
    uint64_t value;

    ModeId() : value(0) {}
    explicit ModeId(uint64_t v) : value(v) {}

    bool valid() const { return value != 0; }

    bool operator==(const ModeId& o) const { return value == o.value; }
    bool operator!=(const ModeId& o) const { return value != o.value; }
    bool operator<(const ModeId& o) const { return value < o.value; }

    std::string toString() const { return std::to_string(value); }

    struct Hash {
        std::size_t operator()(const ModeId& v) const {
            return std::hash<uint64_t>()(v.value);
        }
    };
};

struct TransactionId {
    uint64_t value;

    TransactionId() : value(0) {}
    explicit TransactionId(uint64_t v) : value(v) {}

    bool valid() const { return value != 0; }

    bool operator==(const TransactionId& o) const { return value == o.value; }
    bool operator!=(const TransactionId& o) const { return value != o.value; }
    bool operator<(const TransactionId& o) const { return value < o.value; }

    std::string toString() const { return std::to_string(value); }

    struct Hash {
        std::size_t operator()(const TransactionId& v) const {
            return std::hash<uint64_t>()(v.value);
        }
    };
};

struct DesktopAppId {
    std::string value;

    DesktopAppId() {}
    explicit DesktopAppId(const std::string& v) : value(v) {}
    explicit DesktopAppId(std::string&& v) : value(std::move(v)) {}

    bool valid() const { return !value.empty(); }
    bool empty() const { return value.empty(); }

    bool operator==(const DesktopAppId& o) const { return value == o.value; }
    bool operator!=(const DesktopAppId& o) const { return value != o.value; }
    bool operator<(const DesktopAppId& o) const { return value < o.value; }

    std::string toString() const { return value; }

    struct Hash {
        std::size_t operator()(const DesktopAppId& v) const {
            return std::hash<std::string>()(v.value);
        }
    };
};

struct TaskEntryId {
    std::string value;

    TaskEntryId() {}
    explicit TaskEntryId(const std::string& v) : value(v) {}
    explicit TaskEntryId(std::string&& v) : value(std::move(v)) {}

    bool valid() const { return !value.empty(); }
    bool empty() const { return value.empty(); }

    bool isPinned() const { return value.compare(0, 4, "pin:") == 0; }
    bool isWindow() const { return value.compare(0, 4, "win:") == 0; }

    static TaskEntryId pinned(const std::string& id) { return TaskEntryId("pin:" + id); }
    static TaskEntryId pinned(std::string&& id) { return TaskEntryId(std::string("pin:") + std::move(id)); }
    static TaskEntryId window(const std::string& id) { return TaskEntryId("win:" + id); }
    static TaskEntryId window(uint64_t winId) { return TaskEntryId("win:" + std::to_string(winId)); }

    std::string inner() const {
        if (value.size() > 4 && value[3] == ':') return value.substr(4);
        return value;
    }

    bool operator==(const TaskEntryId& o) const { return value == o.value; }
    bool operator!=(const TaskEntryId& o) const { return value != o.value; }
    bool operator<(const TaskEntryId& o) const { return value < o.value; }

    std::string toString() const { return value; }

    struct Hash {
        std::size_t operator()(const TaskEntryId& v) const {
            return std::hash<std::string>()(v.value);
        }
    };
};

} // namespace api
} // namespace flamewm

#endif // FLAMEWM_API_IDS_H
