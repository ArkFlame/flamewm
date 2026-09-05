#ifndef FLAMEWM_CORE_VERSION_H
#define FLAMEWM_CORE_VERSION_H

namespace flamewm {

#define FLAMEWM_VERSION "0.0.5"
#define ICEWM_BASE_VERSION "4.1.0"
#define FLAMEWM_MILESTONE "V5 addendum over V4"

inline const char* flameVersion() { return FLAMEWM_VERSION; }
inline const char* iceBaseVersion() { return ICEWM_BASE_VERSION; }
inline const char* milestone() { return FLAMEWM_MILESTONE; }

} // namespace flamewm
#endif
