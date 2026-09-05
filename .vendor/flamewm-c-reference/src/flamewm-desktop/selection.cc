#include "selection.h"

namespace flamewm {
namespace desktop {

std::set<std::string> SelectionModel::hitTest(const std::map<std::string, ItemRect>& itemRects) const {
    std::set<std::string> out;
    if(!hasRubber_) return out;
    for(std::map<std::string,ItemRect>::const_iterator it=itemRects.begin(); it!=itemRects.end(); ++it){
        if(it->second.intersects(rubber_)) out.insert(it->first);
    }
    return out;
}

} // namespace desktop
} // namespace flamewm
