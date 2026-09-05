#include "fileactions.h"
#include <sys/stat.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>
#include <cstdio>
#include <cstdlib>

namespace flamewm {
namespace desktop {

std::string FileActions::nextFolderName(const std::string& desktopDir){
    std::string base="New Folder";
    struct stat st;
    std::string cand=desktopDir+"/"+base;
    if(lstat(cand.c_str(),&st)!=0) return base;
    for(int i=2;i<10000;++i){
        char buf[32]; snprintf(buf,sizeof(buf)," %d",i);
        std::string n=base+buf;
        cand=desktopDir+"/"+n;
        if(lstat(cand.c_str(),&st)!=0) return n;
    }
    return base;
}
std::string FileActions::createNewFolder(const std::string& desktopDir,std::string* error){
    std::string name=nextFolderName(desktopDir);
    std::string full=desktopDir+"/"+name;
    if(mkdir(full.c_str(),0755)!=0){ if(error) *error=strerror(errno); return std::string(); }
    return full;
}

std::vector<std::string> FileActions::buildTerminalArgv(const std::string& desktopDir){
    const char* term=getenv("TERMINAL");
    if(!term||!term[0]) term="xterm";
    // Prefer working dir via --working-directory when xterm? Use cwd via chdir in child instead.
    // Return minimal argv; caller will chdir before exec.
    std::vector<std::string> v; v.push_back(term); (void)desktopDir; return v;
}
bool FileActions::openTerminal(const std::string& desktopDir,std::string* error){
    std::vector<std::string> argv=buildTerminalArgv(desktopDir);
    pid_t pid=fork();
    if(pid<0){ if(error) *error=strerror(errno); return false; }
    if(pid==0){
        if(!desktopDir.empty() && chdir(desktopDir.c_str()) != 0) _exit(126);
        // build C argv
        std::vector<char*> cargv;
        for(size_t i=0;i<argv.size();++i) cargv.push_back(const_cast<char*>(argv[i].c_str()));
        cargv.push_back(0);
        execvp(cargv[0], cargv.data());
        _exit(127);
    }
    (void)error; return true;
}
bool FileActions::openPath(const std::string& path,std::string* error){
    pid_t pid=fork();
    if(pid<0){ if(error)*error=strerror(errno); return false; }
    if(pid==0){ execlp("xdg-open","xdg-open",path.c_str(),(char*)0); _exit(127); }
    return true;
}

std::vector<std::string> FileActions::buildSettingsArgv(){
    std::vector<std::string> v; v.push_back("flamewm-settings"); v.push_back("--page"); v.push_back("desktop"); return v;
}
bool FileActions::openDesktopAndWallpaper(std::string* error){
    std::vector<std::string> argv=buildSettingsArgv();
    pid_t pid=fork();
    if(pid<0){ if(error) *error=strerror(errno); return false; }
    if(pid==0){
        std::vector<char*> cargv;
        for(size_t i=0;i<argv.size();++i) cargv.push_back(const_cast<char*>(argv[i].c_str()));
        cargv.push_back(0);
        execvp(cargv[0], cargv.data());
        _exit(127);
    }
    (void)error; return true;
}

} // namespace desktop
} // namespace flamewm
