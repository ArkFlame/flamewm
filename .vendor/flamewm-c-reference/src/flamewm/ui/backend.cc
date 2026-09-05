#include "flamewm/ui/backend.h"
#include "flamewm/api/geometry.h"

#include <vector>
#include <string>

namespace flamewm {
namespace ui {

// ---- Fallback (headless) implementations: guarantee non-null without X ----

namespace {

class FallbackWindow : public Window {
public:
    explicit FallbackWindow(Window* parent = 0) : parent_(parent), rect_(0,0,100,100), visible_(false) {}
    void setGeometry(const api::Rect& r) override { rect_ = r; }
    api::Rect geometry() const override { return rect_; }
    void show() override { visible_ = true; }
    void hide() override { visible_ = false; }
    bool isVisible() const override { return visible_; }
    void repaint() override {}
    void setOnClose(std::function<void()> cb) override { onClose_ = cb; }
    std::function<void()> onClose_;
    api::Rect rect_;
    bool visible_;
    Window* parent_;
};

class FallbackButton : public Button {
public:
    FallbackButton(Window* parent = 0, IconRole role = IconRoleTaskbar) : parent_(parent), rect_(0,0,80,32), visible_(false), enabled_(true), icon_(role), visualState_(0) {}
    void setGeometry(const api::Rect& r) override { rect_=r; }
    api::Rect geometry() const override { return rect_; }
    void show() override { visible_=true; }
    void hide() override { visible_=false; }
    bool isVisible() const override { return visible_; }
    void repaint() override {}
    void setOnClose(std::function<void()> cb) override { onClose_=cb; }
    void setText(const std::string& t) override { text_=t; }
    std::string text() const override { return text_; }
    void setIconRole(IconRole r) override { icon_=r; }
    IconRole iconRole() const override { return icon_; }
    void setIconName(const std::string& n) override { iconName_=n; }
    std::string iconName() const override { return iconName_; }
    void setVisualState(unsigned state) override { visualState_=state; }
    unsigned visualState() const override { return visualState_; }
    void setEnabled(bool e) override { enabled_=e; }
    bool isEnabled() const override { return enabled_; }
    void setOnClick(std::function<void()> cb) override { onClick_=cb; }
    void setOnContextMenu(std::function<void(int,int)> cb) override { onCtx_=cb; }
    void setOnPress(std::function<void(int,int,int)> cb) override { onPress_=cb; }
    void setOnRelease(std::function<void(int,int,int)> cb) override { onRelease_=cb; }
    void setOnMotion(std::function<void(int,int)> cb) override { onMotion_=cb; }
    std::string text_, iconName_; IconRole icon_; unsigned visualState_; bool enabled_;
    std::function<void()> onClick_; std::function<void(int,int)> onCtx_;
    std::function<void(int,int,int)> onPress_, onRelease_;
    std::function<void(int,int)> onMotion_;
    std::function<void()> onClose_; api::Rect rect_; bool visible_;
    Window* parent_;
};

class FallbackToggle : public Toggle {
public:
    FallbackToggle(Window* parent = 0, IconRole role = IconRoleTaskbar) : parent_(parent), rect_(0,0,80,32), visible_(false), enabled_(true), checked_(false), icon_(role), visualState_(0) {}
    void setGeometry(const api::Rect& r) override { rect_=r; }
    api::Rect geometry() const override { return rect_; }
    void show() override { visible_=true; }
    void hide() override { visible_=false; }
    bool isVisible() const override { return visible_; }
    void repaint() override {}
    void setOnClose(std::function<void()> cb) override { onClose_=cb; }
    void setText(const std::string& t) override { text_=t; }
    std::string text() const override { return text_; }
    void setIconRole(IconRole r) override { icon_=r; }
    IconRole iconRole() const override { return icon_; }
    void setIconName(const std::string& n) override { iconName_=n; }
    std::string iconName() const override { return iconName_; }
    void setVisualState(unsigned state) override { visualState_=state; }
    unsigned visualState() const override { return visualState_; }
    void setEnabled(bool e) override { enabled_=e; }
    bool isEnabled() const override { return enabled_; }
    void setOnClick(std::function<void()> cb) override { onClick_=cb; }
    void setOnContextMenu(std::function<void(int,int)> cb) override { onCtx_=cb; }
    void setOnPress(std::function<void(int,int,int)> cb) override { onPress_=cb; }
    void setOnRelease(std::function<void(int,int,int)> cb) override { onRelease_=cb; }
    void setOnMotion(std::function<void(int,int)> cb) override { onMotion_=cb; }
    void setChecked(bool c) override { if (checked_ == c) return; checked_=c; if(onToggled_) onToggled_(c); }
    bool isChecked() const override { return checked_; }
    void setOnToggled(std::function<void(bool)> cb) override { onToggled_=cb; }
    std::string text_, iconName_; IconRole icon_; unsigned visualState_; bool enabled_; bool checked_;
    std::function<void()> onClick_; std::function<void(int,int)> onCtx_;
    std::function<void(int,int,int)> onPress_, onRelease_;
    std::function<void(int,int)> onMotion_;
    std::function<void(bool)> onToggled_;
    std::function<void()> onClose_; api::Rect rect_; bool visible_;
    Window* parent_;
};

class FallbackLabel : public Label {
public:
    explicit FallbackLabel(Window* parent = 0) : parent_(parent), rect_(0,0,100,20), visible_(false), wrap_(false) {}
    void setGeometry(const api::Rect& r) override { rect_=r; }
    api::Rect geometry() const override { return rect_; }
    void show() override { visible_=true; }
    void hide() override { visible_=false; }
    bool isVisible() const override { return visible_; }
    void repaint() override {}
    void setOnClose(std::function<void()> cb) override { onClose_=cb; }
    void setText(const std::string& t) override { text_=t; }
    std::string text() const override { return text_; }
    void setWrap(bool w) override { wrap_=w; }
    bool isWrap() const override { return wrap_; }
    std::string text_; bool wrap_; std::function<void()> onClose_; api::Rect rect_; bool visible_;
    Window* parent_;
};

class FallbackImage : public Image {
public:
    explicit FallbackImage(Window* parent = 0) : parent_(parent), rect_(0,0,100,100), visible_(false), image_() {}
    void setGeometry(const api::Rect& r) override { rect_ = r; }
    api::Rect geometry() const override { return rect_; }
    void show() override { visible_ = true; }
    void hide() override { visible_ = false; }
    bool isVisible() const override { return visible_; }
    void repaint() override {}
    void setOnClose(std::function<void()> cb) override { onClose_ = cb; }
    void setImage(const ImageView& image) override { image_ = image; }
    ImageView image() const override { return image_; }
private:
    Window* parent_;
    api::Rect rect_;
    bool visible_;
    ImageView image_;
    std::function<void()> onClose_;
};

class FallbackSlider : public Slider {
public:
    explicit FallbackSlider(Window* parent = 0) : parent_(parent), rect_(0,0,120,20), visible_(false), enabled_(true), min_(0), max_(100), val_(0) {}
    void setGeometry(const api::Rect& r) override { rect_=r; }
    api::Rect geometry() const override { return rect_; }
    void show() override { visible_=true; }
    void hide() override { visible_=false; }
    bool isVisible() const override { return visible_; }
    void repaint() override {}
    void setEnabled(bool enabled) override { enabled_=enabled; }
    bool isEnabled() const override { return enabled_; }
    void setOnClose(std::function<void()> cb) override { onClose_=cb; }
    void setRange(int a,int b) override { min_=a <= b ? a : b; max_=a <= b ? b : a; int next=val_ < min_ ? min_ : (val_ > max_ ? max_ : val_); if(next != val_) { val_=next; if(onChanged_) onChanged_(val_); } }
    int minimum() const override { return min_; }
    int maximum() const override { return max_; }
    void setValue(int v) override { if (!enabled_) return; int next=v < min_ ? min_ : (v > max_ ? max_ : v); if (next == val_) return; val_=next; if(onChanged_) onChanged_(val_); }
    int value() const override { return val_; }
    void setOnChanged(std::function<void(int)> cb) override { onChanged_=cb; }
    int min_, max_, val_; bool enabled_; std::function<void(int)> onChanged_; std::function<void()> onClose_; api::Rect rect_; bool visible_;
    Window* parent_;
};

class FallbackDialog : public Dialog {
public:
    explicit FallbackDialog(Window* parent = 0) : parent_(parent), rect_(0,0,300,200), visible_(false) {}
    void setGeometry(const api::Rect& r) override { rect_=r; }
    api::Rect geometry() const override { return rect_; }
    void show() override { visible_=true; }
    void hide() override { visible_=false; }
    bool isVisible() const override { return visible_; }
    void repaint() override {}
    void setOnClose(std::function<void()> cb) override { onClose_=cb; }
    void setTitle(const std::string& t) override { title_=t; }
    std::string title() const override { return title_; }
    int exec() override { visible_=true; return -1; }
    void accept() override { visible_=false; }
    void reject() override { visible_=false; }
    std::string title_; std::function<void()> onClose_; api::Rect rect_; bool visible_;
    Window* parent_;
};

class FallbackTextField : public TextField {
public:
    explicit FallbackTextField(Window* parent = 0) : parent_(parent), rect_(0,0,200,28), visible_(false), focused_(false) {}
    void setGeometry(const api::Rect& r) override { rect_=r; }
    api::Rect geometry() const override { return rect_; }
    void show() override { visible_=true; }
    void hide() override { visible_=false; }
    bool isVisible() const override { return visible_; }
    void repaint() override {}
    void setOnClose(std::function<void()> cb) override { onClose_=cb; }
    void setText(const std::string& t) override { text_=t; if(onChanged_) onChanged_(text_); }
    std::string text() const override { return text_; }
    void setPlaceholder(const std::string& p) override { ph_=p; }
    std::string placeholder() const override { return ph_; }
    void setOnChanged(std::function<void(const std::string&)> cb) override { onChanged_=cb; }
    void setOnSubmit(std::function<void(const std::string&)> cb) override { onSubmit_=cb; }
    void focus() override { focused_=true; }
    bool hasFocus() const override { return focused_; }
    std::string text_, ph_; bool focused_; std::function<void(const std::string&)> onChanged_, onSubmit_;
    std::function<void()> onClose_; api::Rect rect_; bool visible_;
    Window* parent_;
};

class FallbackList : public List {
public:
    explicit FallbackList(Window* parent = 0) : parent_(parent), rect_(0,0,200,300), visible_(false), sel_(-1) {}
    void setGeometry(const api::Rect& r) override { rect_=r; }
    api::Rect geometry() const override { return rect_; }
    void show() override { visible_=true; }
    void hide() override { visible_=false; }
    bool isVisible() const override { return visible_; }
    void repaint() override {}
    void setOnClose(std::function<void()> cb) override { onClose_=cb; }
    void setRows(const std::vector<ListRow>& rows) override { rows_=rows; int next=sel_ < (int)rows_.size() ? sel_ : -1; if(next != sel_) { sel_=next; if(onSel_) onSel_(sel_); } }
    std::vector<ListRow> rows() const override { return rows_; }
    void setSelected(int i) override { int next=i>=0 && i<(int)rows_.size()?i:-1; if(next==sel_) return; sel_=next; if(onSel_) onSel_(sel_); }
    int selected() const override { return sel_; }
    std::string selectedId() const override { if(sel_>=0 && sel_<(int)rows_.size()) return rows_[sel_].id; return std::string(); }
    void setOnSelectionChanged(std::function<void(int)> cb) override { onSel_=cb; }
    void setOnActivated(std::function<void(int)> cb) override { onAct_=cb; }
    std::vector<ListRow> rows_; int sel_; std::function<void(int)> onSel_, onAct_; std::function<void()> onClose_; api::Rect rect_; bool visible_;
    Window* parent_;
};

class FallbackPopover : public Popover {
public:
    explicit FallbackPopover(Window* parent = 0) : parent_(parent), rect_(0,0,200,200), visible_(false), anchor_(0,0,0,0) {}
    ~FallbackPopover() { close(); }
    void setGeometry(const api::Rect& r) override { rect_=r; }
    api::Rect geometry() const override { return rect_; }
    void show() override { visible_=true; }
    void hide() override { visible_=false; }
    bool isVisible() const override { return visible_; }
    void repaint() override {}
    void setOnClose(std::function<void()> cb) override { onClose_=cb; }
    void setAnchor(const api::Rect& r) override { anchor_=r; }
    api::Rect anchor() const override { return anchor_; }
    void showAt(const api::Rect& a) override { anchor_=a; visible_=true; }
    void close() override { if (!visible_) return; visible_=false; if(onClosed_) onClosed_(); }
    void setOnClosed(std::function<void()> cb) override { onClosed_=cb; }
    api::Rect anchor_; std::function<void()> onClosed_, onClose_; api::Rect rect_; bool visible_;
    Window* parent_;
};

class TestProvider : public UiProvider {
public:
    TestProvider() : initialized_(false) {}
    bool init() { if (initialized_) return false; initialized_ = true; return true; }
    void shutdown() { initialized_ = false; }
    Window* createWindow(Window* p) { return new FallbackWindow(p); }
    Button* createButton(Window* p, IconRole r) { return new FallbackButton(p, r); }
    Toggle* createToggle(Window* p, IconRole r) { return new FallbackToggle(p, r); }
    Label* createLabel(Window* p) { return new FallbackLabel(p); }
    Slider* createSlider(Window* p) { return new FallbackSlider(p); }
    Dialog* createDialog(Window* p) { return new FallbackDialog(p); }
    TextField* createTextField(Window* p) { return new FallbackTextField(p); }
    List* createList(Window* p) { return new FallbackList(p); }
    Popover* createPopover(Window* p) { return new FallbackPopover(p); }
    Image* createImage(Window* p) { return new FallbackImage(p); }
private:
    bool initialized_;
};

static UiProvider* g_provider = 0;
static bool g_initialized = false;
static TestProvider g_testProvider;

} // namespace

static bool fail(std::string* error, const char* message) {
    if (error) *error = message;
    return false;
}

bool UiBackend::installProvider(UiProvider* provider, std::string* error) {
    if (!provider) return fail(error, "UI provider is null");
    if (g_provider) return fail(error, "UI provider already installed");
    g_provider = provider;
    return true;
}

bool UiBackend::removeProvider(UiProvider* provider, std::string* error) {
    if (g_initialized) return fail(error, "UI provider is initialized");
    if (!g_provider || g_provider != provider) return fail(error, "UI provider is not installed");
    g_provider = 0;
    return true;
}

bool UiBackend::installTestProvider(std::string* error) { return installProvider(&g_testProvider, error); }
bool UiBackend::isAvailable() { return g_initialized; }
bool UiBackend::init() {
    if (!g_provider && !installTestProvider()) return false;
    if (!g_provider || g_initialized) return false;
    if (!g_provider->init()) return false;
    g_initialized = true;
    return true;
}
void UiBackend::shutdown() {
    if (!g_initialized) return;
    g_provider->shutdown();
    g_initialized = false;
}

Window* UiBackend::createWindow(Window* p) { return g_initialized ? g_provider->createWindow(p) : g_testProvider.createWindow(p); }
Button* UiBackend::createButton(Window* p, IconRole r) { return g_initialized ? g_provider->createButton(p, r) : g_testProvider.createButton(p, r); }
Toggle* UiBackend::createToggle(Window* p, IconRole r) { return g_initialized ? g_provider->createToggle(p, r) : g_testProvider.createToggle(p, r); }
Label* UiBackend::createLabel(Window* p) { return g_initialized ? g_provider->createLabel(p) : g_testProvider.createLabel(p); }
Slider* UiBackend::createSlider(Window* p) { return g_initialized ? g_provider->createSlider(p) : g_testProvider.createSlider(p); }
Dialog* UiBackend::createDialog(Window* p) { return g_initialized ? g_provider->createDialog(p) : g_testProvider.createDialog(p); }
TextField* UiBackend::createTextField(Window* p) { return g_initialized ? g_provider->createTextField(p) : g_testProvider.createTextField(p); }
List* UiBackend::createList(Window* p) { return g_initialized ? g_provider->createList(p) : g_testProvider.createList(p); }
Popover* UiBackend::createPopover(Window* p) { return g_initialized ? g_provider->createPopover(p) : g_testProvider.createPopover(p); }
Image* UiBackend::createImage(Window* p) {
    if (g_initialized) {
        Image* image = g_provider->createImage(p);
        if (image) return image;
    }
    return g_testProvider.createImage(p);
}

} // namespace ui
} // namespace flamewm
