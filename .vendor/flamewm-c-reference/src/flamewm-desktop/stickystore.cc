#include "stickynote.h"
#include <cstddef>
#include <string>
#include <vector>
#include <sys/stat.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>
#include <fstream>
#include <sstream>
#include <cstdlib>
#include <cstdio>
#include <algorithm>

namespace flamewm { namespace desktop {
static std::string hex(const std::string& s){ static const char h[]="0123456789abcdef"; std::string r; for(size_t i=0;i<s.size();++i){ r+=h[(unsigned char)s[i]>>4]; r+=h[(unsigned char)s[i]&15]; } return r; }
static std::string unhex(const std::string& s){ std::string r; if(s.size()%2) return r; for(size_t i=0;i<s.size();i+=2){ unsigned v=0; for(int j=0;j<2;++j){ char c=s[i+j]; v=v*16+(c>='0'&&c<='9'?c-'0':c>='a'&&c<='f'?c-'a'+10:c>='A'&&c<='F'?c-'A'+10:99); } if(v>255) return std::string(); r.push_back((char)v); } return r; }
static std::vector<std::string> split(const std::string& line){ std::vector<std::string> v; size_t p=0,n; while((n=line.find('\t',p))!=std::string::npos){ v.push_back(line.substr(p,n-p)); p=n+1; } v.push_back(line.substr(p)); return v; }
std::string StickyNoteStore::defaultPath(){ const char* h=getenv("XDG_DATA_HOME"); std::string base=h&&*h?h:std::string(getenv("HOME")?getenv("HOME"):"/tmp")+"/.local/share"; return base+"/flamewm/sticky-notes"; }
StickyNoteStore::StickyNoteStore(const std::string& path):path_(path.empty()?defaultPath():path),enabled_(true),disablePending_(false){}
 bool StickyNoteStore::load(std::string* error){ notes_.clear(); enabled_=true; disablePending_=false; std::ifstream f(path_.c_str()); if(!f){ if(errno==ENOENT) return true; if(error)*error="cannot read sticky notes"; return false; } std::string line; while(std::getline(f,line)){ if(line=="#disabled"){enabled_=false;continue;} std::vector<std::string> v=split(line); if(v.size()!=14&&v.size()!=15) continue; StickyNote n; n.id=unhex(v[0]); n.workspace=atoi(v[1].c_str()); n.output=unhex(v[2]); n.rect=StickyRect(atoi(v[3].c_str()),atoi(v[4].c_str()),atoi(v[5].c_str()),atoi(v[6].c_str())); n.textUtf8=unhex(v[7]); n.background=StickyColor((unsigned char)atoi(v[8].c_str()),(unsigned char)atoi(v[9].c_str()),(unsigned char)atoi(v[10].c_str())); n.foreground=StickyColor((unsigned char)atoi(v[11].c_str()),(unsigned char)atoi(v[12].c_str()),(unsigned char)atoi(v[13].c_str())); if(v.size()==15)n.textSizeLogical=std::max(10,std::min(32,atoi(v[14].c_str()))); notes_.push_back(n); } return true; }
 bool StickyNoteStore::save(std::string* error) const { std::string parent=path_.substr(0,path_.rfind('/')); std::string data=parent.substr(0,parent.rfind('/')); mkdir(data.c_str(),0700); mkdir(parent.c_str(),0700); std::string tmp=path_+".tmp"; std::ofstream f(tmp.c_str(),std::ios::trunc); if(!f){if(error)*error="cannot write sticky notes";return false;} if(!enabled_) f<<"#disabled\n"; for(size_t i=0;i<notes_.size();++i){const StickyNote& n=notes_[i]; f<<hex(n.id)<<'\t'<<n.workspace<<'\t'<<hex(n.output)<<'\t'<<n.rect.x<<'\t'<<n.rect.y<<'\t'<<n.rect.w<<'\t'<<n.rect.h<<'\t'<<hex(n.textUtf8)<<'\t'<<(int)n.background.red<<'\t'<<(int)n.background.green<<'\t'<<(int)n.background.blue<<'\t'<<(int)n.foreground.red<<'\t'<<(int)n.foreground.green<<'\t'<<(int)n.foreground.blue<<'\t'<<std::max(10,std::min(32,n.textSizeLogical))<<'\n';} f.close(); if(rename(tmp.c_str(),path_.c_str())!=0){if(error)*error=strerror(errno);unlink(tmp.c_str());return false;} return true; }
StickyNote* StickyNoteStore::create(const std::string& output,int workspace){ if(!enabled_) return 0; StickyNote n; std::ostringstream id; id<<getpid()<<'-'<<notes_.size(); n.id=id.str(); n.output=output; n.workspace=workspace; notes_.push_back(n); return &notes_.back(); }
bool StickyNoteStore::remove(const std::string& id){ for(std::vector<StickyNote>::iterator i=notes_.begin();i!=notes_.end();++i)if(i->id==id){notes_.erase(i);return true;}return false; }
void StickyNoteStore::requestDisable(){ disablePending_=true; }
void StickyNoteStore::cancelDisable(){ disablePending_=false; }
bool StickyNoteStore::confirmDisable(std::string* error){
    if(!disablePending_) return true;
    std::vector<StickyNote> oldNotes=notes_;
    bool oldEnabled=enabled_;
    notes_.clear(); enabled_=false; disablePending_=false;
    if(save(error)) return true;
    notes_=oldNotes; enabled_=oldEnabled; disablePending_=true;
    return false;
}
void StickyNoteStore::disable(std::string* error){ requestDisable(); (void)error; }
void StickyNoteStore::setEnabled(bool e,std::string* error){ if(!e) requestDisable(); else { disablePending_=false; enabled_=true; save(error); } }
void StickyNoteStore::shiftWorkspaces(int first,int delta){for(size_t i=0;i<notes_.size();++i)if(notes_[i].workspace>=first)notes_[i].workspace+=delta;}
void StickyNoteStore::removeWorkspace(int workspace,int destination){for(size_t i=0;i<notes_.size();++i){if(notes_[i].workspace==workspace)notes_[i].workspace=destination;else if(notes_[i].workspace>workspace)--notes_[i].workspace;}}
} }
