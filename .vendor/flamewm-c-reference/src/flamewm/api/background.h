#ifndef FLAMEWM_API_BACKGROUND_H
#define FLAMEWM_API_BACKGROUND_H

#include "geometry.h"
#include <string>

namespace flamewm {
namespace api {

struct BackgroundState {
    std::string wallpaperPath;
    std::string fillMode;
    std::string color;
};

} // namespace api
} // namespace flamewm

#endif // FLAMEWM_API_BACKGROUND_H
