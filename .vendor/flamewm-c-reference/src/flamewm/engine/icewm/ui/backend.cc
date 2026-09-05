#include "flamewm/engine/icewm/ui/backend.h"
#include "flamewm/ui/backend.h"

#include "ybutton.h"
#include "yicon.h"
#include "yinputline.h"
#include "ylabel.h"
#include "ypopup.h"
#include "prefs.h"
#include "yprefs.h"
#include "yxapp.h"
#include "wmaction.h"
#include "mstring.h"
#include "flamewm/ui/iconresolver.h"
#include "flamewm/ui/shellstyle.h"
#include "flamewm/engine/icewm/ui/material_painter.h"
#include "flamewm/engine/icewm/ui/native_paint.h"
#include "flamewm/engine/icewm/ui/native_theme_resources.h"
#include "flamewm/ui/style/theme.h"
#include "flamewm/ui/style/visual.h"
#include "flamewm/shell/task_view.h"

#include <vector>
#include <string>
#include <cstdlib>
#include <unistd.h>
#include <X11/keysym.h>

namespace flamewm {
namespace engine {
namespace icewm {
namespace ui {

struct IceWMBackend::Impl {
    bool initialized;
    flamewm::ui::UiProvider* installedProvider;
    Impl() : initialized(false), installedProvider(0) {}
};

static IceWMBackend* g_instance = 0;

namespace {

static const flamewm::ui::style::Theme& flameTheme() {
    static const flamewm::ui::style::Theme theme =
        flamewm::ui::style::Theme::defaults();
    return theme;
}

static flamewm::ui::style::VisualStateMask controlState(
    const flamewm::ui::style::Visual& visual, bool enabled, bool hovered,
    bool pressed, bool selected) {
    flamewm::ui::style::VisualStateMask state = visual.state;
    if (hovered) state |= flamewm::ui::style::VisualHovered;
    if (pressed) state |= flamewm::ui::style::VisualPressed;
    if (selected) state |= flamewm::ui::style::VisualSelected;
    if (!enabled) state |= flamewm::ui::style::VisualDisabled;
    return state;
}

static std::string installedWordmarkPath() {
    const char* dataDir = std::getenv("FLAMEWM_DATADIR");
    if (dataDir && dataDir[0]) {
        const std::string path = std::string(dataDir) + "/flamewm/branding/flamewm-wordmark.png";
        if (access(path.c_str(), R_OK) == 0) return path;
    }
    const char* paths[] = {
        "/usr/share/flamewm/branding/flamewm-wordmark.png",
        "/usr/local/share/flamewm/branding/flamewm-wordmark.png",
        "lib/flamewm/branding/flamewm-wordmark.png"
    };
    for (size_t i = 0; i < sizeof(paths) / sizeof(paths[0]); ++i)
        if (access(paths[i], R_OK) == 0) return paths[i];
    return std::string();
}

static void paintButton(Graphics& g, const YRect& rect,
                        const flamewm::ui::style::Visual& visual,
                        flamewm::ui::style::VisualRole role,
                        const mstring& text, bool enabled, bool hovered,
                        bool pressed, bool selected, ref<YIcon> icon,
                        unsigned iconSize, const char* fallbackGlyph,
                        YFont font) {
    const flamewm::ui::style::VisualStateMask state =
        controlState(visual, enabled, hovered, pressed, selected);
    MaterialPainter::material(g, rect, flameTheme(), role, state);
    const char* label = const_cast<mstring&>(text).c_str();
    if ((!label || !*label) && MaterialPainter::icon(g, icon,
            (rect.width() - static_cast<int>(iconSize)) / 2,
            (rect.height() - static_cast<int>(iconSize)) / 2, iconSize)) return;
    if (!font) return;
    const char* value = label && *label ? label : fallbackGlyph;
    MaterialPainter::text(g, font, flameTheme().color(flamewm::ui::style::VisualRoleText, state),
                          value, (rect.width() - font->textWidth(value)) / 2,
                          (rect.height() - font->height()) / 2 + font->ascent());
}

class NativeWindow : public flamewm::ui::Window, public YWindow {
public:
    explicit NativeWindow(YWindow* parent = 0)
        : YWindow(parent), role_(parent ? flamewm::ui::WindowRole::GenericChild
                                         : flamewm::ui::WindowRole::PanelDock), onClose_() {
        setClassHint("flamewm", "FlameWM");
        applyRole();
    }
    void setGeometry(const flamewm::api::Rect& r) { YWindow::setGeometry(YRect(r.x, r.y, r.w, r.h)); }
    flamewm::api::Rect geometry() const { return flamewm::api::Rect(x(), y(), width(), height()); }
    void show() { YWindow::show(); }
    void hide() { YWindow::hide(); }
    bool isVisible() const { return YWindow::visible(); }
    void repaint() { NativePaint::repaint(*this); }
    void setOnClose(std::function<void()> cb) { onClose_ = cb; }
    void setRole(flamewm::ui::WindowRole role) {
        role_ = role;
        applyRole();
        repaint();
    }
    void setVisual(const flamewm::ui::style::Visual& visual) {
        visual_ = visual;
        repaint();
    }
    flamewm::ui::WindowRole role() const { return role_; }
    void handleClose() { if (onClose_) onClose_(); }
    void paint(Graphics& g, const YRect& rect) override {
        flamewm::ui::style::VisualStateMask state = visual_.state;
        if (!visual_.enabled) state |= flamewm::ui::style::VisualDisabled;
        flamewm::ui::style::VisualRole visualRole = flamewm::ui::style::VisualRoleSurface;
        if (role_ == flamewm::ui::WindowRole::PanelDock)
            visualRole = flamewm::ui::style::VisualRolePanel;
        else if (role_ == flamewm::ui::WindowRole::Popup)
            visualRole = flamewm::ui::style::VisualRoleOverlay;
        else if (role_ == flamewm::ui::WindowRole::ApplicationWindow)
            visualRole = flamewm::ui::style::VisualRoleBackground;
        MaterialPainter::material(g, rect, flameTheme(), visualRole, state);
    }
protected:
    std::function<void()> onClose_;
private:
    void applyRole() {
        switch (role_) {
            case flamewm::ui::WindowRole::PanelDock:
                setTitle("FlameWM Panel");
                setNetWindowType(xapp->atom("_NET_WM_WINDOW_TYPE_DOCK"));
                break;
            case flamewm::ui::WindowRole::ApplicationWindow:
                setTitle("FlameWM");
                setNetWindowType(xapp->atom("_NET_WM_WINDOW_TYPE_NORMAL"));
                break;
            case flamewm::ui::WindowRole::Popup:
                setTitle("FlameWM Popup");
                setNetWindowType(xapp->atom("_NET_WM_WINDOW_TYPE_POPUP_MENU"));
                break;
            case flamewm::ui::WindowRole::GenericChild:
                // Child surfaces are ordinary parented X windows: no dock or
                // popup type may leak from the panel root.
                XDeleteProperty(xapp->display(), handle(), _XA_NET_WM_WINDOW_TYPE);
                break;
        }
    }

    flamewm::ui::WindowRole role_;
};

static YWindow* nativeParent(flamewm::ui::Window* parent) {
    return parent ? dynamic_cast<YWindow*>(parent) : 0;
}

class NativeButton : public flamewm::ui::Button, public YButton {
public:
    NativeButton(YWindow* parent, flamewm::IconRole role)
        : YButton(parent, YAction(actionNull)), role_(role), icon_(), iconSize_(0),
          fallbackGlyph_("[?]"), enabled_(true), pressed_(false), visualState_(0), onClose_() {
        updateRoleIcon();
    }
    void setGeometry(const flamewm::api::Rect& r) { YWindow::setGeometry(YRect(r.x, r.y, r.w, r.h)); }
    flamewm::api::Rect geometry() const { return flamewm::api::Rect(x(), y(), width(), height()); }
    void show() { YWindow::show(); }
    void hide() { YWindow::hide(); }
    bool isVisible() const { return YWindow::visible(); }
    void repaint() { NativePaint::repaint(*this); }
    void setOnClose(std::function<void()> cb) { onClose_ = cb; }
    void setText(const std::string& text) { YButton::setText(mstring(text.c_str())); }
    std::string text() const { return std::string(getText().c_str()); }
    void setIconRole(flamewm::IconRole role) {
        role_ = role;
        updateRoleIcon();
        repaint();
    }
    flamewm::IconRole iconRole() const { return role_; }
    void setIconName(const std::string& name) { iconName_ = name; updateRoleIcon(); repaint(); }
    std::string iconName() const { return iconName_; }
    void setVisualState(unsigned state) { visualState_ = state; repaint(); }
    unsigned visualState() const { return visualState_; }
    void setVisual(const flamewm::ui::style::Visual& visual) {
        visual_ = visual;
        repaint();
    }
    void setEnabled(bool enabled) { enabled_ = enabled; YButton::setEnabled(enabled); }
    bool isEnabled() const { return enabled_; }
    void setOnClick(std::function<void()> cb) { onClick_ = cb; }
    void setOnContextMenu(std::function<void(int,int)> cb) { onContext_ = cb; }
    void setOnPress(std::function<void(int,int,int)> cb) { onPress_ = cb; }
    void setOnRelease(std::function<void(int,int,int)> cb) { onRelease_ = cb; }
    void setOnMotion(std::function<void(int,int)> cb) { onMotion_ = cb; }
    void actionPerformed(YAction action, unsigned modifiers) {
        (void) action; (void) modifiers;
        if (enabled_ && onClick_) onClick_();
    }
    void handleButton(const XButtonEvent& event) {
        if (event.type == ButtonPress && event.button == Button1) { pressed_ = true; repaint(); }
        if (event.type == ButtonRelease && event.button == Button1) { pressed_ = false; repaint(); }
        if (event.type == ButtonPress && onPress_) onPress_(event.x_root, event.y_root, event.button);
        if (event.type == ButtonRelease && onRelease_) onRelease_(event.x_root, event.y_root, event.button);
        if (event.type == ButtonPress && event.button == Button3 && onContext_)
            onContext_(event.x_root, event.y_root);
        YButton::handleButton(event);
    }
    void handleMotion(const XMotionEvent& event) {
        if (onMotion_) onMotion_(event.x_root, event.y_root);
        YWindow::handleMotion(event);
    }
    void paint(Graphics& g, const YRect& r) override {
        const flamewm::ui::style::Visual& visual = flamewm::ui::Window::visual();
        const bool taskHovered = (visualState_ & flamewm::TaskVisualHovered) != 0;
        const bool taskPressed = (visualState_ & flamewm::TaskVisualPressed) != 0;
        paintButton(g, r, visual, visual.role, getText(), enabled_,
                    fOver || taskHovered, pressed_ || taskPressed,
                    visual.hasState(flamewm::ui::style::VisualSelected), icon_, iconSize_,
                    fallbackGlyph_, YFont(getFont()));
    }
private:
    void updateRoleIcon() {
        if (!iconName_.empty()) {
            icon_ = YIcon::getIcon(iconName_.c_str());
            iconSize_ = YIcon::smallSize();
            fallbackGlyph_ = "[?]";
            return;
        }
        const flamewm::IconFallbackEntry* entry = flamewm::iconFallbackForRole(role_);
        if (entry) {
            icon_ = YIcon::getIcon(entry->freedesktopName);
            iconSize_ = YIcon::fixIconSize(
                static_cast<unsigned>(flamewm::logicalIconSizeForRole(role_)));
            fallbackGlyph_ = entry->builtinGlyph;
        } else {
            icon_ = null;
            iconSize_ = YIcon::smallSize();
            fallbackGlyph_ = "[?]";
        }
    }

    flamewm::IconRole role_;
    std::string iconName_;
    ref<YIcon> icon_;
    unsigned iconSize_;
    const char* fallbackGlyph_;
    bool enabled_, pressed_;
    unsigned visualState_;
    std::function<void()> onClick_, onClose_;
    std::function<void(int,int)> onContext_, onMotion_;
    std::function<void(int,int,int)> onPress_, onRelease_;
};

class NativeToggle : public flamewm::ui::Toggle, public YButton {
public:
    NativeToggle(YWindow* parent, flamewm::IconRole role)
        : YButton(parent, YAction(actionNull)), role_(role), icon_(), iconSize_(0),
          fallbackGlyph_("[?]"), enabled_(true), checked_(false), visualState_(0), onClose_() {
        updateRoleIcon();
    }
    void setGeometry(const flamewm::api::Rect& r) { YWindow::setGeometry(YRect(r.x, r.y, r.w, r.h)); }
    flamewm::api::Rect geometry() const { return flamewm::api::Rect(x(), y(), width(), height()); }
    void show() { YWindow::show(); }
    void hide() { YWindow::hide(); }
    bool isVisible() const { return YWindow::visible(); }
    void repaint() { NativePaint::repaint(*this); }
    void setOnClose(std::function<void()> cb) { onClose_ = cb; }
    void setText(const std::string& text) { YButton::setText(mstring(text.c_str())); }
    std::string text() const { return std::string(getText().c_str()); }
    void setIconRole(flamewm::IconRole role) { role_ = role; updateRoleIcon(); repaint(); }
    flamewm::IconRole iconRole() const { return role_; }
    void setIconName(const std::string& name) { iconName_ = name; updateRoleIcon(); repaint(); }
    std::string iconName() const { return iconName_; }
    void setVisualState(unsigned state) { visualState_ = state; repaint(); }
    unsigned visualState() const { return visualState_; }
    void setVisual(const flamewm::ui::style::Visual& visual) {
        visual_ = visual;
        repaint();
    }
    void setEnabled(bool enabled) { enabled_ = enabled; YButton::setEnabled(enabled); }
    bool isEnabled() const { return enabled_; }
    void setOnClick(std::function<void()> cb) { onClick_ = cb; }
    void setOnContextMenu(std::function<void(int,int)> cb) { onContext_ = cb; }
    void setOnPress(std::function<void(int,int,int)> cb) { onPress_ = cb; }
    void setOnRelease(std::function<void(int,int,int)> cb) { onRelease_ = cb; }
    void setOnMotion(std::function<void(int,int)> cb) { onMotion_ = cb; }
    void actionPerformed(YAction action, unsigned modifiers) {
        (void) action; (void) modifiers;
        if (!enabled_) return;
        checked_ = !checked_;
        repaint();
        if (onToggled_) onToggled_(checked_);
        if (onClick_) onClick_();
    }
    void handleButton(const XButtonEvent& event) {
        if (event.type == ButtonPress && onPress_) onPress_(event.x_root, event.y_root, event.button);
        if (event.type == ButtonRelease && onRelease_) onRelease_(event.x_root, event.y_root, event.button);
        if (event.type == ButtonPress && event.button == Button3 && onContext_)
            onContext_(event.x_root, event.y_root);
        YButton::handleButton(event);
    }
    void handleMotion(const XMotionEvent& event) {
        if (onMotion_) onMotion_(event.x_root, event.y_root);
        YWindow::handleMotion(event);
    }
    void setChecked(bool checked) { if (checked_ == checked) return; checked_ = checked; repaint(); }
    bool isChecked() const { return checked_; }
    void setOnToggled(std::function<void(bool)> cb) { onToggled_ = cb; }
    void paint(Graphics& g, const YRect& r) override {
        const flamewm::ui::style::Visual& visual = flamewm::ui::Window::visual();
        paintButton(g, r, visual, visual.role, getText(), enabled_, fOver, false,
                    checked_ || visual.hasState(flamewm::ui::style::VisualSelected),
                    icon_, iconSize_, fallbackGlyph_, YFont(getFont()));
    }
private:
    void updateRoleIcon() {
        if (!iconName_.empty()) {
            icon_ = YIcon::getIcon(iconName_.c_str());
            iconSize_ = YIcon::smallSize();
            fallbackGlyph_ = "[?]";
            return;
        }
        const flamewm::IconFallbackEntry* entry = flamewm::iconFallbackForRole(role_);
        if (entry) {
            icon_ = YIcon::getIcon(entry->freedesktopName);
            iconSize_ = YIcon::fixIconSize(
                static_cast<unsigned>(flamewm::logicalIconSizeForRole(role_)));
            fallbackGlyph_ = entry->builtinGlyph;
        } else {
            icon_ = null;
            iconSize_ = YIcon::smallSize();
            fallbackGlyph_ = "[?]";
        }
    }

    flamewm::IconRole role_;
    std::string iconName_;
    ref<YIcon> icon_;
    unsigned iconSize_;
    const char* fallbackGlyph_;
    bool enabled_, checked_;
    unsigned visualState_;
    std::function<void()> onClick_, onClose_;
    std::function<void(int,int)> onContext_, onMotion_;
    std::function<void(int,int,int)> onPress_, onRelease_;
    std::function<void(bool)> onToggled_;
};

class NativeLabel : public flamewm::ui::Label, public YLabel {
public:
    explicit NativeLabel(YWindow* parent) : YLabel(mstring(""), parent), wrap_(false), onClose_() {}
    void setGeometry(const flamewm::api::Rect& r) { YWindow::setGeometry(YRect(r.x, r.y, r.w, r.h)); }
    flamewm::api::Rect geometry() const { return flamewm::api::Rect(x(), y(), width(), height()); }
    void show() { YWindow::show(); }
    void hide() { YWindow::hide(); }
    bool isVisible() const { return YWindow::visible(); }
    void repaint() { NativePaint::repaint(*this); }
    void setOnClose(std::function<void()> cb) { onClose_ = cb; }
    void setText(const std::string& text) { text_ = text; YLabel::setText(text.c_str()); }
    std::string text() const { return text_; }
    void setWrap(bool wrap) { wrap_ = wrap; multiline(wrap); }
    bool isWrap() const { return wrap_; }
    void setVisual(const flamewm::ui::style::Visual& visual) {
        visual_ = visual;
        repaint();
    }
    void paint(Graphics& g, const YRect& rect) override {
        YFont font;
        font = ::labelFontName;
        if (!font || text_.empty()) return;
        const flamewm::ui::style::Color& color =
             flameTheme().color(flamewm::ui::style::VisualRoleText,
                                flamewm::ui::Window::visual().state);
        g.setFont(font);
        g.setColor(YColor(color.value.c_str()));
        const int lineHeight = font->height();
        int y = font->ascent();
        std::string line;
        for (size_t i = 0; i <= text_.size(); ++i) {
            const bool end = i == text_.size() || text_[i] == '\n';
            const char ch = end ? ' ' : text_[i];
            if (wrap_ && ch != ' ' && !line.empty() &&
                font->textWidth((line + ch).c_str()) > static_cast<int>(rect.width())) {
                g.drawString(0, y, line.c_str());
                y += lineHeight;
                line.clear();
            }
            if (end) {
                if (!line.empty()) g.drawString(0, y, line.c_str());
                line.clear();
                y += lineHeight;
            } else {
                line += ch;
            }
        }
    }
private:
    std::string text_;
    bool wrap_;
    std::function<void()> onClose_;
};

class NativeSlider : public flamewm::ui::Slider, public YWindow {
public:
    explicit NativeSlider(YWindow* parent)
        : YWindow(parent), min_(0), max_(100), value_(0), enabled_(true), dragging_(false), onChanged_(), onClose_() {}
    void setGeometry(const flamewm::api::Rect& r) { YWindow::setGeometry(YRect(r.x, r.y, r.w, r.h)); }
    flamewm::api::Rect geometry() const { return flamewm::api::Rect(x(), y(), width(), height()); }
    void show() { YWindow::show(); }
    void hide() { YWindow::hide(); }
    bool isVisible() const { return YWindow::visible(); }
    void repaint() { NativePaint::repaint(*this); }
    void setEnabled(bool enabled) { enabled_ = enabled; repaint(); }
    bool isEnabled() const { return enabled_; }
    void setVisual(const flamewm::ui::style::Visual& visual) {
        visual_ = visual;
        repaint();
    }
    void setOnClose(std::function<void()> cb) { onClose_ = cb; }
    void setRange(int minV, int maxV) {
        int oldValue = value_;
        min_ = minV <= maxV ? minV : maxV;
        max_ = minV <= maxV ? maxV : minV;
        if (value_ < min_) value_ = min_;
        if (value_ > max_) value_ = max_;
        repaint();
        if (value_ != oldValue && onChanged_) onChanged_(value_);
    }
    int minimum() const { return min_; }
    int maximum() const { return max_; }
    void setValue(int value) {
        if (!enabled_) return;
        int next = value < min_ ? min_ : (value > max_ ? max_ : value);
        if (next == value_) return;
        value_ = next;
        repaint();
        if (onChanged_) onChanged_(value_);
    }
    int value() const { return value_; }
    void setOnChanged(std::function<void(int)> cb) { onChanged_ = cb; }
    void handleButton(const XButtonEvent& event) {
        if (event.button != Button1 || width() <= 1) return;
        if (event.type == ButtonPress) dragging_ = true;
        if (event.type == ButtonRelease) dragging_ = false;
        if (event.type == ButtonPress || event.type == ButtonRelease) {
            int position = event.x < 0 ? 0 : (event.x >= width() ? width() - 1 : event.x);
            setValue(min_ + (max_ - min_) * position / (width() - 1));
        }
    }
    void handleMotion(const XMotionEvent& event) {
        if (dragging_ && width() > 1) {
            int position = event.x < 0 ? 0 : (event.x >= width() ? width() - 1 : event.x);
            setValue(min_ + (max_ - min_) * position / (width() - 1));
        }
        YWindow::handleMotion(event);
    }
    void paint(Graphics& g, const YRect&) override {
        const flamewm::ui::style::VisualStateMask state =
            controlState(visual_, enabled_, false, false, false);
        MaterialPainter::fill(g, 0, height() / 2 - 2, width(), 4,
                              flameTheme().color(flamewm::ui::style::VisualRoleSurface, state));
        int span = max_ - min_;
        int knob = span ? (value_ - min_) * (width() - 1) / span : 0;
        MaterialPainter::fill(g, knob - 5, height() / 2 - 7, 10, 14,
                              flameTheme().color(flamewm::ui::style::VisualRoleIndicator, state));
    }
private:
    int min_, max_, value_;
    bool enabled_, dragging_;
    std::function<void(int)> onChanged_;
    std::function<void()> onClose_;
};

class NativeList : public flamewm::ui::List, public YButton {
public:
    explicit NativeList(YWindow* parent) : YButton(parent, YAction(actionNull)), selected_(-1), scroll_(0), onSelection_(), onActivated_(), onClose_() {}
    void setGeometry(const flamewm::api::Rect& r) { YWindow::setGeometry(YRect(r.x, r.y, r.w, r.h)); }
    flamewm::api::Rect geometry() const { return flamewm::api::Rect(x(), y(), width(), height()); }
    void show() { YWindow::show(); }
    void hide() { YWindow::hide(); }
    bool isVisible() const { return YWindow::visible(); }
    void repaint() { NativePaint::repaint(*this); }
    void setOnClose(std::function<void()> cb) { onClose_ = cb; }
    void setVisual(const flamewm::ui::style::Visual& visual) {
        visual_ = visual;
        repaint();
    }
    void setRows(const std::vector<flamewm::ui::ListRow>& rows) { rows_ = rows; int oldSelected = selected_; if (selected_ >= static_cast<int>(rows_.size())) selected_ = -1; if (selected_ != oldSelected && onSelection_) onSelection_(selected_); repaint(); }
    std::vector<flamewm::ui::ListRow> rows() const { return rows_; }
    void setSelected(int index) {
        int next = index >= 0 && index < static_cast<int>(rows_.size()) ? index : -1;
        if (next == selected_) return;
        selected_ = next;
        if (selected_ >= 0) ensureVisible(selected_);
        repaint();
        if (onSelection_) onSelection_(selected_);
    }
    int selected() const { return selected_; }
    std::string selectedId() const { return selected_ >= 0 && selected_ < static_cast<int>(rows_.size()) ? rows_[selected_].id : std::string(); }
    void setOnSelectionChanged(std::function<void(int)> cb) { onSelection_ = cb; }
    void setOnActivated(std::function<void(int)> cb) { onActivated_ = cb; }
    void handleButton(const XButtonEvent& event) {
        if (event.type != ButtonRelease || event.button != Button1 || event.y < 0) return;
        int index = scroll_ + event.y / 32;
        if (index < 0 || index >= static_cast<int>(rows_.size())) return;
        setSelected(index);
        if (onActivated_) onActivated_(index);
    }
    bool handleKey(const XKeyEvent& event) {
        if (event.type != KeyPress || rows_.empty()) return YButton::handleKey(event);
        KeySym key = XLookupKeysym(const_cast<XKeyEvent*>(&event), 0);
        int next = selected_;
        if (key == XK_Up) next = selected_ > 0 ? selected_ - 1 : 0;
        else if (key == XK_Down) next = selected_ + 1 < static_cast<int>(rows_.size()) ? selected_ + 1 : selected_;
        else if (key == XK_Home) next = 0;
        else if (key == XK_End) next = static_cast<int>(rows_.size()) - 1;
        else if (key == XK_Return || key == XK_KP_Enter) {
            if (selected_ >= 0 && onActivated_) onActivated_(selected_);
            return true;
        } else return YButton::handleKey(event);
        setSelected(next);
        return true;
    }
    void paint(Graphics& g, const YRect&) override {
        YFont font(getFont());
        for (size_t row = 0; row < rows_.size() && static_cast<int>(row * 32) < height(); ++row) {
            size_t i = row + static_cast<size_t>(scroll_);
            if (i >= rows_.size()) break;
            bool selected = static_cast<int>(i) == selected_;
            const flamewm::ui::style::VisualStateMask state =
                controlState(visual_, true, false, false, selected);
             MaterialPainter::material(g, YRect(0, static_cast<int>(row * 32),
                                                width(), static_cast<unsigned>(32)), flameTheme(),
                                     flamewm::ui::style::VisualRoleMenu, state);
            int textX = 8;
            if (!rows_[i].iconName.empty()) {
                ref<YIcon> rowIcon = YIcon::getIcon(rows_[i].iconName.c_str());
                if (MaterialPainter::icon(g, rowIcon, 8, static_cast<int>(row * 32) + 8, 16))
                    textX = 32;
            }
            if (font) {
                g.setFont(font);
                g.setColor(YColor(flameTheme().primaryText().value.c_str()));
                 g.drawString(textX, static_cast<int>(row * 32) + font->ascent() + 8,
                             rows_[i].label.c_str());
            }
        }
    }
private:
    void ensureVisible(int index) {
        int visible = height() / 32;
        if (visible < 1) visible = 1;
        if (index < scroll_) scroll_ = index;
        if (index >= scroll_ + visible) scroll_ = index - visible + 1;
        int maxScroll = static_cast<int>(rows_.size()) - visible;
        if (maxScroll < 0) maxScroll = 0;
        if (scroll_ > maxScroll) scroll_ = maxScroll;
    }
    std::vector<flamewm::ui::ListRow> rows_;
    int selected_, scroll_;
    std::function<void(int)> onSelection_, onActivated_;
    std::function<void()> onClose_;
};

class NativeTextField : public flamewm::ui::TextField, public YInputLine, private YInputListener {
public:
    explicit NativeTextField(YWindow* parent) : YInputLine(parent, this), onClose_() {}
    void setGeometry(const flamewm::api::Rect& r) { YWindow::setGeometry(YRect(r.x, r.y, r.w, r.h)); }
    flamewm::api::Rect geometry() const { return flamewm::api::Rect(x(), y(), width(), height()); }
    void show() { YWindow::show(); }
    void hide() { YWindow::hide(); }
    bool isVisible() const { return YWindow::visible(); }
    void repaint() { NativePaint::repaint(*this); }
    void setOnClose(std::function<void()> cb) { onClose_ = cb; }
    void setText(const std::string& text) {
        YInputLine::setText(mstring(text.c_str()), false);
        if (onChanged_) onChanged_(text);
    }
    std::string text() const { return std::string(const_cast<NativeTextField*>(this)->getText().c_str()); }
    void setPlaceholder(const std::string& placeholder) { placeholder_ = placeholder; repaint(); }
    std::string placeholder() const { return placeholder_; }
    void setOnChanged(std::function<void(const std::string&)> cb) { onChanged_ = cb; }
    void setOnSubmit(std::function<void(const std::string&)> cb) { onSubmit_ = cb; }
    void focus() { requestFocus(true); }
    bool hasFocus() const { return YWindow::isFocused(); }
    void setVisual(const flamewm::ui::style::Visual& visual) {
        visual_ = visual;
        repaint();
    }
    void inputReturn(YInputLine*, bool) { if (onSubmit_) onSubmit_(text()); }
    void inputEscape(YInputLine*) {}
    void inputLostFocus(YInputLine*) {}
    void paint(Graphics& g, const YRect& r) override {
        YInputLine::paint(g, r);
        if (getText().nonempty() || placeholder_.empty() || hasFocus()) return;
        YFont font(getFont());
        if (!font) return;
        g.setFont(font);
        g.setColor(YColor(flameTheme().secondaryText().value.c_str()));
        g.drawString(6, (height() - font->height()) / 2 + font->ascent(), placeholder_.c_str());
    }
private:
    std::string placeholder_;
    std::function<void(const std::string&)> onChanged_, onSubmit_;
    std::function<void()> onClose_;
};

class NativePopover : public flamewm::ui::Popover, public YPopupWindow {
public:
    explicit NativePopover(YWindow* parent) : YPopupWindow(parent), anchor_(0,0,0,0), closed_(true), onClose_(), onClosed_() {
        setTitle("FlameWM Popover");
        setClassHint("flamewm-popover", "FlameWM");
        setNetWindowType(xapp->atom("_NET_WM_WINDOW_TYPE_POPUP_MENU"));
    }
    ~NativePopover() { close(); }
    void setGeometry(const flamewm::api::Rect& r) { YWindow::setGeometry(YRect(r.x, r.y, r.w, r.h)); }
    flamewm::api::Rect geometry() const { return flamewm::api::Rect(x(), y(), width(), height()); }
    void show() { closed_ = false; YPopupWindow::show(); }
    void hide() { YPopupWindow::hide(); }
    bool isVisible() const { return YWindow::visible(); }
    void repaint() { NativePaint::repaint(*this); }
    void setOnClose(std::function<void()> cb) { onClose_ = cb; }
    void setAnchor(const flamewm::api::Rect& r) { anchor_ = r; }
    void setVisual(const flamewm::ui::style::Visual& visual) {
        visual_ = visual;
        repaint();
    }
    flamewm::api::Rect anchor() const { return anchor_; }
    void showAt(const flamewm::api::Rect& r) { setAnchor(r); setPosition(r.x, r.y + r.h); show(); }
    void close() { if (closed_) return; closed_ = true; hide(); if (onClosed_) onClosed_(); }
    void setOnClosed(std::function<void()> cb) { onClosed_ = cb; }
    void handleClose() override { if (closed_) return; close(); if (onClose_) onClose_(); }
    bool handleKey(const XKeyEvent& event) {
        if (event.type == KeyPress && XLookupKeysym(const_cast<XKeyEvent*>(&event), 0) == XK_Escape) {
            close();
            return true;
        }
        return YPopupWindow::handleKey(event);
    }
    void paint(Graphics& g, const YRect& rect) override {
        MaterialPainter::material(g, rect, flameTheme(),
                                  flamewm::ui::style::VisualRoleOverlay,
                                  visual_.state);
    }
    void handleButton(const XButtonEvent& event) {
        if (event.type == ButtonPress &&
            (event.x_root < x() || event.x_root >= x() + width() ||
             event.y_root < y() || event.y_root >= y() + height())) {
            close();
            return;
        }
        YPopupWindow::handleButton(event);
    }
    flamewm::ui::WindowRole role() const { return flamewm::ui::WindowRole::Popup; }
    void setRole(flamewm::ui::WindowRole) {}
private:
    flamewm::api::Rect anchor_;
    bool closed_;
    std::function<void()> onClose_, onClosed_;
};

class NativeDialog : public flamewm::ui::Dialog, public YPopupWindow {
public:
    explicit NativeDialog(YWindow* parent) : YPopupWindow(parent), title_(), result_(0), onClose_() {}
    void setGeometry(const flamewm::api::Rect& r) { YWindow::setGeometry(YRect(r.x, r.y, r.w, r.h)); }
    flamewm::api::Rect geometry() const { return flamewm::api::Rect(x(), y(), width(), height()); }
    void show() { YPopupWindow::show(); }
    void hide() { YPopupWindow::hide(); }
    bool isVisible() const { return YWindow::visible(); }
    void repaint() { NativePaint::repaint(*this); }
    void setOnClose(std::function<void()> cb) { onClose_ = cb; }
    void setTitle(const std::string& title) { title_ = title; YWindow::setTitle(title.c_str()); }
    void setVisual(const flamewm::ui::style::Visual& visual) {
        visual_ = visual;
        repaint();
    }
    std::string title() const { return title_; }
    int exec() { result_ = -1; show(); return result_; }
    void accept() { result_ = 1; hide(); }
    void reject() { result_ = 0; hide(); }
    void handleClose() override { reject(); if (onClose_) onClose_(); }
    flamewm::ui::WindowRole role() const { return flamewm::ui::WindowRole::Popup; }
    void setRole(flamewm::ui::WindowRole) {}
    void paint(Graphics& g, const YRect& rect) override {
        MaterialPainter::material(g, rect, flameTheme(),
                                  flamewm::ui::style::VisualRoleOverlay,
                                  visual_.state);
        YFont font;
        font = ::labelFontName;
        if (font && !title_.empty()) {
            MaterialPainter::text(g, font, flameTheme().primaryText(),
                                  title_.c_str(), 12, font->ascent() + 8);
        }
    }
private:
    std::string title_;
    int result_;
    std::function<void()> onClose_;
};

class NativeImage : public flamewm::ui::Image, public YWindow {
public:
    explicit NativeImage(YWindow* parent) : YWindow(parent), image_(), nativeImage_(), icon_(), iconSize_(0) {}
    void setGeometry(const flamewm::api::Rect& r) { YWindow::setGeometry(YRect(r.x, r.y, r.w, r.h)); }
    flamewm::api::Rect geometry() const { return flamewm::api::Rect(x(), y(), width(), height()); }
    void show() { YWindow::show(); }
    void hide() { YWindow::hide(); }
    bool isVisible() const { return YWindow::visible(); }
    void repaint() { NativePaint::repaint(*this); }
    void setOnClose(std::function<void()> cb) { onClose_ = cb; }
    void setImage(const flamewm::ui::ImageView& image) {
        image_ = image;
        nativeImage_ = null;
        icon_ = null;
        iconSize_ = 0;
        if (image_.asset.id() == flamewm::ui::AssetIdWordmark) {
            const std::string path = installedWordmarkPath();
            if (!path.empty()) nativeImage_ = YImage::load(upath(path.c_str()));
        }
        if (image_.asset.hasIconRole()) {
            const flamewm::IconFallbackEntry* entry =
                flamewm::iconFallbackForRole(image_.asset.iconRole());
            if (entry) {
                icon_ = YIcon::getIcon(entry->freedesktopName);
                iconSize_ = YIcon::fixIconSize(static_cast<unsigned>(
                    flamewm::logicalIconSizeForRole(image_.asset.iconRole())));
            }
        }
        repaint();
    }
    flamewm::ui::ImageView image() const { return image_; }
    void setVisual(const flamewm::ui::style::Visual& visual) {
        visual_ = visual;
        repaint();
    }
    void paint(Graphics& g, const YRect& rect) override {
        MaterialPainter::material(g, rect, flameTheme(),
                                  flamewm::ui::Window::visual().role,
                                  flamewm::ui::Window::visual().state);
        if (nativeImage_ != null) {
            MaterialPainter::image(g, nativeImage_, rect.x(), rect.y(),
                                   rect.width(), rect.height());
        } else if (icon_ != null && iconSize_ != 0)
            MaterialPainter::icon(g, icon_, (rect.width() - iconSize_) / 2,
                                  (rect.height() - iconSize_) / 2, iconSize_);
    }
private:
    flamewm::ui::ImageView image_;
    ref<YImage> nativeImage_;
    ref<YIcon> icon_;
    unsigned iconSize_;
    std::function<void()> onClose_;
};

static NativeThemeResources g_themeResources;

class NativeProvider : public flamewm::ui::UiProvider {
public:
    bool init() {
        if (!(xapp && xapp->display() && desktop)) return false;
        g_themeResources.assign(flameTheme());
        return true;
    }
    void shutdown() { g_themeResources.restore(); }
    flamewm::ui::Window* createWindow(flamewm::ui::Window* p) { return new NativeWindow(nativeParent(p)); }
    flamewm::ui::Button* createButton(flamewm::ui::Window* p, flamewm::IconRole r) { return new NativeButton(nativeParent(p), r); }
    flamewm::ui::Toggle* createToggle(flamewm::ui::Window* p, flamewm::IconRole r) { return new NativeToggle(nativeParent(p), r); }
    flamewm::ui::Label* createLabel(flamewm::ui::Window* p) { return new NativeLabel(nativeParent(p)); }
    flamewm::ui::Slider* createSlider(flamewm::ui::Window* p) { return new NativeSlider(nativeParent(p)); }
    flamewm::ui::Dialog* createDialog(flamewm::ui::Window* p) { return new NativeDialog(nativeParent(p)); }
    flamewm::ui::TextField* createTextField(flamewm::ui::Window* p) { return new NativeTextField(nativeParent(p)); }
    flamewm::ui::List* createList(flamewm::ui::Window* p) { return new NativeList(nativeParent(p)); }
    flamewm::ui::Popover* createPopover(flamewm::ui::Window* p) { return new NativePopover(nativeParent(p)); }
    flamewm::ui::Image* createImage(flamewm::ui::Window* p) { return new NativeImage(nativeParent(p)); }
};

static NativeProvider g_nativeProvider;
}

IceWMBackend::IceWMBackend() : impl_(new Impl()) {
    if (!g_instance) g_instance = this;
}

IceWMBackend::~IceWMBackend() {
    if (g_instance == this) g_instance = 0;
    delete impl_;
}

IceWMBackend& IceWMBackend::instance() {
    static IceWMBackend inst;
    return inst;
}

bool IceWMBackend::isAvailable() const {
    return impl_->initialized && flamewm::ui::UiBackend::isAvailable();
}

bool IceWMBackend::init() {
    if (impl_->initialized) return false;
    std::string error;
    if (!flamewm::ui::UiBackend::installProvider(&g_nativeProvider, &error)) return false;
    impl_->installedProvider = &g_nativeProvider;
    if (!flamewm::ui::UiBackend::init()) {
        flamewm::ui::UiBackend::removeProvider(impl_->installedProvider, 0);
        impl_->installedProvider = 0;
        return false;
    }
    impl_->initialized = true;
    return true;
}

void IceWMBackend::shutdown() {
    if (!impl_->initialized || !impl_->installedProvider) return;
    impl_->initialized = false;
    flamewm::ui::UiBackend::shutdown();
    flamewm::ui::UiBackend::removeProvider(impl_->installedProvider, 0);
    impl_->installedProvider = 0;
}

flamewm::ui::Window* IceWMBackend::createWindow() { return flamewm::ui::UiBackend::createWindow(); }
flamewm::ui::Window* IceWMBackend::createWindow(flamewm::ui::WindowRole role,
                                                flamewm::ui::Window* parent) {
    flamewm::ui::Window* window = flamewm::ui::UiBackend::createWindow(parent);
    if (window) window->setRole(role);
    return window;
}
flamewm::ui::Button* IceWMBackend::createButton() { return flamewm::ui::UiBackend::createButton(); }
flamewm::ui::Toggle* IceWMBackend::createToggle() { return flamewm::ui::UiBackend::createToggle(); }
flamewm::ui::Label* IceWMBackend::createLabel() { return flamewm::ui::UiBackend::createLabel(); }
flamewm::ui::Slider* IceWMBackend::createSlider() { return flamewm::ui::UiBackend::createSlider(); }
flamewm::ui::Dialog* IceWMBackend::createDialog() { return flamewm::ui::UiBackend::createDialog(); }
flamewm::ui::TextField* IceWMBackend::createTextField() { return flamewm::ui::UiBackend::createTextField(); }
flamewm::ui::List* IceWMBackend::createList() { return flamewm::ui::UiBackend::createList(); }
flamewm::ui::Popover* IceWMBackend::createPopover() { return flamewm::ui::UiBackend::createPopover(); }
flamewm::ui::Image* IceWMBackend::createImage() { return flamewm::ui::UiBackend::createImage(); }

} // namespace ui
} // namespace icewm
} // namespace engine
} // namespace flamewm
