#ifndef FLAMEWM_API_ERRORS_H
#define FLAMEWM_API_ERRORS_H

#ifdef None
#pragma push_macro("None")
#undef None
#define FLAMEWM_API_ERRORS_RN
#endif
#ifdef Status
#pragma push_macro("Status")
#undef Status
#define FLAMEWM_API_ERRORS_RS
#endif

#include <string>
#include <utility>

namespace flamewm {
namespace api {

enum class Error {
    None = 0,
    InvalidArgument = 1,
    NotFound = 2,
    StaleRevision = 3,
    Conflict = 4,
    Busy = 5,
    Unsupported = 6,
    Unavailable = 7,
    PermissionDenied = 8,
    Timeout = 9,
    IoFailure = 10,
    EngineRejected = 11,
    InternalFailure = 12
};

inline const char* errorName(Error e) {
    switch (e) {
        case Error::None:             return "None";
        case Error::InvalidArgument:  return "InvalidArgument";
        case Error::NotFound:         return "NotFound";
        case Error::StaleRevision:    return "StaleRevision";
        case Error::Conflict:         return "Conflict";
        case Error::Busy:             return "Busy";
        case Error::Unsupported:      return "Unsupported";
        case Error::Unavailable:      return "Unavailable";
        case Error::PermissionDenied: return "PermissionDenied";
        case Error::Timeout:          return "Timeout";
        case Error::IoFailure:        return "IoFailure";
        case Error::EngineRejected:   return "EngineRejected";
        case Error::InternalFailure:  return "InternalFailure";
        default:                      return "Unknown";
    }
}

struct Status {
    Error code;
    std::string message;

    Status() : code(Error::None), message() {}
    Status(Error c, const std::string& m) : code(c), message(m) {}
    Status(Error c, std::string&& m) : code(c), message(std::move(m)) {}

    bool ok() const { return code == Error::None; }

    static Status Ok() { return Status(Error::None, std::string()); }

    static Status make(Error c, const std::string& m) { return Status(c, m); }
    static Status make(Error c, std::string&& m) { return Status(c, std::move(m)); }
    static Status make(Error c, const char* m) { return Status(c, std::string(m)); }
};

template <typename T>
class Result {
public:
    Result() : ok_(false), value_(), status_(Error::InternalFailure, "uninitialized") {}

    bool isOk() const { return ok_; }
    bool ok() const { return ok_; }

    const T& value() const { return value_; }
    T& value() { return value_; }

    const Status& error() const { return status_; }
    const Status& status() const { return status_; }

    static Result Ok(const T& v) {
        Result r;
        r.ok_ = true;
        r.value_ = v;
        r.status_ = Status::Ok();
        return r;
    }

    static Result Ok(T&& v) {
        Result r;
        r.ok_ = true;
        r.value_ = std::move(v);
        r.status_ = Status::Ok();
        return r;
    }

    static Result Err(const Status& s) {
        Result r;
        r.ok_ = false;
        r.status_ = s;
        return r;
    }

    static Result Err(Status&& s) {
        Result r;
        r.ok_ = false;
        r.status_ = std::move(s);
        return r;
    }

    static Result Err(Error code, const std::string& msg) {
        return Err(Status::make(code, msg));
    }

    static Result Err(Error code, const char* msg) {
        return Err(Status::make(code, msg));
    }

private:
    bool ok_;
    T value_;
    Status status_;
};

template <>
class Result<void> {
public:
    Result() : ok_(false), status_(Error::InternalFailure, "uninitialized") {}
    explicit Result(const Status& s) : ok_(s.ok()), status_(s) {}

    bool isOk() const { return ok_; }
    bool ok() const { return ok_; }

    const Status& error() const { return status_; }
    const Status& status() const { return status_; }

    static Result Ok() {
        Result r;
        r.ok_ = true;
        r.status_ = Status::Ok();
        return r;
    }

    static Result Err(const Status& s) {
        Result r;
        r.ok_ = false;
        r.status_ = s;
        return r;
    }

    static Result Err(Status&& s) {
        Result r;
        r.ok_ = false;
        r.status_ = std::move(s);
        return r;
    }

    static Result Err(Error code, const std::string& msg) {
        return Err(Status::make(code, msg));
    }

private:
    bool ok_;
    Status status_;
};

} // namespace api
} // namespace flamewm

#ifdef FLAMEWM_API_ERRORS_RS
#pragma pop_macro("Status")
#undef FLAMEWM_API_ERRORS_RS
#endif
#ifdef FLAMEWM_API_ERRORS_RN
#pragma pop_macro("None")
#undef FLAMEWM_API_ERRORS_RN
#endif

#endif // FLAMEWM_API_ERRORS_H
