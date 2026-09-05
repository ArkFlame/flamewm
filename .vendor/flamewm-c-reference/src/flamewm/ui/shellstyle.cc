#include "shellstyle.h"
#include "flamewm/ui/style/theme.h"

namespace flamewm {

ShellStyle ShellStyle::defaults() {
    return ::flamewm::ui::style::Theme::defaults().shell;
}

} // namespace flamewm
