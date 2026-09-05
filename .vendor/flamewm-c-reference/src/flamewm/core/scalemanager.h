#ifndef FLAMEWM_CORE_SCALEMANAGER_H
#define FLAMEWM_CORE_SCALEMANAGER_H

#include "types.h"
#include "../ui/metrics.h"
#include <map>
#include <vector>

namespace flamewm {

class ScaleListener {
public:
    virtual ~ScaleListener() {}
    virtual void onScaleChanged(const OutputId& output, int newScalePct) = 0;
};

class ScaleManager {
public:
    ScaleManager();
    ~ScaleManager();

    // Supported buckets 100/125/150/175/200
    bool setScale(const OutputId& output, int pct);
    int scaleFor(const OutputId& output) const; // default 100 if unknown
    int scaleForConnector(const std::string& connector) const;

    // Logical <-> physical
    int toPhysical(int logical, const OutputId& output) const;
    int toLogical(int physical, const OutputId& output) const;

    // Durable identity helpers
    static OutputId makeOutputId(const std::string& connector, const std::string& edidHash);
    static std::string durableKey(const OutputId& id);

    // Notifications
    void addListener(ScaleListener* l);
    void removeListener(ScaleListener* l);

    // Validate bucket
    static bool isValidScale(int pct) { return isSupportedScale(pct); }

    // Cached scaled font/icon keys by role+scale+themeGeneration (bounded)
    int cachedPhysicalSize(int logical, int scalePct, int themeGeneration);
    void setThemeGeneration(int gen);
    int themeGeneration() const { return themeGeneration_; }
    void retireOldGenerations(); // keep last 3 generations
    size_t cacheSize() const { return cache_.size(); }

private:
    std::map<std::string, int> scales_; // durableId/connector -> pct
    std::vector<ScaleListener*> listeners_;

    // generation-aware cache: key = "logical:scale:gen" -> physical
    std::map<std::string, int> cache_;
    int themeGeneration_;
    static const size_t kMaxGenerationsKept = 3;
};

} // namespace flamewm
#endif
