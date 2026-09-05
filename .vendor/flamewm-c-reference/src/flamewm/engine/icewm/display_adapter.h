#pragma once

#include "flamewm/api/ports.h"

namespace flamewm {
namespace engine {
namespace icewm {

class DisplayAdapter : public api::DisplayPort {
public:
    DisplayAdapter();
    ~DisplayAdapter() override;

    api::Result<api::DisplaySnapshot> queryFresh() override;
    api::Status capture() override;
    api::Status applyMode(api::OutputId output, api::ModeId mode, api::TransactionId* outTx) override;
    api::Status restore(api::TransactionId tx) override;

private:
    struct Impl;
    Impl* impl_;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm
