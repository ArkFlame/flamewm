#include "desktopapp.h"
#include <cstdio>
#include <cstdlib>
#include <unistd.h>
#include <signal.h>

using flamewm::desktop::DesktopApp;

char const *ApplicationName = "flamewm-desktop";

static volatile bool gQuit=false;
static void onSig(int){ gQuit=true; }

int main(int argc,char** argv){
    (void)argc; (void)argv;
    signal(SIGTERM, onSig);
    signal(SIGINT, onSig);
    DesktopApp app;
    std::string err;
    if(!app.init(&err)){
        fprintf(stderr,"flamewm-desktop: init failed: %s\n", err.c_str());
        // bounded supervision: backoff then exit, session supervisor restarts
        DesktopApp::BackoffState bo;
        if(bo.shouldGiveUp()) return 1;
        int d=bo.nextDelayMs();
        usleep(d*1000);
        return 1;
    }
    // Advertise desktop window before map inside app
    app.createDesktopWindow(&err);
    while(!gQuit && app.isRunning()){
        app.processEvents(500);
    }
    app.shutdown();
    return 0;
}
