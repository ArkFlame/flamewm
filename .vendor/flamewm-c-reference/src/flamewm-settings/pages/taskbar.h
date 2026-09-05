#ifndef FLAMEWM_SETTINGS_TASKBAR_H
#define FLAMEWM_SETTINGS_TASKBAR_H
#include <string>
#include <cstdint>
#include <sys/stat.h>
namespace flamewm { namespace control { class ControlClient; } }
namespace flamewm { namespace settings {
struct TaskbarPage {
    std::string color;
    int opacity;
    int height;
    std::string startText;
    std::string startIcon;
    bool opacitySupported;
    uint64_t revision;
    flamewm::control::ControlClient* boundControlClient;
    TaskbarPage(): color("#191b1d"), opacity(99), height(44), opacitySupported(false), revision(0), boundControlClient(0) {}
    bool reloadFromControl(flamewm::control::ControlClient* client, std::string* error);
    bool applyColorImmediate(flamewm::control::ControlClient* client, const std::string& value, std::string* error);
    bool applyOpacityImmediate(flamewm::control::ControlClient* client, int value, std::string* error);
    bool applyHeightImmediate(flamewm::control::ControlClient* client, int value, std::string* error);
    bool applyStartTextImmediate(flamewm::control::ControlClient* client, const std::string& value, std::string* error);
    bool applyStartIconImmediate(flamewm::control::ControlClient* client, const std::string& path, std::string* error);
    void refreshOpacityCapability(flamewm::control::ControlClient* client);
    bool setColor(const std::string& value, std::string* error) {
        if (value.size() != 7 || value[0] != '#') { if (error) *error = "taskbarColor must be #RRGGBB"; return false; }
        for (size_t i = 1; i < value.size(); ++i) {
            const char c = value[i];
            if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F'))) {
                if (error) *error = "taskbarColor must be #RRGGBB";
                return false;
            }
        }
        color = value; return true;
    }
    bool setOpacity(int value, std::string* error) {
        if (!opacitySupported) { if (error) *error = "taskbar opacity managed by taskbar owner"; return false; }
        if (value < 0 || value > 100) { if (error) *error = "taskbarOpacity out of range 0..100"; return false; }
        opacity = value; return true;
    }
    bool setHeight(int value, std::string* error) {
        if (value < 34 || value > 72) { if (error) *error = "taskbarHeight out of range 34..72"; return false; }
        height = value; return true;
    }
    void setStartText(const std::string& value) { startText = value; }
    bool importStartIcon(const std::string& path, std::string* error) {
        struct stat info;
        if (path.empty()) { startIcon.clear(); return true; }
        if (stat(path.c_str(), &info) != 0 || !S_ISREG(info.st_mode)) { if (error) *error = "custom Start icon unavailable"; return false; }
        startIcon = path; return true;
    }
    void resetStartIcon() { startIcon.clear(); }
};
}}
#endif
