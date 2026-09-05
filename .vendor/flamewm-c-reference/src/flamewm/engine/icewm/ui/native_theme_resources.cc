#include "native_theme_resources.h"

#include "prefs.h"

namespace flamewm {
namespace engine {
namespace icewm {
namespace ui {

NativeThemeResources::NativeThemeResources()
    : savedDefaultTaskBar_(0), savedNormalButton_(0), savedNormalButtonText_(0),
      savedActiveButton_(0), savedActiveButtonText_(0), savedWorkspaceNormalButton_(0),
      savedWorkspaceNormalButtonText_(0), savedWorkspaceActiveButton_(0),
      savedWorkspaceActiveButtonText_(0) {
    save();
}

void NativeThemeResources::save() {
    // Snapshot the writable slots themselves, not the pointed-to text.
    const char* defaultTaskBar = clrDefaultTaskBar;
    const char* normalButton = clrNormalButton;
    const char* normalButtonText = clrNormalButtonText;
    const char* activeButton = clrActiveButton;
    const char* activeButtonText = clrActiveButtonText;
    const char* workspaceNormalButton = clrWorkspaceNormalButton;
    const char* workspaceNormalButtonText = clrWorkspaceNormalButtonText;
    const char* workspaceActiveButton = clrWorkspaceActiveButton;
    const char* workspaceActiveButtonText = clrWorkspaceActiveButtonText;
    savedDefaultTaskBar_ = defaultTaskBar;
    savedNormalButton_ = normalButton;
    savedNormalButtonText_ = normalButtonText;
    savedActiveButton_ = activeButton;
    savedActiveButtonText_ = activeButtonText;
    savedWorkspaceNormalButton_ = workspaceNormalButton;
    savedWorkspaceNormalButtonText_ = workspaceNormalButtonText;
    savedWorkspaceActiveButton_ = workspaceActiveButton;
    savedWorkspaceActiveButtonText_ = workspaceActiveButtonText;
}

void NativeThemeResources::assign(const flamewm::ui::style::Theme& theme) {
    // Finish all owned projections before publishing their c_str() pointers.
    defaultTaskBar_ = theme.shell.panelColor;
    normalButton_ = theme.surfaceRaised.value;
    normalButtonText_ = theme.primaryText().value;
    activeButton_ = theme.accent.value;
    activeButtonText_ = theme.primaryText().value;
    workspaceNormalButton_ = theme.surface.value;
    workspaceNormalButtonText_ = theme.secondaryText().value;
    workspaceActiveButton_ = theme.accent.value;
    workspaceActiveButtonText_ = theme.primaryText().value;
    clrDefaultTaskBar = defaultTaskBar_.c_str();
    clrNormalButton = normalButton_.c_str();
    clrNormalButtonText = normalButtonText_.c_str();
    clrActiveButton = activeButton_.c_str();
    clrActiveButtonText = activeButtonText_.c_str();
    clrWorkspaceNormalButton = workspaceNormalButton_.c_str();
    clrWorkspaceNormalButtonText = workspaceNormalButtonText_.c_str();
    clrWorkspaceActiveButton = workspaceActiveButton_.c_str();
    clrWorkspaceActiveButtonText = workspaceActiveButtonText_.c_str();
}

void NativeThemeResources::restore() {
    clrDefaultTaskBar = savedDefaultTaskBar_;
    clrNormalButton = savedNormalButton_;
    clrNormalButtonText = savedNormalButtonText_;
    clrActiveButton = savedActiveButton_;
    clrActiveButtonText = savedActiveButtonText_;
    clrWorkspaceNormalButton = savedWorkspaceNormalButton_;
    clrWorkspaceNormalButtonText = savedWorkspaceNormalButtonText_;
    clrWorkspaceActiveButton = savedWorkspaceActiveButton_;
    clrWorkspaceActiveButtonText = savedWorkspaceActiveButtonText_;
}

} // namespace ui
} // namespace icewm
} // namespace engine
} // namespace flamewm
