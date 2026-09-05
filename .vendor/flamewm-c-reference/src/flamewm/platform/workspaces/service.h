#ifndef FLAMEWM_PLATFORM_WORKSPACES_SERVICE_H
#define FLAMEWM_PLATFORM_WORKSPACES_SERVICE_H

#include "flamewm/api/ports.h"
#include "flamewm/api/workspace.h"
#include "flamewm/api/errors.h"
#include "flamewm/api/ids.h"

#include <cstdint>
#include <functional>

namespace flamewm {
namespace platform {
namespace workspaces {

class WorkspaceService {
public:
    explicit WorkspaceService(api::WorkspacePort* port);
    // Compatibility shim for PlatformHost stub transition; extra port ignored
    WorkspaceService(api::WorkspacePort* port, api::WorkAreaPort* /*ignored*/);
    ~WorkspaceService();

    WorkspaceService(const WorkspaceService&) = delete;
    WorkspaceService& operator=(const WorkspaceService&) = delete;

    api::WorkspaceSnapshot snapshot() const;
    uint64_t revision() const;

    api::Status applyTransform(const api::WorkspaceTransform& t);

    api::Status activate(int index, uint64_t expectedRevision);
    api::Status insertAfter(int index, uint64_t expectedRevision);
    api::Status remove(int index, uint64_t expectedRevision);

    api::Status moveWindow(api::WindowRef window, int targetWorkspace);

    // Directional navigation via TwoRowTopology (Ctrl+Super+Arrow)
    api::Status navigateLeft(uint64_t expectedRevision);
    api::Status navigateRight(uint64_t expectedRevision);
    api::Status navigateUp(uint64_t expectedRevision);
    api::Status navigateDown(uint64_t expectedRevision);
    // Convenience overloads using current revision
    api::Status navigateLeft();
    api::Status navigateRight();
    api::Status navigateUp();
    api::Status navigateDown();

    int addListener(std::function<void(uint64_t)> cb);
    void removeListener(int id);

private:
    struct Impl;
    Impl* impl_;
};

} // namespace workspaces
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_WORKSPACES_SERVICE_H
