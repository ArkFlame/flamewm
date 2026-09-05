#include "root.h"
#include "common.h"
#include "pages.h"
#include "flamewm/ui/backend.h"
#include "flamewm/ui/label.h"
#include "flamewm/ui/list.h"
#include "flamewm/ui/window.h"
#include "flamewm/ui/button.h"
#include "flamewm/ui/slider.h"
#include "flamewm/ui/textfield.h"
#include "flamewm/ui/image.h"
#include "flamewm/ui/assets/asset.h"
#include "../pages/appearance.h"
#include "../pages/fonts.h"
#include "../pages/about.h"
#include "../../flamewm/api/shortcuts.h"
#include "../../flamewm/control/client.h"
#include "../../flamewm/core/productmetadata.h"
#include "yxapp.h"
namespace flamewm { namespace settings { namespace ui {
Root::Root(SettingsApp* a): app_(a), window_(0), navigation_(0), navigationSurface_(0), wordmark_(0), title_(0), body_(0), status_(0) {}
Root::~Root() {
    for (int page = 0; page < 7; ++page) for (size_t i = 0; i < pageWidgets_[page].size(); ++i) delete pageWidgets_[page][i];
    for (size_t i = 0; i < navigationItems_.size(); ++i) delete navigationItems_[i];
    delete wordmark_; delete navigation_; delete navigationSurface_;
    delete title_; delete body_; delete status_; delete window_;
}
void Root::add(flamewm::ui::Window* w, int page) {
    if (w && page >= 0 && page < 7) { pageWidgets_[page].push_back(w); w->hide(); }
}
flamewm::ui::Label* Root::label(int page, const flamewm::api::Rect& r, const std::string& text, bool wrap) {
    flamewm::ui::Label* l = flamewm::ui::UiBackend::createLabel(window_);
    if (l) { l->setGeometry(r); l->setText(text); l->setWrap(wrap); add(l, page); }
    return l;
}
flamewm::ui::Button* Root::button(int page, const flamewm::api::Rect& r, const std::string& text, flamewm::IconRole role) {
    flamewm::ui::Button* b = flamewm::ui::UiBackend::createButton(window_, role);
    if (b) { b->setGeometry(r); b->setText(text); add(b, page); }
    return b;
}
static void setStatus(flamewm::ui::Label* status, const std::string& text) { if (status) status->setText(text); }
bool Root::create(std::string* error) {
    if (!flamewm::ui::UiBackend::isAvailable()) { if (error) *error = "UiBackend unavailable"; return false; }
    window_ = flamewm::ui::UiBackend::createWindow();
    navigationSurface_ = flamewm::ui::UiBackend::createWindow(window_);
    navigation_ = flamewm::ui::UiBackend::createList(window_);
    wordmark_ = flamewm::ui::UiBackend::createImage(window_);
    title_ = flamewm::ui::UiBackend::createLabel(window_);
    body_ = flamewm::ui::UiBackend::createLabel(window_);
    status_ = flamewm::ui::UiBackend::createLabel(window_);
    if (!window_ || !navigationSurface_ || !navigation_ || !wordmark_ || !title_ || !body_ || !status_) {
        if (error) *error = "failed to create settings widgets"; return false;
    }
    window_->setRole(flamewm::ui::WindowRole::ApplicationWindow);
    window_->setGeometry(flamewm::api::Rect(0, 0, 720, 480));
    navigationSurface_->setGeometry(flamewm::api::Rect(0, 0, 208, 480));
    settingsNav(navigationSurface_);
    wordmark_->setGeometry(flamewm::api::Rect(24, 24, 152, 40));
    const BrandingAssetSpec& branding = aboutBrandingAsset();
    wordmark_->setAsset(flamewm::ui::AssetRef(branding.asset));
    wordmark_->show();
    window_->setOnClose([]() { xapp->exitLoop(0); });
    std::string controllerError;
    appearancePage_.reloadFromControl(app_->controlClient(), &controllerError);
    desktopPage_.reloadFromControl(app_->controlClient(), &controllerError);
    taskbarPage_.reloadFromControl(app_->controlClient(), &controllerError);
    displaysPage_.reloadFromControl(app_->controlClient(), &controllerError);
    fontsPage_.reloadFromControl(app_->controlClient(), &controllerError);
    hotkeysPage_.reloadFromControl(app_->controlClient(), &controllerError);
    std::vector<flamewm::ui::ListRow> rows; const std::vector<PageSpec>& specs = pageSpecs();
    for (size_t i = 0; i < specs.size(); ++i) rows.push_back(flamewm::ui::ListRow(specs[i].name, specs[i].name));
    navigation_->setRows(rows);
    navigation_->setOnSelectionChanged([this](int index) { select(index); });
    place(navigation_, flamewm::api::Rect(16, 84, 176, 330)); navigation_->hide();
    for (size_t i = 0; i < specs.size(); ++i) {
        flamewm::ui::Button* item = flamewm::ui::UiBackend::createButton(window_, static_cast<flamewm::IconRole>(specs[i].iconRole));
        if (!item) continue;
        item->setGeometry(flamewm::api::Rect(16, 84 + static_cast<int>(i) * 42, 176, 36));
        item->setText(specs[i].name); item->setIconRole(static_cast<flamewm::IconRole>(specs[i].iconRole));
        item->setOnClick([this, i]() { select(static_cast<int>(i)); });
        settingsNavItem(item, i == static_cast<size_t>(app_->currentPage())); item->show(); navigationItems_.push_back(item);
    }
    showLabel(title_, flamewm::api::Rect(232, 18, 464, 34), "FLAMEWM   Settings", false);
    showLabel(status_, flamewm::api::Rect(240, 426, 440, 28), "", true);
    for (int i = 0; i < 7; ++i) {
        flamewm::ui::Window* card = flamewm::ui::UiBackend::createWindow(window_);
        if (card) { card->setGeometry(flamewm::api::Rect(208, 60, 504, 352)); settingsCard(card); add(card, i); }
        flamewm::ui::Window* row = flamewm::ui::UiBackend::createWindow(window_);
        if (row) { row->setGeometry(flamewm::api::Rect(224, 112, 472, 68)); settingsRow(row, false); add(row, i); }
        label(i, flamewm::api::Rect(232, 72, 464, 24), specs[i].name, false);
        label(i, flamewm::api::Rect(232, 94, 464, 34), specs[i].description, true);
    }
    /* Appearance: contract presets and icon theme are live-applied through Control. */
    const std::vector<AppearancePresetSpec>& presets = appearancePresets();
    for (size_t i = 0; i < presets.size(); ++i) {
        const AppearancePresetSpec& preset = presets[i];
        const int column = static_cast<int>(i % 4);
        const int row = static_cast<int>(i / 4);
        flamewm::ui::Button* b = button(PageAppearance,
            flamewm::api::Rect(216 + column * 120, 126 + row * 42, 112, 32),
            preset.name, flamewm::IconRoleAppearance);
        if (b) {
            const std::string color = preset.color;
            b->setOnClick([this, color]() {
                std::string e;
                if (!appearancePage_.applyAccentImmediate(app_->controlClient(), color, &e))
                    setStatus(status_, e);
            });
        }
    }
    flamewm::ui::TextField* theme = flamewm::ui::UiBackend::createTextField(window_);
    if (theme) { theme->setGeometry(flamewm::api::Rect(216, 172, 300, 30)); theme->setPlaceholder("Icon theme"); theme->setText(appearancePage_.iconTheme); add(theme, PageAppearance); theme->setOnSubmit([this](const std::string& value) { std::string e; if (!appearancePage_.applyIconThemeImmediate(app_->controlClient(), value, &e)) setStatus(status_, e); }); }

    flamewm::ui::TextField* wallpaper = flamewm::ui::UiBackend::createTextField(window_);
    if (wallpaper) { wallpaper->setGeometry(flamewm::api::Rect(216, 126, 420, 30)); wallpaper->setPlaceholder("Wallpaper path"); add(wallpaper, PageDesktop); wallpaper->setOnSubmit([this](const std::string& value) { std::string e; if (!desktopPage_.applyWallpaperImmediate(app_->controlClient(), value, &e)) setStatus(status_, e); }); }
    flamewm::ui::Slider* desktopOpacity = flamewm::ui::UiBackend::createSlider(window_);
    if (desktopOpacity) { desktopOpacity->setGeometry(flamewm::api::Rect(216, 174, 300, 24)); desktopOpacity->setRange(0, 60); desktopOpacity->setValue(desktopPage_.desktopSelectionFillOpacity); add(desktopOpacity, PageDesktop); desktopOpacity->setOnChanged([this](int v) { std::string e; if (!desktopPage_.applyDesktopOpacityImmediate(app_->controlClient(), v, &e)) setStatus(status_, e); }); }
    flamewm::ui::Slider* snapOpacity = flamewm::ui::UiBackend::createSlider(window_);
    if (snapOpacity) { snapOpacity->setGeometry(flamewm::api::Rect(216, 218, 300, 24)); snapOpacity->setRange(0, 60); snapOpacity->setValue(desktopPage_.windowSnapPreviewFillOpacity); add(snapOpacity, PageDesktop); snapOpacity->setOnChanged([this](int v) { std::string e; if (!desktopPage_.applySnapOpacityImmediate(app_->controlClient(), v, &e)) setStatus(status_, e); }); }
    flamewm::ui::Toggle* sticky = flamewm::ui::UiBackend::createToggle(window_, flamewm::IconRoleDesktop);
    if (sticky) { sticky->setGeometry(flamewm::api::Rect(216, 266, 220, 32)); sticky->setText("Sticky notes"); sticky->setChecked(desktopPage_.stickyNotesEnabled); add(sticky, PageDesktop); sticky->setOnToggled([this](bool value) { std::string e; if (!desktopPage_.applyStickyImmediate(app_->controlClient(), value, desktopPage_.stickyDisableConfirmationPending, &e)) setStatus(status_, e); }); }
    const DesktopContentSpec& desktop = desktopContent();
    label(PageDesktop, flamewm::api::Rect(456, 126, 200, 30), desktop.watermarkVisible ? "Watermark visible" : "Watermark hidden", false);
    flamewm::ui::Button* resetDesktop = button(PageDesktop, flamewm::api::Rect(456, 174, 200, 32), "Reset desktop", flamewm::IconRoleDesktop);
    if (resetDesktop) {
        resetDesktop->setEnabled(desktop.resetAvailable);
        resetDesktop->setOnClick([this]() {
            if (!app_->controlClient()) { setStatus(status_, "desktop reset failed"); return; }
            flamewm::api::Status result = app_->controlClient()->resetSettingsSection(app_->snapshot().revision, "Desktop");
            if (!result.ok()) setStatus(status_, result.message);
        });
    }

    flamewm::ui::TextField* color = flamewm::ui::UiBackend::createTextField(window_);
    if (color) { color->setGeometry(flamewm::api::Rect(216, 126, 180, 30)); color->setText(taskbarPage_.color); add(color, PageTaskbar); color->setOnSubmit([this](const std::string& value) { std::string e; if (!taskbarPage_.applyColorImmediate(app_->controlClient(), value, &e)) setStatus(status_, e); }); }
    flamewm::ui::Slider* taskOpacity = flamewm::ui::UiBackend::createSlider(window_);
    if (taskOpacity) { taskOpacity->setGeometry(flamewm::api::Rect(216, 174, 300, 24)); taskOpacity->setRange(0, 100); taskOpacity->setValue(taskbarPage_.opacity); add(taskOpacity, PageTaskbar); taskOpacity->setOnChanged([this](int v) { std::string e; if (!taskbarPage_.applyOpacityImmediate(app_->controlClient(), v, &e)) setStatus(status_, e); }); }
    flamewm::ui::Slider* height = flamewm::ui::UiBackend::createSlider(window_);
    if (height) { height->setGeometry(flamewm::api::Rect(216, 218, 300, 24)); height->setRange(34, 72); height->setValue(taskbarPage_.height); add(height, PageTaskbar); height->setOnChanged([this](int v) { std::string e; if (!taskbarPage_.applyHeightImmediate(app_->controlClient(), v, &e)) setStatus(status_, e); }); }
    flamewm::ui::TextField* startText = flamewm::ui::UiBackend::createTextField(window_);
    if (startText) { startText->setGeometry(flamewm::api::Rect(216, 266, 220, 30)); startText->setPlaceholder("Start button label"); startText->setText(taskbarPage_.startText); add(startText, PageTaskbar); startText->setOnSubmit([this](const std::string& value) { std::string e; if (!taskbarPage_.applyStartTextImmediate(app_->controlClient(), value, &e)) setStatus(status_, e); }); }
    flamewm::ui::TextField* startIcon = flamewm::ui::UiBackend::createTextField(window_);
    if (startIcon) { startIcon->setGeometry(flamewm::api::Rect(216, 310, 420, 30)); startIcon->setPlaceholder("Custom Start icon path"); startIcon->setText(taskbarPage_.startIcon); add(startIcon, PageTaskbar); startIcon->setOnSubmit([this](const std::string& value) { std::string e; if (!taskbarPage_.applyStartIconImmediate(app_->controlClient(), value, &e)) setStatus(status_, e); }); }

    flamewm::ui::Button* keep = button(PageDisplays, flamewm::api::Rect(216, 126, 120, 32), "Keep mode", flamewm::IconRoleDisplays);
    if (keep) keep->setOnClick([this]() { std::string e; if (!app_->keepDisplayTx(&e)) setStatus(status_, e); });
    flamewm::ui::Button* revert = button(PageDisplays, flamewm::api::Rect(348, 126, 120, 32), "Revert", flamewm::IconRoleDisplays);
    if (revert) revert->setOnClick([this]() { std::string e; if (!app_->revertDisplayTx(&e)) setStatus(status_, e); });
    flamewm::ui::List* outputs = flamewm::ui::UiBackend::createList(window_);
    if (outputs) {
        std::vector<flamewm::ui::ListRow> rows;
        const std::vector<DisplayTopologySpec> topology = displayTopology(displaysPage_.snapshot());
        for (size_t i = 0; i < topology.size(); ++i) {
            std::string name = topology[i].connector;
            if (topology[i].primary) name += " (Primary)";
            if (!topology[i].connected) name += " (Disconnected)";
            rows.push_back(flamewm::ui::ListRow(topology[i].id, name));
        }
        outputs->setRows(rows); outputs->setGeometry(flamewm::api::Rect(216, 218, 210, 120)); add(outputs, PageDisplays);
        if (!rows.empty()) { outputs->setSelected(0); displaysPage_.selectOutput(rows[0].id); }
    }
    const flamewm::api::OutputSnapshot* primary = primaryOutput(displaysPage_.snapshot());
    label(PageDisplays, flamewm::api::Rect(438, 218, 218, 30), std::string("Primary output: ") + (primary ? primary->connector : "none"), true);
    flamewm::ui::List* modes = 0;
    modes = flamewm::ui::UiBackend::createList(window_);
    if (modes) {
        std::vector<flamewm::ui::ListRow> modeRows;
        const std::vector<flamewm::api::DisplayMode> foundModes = displaysPage_.modesForSelected();
        for (size_t i = 0; i < foundModes.size(); ++i) {
            const flamewm::api::DisplayMode& mode = foundModes[i];
            modeRows.push_back(flamewm::ui::ListRow(mode.id.toString(), std::to_string(mode.resolution.w) + "x" + std::to_string(mode.resolution.h)));
        }
        modes->setRows(modeRows); modes->setGeometry(flamewm::api::Rect(438, 262, 218, 100)); add(modes, PageDisplays);
        int current = -1;
        const flamewm::api::OutputSnapshot* selected = displaysPage_.selectedOutput();
        if (selected) for (size_t i = 0; i < foundModes.size(); ++i) if (foundModes[i].id == selected->currentMode) current = static_cast<int>(i);
        if (current >= 0) modes->setSelected(current);
        modes->setOnSelectionChanged([this](int index) {
            const std::vector<flamewm::api::DisplayMode> available = displaysPage_.modesForSelected();
            if (index >= 0 && index < static_cast<int>(available.size())) { std::string e; if (!displaysPage_.setResolutionForSelected(app_->controlClient(), available[index].id, &e)) setStatus(status_, e); }
        });
    }
    flamewm::ui::List* scales = 0;
    scales = flamewm::ui::UiBackend::createList(window_);
    if (scales) {
        std::vector<flamewm::ui::ListRow> scaleRows;
        const std::vector<int> availableScales = displaysPage_.scaleOptions();
        for (size_t i = 0; i < availableScales.size(); ++i) scaleRows.push_back(flamewm::ui::ListRow(std::to_string(availableScales[i]), std::to_string(availableScales[i]) + "%"));
        scales->setRows(scaleRows); scales->setGeometry(flamewm::api::Rect(438, 126, 100, 80)); add(scales, PageDisplays);
        const flamewm::api::OutputSnapshot* selected = displaysPage_.selectedOutput();
        if (selected) for (size_t i = 0; i < availableScales.size(); ++i) if (availableScales[i] == selected->shellScalePercent) scales->setSelected(static_cast<int>(i));
        scales->setOnSelectionChanged([this](int index) {
            const std::vector<int> available = displaysPage_.scaleOptions();
            if (index >= 0 && index < static_cast<int>(available.size())) { std::string e; if (!displaysPage_.setScaleForSelected(app_->controlClient(), available[index], &e)) setStatus(status_, e); }
        });
        if (outputs) {
            outputs->setOnSelectionChanged([this, modes, scales](int index) {
                if (!modes || !scales) return;
                const std::vector<flamewm::api::OutputSnapshot>& available = displaysPage_.snapshot().outputs;
                if (index < 0 || index >= static_cast<int>(available.size())) return;
                displaysPage_.selectOutput(available[index].id.key);
                const flamewm::api::OutputSnapshot* selected = displaysPage_.selectedOutput();
                const std::vector<flamewm::api::DisplayMode> selectedModes = displaysPage_.modesForSelected();
                std::vector<flamewm::ui::ListRow> modeRows;
                int currentMode = -1;
                for (size_t i = 0; i < selectedModes.size(); ++i) {
                    modeRows.push_back(flamewm::ui::ListRow(selectedModes[i].id.toString(), std::to_string(selectedModes[i].resolution.w) + "x" + std::to_string(selectedModes[i].resolution.h)));
                    if (selected && selectedModes[i].id == selected->currentMode) currentMode = static_cast<int>(i);
                }
                modes->setRows(modeRows); if (currentMode >= 0) modes->setSelected(currentMode);
                const std::vector<int> selectedScales = displaysPage_.scaleOptions();
                std::vector<flamewm::ui::ListRow> scaleRows;
                int currentScale = -1;
                for (size_t i = 0; i < selectedScales.size(); ++i) {
                    scaleRows.push_back(flamewm::ui::ListRow(std::to_string(selectedScales[i]), std::to_string(selectedScales[i]) + "%"));
                    if (selected && selectedScales[i] == selected->shellScalePercent) currentScale = static_cast<int>(i);
                }
                scales->setRows(scaleRows); if (currentScale >= 0) scales->setSelected(currentScale);
            });
            outputs->setSelected(outputs->selected());
        }
    }
    label(PageDisplays, flamewm::api::Rect(216, 174, 440, 34), "Select an output, then choose resolution or scale. Changes have WM-owned rollback.", true);

    flamewm::ui::TextField* family = flamewm::ui::UiBackend::createTextField(window_);
    if (family) { family->setGeometry(flamewm::api::Rect(216, 126, 300, 30)); family->setText(fontsPage_.family); family->setPlaceholder("Font family"); add(family, PageFonts); family->setOnSubmit([this](const std::string& value) { std::string e; if (!fontsPage_.applyImmediate(app_->controlClient(), value, fontsPage_.globalBold, fontsPage_.sizeOffset, &e)) setStatus(status_, e); }); }
    flamewm::ui::List* families = flamewm::ui::UiBackend::createList(window_);
    if (families) {
        std::vector<flamewm::ui::ListRow> familyRows;
        const std::vector<std::string>& availableFamilies = fontFamilies();
        for (size_t i = 0; i < availableFamilies.size(); ++i) familyRows.push_back(flamewm::ui::ListRow(availableFamilies[i], availableFamilies[i]));
        families->setRows(familyRows); families->setGeometry(flamewm::api::Rect(528, 126, 128, 100)); add(families, PageFonts);
        for (size_t i = 0; i < availableFamilies.size(); ++i)
            if (availableFamilies[i] == fontsPage_.family) families->setSelected(static_cast<int>(i));
        families->setOnSelectionChanged([this](int index) {
            const std::vector<std::string>& available = fontFamilies();
            if (index < 0 || index >= static_cast<int>(available.size())) return;
            std::string e;
            if (!fontsPage_.applyImmediate(app_->controlClient(), available[index], fontsPage_.globalBold, fontsPage_.sizeOffset, &e))
                setStatus(status_, e);
        });
    }
    flamewm::ui::Toggle* bold = flamewm::ui::UiBackend::createToggle(window_, flamewm::IconRoleFonts);
    if (bold) { bold->setGeometry(flamewm::api::Rect(216, 174, 160, 32)); bold->setText("Global bold"); bold->setChecked(fontsPage_.globalBold); add(bold, PageFonts); bold->setOnToggled([this](bool value) { std::string e; if (!fontsPage_.applyImmediate(app_->controlClient(), fontsPage_.family, value, fontsPage_.sizeOffset, &e)) setStatus(status_, e); }); }
    flamewm::ui::Slider* fontSize = flamewm::ui::UiBackend::createSlider(window_);
    if (fontSize) { fontSize->setGeometry(flamewm::api::Rect(216, 218, 300, 24)); fontSize->setRange(fontSizeOffsetMinimum(), fontSizeOffsetMaximum()); fontSize->setValue(fontsPage_.sizeOffset); add(fontSize, PageFonts); fontSize->setOnChanged([this](int value) { std::string e; if (!fontsPage_.applyImmediate(app_->controlClient(), fontsPage_.family, fontsPage_.globalBold, value, &e)) setStatus(status_, e); }); }

    const std::vector<HotkeySpec>& hotkeys = hotkeySpecs();
    for (size_t i = 0; i < hotkeys.size(); ++i) {
        const std::string action = hotkeys[i].action;
        const int y = 126 + static_cast<int>(i) * 54;
        label(PageHotkeys, flamewm::api::Rect(216, y, 220, 30), hotkeys[i].label, false);
        label(PageHotkeys, flamewm::api::Rect(330, y, 100, 30), HotkeysPage::displayFor(hotkeysPage_.cachedSnapshot.get(action)), false);
        flamewm::ui::Button* capture = button(PageHotkeys, flamewm::api::Rect(442, y, 104, 30), "Capture", flamewm::IconRoleHotkeys);
        if (capture) capture->setOnClick([this, action]() { hotkeysPage_.beginCapture(action); setStatus(status_, "Press a key; Escape clears capture"); });
        flamewm::ui::Button* reset = button(PageHotkeys, flamewm::api::Rect(554, y, 100, 30), "Reset", flamewm::IconRoleHotkeys);
        if (reset) reset->setOnClick([this, action]() { std::string e; if (!hotkeysPage_.resetBinding(app_->controlClient(), action, &e)) setStatus(status_, e); });
    }
    label(PageAbout, flamewm::api::Rect(216, 126, 440, 52), "FlameWM\n" + AboutPage::tagline(), true);
    const std::vector<AboutLinkSpec>& links = aboutLinks();
    for (size_t i = 0; i < links.size(); ++i) {
        flamewm::ui::Button* link = button(PageAbout,
            flamewm::api::Rect(216 + static_cast<int>(i) * 142, 198, 130, 32),
            links[i].label, flamewm::IconRoleAbout);
        if (link) {
            const std::string url = links[i].url;
            link->setOnClick([url]() {
                flamewm::DefaultSafeUriLauncher launcher; std::string e;
                launcher.launchUri(url, &e);
            });
        }
    }
    select(static_cast<int>(app_->currentPage())); return true;
}
void Root::select(int index) {
    const std::vector<PageSpec>& specs = pageSpecs();
    if (index < 0 || index >= static_cast<int>(specs.size())) return;
    app_->setPage(static_cast<SettingsPageId>(index));
    if (navigation_->selected() != index) navigation_->setSelected(index);
    for (size_t i = 0; i < navigationItems_.size(); ++i) settingsNavItem(navigationItems_[i], static_cast<int>(i) == index);
    for (int page = 0; page < 7; ++page) for (size_t i = 0; i < pageWidgets_[page].size(); ++i) {
        if (page == index) pageWidgets_[page][i]->show(); else pageWidgets_[page][i]->hide();
    }
    if (body_) body_->hide();
}
void Root::show() { if (window_) window_->show(); }
}}}
