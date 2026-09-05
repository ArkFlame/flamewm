#include "model.h"
#include <cstdlib>
#include <cstdio>
#include <fstream>
#include <sstream>
#include <sys/stat.h>
#include <dirent.h>
#include <unistd.h>

namespace flamewm {
namespace desktop {

std::string DesktopDirResolver::homeDir() {
    const char* h = getenv("HOME");
    if (h && h[0]) return std::string(h);
    return std::string("/tmp");
}
std::string DesktopDirResolver::xdgDataHome() {
    const char* v = getenv("XDG_DATA_HOME");
    if (v && v[0]) return std::string(v);
    return homeDir() + "/.local/share";
}

std::string DesktopDirResolver::expandHomeVars(const std::string& raw, const std::string& home) {
    std::string out;
    out.reserve(raw.size());
    for (size_t i = 0; i < raw.size(); ) {
        if (raw.compare(i, 5, "$HOME") == 0) {
            out += home; i += 5;
        } else if (raw.compare(i, 7, "${HOME}") == 0) {
            out += home; i += 7;
        } else {
            out += raw[i]; ++i;
        }
    }
    return out;
}

std::string DesktopDirResolver::parseUserDirsContent(const std::string& content,
                                                     const std::string& home) {
    std::istringstream iss(content);
    std::string line;
    while (std::getline(iss, line)) {
        // trim leading ws
        size_t s = 0;
        while (s < line.size() && (line[s]==' '||line[s]=='\t')) ++s;
        if (s >= line.size() || line[s]=='#') continue;
        const std::string key = "XDG_DESKTOP_DIR";
        if (line.compare(s, key.size(), key) != 0) continue;
        size_t eq = line.find('=', s);
        if (eq == std::string::npos) continue;
        std::string val = line.substr(eq+1);
        // trim
        size_t a=0; while(a<val.size()&&(val[a]==' '||val[a]=='\t')) ++a;
        size_t b=val.size(); while(b>a&&(val[b-1]==' '||val[b-1]=='\t'||val[b-1]=='\r'||val[b-1]=='\n')) --b;
        val = val.substr(a,b-a);
        // strip quotes
        if (val.size()>=2 && val.front()=='"' && val.back()=='"') val=val.substr(1,val.size()-2);
        else if (val.size()>=2 && val.front()=='\'' && val.back()=='\'') val=val.substr(1,val.size()-2);
        std::string expanded = expandHomeVars(val, home);
        if (!expanded.empty()) return expanded;
    }
    return std::string();
}

std::string DesktopDirResolver::resolve(std::string* error) {
    std::string home = homeDir();
    std::string configHome;
    const char* xdg = getenv("XDG_CONFIG_HOME");
    if (xdg && xdg[0]) configHome = std::string(xdg);
    else configHome = home + "/.config";
    std::string userDirs = configHome + "/user-dirs.dirs";
    std::ifstream f(userDirs.c_str());
    if (f) {
        std::string content((std::istreambuf_iterator<char>(f)), std::istreambuf_iterator<char>());
        std::string parsed = parseUserDirsContent(content, home);
        if (!parsed.empty()) return parsed;
    }
    // fallback
    (void)error;
    return home + "/Desktop";
}

// DesktopModel
DesktopModel::DesktopModel(): desktopPath_(DesktopDirResolver::resolve(0)) {}
DesktopModel::DesktopModel(const std::string& p): desktopPath_(p) {}

ItemKind DesktopModel::kindForPath(const std::string& path) {
    struct stat st;
    if (lstat(path.c_str(), &st) != 0) return KindRegularFile;
    if (S_ISLNK(st.st_mode)) return KindSymlink;
    if (S_ISDIR(st.st_mode)) return KindDirectory;
    // .desktop -> launcher
    if (path.size() >= 8 && path.compare(path.size()-8,8,".desktop")==0) return KindDesktopLauncher;
    return KindRegularFile;
}

void DesktopModel::rebuildCellMap() {
    cellMap_.clear();
    for (size_t i=0;i<items_.size();++i) {
        const DesktopItem& it = items_[i];
        if (it.cellCol>=0 && it.cellRow>=0) {
            cellMap_[it.path] = CellPos(it.outputIdentity, it.cellCol, it.cellRow);
        }
    }
}

bool DesktopModel::rescan(std::string* error) {
    DIR* d = opendir(desktopPath_.c_str());
    if (!d) { if(error) *error="cannot open desktop dir"; return false; }
    std::map<std::string, DesktopItem> byPath;
    for (size_t i=0;i<items_.size();++i) byPath[items_[i].path]=items_[i];
    std::vector<DesktopItem> next;
    next.reserve(items_.size()+8);
    struct dirent* e;
    while ((e=readdir(d))!=0) {
        std::string nm(e->d_name);
        if (nm=="."||nm=="..") continue;
        // skip hidden layout file
        if (nm==".flamewm-desktop.layout") continue;
        std::string full = desktopPath_ + "/" + nm;
        DesktopItem it;
        std::map<std::string,DesktopItem>::iterator it2 = byPath.find(full);
        if (it2!=byPath.end()) {
            it = it2->second;
            it.name = nm;
            it.kind = kindForPath(full);
        } else {
            it.name = nm;
            it.path = full;
            it.kind = kindForPath(full);
            // assign cell from cellMap if present else -1 (will be assigned via firstFree)
            std::map<std::string,CellPos>::iterator cm = cellMap_.find(full);
            if (cm!=cellMap_.end()) { it.cellCol=cm->second.col; it.cellRow=cm->second.row; it.outputIdentity=cm->second.outputIdentity; }
        }
        next.push_back(it);
    }
    closedir(d);
    // preserve trash pseudo items
    for (size_t i=0;i<items_.size();++i) if(items_[i].kind==KindTrashPseudoItem) next.push_back(items_[i]);
    items_=next;
    rebuildCellMap();
    return true;
}

DesktopItem* DesktopModel::findByPath(const std::string& path) {
    for(size_t i=0;i<items_.size();++i) if(items_[i].path==path) return &items_[i];
    return 0;
}
const DesktopItem* DesktopModel::findByPath(const std::string& path) const {
    for(size_t i=0;i<items_.size();++i) if(items_[i].path==path) return &items_[i];
    return 0;
}
DesktopItem* DesktopModel::findByName(const std::string& name) {
    for(size_t i=0;i<items_.size();++i) if(items_[i].name==name) return &items_[i];
    return 0;
}
bool DesktopModel::handleRename(const std::string& oldPath, const std::string& newPath) {
    std::map<std::string,CellPos>::iterator it = cellMap_.find(oldPath);
    CellPos pos;
    bool had = false;
    if (it!=cellMap_.end()) { pos=it->second; had=true; cellMap_.erase(it); }
    DesktopItem* item = findByPath(oldPath);
    if (item) {
        item->path = newPath;
        size_t slash = newPath.rfind('/');
        item->name = (slash==std::string::npos? newPath : newPath.substr(slash+1));
        if (had) { item->cellCol=pos.col; item->cellRow=pos.row; item->outputIdentity=pos.outputIdentity; cellMap_[newPath]=pos; }
        return true;
    }
    if (had) cellMap_[newPath]=pos;
    return had;
}

bool DesktopModel::loadLayout(const std::string& layoutFile, std::string* error) {
    std::ifstream f(layoutFile.c_str());
    if (!f) return true; // no file yet -> ok
    std::string line;
    while (std::getline(f,line)) {
        if(line.empty()||line[0]=='#') continue;
        // format: outputIdentity|col|row|path
        size_t p1=line.find('|'); if(p1==std::string::npos) continue;
        size_t p2=line.find('|',p1+1); if(p2==std::string::npos) continue;
        size_t p3=line.find('|',p2+1); if(p3==std::string::npos) continue;
        std::string out=line.substr(0,p1);
        int c=atoi(line.substr(p1+1,p2-p1-1).c_str());
        int r=atoi(line.substr(p2+1,p3-p2-1).c_str());
        std::string path=line.substr(p3+1);
        cellMap_[path]=CellPos(out,c,r);
    }
    // apply to items
    for(size_t i=0;i<items_.size();++i){
        std::map<std::string,CellPos>::iterator it=cellMap_.find(items_[i].path);
        if(it!=cellMap_.end()){ items_[i].outputIdentity=it->second.outputIdentity; items_[i].cellCol=it->second.col; items_[i].cellRow=it->second.row; }
    }
    (void)error; return true;
}
bool DesktopModel::saveLayout(const std::string& layoutFile, std::string* error) const {
    std::string tmp = layoutFile + ".tmp";
    std::ofstream f(tmp.c_str(), std::ios::trunc);
    if(!f){ if(error) *error="cannot write layout tmp"; return false; }
    for(std::map<std::string,CellPos>::const_iterator it=cellMap_.begin(); it!=cellMap_.end(); ++it){
        f<<it->second.outputIdentity<<"|"<<it->second.col<<"|"<<it->second.row<<"|"<<it->first<<"\n";
    }
    f.close(); if(!f){ if(error) *error="write failed"; return false; }
    if(rename(tmp.c_str(), layoutFile.c_str())!=0){ if(error) *error="rename failed"; return false; }
    return true;
}
void DesktopModel::setCell(const std::string& path, const CellPos& pos){
    cellMap_[path]=pos;
    DesktopItem* it=findByPath(path); if(it){ it->outputIdentity=pos.outputIdentity; it->cellCol=pos.col; it->cellRow=pos.row; }
}
bool DesktopModel::getCell(const std::string& path, CellPos* out) const {
    std::map<std::string,CellPos>::const_iterator it=cellMap_.find(path);
    if(it==cellMap_.end()) return false;
    if (out) *out = it->second;
    return true;
}
bool DesktopModel::isOccupied(int col,int row,const std::string& output) const {
    for(std::map<std::string,CellPos>::const_iterator it=cellMap_.begin(); it!=cellMap_.end(); ++it){
        if(it->second.outputIdentity==output && it->second.col==col && it->second.row==row) return true;
    }
    return false;
}
CellPos DesktopModel::firstFreeCell(const std::string& outputIdentity,int cols,int rows) const {
    for(int r=0;r<rows;++r) for(int c=0;c<cols;++c) if(!isOccupied(c,r,outputIdentity)) return CellPos(outputIdentity,c,r);
    return CellPos(outputIdentity,0,0);
}
void DesktopModel::reflowAfterTopologyChange(const std::string& primaryOutput,int cols,int rows) {
    // retain valid cells; reflow invalid to first-free on primary
    std::map<std::string,bool> valid;
    for(std::map<std::string,CellPos>::iterator it=cellMap_.begin(); it!=cellMap_.end(); ++it){
        CellPos& p=it->second;
        bool ok = p.outputIdentity==primaryOutput && p.col>=0 && p.row>=0 && p.col<cols && p.row<rows;
        valid[it->first]=ok;
        if(!ok){ CellPos np = firstFreeCell(primaryOutput,cols,rows); p=np; // occupy
                 // update cellMap occupancy by inserting, next isOccupied will see it
        }
    }
    for(size_t i=0;i<items_.size();++i){
        std::map<std::string,CellPos>::iterator it=cellMap_.find(items_[i].path);
        if(it!=cellMap_.end()){ items_[i].outputIdentity=it->second.outputIdentity; items_[i].cellCol=it->second.col; items_[i].cellRow=it->second.row; }
    }
    (void)valid;
}

} // namespace desktop
} // namespace flamewm
