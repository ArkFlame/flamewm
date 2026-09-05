#pragma once

#ifdef Status
#pragma push_macro("Status")
#undef Status
#define FLAMEWM_WINDOW_ADAPTER_RS
#endif

#include "flamewm/api/ports.h"

#include <cstdint>

namespace flamewm {
namespace engine {
namespace icewm {

class WindowAdapter : public api::WindowPort {
public:
    WindowAdapter();
    ~WindowAdapter() override;

    api::Result<api::WindowSnapshot> get(api::WindowRef ref) override;
    std::vector<api::WindowSnapshot> snapshot() override;
    api::Status activate(api::WindowRef ref) override;
    api::Status minimize(api::WindowRef ref) override;
    api::Status maximize(api::WindowRef ref) override;
    api::Status restore(api::WindowRef ref) override;
    api::Status close(api::WindowRef ref) override;
    api::Status setOuterGeometry(api::WindowRef ref, api::Rect rect) override;
    api::Result<api::Rect> workArea(api::WindowRef ref) override;
    api::Result<api::OutputId> output(api::WindowRef ref) override;

    // Resolve current IceWM window identity, including reuse generation.
    static api::WindowRef currentRef(uint64_t id);

private:
    struct Impl;
    Impl* impl_;
    static WindowAdapter* active_;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm

#ifdef FLAMEWM_WINDOW_ADAPTER_RS
#pragma pop_macro("Status")
#undef FLAMEWM_WINDOW_ADAPTER_RS
#endif
