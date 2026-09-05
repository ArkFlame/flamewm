#ifndef FLAMEWM_UI_LABEL_H
#define FLAMEWM_UI_LABEL_H

#include "flamewm/ui/window.h"
#include "flamewm/ui/style/typography.h"

#include <string>

namespace flamewm {
namespace ui {

class Label : public Window {
public:
    virtual void setText(const std::string& text) = 0;
    virtual std::string text() const = 0;
    virtual void setWrap(bool wrap) = 0;
    virtual bool isWrap() const = 0;
    virtual void setTypography(style::TypographyRole role) { typography_ = role; }
    virtual style::TypographyRole typography() const { return typography_; }
    virtual ~Label() {}

protected:
    style::TypographyRole typography_ = style::TypographyBody;
};

} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_LABEL_H
