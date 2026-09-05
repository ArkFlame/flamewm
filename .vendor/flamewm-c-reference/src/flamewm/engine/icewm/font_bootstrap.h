#ifndef FLAMEWM_ENGINE_ICEWM_FONT_BOOTSTRAP_H
#define FLAMEWM_ENGINE_ICEWM_FONT_BOOTSTRAP_H

namespace flamewm {
namespace engine {
namespace icewm {

class FontBootstrap {
public:
    // Register bundled product fonts with the process Fontconfig database.
    // Safe to call repeatedly; vanilla builds do nothing.
    static void initialize();

private:
    FontBootstrap() = delete;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm

#endif // FLAMEWM_ENGINE_ICEWM_FONT_BOOTSTRAP_H
