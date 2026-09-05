#ifndef FLAMEWM_SETTINGS_ABOUT_H
#define FLAMEWM_SETTINGS_ABOUT_H
#include "../../flamewm/core/productmetadata.h"
#include <string>
namespace flamewm { namespace control { class ControlClient; } }
namespace flamewm { namespace settings {
struct AboutPage {
    static std::string tagline(){ return "A lightweight desktop by ArkFlame Studios"; }
    static std::string donateUrl(){ return ProductMetadata::donateUrl(); }
    static std::string sourceUrl(){ return ProductMetadata::sourceUrl(); }
    static std::string websiteUrl(){ return ProductMetadata::websiteUrl(); }
    static bool wordmarkPreserveAspect(){ return true; }
    static bool launchDonate(SafeUriLauncher* launcher, std::string* err){ return launcher?launcher->launchUri(donateUrl(),err):false; }
    static bool launchSource(SafeUriLauncher* launcher, std::string* err){ return launcher?launcher->launchUri(sourceUrl(),err):false; }
    static bool launchWebsite(SafeUriLauncher* launcher, std::string* err){ return launcher?launcher->launchUri(websiteUrl(),err):false; }
    // verify build identity via Control version when available
    static bool verifyControlVersion(flamewm::control::ControlClient* client, std::string* versionOut, std::string* error);
};
}} // namespace
#endif
