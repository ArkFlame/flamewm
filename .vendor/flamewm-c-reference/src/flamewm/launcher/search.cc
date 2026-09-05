#include "search.h"
#include <cctype>

namespace flamewm {
namespace launcher {

std::string SearchHelper::toLower(const std::string& s) {
    std::string out;
    out.reserve(s.size());
    for (size_t i = 0; i < s.size(); ++i) out.push_back((char)tolower((unsigned char)s[i]));
    return out;
}

bool SearchHelper::matches(const SearchEntry& e, const std::string& query) {
    if (query.empty()) return true;
    std::string q = toLower(query);
    std::string nl = toLower(e.name);
    if (nl.find(q) != std::string::npos) return true;
    std::string el = toLower(e.exec);
    if (el.find(q) != std::string::npos) return true;
    std::string kl = toLower(e.keywords);
    if (kl.find(q) != std::string::npos) return true;
    // Also match id
    std::string il = toLower(e.id);
    if (il.find(q) != std::string::npos) return true;
    return false;
}

std::vector<SearchEntry> SearchHelper::filter(const std::vector<SearchEntry>& entries,
                                              const std::string& query) {
    if (query.empty()) return entries;
    std::vector<SearchEntry> out;
    out.reserve(entries.size());
    for (size_t i = 0; i < entries.size(); ++i) {
        if (matches(entries[i], query)) out.push_back(entries[i]);
    }
    return out;
}

std::string SearchHelper::sanitizeExecForDisplay(const std::string& exec) {
    // Remove field codes %F %U %f %u %D %N %i %c %k etc for display.
    std::string out;
    out.reserve(exec.size());
    for (size_t i = 0; i < exec.size(); ++i) {
        if (exec[i] == '%' && i + 1 < exec.size()) {
            char c = exec[i+1];
            // Valid field codes per FDO spec: %f %F %u %U %d %D %n %N %i %c %k %v %m
            if (c=='f'||c=='F'||c=='u'||c=='U'||c=='d'||c=='D'||c=='n'||c=='N'
                ||c=='i'||c=='c'||c=='k'||c=='v'||c=='m'||c=='%') {
                ++i; // skip code
                continue;
            }
        }
        out.push_back(exec[i]);
    }
    // trim double spaces
    std::string trimmed;
    trimmed.reserve(out.size());
    bool lastSpace = false;
    for (size_t i = 0; i < out.size(); ++i) {
        bool isSpace = (out[i]==' '||out[i]=='\t');
        if (isSpace && lastSpace) continue;
        trimmed.push_back(out[i]);
        lastSpace = isSpace;
    }
    // trim leading/trailing
    size_t s = 0; while (s < trimmed.size() && trimmed[s]==' ') ++s;
    size_t e = trimmed.size(); while (e > s && trimmed[e-1]==' ') --e;
    return trimmed.substr(s, e-s);
}

bool SearchHelper::containsShellMeta(const std::string& s) {
    for (size_t i = 0; i < s.size(); ++i) {
        char c = s[i];
        if (c=='`'||c=='$'||c=='|'||c=='&'||c==';'||c=='<'||c=='>'
            ||c=='('||c==')'||c=='{'||c=='}'||c=='!'||c=='\\'||c=='\n'||c=='\r')
            return true;
    }
    return false;
}

} // namespace launcher
} // namespace flamewm
