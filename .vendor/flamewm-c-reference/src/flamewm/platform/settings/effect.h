#ifndef FLAMEWM_PLATFORM_SETTINGS_EFFECT_H
#define FLAMEWM_PLATFORM_SETTINGS_EFFECT_H

#include "flamewm/api/errors.h"
#include "flamewm/api/settings.h"

#include <string>
#include <vector>

namespace flamewm {
namespace platform {
namespace settings {

// Reversible participant for Settings transaction (C4).
// Lifecycle owned externally; SettingsService holds non-owning pointer.
// Contract:
//   prepare(candidate, changedKeys) -> stage resources, validate subset;
//     return non-ok to abort transaction.
//   applyPrepared() -> activate staged state on legal owner thread;
//     return non-ok to abort (service will rollback).
//   rollbackPrepared() -> discard staged state, restore old (idempotent).
//   commitPrepared() -> free old resources after persistence+publish.
// Service order: stale check -> validate -> prepare(all) -> apply(all)
//   -> persist atomic -> publish -> commit(all) -> notify.
// Any failure before publish: rollback all prepared, no persist, no rev bump.
class SettingsEffect {
public:
    virtual ~SettingsEffect() {}

    virtual api::Status prepare(const api::SettingsSnapshot& candidate,
                                const std::vector<std::string>& changedKeys) = 0;

    virtual api::Status applyPrepared() = 0;

    virtual void rollbackPrepared() = 0;

    virtual void commitPrepared() = 0;
};

} // namespace settings
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_SETTINGS_EFFECT_H
