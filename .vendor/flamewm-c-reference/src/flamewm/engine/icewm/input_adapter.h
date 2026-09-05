#pragma once

#include "flamewm/api/ports.h"

namespace flamewm {
namespace engine {
namespace icewm {

class InputAdapter : public api::InputPort {
public:
    InputAdapter();
    explicit InputAdapter(void* display);
    ~InputAdapter();

    void setDisplay(void* display);
    void* display() const;

    api::Result<api::PointerPosition> rootPointer() override;

private:
    void* display_;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm
