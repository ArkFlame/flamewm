#ifndef FLAMEWM_INTEGRATIONS_DISPLAYMANAGER_H
#define FLAMEWM_INTEGRATIONS_DISPLAYMANAGER_H
// V5 alias: WM/display authority owns RandR observe + tx; Settings is confirmation UI only.
// This header forwards to the canonical model in flamewm/core/display.h
#include "../core/display.h"
namespace flamewm { namespace integrations {
using DisplayManager = ::flamewm::DisplayManager;
using DisplaySnapshot = ::flamewm::DisplaySnapshot;
using OutputInfo = ::flamewm::OutputInfo;
using DisplayMode = ::flamewm::DisplayMode;
using ResolutionTransaction = ::flamewm::ResolutionTransaction;
}} // namespace
#endif
