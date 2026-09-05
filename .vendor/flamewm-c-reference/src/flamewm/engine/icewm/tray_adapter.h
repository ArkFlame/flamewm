#pragma once

#include "flamewm/api/ports.h"
#include "flamewm/api/ids.h"

namespace flamewm {
namespace engine {
namespace icewm {
namespace ui { class PanelRegistry; }

class TrayAdapter : public api::TrayPort {
public:
    TrayAdapter();
    explicit TrayAdapter(void* display);
    ~TrayAdapter();

    void setDisplay(void* display);
    void* display() const;

    // Engine wiring: native YXTray* as void* (X11-free header).
    // Registry is engine-only OutputId -> native panel surface. When
    // nullptr, PanelRegistry::instance() is used as fallback.
    void setTray(void* tray);
    void* tray() const;
    void setRegistry(ui::PanelRegistry* registry);
    ui::PanelRegistry* registry() const;

    // One XEmbed selection owner per process. Reuses mature YXTray;
    // secondary panels must not claim a second selection. Changing
    // owner reparents the single native tray presentation to the
    // selected panel.
    api::Status setOwner(api::OutputId output) override;
    api::Result<api::OutputId> owner() override;

private:
    void* display_;
    void* tray_;
    ui::PanelRegistry* registry_;
    api::OutputId owner_;
    bool hasOwner_;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm
