#include "scalemanager.h"

namespace flamewm {

ScaleManager::ScaleManager() : themeGeneration_(0) {}
ScaleManager::~ScaleManager() {}

bool ScaleManager::setScale(const OutputId& output, int pct) {
    if (!isSupportedScale(pct)) return false;
    std::string key = durableKey(output);
    int old = scaleFor(output);
    scales_[key] = pct;
    if (old != pct) {
        for (size_t i = 0; i < listeners_.size(); ++i) {
            if (listeners_[i]) listeners_[i]->onScaleChanged(output, pct);
        }
    }
    return true;
}

int ScaleManager::scaleFor(const OutputId& output) const {
    std::string key = durableKey(output);
    std::map<std::string,int>::const_iterator it = scales_.find(key);
    if (it != scales_.end()) return it->second;
    // try connector fallback
    if (!output.connector.empty()) {
        it = scales_.find(output.connector);
        if (it != scales_.end()) return it->second;
    }
    return 100;
}

int ScaleManager::scaleForConnector(const std::string& connector) const {
    std::map<std::string,int>::const_iterator it = scales_.find(connector);
    return it != scales_.end() ? it->second : 100;
}

int ScaleManager::toPhysical(int logical, const OutputId& output) const {
    return FlameMetrics::logicalToPhysical(logical, scaleFor(output));
}
int ScaleManager::toLogical(int physical, const OutputId& output) const {
    return FlameMetrics::physicalToLogical(physical, scaleFor(output));
}

OutputId ScaleManager::makeOutputId(const std::string& connector, const std::string& edidHash) {
    OutputId id;
    id.connector = connector;
    id.edidHash = edidHash;
    if (!edidHash.empty()) id.durableId = edidHash + ":" + connector;
    else id.durableId = connector;
    return id;
}

std::string ScaleManager::durableKey(const OutputId& id) {
    if (!id.durableId.empty()) return id.durableId;
    if (!id.connector.empty()) return id.connector;
    return "";
}

void ScaleManager::addListener(ScaleListener* l) {
    if (!l) return;
    for (size_t i=0;i<listeners_.size();++i) if (listeners_[i]==l) return;
    listeners_.push_back(l);
}
void ScaleManager::removeListener(ScaleListener* l) {
    for (size_t i=0;i<listeners_.size();++i) if (listeners_[i]==l) { listeners_.erase(listeners_.begin()+i); break; }
}

int ScaleManager::cachedPhysicalSize(int logical, int scalePct, int gen) {
    if (!isSupportedScale(scalePct)) scalePct = nearestSupportedScale(scalePct);
    // key: logical:scale:gen
    char buf[64];
    // manual format to avoid printf dependency issues; simple string concat
    std::string key = std::to_string(logical) + ":" + std::to_string(scalePct) + ":" + std::to_string(gen);
    (void)buf;
    std::map<std::string,int>::iterator it = cache_.find(key);
    if (it != cache_.end()) return it->second;
    int phys = FlameMetrics::logicalToPhysical(logical, scalePct);
    cache_[key] = phys;
    return phys;
}

void ScaleManager::setThemeGeneration(int gen) {
    themeGeneration_ = gen;
    retireOldGenerations();
}

void ScaleManager::retireOldGenerations() {
    if (cache_.empty()) return;
    // Retain only entries for the last kMaxGenerationsKept generations (gen, gen-1, gen-2)
    // Parse generation from key suffix after last ':'
    std::map<std::string,int> kept;
    for (std::map<std::string,int>::iterator it = cache_.begin(); it != cache_.end(); ++it) {
        const std::string& k = it->first;
        size_t pos = k.rfind(':');
        if (pos == std::string::npos) continue;
        int gen = 0;
        // simple parse
        for (size_t i = pos+1; i < k.size(); ++i) {
            char c = k[i];
            if (c=='-' ) { gen = 0; break; } // negative not expected, drop
            if (c < '0' || c > '9') break;
            gen = gen * 10 + (c - '0');
        }
        if (gen >= themeGeneration_ - (int)kMaxGenerationsKept + 1 && gen <= themeGeneration_) {
            kept[k] = it->second;
        }
    }
    // Only prune if we actually have newer generations to keep; otherwise keep all
    if (!kept.empty() || themeGeneration_ > 10) {
        // If kept is empty because entries are all old, keep none (full retirement)
        // but avoid wiping when still early generations: if themeGeneration < 3 keep all
        if (themeGeneration_ < (int)kMaxGenerationsKept) return;
        cache_.swap(kept);
    }
}

} // namespace flamewm
