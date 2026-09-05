#pragma once

#include "flamewm/api/ports.h"

#include <utility>
#include <vector>

namespace flamewm {
namespace engine {
namespace icewm {

class WorkAreaAdapter : public api::WorkAreaPort {
public:
    WorkAreaAdapter();
    ~WorkAreaAdapter() override;

    std::vector<api::Rect> baseWorkAreas() override;
    void applyFlameReservations(const std::vector<std::pair<api::OutputId, api::Rect> >& reservations) override;
    void requestRecompute() override;

private:
    struct Impl;
    Impl* impl_;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm
