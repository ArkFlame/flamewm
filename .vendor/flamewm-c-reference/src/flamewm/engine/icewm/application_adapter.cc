#include "flamewm/engine/icewm/application_adapter.h"

#include <unistd.h>

#include <cstdlib>
#include <vector>

namespace flamewm {
namespace engine {
namespace icewm {

ApplicationAdapter::ApplicationAdapter() {}

ApplicationAdapter::~ApplicationAdapter() {}

bool ApplicationAdapter::isSafeUri(const std::string& uri) {
    if (uri.empty())
        return false;
    // Reject shell metacharacters; allow only URI-safe launch.
    // Actual launch uses exec vector (no shell) — this is defense-in-depth.
    if (uri.find('\0') != std::string::npos)
        return false;
    if (uri.find('\n') != std::string::npos || uri.find('\r') != std::string::npos)
        return false;
    // Defense-in-depth: reject shell injection primitives even for
    // whitelisted schemes (space, ;, |, $, `, <>, (), {}, !, \, '", ~).
    // NOTE: '&' is intentionally allowed here — it is a valid URL query
    // delimiter and exec-vector keeps it as data, not a shell operator.
    // ';' and space remain rejected to block "https://x; rm -rf" style.
    const std::string shellDanger = " \t`$;|<>`(){}!\\\"'~";
    // Also reject backtick/$/; etc. before whitelist early-return so
    // https:// + injection cannot bypass via early return.
    if (uri.find_first_of(shellDanger) != std::string::npos) {
        // Allow ':' '/' '?' '=' '&' '%' '#' '+' '@' '-' '.' '_' etc.
        // shellDanger already excludes those, so any hit is a real reject.
        return false;
    }
    // Whitelisted schemes / absolute path — already screened for shell chars.
    if (uri.compare(0, 7, "http://") == 0) return true;
    if (uri.compare(0, 8, "https://") == 0) return true;
    if (uri.compare(0, 7, "file://") == 0) return true;
    if (uri.compare(0, 7, "mailto:") == 0) return true;
    if (!uri.empty() && uri[0] == '/') return true;
    // Generic scheme: form (e.g., obsidian://) — also screened above but
    // keep extra guard for any remaining forbidden chars. Note: '&' allowed
    // as query delimiter (exec-vector safe).
    const std::string forbidden = " \t`$(){};|<>!*?\\\"'~#";
    if (uri.find_first_of(forbidden) != std::string::npos) {
        const std::string shell = "`$;|<>(){}!\\\"'~";
        if (uri.find_first_of(shell) != std::string::npos)
            return false;
    }
    // Require at least a ':' scheme separator for non-path URIs.
    if (uri.find(':') == std::string::npos)
        return false;
    return true;
}

api::Status ApplicationAdapter::launchExecVector(const std::vector<std::string>& argv) {
    if (argv.empty() || argv[0].empty()) {
        return api::Status::make(api::Error::InvalidArgument, "empty argv");
    }
    // Validate no embedded NUL and no empty leading arg.
    for (size_t i = 0; i < argv.size(); ++i) {
        if (argv[i].find('\0') != std::string::npos) {
            return api::Status::make(api::Error::InvalidArgument, "argv contains NUL");
        }
    }

    pid_t pid = fork();
    if (pid < 0) {
        return api::Status::make(api::Error::IoFailure, "fork failed");
    }
    if (pid == 0) {
        // Child: build C argv and exec. No shell interpolation.
        std::vector<char*> cargv;
        cargv.reserve(argv.size() + 1);
        for (size_t i = 0; i < argv.size(); ++i) {
            cargv.push_back(const_cast<char*>(argv[i].c_str()));
        }
        cargv.push_back(nullptr);
        execvp(cargv[0], cargv.data());
        _exit(127);
    }
    // Parent: success — child owns execution. No wait (fire-and-forget).
    return api::Status::Ok();
}

api::Status ApplicationAdapter::launch(const api::DesktopAppId& id,
                                       const std::vector<std::string>& args) {
    if (id.empty()) {
        return api::Status::make(api::Error::InvalidArgument, "empty app id");
    }
    for (size_t i = 0; i < args.size(); ++i) {
        if (args[i].find('\0') != std::string::npos) {
            return api::Status::make(api::Error::InvalidArgument, "arg contains NUL");
        }
    }

#if __has_include(<gio/gio.h>)
    // When Gio is available, prefer Gio::AppInfo / g_app_info_launch.
    // Keep X11-free header inclusion: probe at runtime via exec path.
    // For syntax-only / C++11 stub, use xdg-open vector (no shell).
    (void)0;
#endif

    // Safe argv launch: resolve DesktopAppId to desktop file exec.
    // Until launcher registry is wired, launch via gio/xdg-open vector:
    //   gio launch <desktop-id>  or  gtk-launch <id>  or  xdg-open
    // For now, treat DesktopAppId value as executable name (validated,
    // no shell). Desktop file Exec expansion will be added when
    // integrations/launcher is connected. Use exec vector directly.
    std::vector<std::string> argv;
    argv.reserve(1 + args.size());
    argv.push_back(id.value);
    for (size_t i = 0; i < args.size(); ++i) {
        argv.push_back(args[i]);
    }
    // Basic allowlist: reject ids containing shell metachars / path traversal.
    const std::string shell = "`$;|&<>(){}!*?\\\"'~#";
    if (id.value.find_first_of(shell) != std::string::npos) {
        return api::Status::make(api::Error::InvalidArgument, "app id contains shell metachar");
    }
    if (id.value.find("..") != std::string::npos) {
        return api::Status::make(api::Error::InvalidArgument, "app id contains ..");
    }
    return launchExecVector(argv);
}

api::Status ApplicationAdapter::launchUri(const std::string& uri) {
    if (!isSafeUri(uri)) {
        return api::Status::make(api::Error::InvalidArgument, "invalid uri");
    }

#if __has_include(<gio/gio.h>)
    (void)0;
#endif
    // Safe URI launch: Gio::AppInfo / g_app_info_launch_default_for_uri
    // when available, else xdg-open via exec vector (no shell).
    std::vector<std::string> argv;
    argv.push_back("xdg-open");
    argv.push_back(uri);
    return launchExecVector(argv);
}

} // namespace icewm
} // namespace engine
} // namespace flamewm
