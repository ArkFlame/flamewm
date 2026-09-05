#ifndef FLAMEWM_ENGINE_ICEWM_UI_NATIVE_THEME_RESOURCES_H
#define FLAMEWM_ENGINE_ICEWM_UI_NATIVE_THEME_RESOURCES_H

#include "flamewm/ui/style/theme.h"

#include <string>

namespace flamewm {
namespace engine {
namespace icewm {
namespace ui {

class NativeThemeResources {
public:
    NativeThemeResources();

    void save();
    void assign(const flamewm::ui::style::Theme& theme);
    void restore();

private:
    const char* savedDefaultTaskBar_;
    const char* savedNormalButton_;
    const char* savedNormalButtonText_;
    const char* savedActiveButton_;
    const char* savedActiveButtonText_;
    const char* savedWorkspaceNormalButton_;
    const char* savedWorkspaceNormalButtonText_;
    const char* savedWorkspaceActiveButton_;
    const char* savedWorkspaceActiveButtonText_;
    std::string defaultTaskBar_;
    std::string normalButton_;
    std::string normalButtonText_;
    std::string activeButton_;
    std::string activeButtonText_;
    std::string workspaceNormalButton_;
    std::string workspaceNormalButtonText_;
    std::string workspaceActiveButton_;
    std::string workspaceActiveButtonText_;
};

} // namespace ui
} // namespace icewm
} // namespace engine
} // namespace flamewm

#endif
