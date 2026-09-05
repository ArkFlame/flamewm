#ifndef WMCONFIG_H
#define WMCONFIG_H

struct cfoption;
#include "mstring.h"

class WMConfig {
public:
    static void loadConfiguration(const char* fileName);
    static bool loadThemeConfiguration();
    static void freeConfiguration();
    static void setDefault(const char *basename, mstring config);
    static void setDefaultFocus(long focusMode);
    static void setDefaultTheme(mstring themeName);
    static void printPrefs(long focus, cfoption* startup);
    static void print_options(cfoption* options);
    static int rewritePrefs(cfoption* startup, const char* config);
    // FW-CONFIG-01: apply Flame effective snapshot to live IceWM prefs where owned.
    static void applyRuntimeEffective();
    static const char* flameConfigFileName() { return "flame.conf"; }
};

// functions that are used in preferences options:

void addWorkspace(const char *name, const char *value, bool append);
void addKeyboard(const char *name, const char *value, bool append);
void setLook(const char *name, const char *value, bool append);

extern class YStringArray configWorkspaces;
extern class MStringArray configKeyboards;

#endif

// vim: set sw=4 ts=4 et:
