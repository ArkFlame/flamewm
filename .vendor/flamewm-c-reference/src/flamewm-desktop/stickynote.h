#ifndef FLAMEWM_DESKTOP_STICKYNOTE_H
#define FLAMEWM_DESKTOP_STICKYNOTE_H

#include <string>
#include <vector>

namespace flamewm { namespace desktop {

struct StickyColor { unsigned char red, green, blue; StickyColor():red(255),green(230),blue(120){} StickyColor(unsigned char r,unsigned char g,unsigned char b):red(r),green(g),blue(b){} };
struct StickyRect { int x,y,w,h; StickyRect():x(0),y(0),w(240),h(180){} StickyRect(int X,int Y,int W,int H):x(X),y(Y),w(W),h(H){} };
struct StickyNote {
    std::string id;
    int workspace;
    std::string output;
    StickyRect rect;
    std::string textUtf8;
    StickyColor background;
    StickyColor foreground;
    int textSizeLogical;
    StickyNote():workspace(0),background(),foreground(0,0,0),textSizeLogical(14){}
};

class StickyNoteStore {
public:
    explicit StickyNoteStore(const std::string& path = std::string());
    bool load(std::string* error);
    bool save(std::string* error) const;
    const std::vector<StickyNote>& notes() const { return notes_; }
    std::vector<StickyNote>& notes() { return notes_; }
    StickyNote* create(const std::string& output, int workspace);
    bool remove(const std::string& id);
    void requestDisable();
    void cancelDisable();
    bool confirmDisable(std::string* error);
    void disable(std::string* error);
    bool disablePending() const { return disablePending_; }
    bool enabled() const { return enabled_; }
    void setEnabled(bool enabled, std::string* error);
    void shiftWorkspaces(int first, int delta);
    void removeWorkspace(int workspace, int destination);
    static std::string defaultPath();
    bool saveNote(const StickyNote& note, std::string* error) const { (void)note; return save(error); }
private:
    std::string path_;
    std::vector<StickyNote> notes_;
    bool enabled_;
    bool disablePending_;
};

} }
#endif
