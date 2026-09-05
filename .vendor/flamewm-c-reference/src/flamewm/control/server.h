#ifndef FLAMEWM_CONTROL_SERVER_H
#define FLAMEWM_CONTROL_SERVER_H

namespace flamewm {
namespace platform {
class PlatformHost;
}
namespace control {

class ControlServer {
public:
    explicit ControlServer(platform::PlatformHost* host);
    ~ControlServer();

    ControlServer(const ControlServer&) = delete;
    ControlServer& operator=(const ControlServer&) = delete;

    // Owns bus name com.arkflame.FlameWM1, registers object, methods.
    // Returns false on failure (bus unavailable, name taken, already running).
    bool start();

    void stop();

    bool isRunning() const;

private:
    struct Impl;
    Impl* impl_;
};

} // namespace control
} // namespace flamewm

#endif // FLAMEWM_CONTROL_SERVER_H
