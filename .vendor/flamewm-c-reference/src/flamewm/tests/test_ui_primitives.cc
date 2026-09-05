#include <iostream>
#include "../ui/style/theme.h"
#include "../ui/style/box.h"
#include "../ui/style/visual.h"
#include "../ui/assets/asset.h"
#include "../ui/image.h"
#include "../ui/layout.h"

#define CHECK(c, m) do { if (!(c)) { std::cerr << "FAIL: " << m << "\n"; return 1; } } while (0)

int main() {
    flamewm::ui::style::Theme theme = flamewm::ui::style::Theme::defaults();
    CHECK(theme.accentHover.value == "#F2575E", "theme accent hover");
    CHECK(theme.background.value == "#202326" && theme.panel.value == "#191B1D",
          "theme surface palette");
    CHECK(theme.primaryText().value == theme.text.value, "theme primary text");
    CHECK(theme.color(flamewm::ui::style::VisualRoleText,
                      flamewm::ui::style::VisualNormal).value == theme.text.value,
          "theme semantic color");
    CHECK(theme.typography.effectiveSize() == 13, "typography size");
    CHECK(theme.typography.sizeFor(flamewm::ui::style::TypographyTitle) == 21,
          "typography title role");
    CHECK(theme.settings.navigationWidth == 192 && theme.settings.pagePadding == 24 &&
              theme.settings.rowHeight >= 68,
          "V8 settings metrics");

    flamewm::ui::style::Box box(10, 20, 40, 30);
    CHECK(box.contains(10, 20) && !box.contains(50, 20), "box half open");
    CHECK(box.inset(5).width == 30, "box inset");
    flamewm::ui::style::Visual visual(flamewm::ui::style::VisualRoleControl,
                                      flamewm::ui::style::VisualDisabled);
    CHECK(!visual.interactive(), "disabled visual");
    flamewm::ui::style::Visual states(flamewm::ui::style::VisualRoleControl,
                                      flamewm::ui::style::VisualHovered | flamewm::ui::style::VisualFocused);
    CHECK(states.hasState(flamewm::ui::style::VisualHovered) && states.interactive(),
          "visual state mask");
    CHECK((flamewm::ui::style::VisualHovered | states.state) == states.state,
          "visual state composition");
    CHECK(flamewm::ui::style::hasVisualState(flamewm::ui::style::VisualNormal,
                                             flamewm::ui::style::VisualNormal),
          "normal visual state");

    flamewm::ui::style::Typography small;
    small.size = 2;
    CHECK(small.sizeFor(flamewm::ui::style::TypographyCaption) == 1,
          "typography minimum size");

    flamewm::ui::style::Material material(flamewm::ui::style::Color("#111111"),
                                           flamewm::ui::style::Color("#222222"),
                                           flamewm::ui::style::Color("#000000"), 4, 1);
    flamewm::ui::style::BoxStyle styledBox;
    styledBox.material = material;
    CHECK(styledBox.hasFill() && styledBox.hasBorder(), "material box style");

    flamewm::ui::AssetRef appearance(flamewm::ui::AssetIdAppearance);
    CHECK(appearance.valid() && std::string(flamewm::ui::assetIdName(appearance.id())) == "Appearance",
          "semantic asset");
    unsigned char pixel = 0;
    flamewm::ui::ImageView image(appearance, &pixel, 1, 2, 3);
    CHECK(image.valid() && image.drawable() && image.intrinsicWidth == 2, "image view");
    image.opacity = 2.0f;
    CHECK(!image.drawable(), "image opacity bounds");
    CHECK(flamewm::ui::isRuntimeAssetPath("lib/flamewm/icons/flamewm-start.svg"),
          "runtime asset path");
    CHECK(!flamewm::ui::isRuntimeAssetPath("lib/flamewm/../icons/flamewm-start.svg"),
          "engineering path rejected");

    flamewm::ui::Grid grid;
    grid.add("wide", 2);
    grid.add("next", 2);
    std::vector<flamewm::ui::LayoutResult> gridResults =
        flamewm::ui::solveGrid(grid, flamewm::ui::Rect(0, 0, 300, 100), 3);
    CHECK(gridResults.size() == 2 && gridResults[0].rect.w == 200 &&
               gridResults[1].rect.y > gridResults[0].rect.y,
           "grid spans consume columns");

    flamewm::ui::Grid hiddenGrid;
    hiddenGrid.add(flamewm::ui::LayoutItem("hidden", 2).setVisible(false));
    hiddenGrid.add("first");
    hiddenGrid.add("second");
    std::vector<flamewm::ui::LayoutResult> hiddenResults =
        flamewm::ui::solveGrid(hiddenGrid, flamewm::ui::Rect(0, 0, 300, 100), 3);
    CHECK(hiddenResults.size() == 3 && !hiddenResults[0].visible &&
              hiddenResults[0].rect.w == 0 && hiddenResults[0].rect.h == 0 &&
              hiddenResults[1].rect.x == 0 && hiddenResults[2].rect.x == 100 &&
              hiddenResults[1].rect.y == hiddenResults[2].rect.y,
          "hidden grid items consume no cells");
    return 0;
}
