#ifndef FLAMEWM_PANEL_TASKCONTEXT_H
#define FLAMEWM_PANEL_TASKCONTEXT_H

#include "../core/appidentity.h"
#include <vector>

namespace flamewm { namespace panel {

enum TaskWindowState {
    TaskWindowHidden = 0,
    TaskWindowVisible = 1
};

class TaskContext {
public:
    static std::vector<ContextAction> actions(bool pinned, bool hasWindow,
                                               TaskWindowState visibility, bool maximized);
    static bool contains(const std::vector<ContextAction>& actions, ContextAction action);
};

} }
#endif
