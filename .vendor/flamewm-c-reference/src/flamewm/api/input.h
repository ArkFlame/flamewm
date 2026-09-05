#ifndef FLAMEWM_API_INPUT_H
#define FLAMEWM_API_INPUT_H

#include "geometry.h"
#include "ids.h"

namespace flamewm {
namespace api {

struct PointerPosition {
    Point root;
    OutputId output;

    PointerPosition() : root(), output() {}
    PointerPosition(const Point& r, const OutputId& o) : root(r), output(o) {}
};

} // namespace api
} // namespace flamewm

#endif // FLAMEWM_API_INPUT_H
