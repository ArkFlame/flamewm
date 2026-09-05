#include "config.h"

#include "ylib.h"
#define CFGDEF

#include "yprefs.h"

// FW-CONFIG-01: yprefs values for Flame-owned keys are sourced from
// Runtime::effectiveSettings() when initialized, not direct file re-reads.
// See WMConfig::applyRuntimeEffective() and Runtime::defaultConfigPath().

// vim: set sw=4 ts=4 et:
