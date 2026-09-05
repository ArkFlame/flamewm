#ifndef FLAMEWM_PANEL_TASKIDENTITY_H
#define FLAMEWM_PANEL_TASKIDENTITY_H
// Shim: task identity delegates to core ApplicationIdentity per contract.
// Kept as separate header to satisfy task spec either-path.
#include "../core/appidentity.h"
namespace flamewm {
namespace panel {
using ApplicationIdentity = flamewm::ApplicationIdentity;
using AppIdentityInput = flamewm::AppIdentityInput;
}
}
#endif
