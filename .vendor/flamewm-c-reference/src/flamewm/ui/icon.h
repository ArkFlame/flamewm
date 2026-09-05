#ifndef FLAMEWM_UI_ICON_H
#define FLAMEWM_UI_ICON_H

#include "flamewm/ui/iconroles.h"

#include <string>

namespace flamewm {
namespace ui {

class Icon {
public:
    explicit Icon(IconRole role) : role_(role) {}
    virtual ~Icon() {}

    IconRole role() const { return role_; }
    virtual std::string name() const { return iconRoleName(role_); }

private:
    IconRole role_;
};

} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_ICON_H
