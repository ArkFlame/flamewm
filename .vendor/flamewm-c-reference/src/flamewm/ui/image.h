#ifndef FLAMEWM_UI_IMAGE_H
#define FLAMEWM_UI_IMAGE_H

#include "assets/asset.h"
#include "style/theme.h"
#include "window.h"

#include <cstddef>

namespace flamewm {
namespace ui {

enum ImageFit {
    ImageFitNone = 0,
    ImageFitContain,
    ImageFitCover,
    ImageFitStretch
};

enum ImageAlignment {
    ImageAlignStart = 0,
    ImageAlignCenter,
    ImageAlignEnd
};

// Non-owning image value. Pixels are supplied by a runtime loader; the view
// itself has no path, toolkit, or filesystem dependency.
struct ImageView {
    AssetRef asset;
    const unsigned char* pixels;
    std::size_t byteCount;
    unsigned width;
    unsigned height;
    ImageFit fit;
    ImageAlignment alignment;
    ImageAlignment horizontalAlignment;
    ImageAlignment verticalAlignment;
    float scale;
    unsigned scalePercent;
    float opacity;
    style::Color tint;
    unsigned intrinsicWidth;
    unsigned intrinsicHeight;

    ImageView()
        : asset(), pixels(0), byteCount(0), width(0), height(0), fit(ImageFitContain),
          alignment(ImageAlignCenter), horizontalAlignment(ImageAlignCenter),
          verticalAlignment(ImageAlignCenter), scale(1.0f), scalePercent(100),
          opacity(1.0f), tint(), intrinsicWidth(0), intrinsicHeight(0) {}

    ImageView(const AssetRef& assetRef, const unsigned char* imagePixels,
              std::size_t imageByteCount, unsigned imageWidth,
              unsigned imageHeight)
        : asset(assetRef), pixels(imagePixels), byteCount(imageByteCount),
          width(imageWidth), height(imageHeight), fit(ImageFitContain),
          alignment(ImageAlignCenter), horizontalAlignment(ImageAlignCenter),
          verticalAlignment(ImageAlignCenter), scale(1.0f), scalePercent(100),
          opacity(1.0f), tint(), intrinsicWidth(imageWidth), intrinsicHeight(imageHeight) {}

    bool valid() const {
        return asset.valid() && pixels != 0 && byteCount != 0 &&
               width != 0 && height != 0 && intrinsicWidth != 0 && intrinsicHeight != 0;
    }

    bool drawable() const {
        return valid() && opacity > 0.0f && opacity <= 1.0f && scale > 0.0f && scalePercent != 0;
    }
    unsigned intrinsicWidthValue() const { return intrinsicWidth; }
    unsigned intrinsicHeightValue() const { return intrinsicHeight; }
    bool hasTint() const { return !tint.empty(); }
};

class Image : public Window {
public:
    virtual void setImage(const ImageView& image) = 0;
    virtual ImageView image() const = 0;
    virtual void setAsset(const AssetRef& asset) {
        ImageView next = image();
        next.asset = asset;
        setImage(next);
    }
    virtual AssetRef asset() const { return image().asset; }
    virtual ~Image() {}
};

} // namespace ui
} // namespace flamewm

#endif
