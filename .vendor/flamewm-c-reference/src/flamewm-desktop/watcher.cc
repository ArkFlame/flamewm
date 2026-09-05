#include "watcher.h"
#include <unistd.h>
#include <sys/inotify.h>
#include <errno.h>
#include <string.h>
#include <cstdio>

namespace flamewm {
namespace desktop {

DesktopWatcher::DesktopWatcher(const std::string& dir): dir_(dir), fd_(-1), wd_(-1), listener_(0){}
DesktopWatcher::~DesktopWatcher(){ stop(); }

bool DesktopWatcher::start(std::string* error){
    fd_ = inotify_init1(IN_NONBLOCK | IN_CLOEXEC);
    if(fd_<0){ if(error) *error=strerror(errno); return false; }
    unsigned mask = IN_CREATE|IN_DELETE|IN_MOVED_FROM|IN_MOVED_TO|IN_CLOSE_WRITE|IN_ATTRIB|IN_DELETE_SELF|IN_MOVE_SELF;
    wd_ = inotify_add_watch(fd_, dir_.c_str(), mask);
    if(wd_<0){ if(error) *error=strerror(errno); close(fd_); fd_=-1; return false; }
    return true;
}
void DesktopWatcher::stop(){
    if(wd_>=0 && fd_>=0) inotify_rm_watch(fd_, wd_);
    wd_=-1;
    if(fd_>=0) close(fd_);
    fd_=-1;
}
bool DesktopWatcher::reAdd(std::string* error){
    if(fd_<0) return start(error);
    unsigned mask = IN_CREATE|IN_DELETE|IN_MOVED_FROM|IN_MOVED_TO|IN_CLOSE_WRITE|IN_ATTRIB|IN_DELETE_SELF|IN_MOVE_SELF;
    wd_ = inotify_add_watch(fd_, dir_.c_str(), mask);
    if(wd_<0){ if(error) *error=strerror(errno); return false; }
    return true;
}

std::vector<DesktopWatcher::Event> DesktopWatcher::coalesceAndPair(const std::vector<Event>& raw){
    // Pair MOVED_FROM/TO by cookie; preserve order; deduplicate same-name creates etc not needed.
    // Burst coalescing: collapse consecutive identical create/delete for same name to last occurrence.
    // For now: return raw with pairing metadata intact; pairing handled by consumer via cookie.
    // Simple coalesce: last event per name wins within burst for non-move types.
    std::map<std::string, Event> last;
    std::vector<Event> out;
    std::map<uint32_t, std::string> fromByCookie;
    for(size_t i=0;i<raw.size();++i){
        const Event& e=raw[i];
        if(e.type==EvMoveFrom){ fromByCookie[e.cookie]=e.name; out.push_back(e); }
        else if(e.type==EvMoveTo){ out.push_back(e); }
        else if(e.type==EvOverflow || e.type==EvInvalidated){ out.push_back(e); }
        else {
            // coalesce: keep last per name
            last[e.name]=e;
        }
    }
    for(std::map<std::string,Event>::iterator it=last.begin(); it!=last.end(); ++it) out.push_back(it->second);
    return out;
}

bool DesktopWatcher::handleEvents(){
    if(fd_<0) return false;
    char buf[8192];
    ssize_t len = read(fd_, buf, sizeof(buf));
    if(len<0){
        if(errno==EAGAIN||errno==EINTR) return true;
        return false;
    }
    if(len==0) return true;
    std::vector<Event> raw;
    size_t off=0;
    while(off + sizeof(struct inotify_event) <= (size_t)len){
        struct inotify_event* ev = (struct inotify_event*)(buf+off);
        if(ev->mask & IN_Q_OVERFLOW){
            raw.push_back(Event()); raw.back().type=EvOverflow;
        } else if(ev->mask & (IN_IGNORED|IN_DELETE_SELF|IN_MOVE_SELF)){
            Event e; e.type=EvInvalidated; e.name=""; e.cookie=ev->cookie; raw.push_back(e);
        } else {
            std::string name = ev->len ? std::string(ev->name) : std::string();
            if(ev->mask & IN_CREATE){ Event e; e.type=EvCreate; e.name=name; e.cookie=ev->cookie; raw.push_back(e); }
            if(ev->mask & IN_DELETE){ Event e; e.type=EvDelete; e.name=name; e.cookie=ev->cookie; raw.push_back(e); }
            if(ev->mask & IN_MOVED_FROM){ Event e; e.type=EvMoveFrom; e.name=name; e.cookie=ev->cookie; raw.push_back(e); }
            if(ev->mask & IN_MOVED_TO){ Event e; e.type=EvMoveTo; e.name=name; e.cookie=ev->cookie; raw.push_back(e); }
            if(ev->mask & (IN_CLOSE_WRITE|IN_ATTRIB|IN_MODIFY)){ Event e; e.type=EvModify; e.name=name; e.cookie=ev->cookie; raw.push_back(e); }
        }
        off += sizeof(struct inotify_event) + ev->len;
    }
    // Check overflow
    for(size_t i=0;i<raw.size();++i) if(raw[i].type==EvOverflow){
        if(listener_) listener_->onOverflowNeedsRescan();
        // full rescan requested; still dispatch others? dispatch burst after recovery.
        dispatch(coalesceAndPair(raw));
        return true;
    }
    bool invalid=false;
    for(size_t i=0;i<raw.size();++i) if(raw[i].type==EvInvalidated) invalid=true;
    std::vector<Event> cooked = coalesceAndPair(raw);
    dispatch(cooked);
    if(invalid){
        std::string err;
        reAdd(&err);
        if(listener_) listener_->onWatchInvalidated();
    }
    return true;
}
void DesktopWatcher::dispatch(const std::vector<Event>& ev){
    if(listener_ && !ev.empty()) listener_->onWatcherEvents(ev);
}

} // namespace desktop
} // namespace flamewm
