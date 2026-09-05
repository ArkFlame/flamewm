#ifndef FLAMEWM_SHELL_TRAY_HOST_H
#define FLAMEWM_SHELL_TRAY_HOST_H

#include "flamewm/api/ids.h"
#include "flamewm/api/ports.h"
#include "flamewm/ui/window.h"

#if defined(__has_include)
#if __has_include("flamewm/engine/icewm/tray_adapter.h")
#include "flamewm/engine/icewm/tray_adapter.h"
#endif
#endif

namespace flamewm {
namespace shell {

// TrayHost — single XEmbed owner, exposed on one panel.
// IceWM remains the single manager; per-output tray duplication is forbidden.
// Ownership is delegated to api::TrayPort (TrayAdapter). TrayHost validates
// the target OutputId and performs idempotent owner moves without claiming
// a second X selection. Engine PanelRegistry is consulted inside TrayAdapter
// to reparent the single native YXTray presentation to the selected panel
// surface; TrayHost never constructs a second tray.
class TrayHost {
public:
    explicit TrayHost(api::TrayPort* tray);
    ~TrayHost();

    TrayHost(const TrayHost&) = delete;
    TrayHost& operator=(const TrayHost&) = delete;

    void setTrayPort(api::TrayPort* t);
    api::TrayPort* trayPort() const;

    void setContainer(flamewm::ui::Window* c);
    flamewm::ui::Window* container() const;

    api::OutputId owner() const;
    bool hasOwner() const;
    api::Status setOwner(const api::OutputId& o);
    void adopt(api::OutputId o);

    void render();
    void invalidate();

private:
    api::TrayPort* tray_;
    flamewm::ui::Window* container_;
    api::OutputId owner_;
    bool hasOwner_;
};

} // namespace shell
} // namespace flamewm

#endif // FLAMEWM_SHELL_TRAY_HOST_H
