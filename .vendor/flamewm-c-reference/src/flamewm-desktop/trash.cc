#include "trash.h"
#include "model.h"
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#include <dirent.h>
#include <errno.h>
#include <string.h>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <sstream>
#include <ctime>
#include <vector>

namespace flamewm {
namespace desktop {

std::string TrashService::homeTrashDir(){
    std::string base = DesktopDirResolver::xdgDataHome();
    return base + "/Trash";
}
std::string TrashService::homeTrashFiles(){ return homeTrashDir()+"/files"; }
std::string TrashService::homeTrashInfo(){ return homeTrashDir()+"/info"; }

std::string TrashService::trashInfoContent(const std::string& originalPath){
    char tbuf[64];
    time_t now=time(0);
    struct tm tmv; gmtime_r(&now,&tmv);
    strftime(tbuf,sizeof(tbuf),"%Y-%m-%dT%H:%M:%S",&tmv);
    std::ostringstream oss;
    oss<<"[Trash Info]\nPath="<<originalPath<<"\nDeletionDate="<<tbuf<<"\n";
    return oss.str();
}
bool TrashService::isInTrash(const std::string& path){
    std::string tf=homeTrashFiles();
    return path.compare(0,tf.size(),tf)==0;
}
bool TrashService::ensureTrashDirs(const std::string& filesDir,const std::string& infoDir,std::string* error){
    struct stat st;
    size_t slash=filesDir.rfind('/');
    std::string parent=slash==std::string::npos?std::string():filesDir.substr(0,slash);
    if(!parent.empty() && stat(parent.c_str(),&st)!=0){
        if(mkdir(parent.c_str(),0700)!=0 && errno!=EEXIST){ if(error)*error="mkdir trash"; return false; }
    }
    if(stat(filesDir.c_str(),&st)!=0){ if(mkdir(filesDir.c_str(),0700)!=0 && errno!=EEXIST){ if(error)*error="mkdir files"; return false; } }
    if(stat(infoDir.c_str(),&st)!=0){ if(mkdir(infoDir.c_str(),0700)!=0 && errno!=EEXIST){ if(error)*error="mkdir info"; return false; } }
    return true;
}

static std::string trashTopdir(const std::string& path, dev_t device){
    std::string current=path;
    size_t slash=current.rfind('/');
    current=slash==std::string::npos?std::string("."):(slash==0?std::string("/"):current.substr(0,slash));
    std::string top=current;
    while(!current.empty()){
        struct stat here;
        if(stat(current.c_str(),&here)!=0 || here.st_dev!=device) break;
        top=current;
        if(current=="/") break;
        slash=current.rfind('/');
        if(slash==std::string::npos) break;
        current=slash==0?std::string("/"):current.substr(0,slash);
    }
    return top;
}

static bool runRemove(const std::string& path){
    struct stat st;
    if(lstat(path.c_str(),&st)!=0) return false;
    if(!S_ISDIR(st.st_mode)) return unlink(path.c_str())==0;
    pid_t pid=fork();
    if(pid==0){ execl("/bin/rm","/bin/rm","-rf",path.c_str(),(char*)0); _exit(127); }
    if(pid<0) return false;
    int status=0;
    return waitpid(pid,&status,0)>=0 && WIFEXITED(status) && WEXITSTATUS(status)==0;
}

static bool copyAcrossFilesystems(const std::string& source,const std::string& destination){
    pid_t pid=fork();
    if(pid==0){ execl("/bin/cp","/bin/cp","-a","--",source.c_str(),destination.c_str(),(char*)0); _exit(127); }
    if(pid<0) return false;
    int status=0;
    if(waitpid(pid,&status,0)<0 || !WIFEXITED(status) || WEXITSTATUS(status)!=0) return false;
    return runRemove(source);
}
std::string TrashService::uniqueTrashName(const std::string& base,const std::string& trashFilesDir){
    std::string cand=base;
    struct stat st;
    std::string full=trashFilesDir+"/"+cand;
    if(stat(full.c_str(),&st)!=0) return cand;
    for(int i=2;i<10000;++i){
        char buf[32]; snprintf(buf,sizeof(buf),".%d",i);
        cand=base+buf;
        full=trashFilesDir+"/"+cand;
        if(stat(full.c_str(),&st)!=0) return cand;
    }
    return cand;
}

TrashService::TrashResult TrashService::trash(const std::string& path,std::string* error){
    TrashResult r; r.ok=false;
    if(isInTrash(path)){ if(error) *error="already in trash"; r.error="already in trash"; return r; }
    struct stat st;
    if(lstat(path.c_str(),&st)!=0){ if(error) *error="not found"; r.error="not found"; return r; }
    std::string filesDir=homeTrashFiles();
    std::string infoDir=homeTrashInfo();
    if(st.st_dev!=0){
        struct stat homeSt;
        if(stat(homeTrashDir().c_str(),&homeSt)==0 && homeSt.st_dev!=st.st_dev){
            std::string top=trashTopdir(path,st.st_dev);
            uid_t uid=getuid();
            std::ostringstream local;
            struct stat sharedTrash;
            std::string candidate;
            if(stat((top+"/.Trash").c_str(),&sharedTrash)==0 && S_ISDIR(sharedTrash.st_mode) && access((top+"/.Trash").c_str(),W_OK|X_OK)==0){
                local<<top<<"/.Trash/"<<uid;
                candidate=local.str();
                if(mkdir(candidate.c_str(),0700)!=0 && errno!=EEXIST) candidate.clear();
            }
            if(candidate.empty()){
                local.str(std::string());
                local.clear();
                local<<top<<"/.Trash-"<<uid;
                candidate=local.str();
            }
            if(access(candidate.c_str(),W_OK|X_OK)==0 || mkdir(candidate.c_str(),0700)==0){
                filesDir=candidate+"/files";
                infoDir=candidate+"/info";
            }
        }
    }
    std::string err;
    if(!ensureTrashDirs(filesDir,infoDir,&err)){ if(error)*error=err; r.error=err; return r; }
    // basename
    size_t slash=path.rfind('/'); std::string base=(slash==std::string::npos? path: path.substr(slash+1));
    std::string tname=uniqueTrashName(base, filesDir);
    std::string destFiles=filesDir+"/"+tname;
    std::string destInfo=infoDir+"/"+tname+".trashinfo";
    // write info first
    std::string infoContent=trashInfoContent(path);
    std::string tmpInfo=destInfo+".tmp";
    std::ofstream f(tmpInfo.c_str(),std::ios::trunc);
    if(!f){ if(error)*error="cannot write trashinfo"; r.error="cannot write trashinfo"; return r; }
    f<<infoContent; f.close();
    if(rename(tmpInfo.c_str(), destInfo.c_str())!=0){ if(error)*error="rename trashinfo"; r.error="rename trashinfo"; unlink(tmpInfo.c_str()); return r; }
    bool moved=rename(path.c_str(), destFiles.c_str())==0;
    if(!moved && errno==EXDEV) moved=copyAcrossFilesystems(path,destFiles);
    if(!moved){
        unlink(destInfo.c_str());
        if(error) *error=strerror(errno);
        r.error=strerror(errno);
        return r;
    }
    r.ok=true; r.trashedPath=destFiles; return r;
}

TrashService::EmptyResult TrashService::emptyTrash(std::string* error){
    EmptyResult res; res.removed=0;
    std::string filesDir=homeTrashFiles();
    std::string infoDir=homeTrashInfo();
    DIR* d=opendir(filesDir.c_str());
    if(!d){ if(error) *error="no trash files"; return res; }
    std::vector<std::string> names;
    struct dirent* e;
    while((e=readdir(d))!=0){ std::string n(e->d_name); if(n=="."||n=="..") continue; names.push_back(n); }
    closedir(d);
    for(size_t i=0;i<names.size();++i){
        std::string fp=filesDir+"/"+names[i];
        std::string ip=infoDir+"/"+names[i]+".trashinfo";
        // remove file/dir recursively via system rm -rf argv? use unlink/rmdir recursion simple
        struct stat st;
        if(lstat(fp.c_str(),&st)!=0){ res.failures.push_back(fp+": stat failed"); continue; }
        int rc=-1;
        if(S_ISDIR(st.st_mode)){
            // recursive: use safe argv via /bin/rm -rf single arg (exec vector)
            pid_t pid=fork();
            if(pid==0){ execl("/bin/rm","/bin/rm","-rf",fp.c_str(),(char*)0); _exit(127); }
            else if(pid>0){ int status=0; waitpid(pid,&status,0); rc=(WIFEXITED(status)&&WEXITSTATUS(status)==0?0:-1); }
        } else {
            rc=unlink(fp.c_str());
        }
        if(rc!=0){ res.failures.push_back(fp+": remove failed"); continue; }
        unlink(ip.c_str()); // best effort
        res.removed++;
    }
    (void)error;
    return res;
}

} // namespace desktop
} // namespace flamewm
