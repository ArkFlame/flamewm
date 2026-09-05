#pragma once

#include "flamewm/api/ports.h"

#include <cstdint>

namespace flamewm {
namespace engine {
namespace icewm {

class WorkspaceAdapter : public api::WorkspacePort {
public:
    WorkspaceAdapter();
    ~WorkspaceAdapter() override;

    api::Result<api::WorkspaceSnapshot> snapshot() override;
    api::Status activate(int index, uint64_t expectedRevision) override;
    api::Status moveWindow(api::WindowRef ref, int targetWorkspace) override;
    api::Status applyTransform(const api::WorkspaceTransform& t) override;

private:
    struct Impl;
    Impl* impl_;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm
