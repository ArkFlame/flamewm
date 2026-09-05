#include "settingsapp.h"
#include "ui/root.h"
#include "ui/pages.h"
#include "../flamewm/control/client.h"
#include <fstream>
#include <sstream>
#include <vector>
#include <map>
#include <cstdlib>
#include <cctype>
#include "../yxapp.h"
#include "../ywindow.h"
#include "../flamewm/engine/icewm/ui/backend.h"
#ifdef Status
#undef Status
#endif

namespace flamewm { namespace settings {
SettingsApp::SettingsApp(): current_(PageAppearance), inited_(false), controlClient_(0) {
    draft_ = SettingsSnapshot::defaults();
}
SettingsApp::~SettingsApp() { shutdown(); }
bool SettingsApp::parseCommandLine(int argc, char** argv, std::string* configPath,
                                   SettingsPageId* page, std::string* error) {
    if (!configPath || !page) { if (error) *error = "invalid command line outputs"; return false; }
    const char* home = std::getenv("HOME");
    *configPath = std::string(home ? home : "/tmp") + "/.config/flamewm/flame.conf";
    *page = PageAppearance;
    bool positionalConfig = false;
    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i] ? argv[i] : "";
        if (arg == "--config") {
            if (i + 1 >= argc || !argv[i + 1] || !argv[i + 1][0]) {
                if (error) *error = "--config requires PATH";
                return false;
            }
            *configPath = argv[++i];
        } else if (arg == "--page") {
            if (i + 1 >= argc || !argv[i + 1] || !argv[i + 1][0]) {
                if (error) *error = "--page requires a value";
                return false;
            }
            std::string value = argv[++i];
            for (size_t j = 0; j < value.size(); ++j)
                value[j] = static_cast<char>(std::tolower(static_cast<unsigned char>(value[j])));
            int selected = -1;
            const std::vector<ui::PageSpec>& specs = ui::pageSpecs();
            for (int j = 0; j < static_cast<int>(specs.size()) && j < static_cast<int>(PageCount); ++j) {
                std::string name = specs[j].name ? specs[j].name : "";
                for (size_t k = 0; k < name.size(); ++k)
                    name[k] = static_cast<char>(std::tolower(static_cast<unsigned char>(name[k])));
                if (value == name) { selected = j; break; }
            }
            if (selected < 0) {
                if (error) *error = "unknown --page value: " + value;
                return false;
            }
            *page = static_cast<SettingsPageId>(selected);
        } else if (arg.size() > 0 && arg[0] != '-') {
            if (positionalConfig) {
                if (error) *error = "multiple positional config paths";
                return false;
            }
            *configPath = arg;
            positionalConfig = true;
        }
    }
    return true;
}
bool SettingsApp::init(const std::string& path, std::string* error) {
    configPath_ = path; std::ifstream f(path.c_str());
    if (f) { std::ostringstream t; t << f.rdbuf(); ConfigStore codec; SettingsSnapshot loaded;
        if (!codec.parseSnapshot(t.str(), loaded, error)) return false; draft_ = loaded; }
    inited_ = true; controlClient_ = new flamewm::control::ControlClient(); controlClient_->connect();
    if (controlClient_->isConnected() && !reloadFromControl(error)) return false;
    return true;
}
void SettingsApp::shutdown() { delete controlClient_; controlClient_ = 0; inited_ = false; }
bool SettingsApp::reloadFromControl(std::string* error) {
    if (!controlClient_ || !controlClient_->isConnected()) { if (error) *error = "ControlClient not connected"; return false; }
    flamewm::api::Result<flamewm::api::SettingsSnapshot> r = controlClient_->getSettingsSnapshot();
    if (!r.isOk()) { if (error) *error = r.error().message; return false; }
    ConfigStore codec; std::ostringstream text;
    for (std::map<std::string, std::string>::const_iterator it = r.value().values.begin(); it != r.value().values.end(); ++it)
        text << it->first << "=" << it->second << "\n";
    SettingsSnapshot loaded; if (!codec.parseSnapshot(text.str(), loaded, error)) return false;
    loaded.revision = r.value().revision; draft_ = loaded; return true;
}
flamewm::control::ControlClient* SettingsApp::controlClient() { return controlClient_; }
static std::vector<flamewm::api::SettingsChange> snapshotDiff(const SettingsSnapshot& from, const SettingsSnapshot& to) {
    std::vector<flamewm::api::SettingsChange> out; ConfigStore c1, c2; std::map<std::string, std::string> a, b;
    std::istringstream f1(c1.serialize(from)), f2(c2.serialize(to)); std::string line;
    while (std::getline(f1, line)) { size_t p = line.find('='); if (p != std::string::npos && line.substr(0, p) != "revision") a[line.substr(0, p)] = line.substr(p + 1); }
    while (std::getline(f2, line)) { size_t p = line.find('='); if (p != std::string::npos && line.substr(0, p) != "revision") b[line.substr(0, p)] = line.substr(p + 1); }
    for (std::map<std::string, std::string>::const_iterator it = b.begin(); it != b.end(); ++it)
        if (a.find(it->first) == a.end() || a[it->first] != it->second) out.push_back(flamewm::api::SettingsChange(it->first, it->second));
    for (std::map<std::string, std::string>::const_iterator it = a.begin(); it != a.end(); ++it)
        if (b.find(it->first) == b.end()) out.push_back(flamewm::api::SettingsChange(it->first, std::string()));
    return out;
}
bool SettingsApp::applySnapshot(const SettingsSnapshot& snap, std::string* error) {
    if (snap.revision <= draft_.revision) { if (error) *error = "stale revision"; return false; }
    if (!snap.isValid(error)) return false;
    if (!controlClient_ || !controlClient_->isConnected()) { if (error) *error = "Control not connected"; return false; }
    std::vector<flamewm::api::SettingsChange> changes = snapshotDiff(draft_, snap);
    if (changes.empty()) {
        std::string syncError;
        if (!reloadFromControl(&syncError)) {
            if (error) *error = "settings sync error: " + syncError;
            return false;
        }
        return true;
    }
    flamewm::api::Status st = controlClient_->applySettings(draft_.revision, changes);
    if (!st.ok()) { if (error) *error = st.message; return false; }
    std::string syncError;
    if (!reloadFromControl(&syncError)) {
        if (error) *error = "settings sync error: " + syncError;
        return false;
    }
    return true;
}
bool SettingsApp::keepDisplayTx(std::string* error) {
    if (!controlClient_ || !controlClient_->isConnected()) { if (error) *error = "Control not connected"; return false; }
    flamewm::api::Result<flamewm::api::DisplaySnapshot> ds = controlClient_->getDisplaySnapshot();
    if (!ds.isOk()) { if (error) *error = ds.error().message; return false; }
    if (!ds.value().hasPending()) { if (error) *error = "no pending display tx"; return false; }
    flamewm::api::Status st = controlClient_->keepDisplayMode(ds.value().pending.tx); if (!st.ok()) { if (error) *error = st.message; return false; } return true;
}
bool SettingsApp::revertDisplayTx(std::string* error) {
    if (!controlClient_ || !controlClient_->isConnected()) { if (error) *error = "Control not connected"; return false; }
    flamewm::api::Result<flamewm::api::DisplaySnapshot> ds = controlClient_->getDisplaySnapshot();
    if (!ds.isOk()) { if (error) *error = ds.error().message; return false; }
    if (!ds.value().hasPending()) { if (error) *error = "no pending display tx"; return false; }
    flamewm::api::Status st = controlClient_->revertDisplayMode(ds.value().pending.tx); if (!st.ok()) { if (error) *error = st.message; return false; } return true;
}
int SettingsApp::runNative(int argc, char** argv) {
    YXApplication app(&argc, &argv);
    // YXApplication is the IceWM lifecycle owner and normally constructs the
    // desktop root. Keep the standalone process valid if that owner was not
    // installed by a downstream lifecycle variant.
    bool ownsDesktop = false;
    if (!desktop) {
        new YDesktop(0, app.root());
        ownsDesktop = true;
    }
    flamewm::engine::icewm::ui::IceWMBackend& backend = flamewm::engine::icewm::ui::IceWMBackend::instance();
    if (!backend.init()) {
        if (ownsDesktop) delete desktop;
        return 1;
    }
    ui::Root root(this); std::string error;
    if (!root.create(&error)) {
        backend.shutdown();
        if (ownsDesktop) delete desktop;
        return 1;
    }
    root.show(); int result = app.mainLoop(); backend.shutdown();
    if (ownsDesktop) delete desktop;
    return result;
}
}}
