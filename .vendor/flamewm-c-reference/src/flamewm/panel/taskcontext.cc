#include "taskcontext.h"

namespace flamewm { namespace panel {

std::vector<ContextAction> TaskContext::actions(bool pinned, bool hasWindow,
                                                 TaskWindowState visibility, bool maximized) {
    return ApplicationIdentity::contextActions(pinned, hasWindow,
                                               visibility == TaskWindowVisible, maximized);
}

bool TaskContext::contains(const std::vector<ContextAction>& actions, ContextAction action) {
    for (size_t i = 0; i < actions.size(); ++i) if (actions[i] == action) return true;
    return false;
}

} }
