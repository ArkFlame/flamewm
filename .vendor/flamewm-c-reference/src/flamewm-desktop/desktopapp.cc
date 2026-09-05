#include "desktopapp.h"
#include "layout.h"
#include "fileactions.h"
#include "watermark.h"
#include "render/material_x11.h"
#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>
#include <map>
#include <unistd.h>
#include <poll.h>
#include <algorithm>
#include <errno.h>
#include <cstdlib>

#include <X11/Xlib.h>
#include <X11/Xatom.h>
#include <X11/keysym.h>
#include <X11/Xutil.h>
#ifdef Status
#undef Status
#endif

namespace flamewm {
namespace desktop {

int DesktopApp::BackoffState::nextDelayMs(){
    if(shouldGiveUp()) return maxMs;
    int d = baseMs * (1 << attempts);
    if(d > maxMs) d = maxMs;
    ++attempts;
    return d;
}

DesktopApp::DesktopApp(): watcher_(0), running_(false)
    , display_(0), window_(0), menuWindow_(0), gc_(0), menuGc_(0), menuX_(0), menuY_(0), draggingSelection_(false), draggingNote_(false), resizingNote_(false), editingNote_(false), dragX_(0), dragY_(0), activeNote_(0), lastClickTime_(0), control_(0)
{
    layoutFile_ = std::string(getenv("HOME")?getenv("HOME"):"/tmp") + "/.config/flamewm/desktop.layout";
}
DesktopApp::~DesktopApp(){ shutdown(); }

bool DesktopApp::init(std::string* error){
    std::string desktopPath = DesktopDirResolver::resolve(error);
    model_.setDesktopPath(desktopPath);
    model_.loadLayout(layoutFile_, error);
    model_.rescan(error);
    if(!stickyNotes_.load(error)) return false;
    view_.setModel(&model_);
    view_.setWorkArea(WorkArea(0,0,1,1));
    updateWorkArea();
    GridConfig g = view_.grid();
    for(size_t i=0;i<model_.items().size();++i){
        DesktopItem& it=model_.items()[i];
        if(it.cellCol<0){
            CellPos p = model_.firstFreeCell("primary", g.cols, g.rows);
            model_.setCell(it.path, p);
        }
    }
    model_.saveLayout(layoutFile_, 0);
    watcher_ = new DesktopWatcher(desktopPath);
    watcher_->setListener(this);
    if(!watcher_->start(error)){
        // watcher failure not fatal: keep running without live updates
    }
    running_=true;
    backoff_.reset();
    return true;
}
void DesktopApp::shutdown(){
    running_=false;
    if(watcher_){ watcher_->stop(); delete watcher_; watcher_=0; }
    if(display_){
        hideBlankContextMenu();
        if(gc_) XFreeGC(display_,reinterpret_cast<GC>(gc_));
        if(window_) XDestroyWindow(display_, window_);
        XCloseDisplay(display_);
        display_=0; window_=0;
    }
}
bool DesktopApp::createStickyNote(const std::string& output,int workspace,std::string* error){
    if(!stickyNotes_.enabled()){ if(error)*error="sticky notes disabled"; return false; }
    if(!stickyNotes_.create(output,workspace)){ if(error)*error="cannot create sticky note"; return false; }
    StickyNote* n=stickyNotes_.notes().empty()?0:&stickyNotes_.notes().back();
    if(n){ n->rect.x=view_.workArea().x+40; n->rect.y=view_.workArea().y+40; clampNote(n); }
    redraw();
    return stickyNotes_.save(error);
}
bool DesktopApp::deleteStickyNote(const std::string& id, std::string* error){
    if(!stickyNotes_.remove(id)){ if(error) *error="sticky note not found"; return false; }
    if(activeNote_ && activeNote_->id==id){ activeNote_=0; draggingNote_=resizingNote_=editingNote_=false; }
    redraw();
    return stickyNotes_.save(error);
}
bool DesktopApp::updateStickyNoteText(const std::string& id, const std::string& text, std::string* error){
    for(size_t i=0;i<stickyNotes_.notes().size();++i) if(stickyNotes_.notes()[i].id==id){
        stickyNotes_.notes()[i].textUtf8=text;
        redraw();
        return stickyNotes_.save(error);
    }
    if(error) *error="sticky note not found"; return false;
}
bool DesktopApp::updateStickyNoteGeometry(const std::string& id, const StickyRect& rect, std::string* error){
    for(size_t i=0;i<stickyNotes_.notes().size();++i) if(stickyNotes_.notes()[i].id==id){
        stickyNotes_.notes()[i].rect=rect; clampNote(&stickyNotes_.notes()[i]);
        redraw();
        return stickyNotes_.save(error);
    }
    if(error) *error="sticky note not found"; return false;
}
bool DesktopApp::updateStickyNoteSettings(const std::string& id, const StickyColor& bg, const StickyColor& fg, int textSize, std::string* error){
    for(size_t i=0;i<stickyNotes_.notes().size();++i) if(stickyNotes_.notes()[i].id==id){
        stickyNotes_.notes()[i].background=bg; stickyNotes_.notes()[i].foreground=fg;
        if(textSize>=10&&textSize<=32) stickyNotes_.notes()[i].textSizeLogical=textSize;
        redraw();
        return stickyNotes_.save(error);
    }
    if(error) *error="sticky note not found"; return false;
}
void DesktopApp::onWorkspaceSnapshotChanged(const flamewm::api::WorkspaceSnapshot& snap){
    if(!flamewm::api::isValid(snap)) return;
    bool changed=false;
    for(size_t i=0;i<stickyNotes_.notes().size();++i){
        int ws=stickyNotes_.notes()[i].workspace;
        if(ws<0){ stickyNotes_.notes()[i].workspace=0; changed=true; }
        else if(ws>=snap.count){ stickyNotes_.notes()[i].workspace=snap.count-1; changed=true; }
    }
    if(changed) stickyNotes_.save(0);
    redraw();
}

bool DesktopApp::controlConnected() const {
    return control_ && control_->isConnected();
}
bool DesktopApp::requestAddWorkspace(std::string* error){
    if(!control_){ if(error) *error="control not configured"; return false; }
    if(!control_->isConnected() && !control_->connect()){
        if(error) *error="control not connected";
        return false;
    }
    flamewm::api::Result<flamewm::api::WorkspaceSnapshot> snap = control_->getWorkspaceSnapshot();
    if(!snap.ok()){
        if(error) *error=snap.status().message;
        return false;
    }
    int after = snap.value().count - 1;
    uint64_t rev = snap.value().revision;
    flamewm::api::WorkspaceTransform tr = flamewm::api::WorkspaceTransform::InsertAfter(after, rev);
    ::flamewm::api::Status st = control_->applyWorkspaceTransform(tr);
    if(!st.ok()){ if(error) *error=st.message; return false; }
    return true;
}
bool DesktopApp::requestActivateWorkspace(int index, std::string* error){
    if(!control_){ if(error) *error="control not configured"; return false; }
    if(!control_->isConnected() && !control_->connect()){
        if(error) *error="control not connected";
        return false;
    }
    flamewm::api::Result<flamewm::api::WorkspaceSnapshot> snap = control_->getWorkspaceSnapshot();
    if(!snap.ok()){
        if(error) *error=snap.status().message;
        return false;
    }
    ::flamewm::api::Status st = control_->activateWorkspace(index, snap.value().revision);
    if(!st.ok()){ if(error) *error=st.message; return false; }
    return true;
}
bool DesktopApp::openDesktopSettings(std::string* error){
    // Settings UI is a separate process (flamewm-settings --page desktop).
    // Keep argv launch intact (fileactions, no shell); control presence only verifies
    // WM is reachable. No separate D-Bus verb for opening settings UI.
    if(control_){
        if(!control_->isConnected()) control_->connect();
    }
    return FileActions::openDesktopAndWallpaper(error);
}
bool DesktopApp::executeDesktopAction(const std::string& action, std::string* error){
    if(action=="Add Virtual Desktop") return requestAddWorkspace(error);
    if(action=="Desktop and Wallpaper") return openDesktopSettings(error);
    if(action=="Open Terminal") return FileActions::openTerminal(model_.desktopPath(), error);
    if(action=="Create New Folder"){
        std::string created = FileActions::createNewFolder(model_.desktopPath(), error);
        if(created.empty()) return false;
        // assign first free cell and persist
        std::string err2;
        model_.rescan(&err2);
        GridConfig g = view_.grid();
        CellPos p = model_.firstFreeCell("primary", g.cols, g.rows);
        model_.setCell(created, p);
        model_.saveLayout(layoutFile_, 0);
        redraw();
        return true;
    }
    if(action=="New Sticky Note"){
        return createStickyNote("primary", 0, error);
    }
    return DesktopView::executeBlankContextAction(action, model_.desktopPath(), error);
}

bool DesktopApp::processEvents(int timeoutMs){
    if(!running_) return false;
    struct pollfd fds[2]; nfds_t count=0;
    if(display_){ fds[count].fd=ConnectionNumber(display_); fds[count].events=POLLIN; ++count; }
    if(watcher_ && watcher_->isStarted()){ fds[count].fd=watcher_->fd(); fds[count].events=POLLIN; ++count; }
    if(count==0){
        int rc=poll(0,0,timeoutMs);
        if(rc<0 && errno!=EINTR){ running_=false; return false; }
        return running_;
    }
    int rc=poll(fds,count,timeoutMs);
    if(rc<0){ if(errno==EINTR) return running_; running_=false; return false; }
    nfds_t i=0;
    if(display_){
        if(fds[i].revents&(POLLIN|POLLERR|POLLHUP)){
            while(XPending(display_)){ XEvent event; XNextEvent(display_, &event); handleXEvent(&event); }
            updateWorkArea();
        }
        ++i;
    }
    if(watcher_ && watcher_->isStarted() && i<count && (fds[i].revents&POLLIN)) watcher_->handleEvents();
    return running_;
}

void DesktopApp::onWatcherEvents(const std::vector<DesktopWatcher::Event>& ev){
    handleMovePairing(ev);
    std::string err;
    model_.rescan(&err);
    model_.saveLayout(layoutFile_, 0);
    redraw();
}
void DesktopApp::handleMovePairing(const std::vector<DesktopWatcher::Event>& ev){
    std::map<uint32_t, std::string> fromByCookie;
    for(size_t i=0;i<ev.size();++i) if(ev[i].type==DesktopWatcher::EvMoveFrom) fromByCookie[ev[i].cookie]=ev[i].name;
    for(size_t i=0;i<ev.size();++i) if(ev[i].type==DesktopWatcher::EvMoveTo){
        std::map<uint32_t,std::string>::iterator it=fromByCookie.find(ev[i].cookie);
        if(it!=fromByCookie.end()){
            std::string oldPath=model_.desktopPath()+"/"+it->second;
            std::string newPath=model_.desktopPath()+"/"+ev[i].name;
            model_.handleRename(oldPath,newPath);
        }
    }
}
void DesktopApp::onOverflowNeedsRescan(){
    std::string err; model_.rescan(&err); model_.saveLayout(layoutFile_,0);
    redraw();
}
void DesktopApp::onWatchInvalidated(){
    std::string err; model_.rescan(&err);
    redraw();
}

bool DesktopApp::createDesktopWindow(std::string* error){
    if(display_) return true;
    display_ = XOpenDisplay(0);
    if(!display_){ if(error) *error="no X display"; return false; }
    Atom wmType = XInternAtom(display_, "_NET_WM_WINDOW_TYPE", False);
    Atom desktopType = XInternAtom(display_, "_NET_WM_WINDOW_TYPE_DESKTOP", False);
    Atom state = XInternAtom(display_, "_NET_WM_STATE", False);
    Atom skipTaskbar = XInternAtom(display_, "_NET_WM_STATE_SKIP_TASKBAR", False);
    Atom skipPager = XInternAtom(display_, "_NET_WM_STATE_SKIP_PAGER", False);
    Atom sticky = XInternAtom(display_, "_NET_WM_STATE_STICKY", False);
    Atom desktop = XInternAtom(display_, "_NET_WM_DESKTOP", False);
    Window root = DefaultRootWindow(display_);
    XWindowAttributes rootAttrs;
    XGetWindowAttributes(display_, root, &rootAttrs);
    XSetWindowAttributes attrs; attrs.override_redirect=False; attrs.background_pixmap=ParentRelative;
    window_ = XCreateWindow(display_, root, 0,0, rootAttrs.width,rootAttrs.height, 0, CopyFromParent, InputOutput, CopyFromParent, CWBackPixmap, &attrs);
    // Advertise _NET_WM_WINDOW_TYPE_DESKTOP before map
    XChangeProperty(display_, window_, wmType, XA_ATOM, 32, PropModeReplace, (unsigned char*)&desktopType, 1);
    Atom states[3]={skipTaskbar, skipPager, sticky};
    XChangeProperty(display_, window_, state, XA_ATOM, 32, PropModeReplace, (unsigned char*)states, 3);
    unsigned long allDesktops=0xffffffffUL;
    XChangeProperty(display_, window_, desktop, XA_CARDINAL, 32, PropModeReplace, (unsigned char*)&allDesktops, 1);
    XSelectInput(display_, root, PropertyChangeMask|StructureNotifyMask);
    XSelectInput(display_, window_, ExposureMask|StructureNotifyMask|ButtonPressMask|ButtonReleaseMask|PointerMotionMask|KeyPressMask);
    gc_=reinterpret_cast<unsigned long>(XCreateGC(display_,window_,0,0));
    XMapWindow(display_, window_);
    XFlush(display_);
    updateWorkArea();
    redraw();
    (void)error; return true;
}

void DesktopApp::updateWorkArea(){
    if(!display_) return;
    Window root=DefaultRootWindow(display_);
    XWindowAttributes a;
    if(!XGetWindowAttributes(display_,root,&a)) return;
    WorkArea wa(0,0,a.width,a.height);
    Atom prop=XInternAtom(display_,"_NET_WORKAREA",False), actual;
    int format=0; unsigned long count=0, remaining=0; unsigned char* data=0;
    if(XGetWindowProperty(display_,root,prop,0,4,False,XA_CARDINAL,&actual,&format,&count,&remaining,&data)==Success && data && format==32 && count>=4){
        unsigned long* p=(unsigned long*)data;
        wa=WorkArea((int)p[0],(int)p[1],(int)p[2],(int)p[3]);
    }
    if(data) XFree(data);
    view_.setWorkArea(wa);
    for(size_t i=0;i<stickyNotes_.notes().size();++i) clampNote(&stickyNotes_.notes()[i]);
    redraw();
}

StickyNote* DesktopApp::noteAt(int x,int y){
    for(std::vector<StickyNote>::reverse_iterator i=stickyNotes_.notes().rbegin();i!=stickyNotes_.notes().rend();++i)
        if(x>=i->rect.x&&y>=i->rect.y&&x<i->rect.x+i->rect.w&&y<i->rect.y+i->rect.h) return &*i;
    return 0;
}
void DesktopApp::clampNote(StickyNote* n){
    if (!n) return;
    const WorkArea& w = view_.workArea();
    if (n->rect.w > w.w) n->rect.w = w.w;
    if (n->rect.h > w.h) n->rect.h = w.h;
    n->rect.x=std::max(w.x,std::min(n->rect.x,w.x+w.w-n->rect.w));
    n->rect.y=std::max(w.y,std::min(n->rect.y,w.y+w.h-n->rect.h));
}
void DesktopApp::redraw(){
    if(!display_||!window_||!gc_)return;
    // ParentRelative keeps wallpaper owned by the wallpaper component while
    // clearing removes pixels from the previous frame before drawing again.
    XClearArea(display_, window_, 0, 0, 0, 0, False);
    std::set<std::string> live;
    for(size_t i=0;i<model_.items().size();++i) live.insert(model_.items()[i].path);
    std::set<std::string> selected;
    for(std::set<std::string>::const_iterator i=selection_.selected().begin();
        i!=selection_.selected().end();++i)
        if(live.find(*i)!=live.end()) selected.insert(*i);
    selection_.setSelected(selected);
    for(size_t i=0;i<model_.items().size();++i)
        model_.items()[i].selected=selection_.isSelected(model_.items()[i].path);
    Watermark::draw(display_, window_, (void*)gc_, view_.workArea(), false, true);
    view_.render(display_,window_,(void*)gc_);
    if(selection_.hasRubber()){
        ItemRect r=selection_.rubber();
        MaterialX11::fill(display_, window_, (GC)gc_, r.x, r.y, r.w, r.h,
                          MaterialX11::blend(0xff5533UL, 0x111111UL, 20));
        XSetForeground(display_,(GC)gc_,
                       MaterialX11::blend(0xff5533UL, 0xe6e6e6UL, 60));
        XDrawRectangle(display_,window_,(GC)gc_,r.x,r.y,r.w,r.h);
    }
    for(size_t i=0;i<stickyNotes_.notes().size();++i){
        const StickyNote& n=stickyNotes_.notes()[i];
        XSetForeground(display_,(GC)gc_,(n.background.red<<16)|(n.background.green<<8)|n.background.blue);
        XFillRectangle(display_,window_,(GC)gc_,n.rect.x,n.rect.y,n.rect.w,n.rect.h);
        XSetForeground(display_,(GC)gc_,(n.foreground.red<<16)|(n.foreground.green<<8)|n.foreground.blue);
        int y=n.rect.y+20; size_t start=0;
        for(size_t p=0;p<=n.textUtf8.size();++p) if(p==n.textUtf8.size()||n.textUtf8[p]=='\n'){
            XDrawString(display_,window_,(GC)gc_,n.rect.x+8,y,n.textUtf8.substr(start,p-start).c_str(),(int)(p-start)); y+=n.textSizeLogical+3; start=p+1;
        }
        XDrawRectangle(display_,window_,(GC)gc_,n.rect.x,n.rect.y,n.rect.w,n.rect.h);
    }
    if (menuWindow_) drawBlankContextMenu();
    XFlush(display_);
}
void DesktopApp::showBlankContextMenu(int x, int y){
    hideBlankContextMenu();
    menuItems_ = DesktopView::blankContextMenuItems(stickyNotes_.enabled());
    if (menuItems_.empty()) return;
    const int width = 240;
    const int height = static_cast<int>(menuItems_.size()) * 32;
    XWindowAttributes rootAttributes;
    Window root = DefaultRootWindow(display_);
    XGetWindowAttributes(display_, root, &rootAttributes);
    menuX_ = std::max(0, std::min(x, rootAttributes.width - width));
    menuY_ = std::max(0, std::min(y, rootAttributes.height - height));
    XSetWindowAttributes attributes;
    attributes.override_redirect = True;
    menuWindow_ = XCreateWindow(display_, root, menuX_, menuY_, width, height,
                                1, CopyFromParent, InputOutput, CopyFromParent,
                                CWOverrideRedirect, &attributes);
    menuGc_ = reinterpret_cast<unsigned long>(XCreateGC(display_, menuWindow_, 0, 0));
    XSelectInput(display_, menuWindow_, ExposureMask | ButtonPressMask |
                 ButtonReleaseMask | PointerMotionMask);
    XMapRaised(display_, menuWindow_);
    XGrabPointer(display_, menuWindow_, True, ButtonPressMask | ButtonReleaseMask |
                 PointerMotionMask, GrabModeAsync, GrabModeAsync, None, None,
                 CurrentTime);
    drawBlankContextMenu();
}
void DesktopApp::hideBlankContextMenu(){
    if (!display_ || !menuWindow_) return;
    XUngrabPointer(display_, CurrentTime);
    if (menuGc_) XFreeGC(display_, reinterpret_cast<GC>(menuGc_));
    XDestroyWindow(display_, menuWindow_);
    menuWindow_ = 0;
    menuGc_ = 0;
    menuItems_.clear();
}
void DesktopApp::drawBlankContextMenu(){
    if (!display_ || !menuWindow_ || !menuGc_) return;
    GC gc = reinterpret_cast<GC>(menuGc_);
    XSetForeground(display_, gc, 0x171717UL);
    XFillRectangle(display_, menuWindow_, gc, 0, 0, 240,
                   static_cast<unsigned>(menuItems_.size() * 32));
    XSetForeground(display_, gc, 0xff5533UL);
    XDrawRectangle(display_, menuWindow_, gc, 0, 0, 239,
                   static_cast<unsigned>(menuItems_.size() * 32 - 1));
    XSetForeground(display_, gc, 0xe6e6e6UL);
    for (size_t i = 0; i < menuItems_.size(); ++i)
        XDrawString(display_, menuWindow_, gc, 16, static_cast<int>(i * 32 + 21),
                    menuItems_[i].c_str(), static_cast<int>(menuItems_[i].size()));
}
void DesktopApp::handleXEvent(const void* raw){
    const XEvent& e=*static_cast<const XEvent*>(raw); int x=0,y=0;
    if (menuWindow_ && e.xany.window == menuWindow_) {
        if (e.type == Expose) drawBlankContextMenu();
        else if (e.type == ButtonRelease && e.xbutton.button == Button1) {
            const int row = e.xbutton.y / 32;
            if (row >= 0 && row < static_cast<int>(menuItems_.size())) {
                std::string error;
                executeDesktopAction(menuItems_[static_cast<size_t>(row)], &error);
            }
            hideBlankContextMenu();
            redraw();
        } else if (e.type == ButtonPress &&
                   (e.xbutton.x < 0 || e.xbutton.x >= 240 ||
                    e.xbutton.y < 0 ||
                    e.xbutton.y >= static_cast<int>(menuItems_.size() * 32))) {
            hideBlankContextMenu();
            redraw();
        }
        return;
    }
    if(e.type==Expose){ if(e.xexpose.count==0) redraw(); return; }
    if(e.type==ConfigureNotify){ updateWorkArea(); return; }
    if(e.type==ButtonPress){
        x=e.xbutton.x; y=e.xbutton.y; activeNote_=noteAt(x,y);
        if(e.xbutton.button==Button3){
            if (!activeNote_) showBlankContextMenu(e.xbutton.x_root, e.xbutton.y_root);
            return;
        }
        if(activeNote_){ editingNote_=true; draggingNote_=true; dragX_=x-activeNote_->rect.x; dragY_=y-activeNote_->rect.y; resizingNote_=(x>activeNote_->rect.x+activeNote_->rect.w-12&&y>activeNote_->rect.y+activeNote_->rect.h-12); XSetInputFocus(display_,window_,RevertToParent,CurrentTime); }
        else { std::map<std::string,ItemRect> rs=view_.buildItemRects(); std::string hit; for(std::map<std::string,ItemRect>::const_iterator i=rs.begin();i!=rs.end();++i)if(i->second.contains(x,y))hit=i->first;
            if(!hit.empty()){ DesktopItem* item=model_.findByPath(hit); if(item&&!selection_.isSelected(hit)){ selection_.clear(); selection_.select(hit); }
                if(hit==lastClickPath_ && e.xbutton.time-lastClickTime_<400){ std::string err; FileActions::openPath(hit,&err); lastClickPath_.clear(); }
                else { lastClickPath_=hit; lastClickTime_=e.xbutton.time; } dragPath_=hit; dragX_=x;dragY_=y; }
            else { selection_.clear(); selection_.setRubberRect(ItemRect{x,y,1,1}); draggingSelection_=true;dragX_=x;dragY_=y; }
        } redraw(); return;
    }
    if(e.type==MotionNotify){ x=e.xmotion.x;y=e.xmotion.y;
        if(draggingSelection_){ ItemRect r; r.x=std::min(dragX_,x);r.y=std::min(dragY_,y);r.w=std::abs(x-dragX_);r.h=std::abs(y-dragY_);selection_.setRubberRect(r);selection_.setSelected(selection_.hitTest(view_.buildItemRects())); }
        else if(draggingNote_&&activeNote_){ if(resizingNote_){activeNote_->rect.w=std::max(80,x-activeNote_->rect.x);activeNote_->rect.h=std::max(60,y-activeNote_->rect.y);}else{activeNote_->rect.x=x-dragX_;activeNote_->rect.y=y-dragY_;} clampNote(activeNote_); }
        redraw(); return;
    }
    if(e.type==ButtonRelease){
        if(!draggingNote_ && !draggingSelection_ && !dragPath_.empty() && std::abs(e.xbutton.x-dragX_)>8){
            int dc=0, dr=0; LayoutEngine::pixelToCell(view_.grid(),e.xbutton.x,e.xbutton.y,&dc,&dr);
            int sc=0, sr=0; LayoutEngine::pixelToCell(view_.grid(),dragX_,dragY_,&sc,&sr); dc-=sc; dr-=sr;
            const std::set<std::string>& chosen=selection_.selected();
            std::set<std::string> effective = chosen;
            if(effective.empty() && !dragPath_.empty()) effective.insert(dragPath_);
            std::vector<LayoutEngine::DragItem> sel;
            sel.reserve(effective.size());
            for(std::set<std::string>::const_iterator p=effective.begin();p!=effective.end();++p){
                DesktopItem* it=model_.findByPath(*p);
                if(it && it->cellCol>=0 && it->cellRow>=0){ LayoutEngine::DragItem d; d.path=*p; d.col=it->cellCol; d.row=it->cellRow; sel.push_back(d); }
            }
            std::set<std::pair<int,int> > unselected;
            const std::vector<DesktopItem>& all=model_.items();
            for(size_t i=0;i<all.size();++i) if(effective.find(all[i].path)==effective.end() && all[i].cellCol>=0)
                unselected.insert(std::make_pair(all[i].cellCol, all[i].cellRow));
            std::map<std::string,std::pair<int,int> > out;
            std::string terr;
            if(!sel.empty() && LayoutEngine::groupDragTransaction(sel, unselected, dc, dr, view_.grid(), &out, &terr)){
                for(std::map<std::string,std::pair<int,int> >::iterator it=out.begin(); it!=out.end(); ++it){
                    CellPos pos("primary", it->second.first, it->second.second);
                    // preserve outputIdentity if already tracked
                    CellPos cur; if(model_.getCell(it->first,&cur)) pos.outputIdentity=cur.outputIdentity;
                    model_.setCell(it->first, pos);
                }
                model_.saveLayout(layoutFile_,0);
            }
        }
        if(draggingNote_&&activeNote_) stickyNotes_.save(0);
        draggingSelection_=draggingNote_=resizingNote_=false; selection_.clearRubber(); dragPath_.clear(); redraw(); return;
    }
    if(e.type==KeyPress&&editingNote_&&activeNote_){ char buf[64]; KeySym key; int n=XLookupString(const_cast<XKeyEvent*>(&e.xkey),buf,sizeof(buf),&key,0);
        if(key==XK_Escape) editingNote_=false; else if(key==XK_BackSpace&&!activeNote_->textUtf8.empty())activeNote_->textUtf8.erase(activeNote_->textUtf8.size()-1); else if(key==XK_Return)activeNote_->textUtf8.push_back('\n'); else if(n>0)activeNote_->textUtf8.append(buf,n); stickyNotes_.save(0); redraw(); }
}

} // namespace desktop
} // namespace flamewm
