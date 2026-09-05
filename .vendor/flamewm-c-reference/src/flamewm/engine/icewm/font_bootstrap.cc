#include "flamewm/engine/icewm/font_bootstrap.h"

#ifdef FLAMEWM_PRODUCT_BUILD

#include "yapp.h"
#include <fontconfig/fontconfig.h>
#include <stdio.h>
#include <string>

namespace flamewm {
namespace engine {
namespace icewm {
namespace {

static bool initialized = false;

} // namespace

void FontBootstrap::initialize() {
    if (initialized)
        return;
    initialized = true;

    if (!FcInit()) {
        fprintf(stderr, "FlameWM: bundled font bootstrap unavailable\n");
        return;
    }

    FcConfig* config = FcConfigGetCurrent();
    if (config == nullptr) {
        fprintf(stderr, "FlameWM: bundled font bootstrap unavailable\n");
        return;
    }

    const char* names[] = {
        "IBMPlexSans-Regular.ttf",
        "IBMPlexSans-Medium.ttf",
        "IBMPlexSans-SemiBold.ttf",
        "IBMPlexSans-Bold.ttf"
    };
    bool added = false;
    upath root = YApplication::getLibDir() + "/flamewm/fonts/";
    for (size_t i = 0; i < sizeof(names) / sizeof(names[0]); ++i) {
        upath path = root + names[i];
        if (FcConfigAppFontAddFile(config,
                                   reinterpret_cast<const FcChar8*>(path.string())))
            added = true;
    }

    if (added)
        FcConfigBuildFonts(config);
    else
        fprintf(stderr, "FlameWM: bundled font files not found under %s\n",
                root.string());
}

} // namespace icewm
} // namespace engine
} // namespace flamewm

#endif
