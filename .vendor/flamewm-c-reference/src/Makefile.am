
########
########    WARNING: DO NOT CHANGE THIS FILE WITHOUT RUNNING dist.sh SUCCESSFULLY
########    BEFORE PUSHING YOUR CHANGES!
########


AM_CPPFLAGS = -include ../config.h $(FLAMEWM_DBUS_CFLAGS) $(FLAMEWM_PULSE_CFLAGS)
AM_CXXFLAGS = $(CORE_CFLAGS) $(XSM_CFLAGS) $(IMAGE_CFLAGS) $(AUDIO_CFLAGS) $(GIO_CFLAGS) -DEXEEXT=$(EXEEXT)

EXTRA_DIST = \
	ypointer.h \
	globit.c

EXTRA_PROGRAMS = \
	icehelp \
	icesound \
	icewm-menu-fdo \
	testarray \
	testlocale \
	testmap \
	testmenus \
	testnetwmhints \
	testpointer \
	testwinhints \
	iceview \
	icesame \
	iceicon \
	icerun \
	icelist \
	strtest \
	iceskt \
	iceclock

noinst_LTLIBRARIES = \
	libice.la \
	libitk.la \
	libflamewm.la

# FLAMEWM-UPSTREAM-HOOK-BEGIN(FW-CORE-01)
libflamewm_la_SOURCES = \
	flamewm/ui/palette.cc \
	flamewm/ui/palette.h \
	flamewm/ui/metrics.cc \
	flamewm/ui/metrics.h \
	flamewm/ui/iconroles.h \
	flamewm/ui/iconresolver.cc \
	flamewm/ui/iconresolver.h \
	flamewm/ui/popoveranchor.cc \
	flamewm/ui/popoveranchor.h \
	flamewm/ui/assets/asset.cc \
	flamewm/ui/assets/asset.h \
	flamewm/ui/image.h \
	flamewm/ui/layout.cc \
	flamewm/ui/layout.h \
	flamewm/ui/menu/model.cc \
	flamewm/ui/menu/model.h \
	flamewm/ui/style/box.cc \
	flamewm/ui/style/box.h \
	flamewm/ui/style/theme.cc \
	flamewm/ui/style/theme.h \
	flamewm/ui/style/typography.cc \
	flamewm/ui/style/typography.h \
	flamewm/ui/style/visual.h \
	flamewm/ui/backend.cc \
	flamewm/ui/backend.h \
	flamewm/ui/textfield.h \
	flamewm/ui/list.h \
	flamewm/ui/popover.h \
	flamewm/ui/window.h \
	flamewm/ui/button.h \
	flamewm/ui/label.h \
	flamewm/ui/slider.h \
	flamewm/ui/dialog.h \
	flamewm/core/types.h \
	flamewm/core/scalemanager.cc \
	flamewm/core/scalemanager.h \
	flamewm/core/config.cc \
	flamewm/core/config.h \
	flamewm/core/shortcutregistry.cc \
	flamewm/core/shortcutregistry.h \
	flamewm/core/appidentity.cc \
	flamewm/core/appidentity.h \
	flamewm/panel/taskidentity.h \
	flamewm/snap/types.h \
	flamewm/snap/state.h \
	flamewm/snap/state.cc \
	flamewm/snap/controller.h \
	flamewm/snap/controller.cc \
	flamewm/snap/overlay.h \
	flamewm/snap/overlay.cc \
	flamewm/workspace/topology.cc \
	flamewm/workspace/topology.h \
	flamewm/workspace/transaction.h \
	flamewm/workspace/transaction.cc \
	flamewm/core/display.cc \
	flamewm/core/display.h \
	flamewm/core/productmetadata.cc \
	flamewm/core/productmetadata.h \
	flamewm/core/version.cc \
	flamewm/core/version.h \
	flamewm/core/runtime.cc \
	flamewm/core/runtime.h \
	flamewm/ui/shellstyle.cc \
	flamewm/ui/shellstyle.h \
	flamewm/panel/types.h \
	flamewm/panel/outputpanel.cc \
	flamewm/panel/outputpanel.h \
	flamewm/panel/panelmanager.cc \
	flamewm/panel/panelmanager.h \
	flamewm/panel/startcontroller.cc \
	flamewm/panel/startcontroller.h \
	flamewm/panel/composer.cc \
	flamewm/panel/composer.h \
	flamewm/launcher/search.cc \
	flamewm/launcher/search.h \
	flamewm/launcher/appmodel.cc \
	flamewm/launcher/appmodel.h \
	flamewm/integrations/dbusdispatcher.cc \
	flamewm/integrations/dbusdispatcher.h \
	flamewm/integrations/networkmanager.cc \
	flamewm/integrations/networkmanager.h \
	flamewm/integrations/mpris.cc \
	flamewm/integrations/mpris.h \
	flamewm/integrations/pulse.cc \
	flamewm/integrations/pulse.h \
	flamewm/panel/anchor.h \
	flamewm/panel/network.cc \
	flamewm/panel/network.h \
	flamewm/panel/media.cc \
	flamewm/panel/media.h \
	flamewm/panel/audio.cc \
	flamewm/panel/audio.h \
	flamewm/panel/clock.cc \
	flamewm/panel/clock.h \
	flamewm/panel/pinnedlauncher.cc \
	flamewm/panel/taskcontext.cc \
	flamewm/panel/taskstrip.cc
libflamewm_la_CXXFLAGS = $(AM_CXXFLAGS) -std=c++11 -Wall -Wextra
# FLAMEWM-UPSTREAM-HOOK-END(FW-CORE-01)

# FLAMEWM-BRIDGE-HOOK-BEGIN(C-BUILD-01)
include flamewm/Makefile.am.inc
# ReactorService link: dbusdispatcher.cc in libflamewm.la uses
# flamewm::platform::reactor::ReactorService — pull platform archive
# transitively so consumers linking only libflamewm.la resolve symbols.
libflamewm_la_LIBADD = libflamewm_platform.a $(FLAMEWM_DBUS_LIBS) $(FLAMEWM_PULSE_LIBS)
# Product binaries carry FLAMEWM_PRODUCT_BUILD; icewm stays vanilla
flamewm_bin_CPPFLAGS = $(AM_CPPFLAGS) -DFLAMEWM_PRODUCT_BUILD=1
flamewm_session_CPPFLAGS = $(AM_CPPFLAGS) -DFLAMEWM_PRODUCT_BUILD=1
# FLAMEWM-BRIDGE-HOOK-END(C-BUILD-01)

flamewm_settings_bin_SOURCES = \
	flamewm-settings/main.cc \
	flamewm-settings/settingsapp.cc \
	flamewm-settings/ui/common.cc \
	flamewm-settings/ui/pages.cc \
	flamewm-settings/ui/root.cc \
	flamewm-settings/settingsapp.h \
	flamewm-settings/pages/appearance.cc \
	flamewm-settings/pages/appearance.h \
	flamewm-settings/pages/desktop.cc \
	flamewm-settings/pages/desktop.h \
	flamewm-settings/pages/taskbar.cc \
	flamewm-settings/pages/displays.cc \
	flamewm-settings/pages/displays.h \
	flamewm-settings/pages/fonts.cc \
	flamewm-settings/pages/fonts.h \
	flamewm-settings/pages/hotkeys.cc \
	flamewm-settings/pages/hotkeys.h \
	flamewm-settings/pages/about.cc \
	flamewm-settings/pages/about.h \
	flamewm-settings/appname.cc \
	flamewm-settings/pages/taskbar.h \
	flamewm-settings/widgets/keycapture.h \
	flamewm-settings/widgets/colorpicker.h \
	flamewm-settings/widgets/previewcard.h \
	flamewm/engine/icewm/font_bootstrap.cc \
	flamewm/engine/icewm/font_bootstrap.h
flamewm_settings_bin_CPPFLAGS = $(AM_CPPFLAGS) -DFLAMEWM_PRODUCT_BUILD=1
flamewm_settings_bin_LDADD = libflamewm_engine_icewm.a libflamewm.la libflamewm_control_client.a libflamewm_control_common.a libitk.la libice.la $(FLAMEWM_DBUS_LIBS) $(FLAMEWM_PULSE_LIBS) $(IMAGE_LIBS) $(CORE_LIBS) @LIBINTL@ @LIBICONV@

flamewmctl_bin_SOURCES = flamewmctl/main.cc
flamewmctl_bin_CXXFLAGS = $(AM_CXXFLAGS) -std=c++11 -Wall -Wextra
flamewmctl_bin_LDADD = libflamewm_control_client.a libflamewm_control_common.a $(FLAMEWM_DBUS_LIBS)

bin_PROGRAMS = \
	icewm \
	icewm-session \
	flamewm-session \
	icesh \
	icehelp \
	icewmhint \
	icewmbg

noinst_PROGRAMS = \
	icecursor \
	genpref \
	flamewm-bin \
	flamewm-settings-bin \
	flamewm-desktop-bin \
	flamewmctl-bin

install-exec-local:
	$(MKDIR_P) "$(DESTDIR)$(bindir)"
	$(INSTALL_PROGRAM) flamewm-bin$(EXEEXT) "$(DESTDIR)$(bindir)/flamewm$(EXEEXT)"
	$(INSTALL_PROGRAM) flamewm-settings-bin$(EXEEXT) "$(DESTDIR)$(bindir)/flamewm-settings$(EXEEXT)"
	$(INSTALL_PROGRAM) flamewm-desktop-bin$(EXEEXT) "$(DESTDIR)$(bindir)/flamewm-desktop$(EXEEXT)"
	$(INSTALL_PROGRAM) flamewmctl-bin$(EXEEXT) "$(DESTDIR)$(bindir)/flamewmctl$(EXEEXT)"

uninstall-local:
	rm -f "$(DESTDIR)$(bindir)/flamewm$(EXEEXT)"
	rm -f "$(DESTDIR)$(bindir)/flamewm-settings$(EXEEXT)"
	rm -f "$(DESTDIR)$(bindir)/flamewm-desktop$(EXEEXT)"
	rm -f "$(DESTDIR)$(bindir)/flamewmctl$(EXEEXT)"

TESTS = strtest testpointer testarray

if BUILD_TESTS
noinst_PROGRAMS += \
	testarray \
	testlocale \
	testmap \
	testmenus \
	testnetwmhints \
	testpointer \
	testwinhints \
	iceview \
	icesame \
	iceicon \
	icerun \
	icelist \
	strtest \
	test_desktop \
	test_ui_primitives \
	flamewm_test_native_ui_render
TESTS += test_desktop test_ui_primitives flamewm_test_native_ui_render
endif

if BUILD_SOUND
bin_PROGRAMS += icesound
endif

if BUILD_MENU_FDO
bin_PROGRAMS += icewm-menu-fdo
endif

if BUILD_ICEWMTRAY
bin_PROGRAMS += icewmtray
AM_INSTALLCHECK_STD_OPTIONS_EXEMPT = icewmtray
endif

libice_la_SOURCES = \
	MwmUtil.h \
	WinMgr.h \
	argument.h \
	ascii.h \
	base.h \
	debug.h \
	default.h \
	guievent.h \
	intl.h \
	keysyms.cc \
	keysyms.h \
	logevent.cc \
	logevent.h \
	misc.cc \
	mstring.cc \
	mstring.h \
	objmenu.cc \
	objmenu.h \
	prefs.h \
	ref.cc \
	ref.h \
	sysdep.h \
	themable.h \
	udir.cc \
	udir.h \
	upath.cc \
	upath.h \
	wmkey.cc \
	wmkey.h \
	wmmgr.h \
	wmprog.h \
	workspaces.h \
	yaction.h \
	yapp.cc \
	yapp.h \
	yarray.cc \
	yarray.h \
	ybidi.h \
	ycolor.cc \
	ycolor.h \
	yconfig.cc \
	yconfig.h \
	ycursor.cc \
	ycursor.h \
	yfileio.cc \
	yfileio.h \
	yfont.cc \
	yfontbase.h \
	yfontcache.h \
	yfontcore.cc \
	fontmacro.h \
	yfontname.h \
	yfontxft.cc \
	yfull.h \
	yimage.h \
	yimage2.cc \
	yimage2.h \
	yimage_gdk.cc \
	yimage_gdk.h \
	ykey.h \
	ylayout.h \
	ylib.h \
	ylist.h \
	ylocale.cc \
	ylocale.h \
	ymenu.h \
	ymsgbox.h \
	ypaint.cc \
	ypaint.h \
	ypipereader.cc \
	ypipereader.h \
	ypixmap.cc \
	ypixmap.h \
	ypointer.h \
	ypoll.h \
	ypopup.cc \
	ypopup.h \
	yprefs.cc \
	yprefs.h \
	yrect.h \
	ysocket.cc \
	ysocket.h \
	ystring.cc \
	ystring.h \
	ysvg.cc \
	ytime.cc \
	ytime.h \
	ytimer.cc \
	ytimer.h \
	ytooltip.cc \
	ytooltip.h \
	ytrace.h \
	ywindow.cc \
	ywindow.h \
	ywordexp.h \
	yxapp.cc \
	yxapp.h \
	yxcontext.h \
	yxembed.cc \
	yxembed.h \
	yximage.cc

libitk_la_SOURCES = \
	WinMgr.h \
	applet.h \
	appnames.h \
	ascii.h \
	base.h \
	bindkey.cc \
	bindkey.h \
	debug.h \
	default.h \
	globit.cc \
	globit.h \
	intl.h \
	logevent.h \
	mstring.h \
	prefs.h \
	ref.h \
	sysdep.h \
	themable.h \
	udir.h \
	upath.h \
	wmaction.h \
	wmclient.h \
	wmconfig.cc \
	wmconfig.h \
	wmmgr.h \
	wmoption.h \
	wmsave.cc \
	wmsave.h \
	wmtaskbar.h \
	workspaces.h \
	wpixmaps.h \
	wpixres.cc \
	wpixres.h \
	yaction.h \
	yapp.h \
	yarray.h \
	ybutton.cc \
	ybutton.h \
	ycolor.h \
	yconfig.h \
	ycursor.h \
	yfull.h \
	yicon.cc \
	yicon.h \
	yimage.h \
	yinputline.cc \
	yinputline.h \
	ykey.h \
	ylabel.cc \
	ylabel.h \
	ylayout.h \
	ylib.h \
	ylist.h \
	ylistbox.cc \
	ylistbox.h \
	ymenu.cc \
	ymenu.h \
	ymenuitem.cc \
	ymenuitem.h \
	ymsgbox.h \
	ypaint.h \
	ypixmap.h \
	ypointer.h \
	ypoll.h \
	ypopup.h \
	yprefs.h \
	yrect.h \
	yscrollbar.cc \
	yscrollbar.h \
	yscrollview.cc \
	yscrollview.h \
	ytime.h \
	ytimer.h \
	ytrace.h \
	ywindow.h \
	ywordexp.h \
	yxapp.h \
	yxtray.h
libitk_la_LIBADD = libice.la

genpref_SOURCES = \
	base.h \
	ascii.h \
	appnames.h \
	ykey.h \
	sysdep.h \
	yprefs.h \
	bindkey.h \
	default.h \
	themable.h \
	icewmbg_prefs.h \
	genpref.cc
genpref_LDADD = libice.la @LIBINTL@ @LIBICONV@

icecursor_SOURCES = icecursor.cc
icecursor_LDADD = $(IMAGE_LIBS) $(CORE_LIBS)

icewm_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	bindkey.h \
	themable.h \
	default.h \
	prefs.h \
	obj.h \
	yaction.h \
	yinputline.h \
	yactionbutton.h \
	objmenu.h \
	objbutton.h \
	objbutton.cc \
	WinMgr.h \
	ypoll.h \
	ylib.h \
	ymsgbox.cc \
	ymsgbox.h \
	ydialog.cc \
	ydialog.h \
	yurl.cc \
	yurl.h \
	yrect.h \
	wmsession.cc \
	wmsession.h \
	wmwinlist.cc \
	wmwinlist.h \
	wmtaskbar.cc \
	wmtaskbar.h \
	wmwinmenu.cc \
	wmwinmenu.h \
	wmdialog.cc \
	wmdialog.h \
	wmabout.cc \
	wmabout.h \
	wmswitch.cc \
	wmswitch.h \
	preview.cc \
	preview.h \
	switcher.h \
	wmstatus.cc \
	wmstatus.h \
	wmoption.cc \
	wmoption.h \
	wmaction.h \
	wmcontainer.cc \
	wmcontainer.h \
	wmclient.cc \
	MwmUtil.h \
	wmclient.h \
	wmdock.cc \
	wmdock.h \
	wmmgr.cc \
	wmmgr.h \
	workspaces.h \
	appnames.h \
	guievent.h \
	wmapp.cc \
	wmapp.h \
	wmframe.cc \
	wmframe.h \
	wmbutton.cc \
	wmbutton.h \
	wmminiicon.cc \
	wmminiicon.h \
	wmtitle.cc \
	wmtitle.h \
	movesize.cc \
	ascii.h \
	themes.cc \
	themes.h \
	theminst.cc \
	theminst.h \
	decorate.cc \
	browse.cc \
	browse.h \
	wmmenu.cc \
	wmprog.cc \
	wmprog.h \
	wmpref.cc \
	wmpref.h \
	atasks.cc \
	atasks.h \
	aworkspaces.cc \
	aworkspaces.h \
	amailbox.cc \
	amailbox.h \
	akeyboard.cc \
	akeyboard.h \
	aclock.cc \
	aclock.h \
	acpustatus.cc \
	acpustatus.h \
	amemstatus.cc \
	amemstatus.h \
	applet.cc \
	applet.h \
	apppstatus.cc \
	apppstatus.h \
	aaddressbar.cc \
	aaddressbar.h \
	objbar.cc \
	objbar.h \
	aapm.cc \
	aapm.h \
	atray.cc \
	atray.h \
	ysmapp.cc \
	ysmapp.h \
	yxcontext.h \
	yxtray.cc \
	yxtray.h
icewm_LDADD = libitk.la libice.la $(IMAGE_LIBS) $(XSM_LIBS) $(CORE_LIBS) @LIBINTL@ @LIBICONV@

flamewm_bin_SOURCES = $(icewm_SOURCES) \
	flamewm/engine/icewm/font_bootstrap.cc \
	flamewm/engine/icewm/font_bootstrap.h
# FLAMEWM-BRIDGE-HOOK-BEGIN(C-BUILD-01)
# Link product flamewm against canonical graph; icewm stays vanilla above.
flamewm_bin_LDADD = libflamewm_engine_icewm.a libflamewm_shell.a libflamewm_platform.a libflamewm_control_common.a libflamewm_control_server.a libflamewm_control_client.a libflamewm.la libitk.la libice.la $(IMAGE_LIBS) $(XSM_LIBS) $(CORE_LIBS) $(FLAMEWM_DBUS_LIBS) $(FLAMEWM_PULSE_LIBS) @LIBINTL@ @LIBICONV@
# FLAMEWM-BRIDGE-HOOK-END(C-BUILD-01)

icesh_SOURCES = \
	intl.h \
	ascii.h \
	debug.h \
	base.h \
	MwmUtil.h \
	WinMgr.h \
	guievent.h \
	logevent.h \
	wmaction.h \
	ypointer.h \
	ytime.h \
	yrect.h \
	icesh.cc
icesh_LDADD = libice.la $(X_LIBS) $(RANDR_LIBS) $(XINERAMA_LIBS) $(XRES_LIBS) -lX11 @LIBINTL@ @LIBICONV@

icewm_session_SOURCES = \
	appnames.h \
	base.h \
	debug.h \
	intl.h \
	sysdep.h \
	yapp.h \
	ytime.h \
	ytimer.h \
	icesm.cc
icewm_session_LDADD = libice.la $(X_LIBS) -lX11 @LIBINTL@ @LIBICONV@

# FLAMEWM-BRIDGE-HOOK-BEGIN(C-BUILD-01)
flamewm_session_SOURCES = \
	flamewm/session/main.cc \
	flamewm/session/desktop_supervisor.h \
	flamewm/session/desktop_supervisor.cc
flamewm_session_LDADD = libflamewm_platform.a libice.la libflamewm.la $(X_LIBS) $(CORE_LIBS) -lX11 @LIBINTL@ @LIBICONV@
flamewm_session_CXXFLAGS = $(AM_CXXFLAGS) -std=c++11 -Wall -Wextra
# FLAMEWM-BRIDGE-HOOK-END(C-BUILD-01)

icewmhint_SOURCES = \
	intl.h \
	debug.h \
	base.h \
	icewmhint.cc
icewmhint_LDADD = libice.la $(CORE_LIBS) @LIBINTL@ @LIBICONV@

icewmbg_SOURCES = \
	intl.h \
	debug.h \
	base.h \
	themable.h \
	default.h \
	yxapp.h \
	yprefs.h \
	icewmbg.cc \
	icewmbg_prefs.h
icewmbg_LDADD = libice.la $(IMAGE_LIBS) $(CORE_LIBS) @LIBINTL@ @LIBICONV@

icesound_SOURCES = \
	base.h \
	debug.h \
	guievent.h \
	intl.h \
	upath.h \
	ypointer.h \
	ytime.h \
	ytimer.h \
	icesound.cc
icesound_LDADD = libice.la $(AUDIO_LIBS) $(CORE_LIBS) @LIBINTL@ @LIBICONV@

icewm_menu_fdo_SOURCES = \
	appnames.h \
	base.h \
	debug.h \
	fdospecgen.h \
	intl.h \
	sysdep.h \
	yarray.h \
	sysdep.h \
	fdomenu.cc
icewm_menu_fdo_LDADD = libice.la $(GIO_LIBS)

icehelp_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	ascii.h \
	bindkey.h \
	themable.h \
	default.h \
	prefs.h \
	yaction.h \
	ylib.h \
	yrect.h \
	icehelp.cc
icehelp_LDADD = libitk.la libice.la $(IMAGE_LIBS) $(CORE_LIBS) @LIBINTL@ @LIBICONV@

flamewm_desktop_bin_SOURCES = \
	flamewm-desktop/main.cc \
	flamewm-desktop/model.cc \
	flamewm-desktop/model.h \
	flamewm-desktop/layout.cc \
	flamewm-desktop/layout.h \
	flamewm-desktop/item.h \
	flamewm-desktop/selection.cc \
	flamewm-desktop/selection.h \
	flamewm-desktop/watcher.cc \
	flamewm-desktop/watcher.h \
	flamewm-desktop/trash.cc \
	flamewm-desktop/trash.h \
	flamewm-desktop/fileactions.cc \
	flamewm-desktop/fileactions.h \
	flamewm-desktop/watermark.cc \
	flamewm-desktop/watermark.h \
	flamewm-desktop/view.cc \
	flamewm-desktop/view.h \
	flamewm-desktop/desktopapp.cc \
	flamewm-desktop/desktopapp.h \
	flamewm-desktop/stickystore.cc
flamewm_desktop_bin_CXXFLAGS = $(AM_CXXFLAGS) -std=c++11 -Wall -Wextra
flamewm_desktop_bin_LDADD = libflamewm.la libflamewm_control_client.a libflamewm_control_common.a libice.la $(CORE_LIBS) $(FLAMEWM_DBUS_LIBS) $(FLAMEWM_PULSE_LIBS) @LIBINTL@ @LIBICONV@

iceclock_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	iceclock.cc \
	bindkey.h \
	themable.h \
	default.h \
	prefs.h \
	applet.cc \
	applet.h \
	aclock.cc \
	aclock.h
iceclock_LDADD = libitk.la libice.la $(IMAGE_LIBS) $(CORE_LIBS) @LIBINTL@ @LIBICONV@

strtest_SOURCES = \
	base.h \
	udir.h \
	upath.h \
	strtest.cc \
	mstring.h
strtest_LDADD = libice.la

test_desktop_SOURCES = \
	flamewm-desktop/tests/test_desktop.cc \
	flamewm-desktop/model.cc \
	flamewm-desktop/layout.cc \
	flamewm-desktop/selection.cc \
	flamewm-desktop/watcher.cc \
	flamewm-desktop/trash.cc \
	flamewm-desktop/fileactions.cc \
	flamewm-desktop/watermark.cc \
	flamewm-desktop/view.cc \
	flamewm-desktop/desktopapp.cc \
	flamewm-desktop/stickystore.cc \
	flamewm/tests/appname_stub.cc
test_desktop_CXXFLAGS = $(AM_CXXFLAGS) -std=c++11 -Wall -Wextra
test_desktop_LDADD = libflamewm.la libflamewm_control_client.a libflamewm_control_common.a libice.la $(CORE_LIBS) $(FLAMEWM_DBUS_LIBS) $(FLAMEWM_PULSE_LIBS) @LIBINTL@ @LIBICONV@

test_ui_primitives_SOURCES = flamewm/tests/test_ui_primitives.cc
test_ui_primitives_CXXFLAGS = $(AM_CXXFLAGS) -std=c++11 -Wall -Wextra
test_ui_primitives_LDADD = libflamewm.la libflamewm_platform.a libitk.la libice.la $(CORE_LIBS) @LIBINTL@ @LIBICONV@

flamewm_test_native_ui_render_SOURCES = flamewm/tests/test_native_ui_render.cc
flamewm_test_native_ui_render_CXXFLAGS = $(AM_CXXFLAGS) -std=c++11 -Wall -Wextra
flamewm_test_native_ui_render_LDADD = libflamewm.la libflamewm_platform.a libitk.la libice.la $(CORE_LIBS) @LIBINTL@ @LIBICONV@

icewmtray_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	bindkey.h \
	themable.h \
	default.h \
	prefs.h \
	yxtray.cc \
	yxtray.h \
	wmoption.cc \
	wmoption.h \
	ylib.h \
	icetray.cc
icewmtray_LDADD = libitk.la libice.la $(IMAGE_LIBS) $(CORE_LIBS)

icesame_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	MwmUtil.h \
	yaction.h \
	yrect.h \
	ykey.h \
	yfull.h \
	icesame.cc
icesame_LDADD = libitk.la libice.la $(IMAGE_LIBS) $(CORE_LIBS)

icelist_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	yaction.h \
	yinputline.h \
	yrect.h \
	ykey.h \
	yfull.h \
	icelist.cc
icelist_LDADD = libitk.la libice.la $(IMAGE_LIBS) $(CORE_LIBS)

iceview_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	bindkey.h \
	themable.h \
	default.h \
	prefs.h \
	yaction.h \
	ylib.h \
	yrect.h \
	iceview.cc
iceview_LDADD = libitk.la libice.la $(IMAGE_LIBS) $(CORE_LIBS)

iceicon_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	yaction.h \
	ylib.h \
	yrect.h \
	iceicon.cc
iceicon_LDADD = libitk.la libice.la $(IMAGE_LIBS) $(CORE_LIBS)

icerun_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	bindkey.h \
	themable.h \
	default.h \
	prefs.h \
	yaction.h \
	yinputline.h \
	icerun.cc
icerun_LDADD = libitk.la libice.la $(IMAGE_LIBS) $(CORE_LIBS)

iceskt_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	iceskt.cc
iceskt_LDADD = libitk.la libice.la $(IMAGE_LIBS) $(CORE_LIBS)

testmenus_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	obj.h \
	yaction.h \
	testmenus.cc \
	bindkey.h \
	themable.h \
	default.h \
	prefs.h \
	wmmenu.cc \
	wmprog.cc \
	wmprog.h \
	wmaction.h \
	ascii.h \
	themes.cc \
	themes.h \
	browse.cc \
	browse.h
testmenus_LDADD = libitk.la libice.la $(IMAGE_LIBS) $(CORE_LIBS)

testwinhints_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	WinMgr.h \
	testwinhints.cc
testwinhints_LDFLAGS = $(CORE_LIBS)

testnetwmhints_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	WinMgr.h \
	testnetwmhints.cc
testnetwmhints_LDFLAGS = libice.la $(CORE_LIBS)

testmap_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	WinMgr.h \
	testmap.cc
testmap_LDFLAGS = libice.la $(CORE_LIBS)

testlocale_SOURCES = \
	intl.h \
	debug.h \
	sysdep.h \
	base.h \
	ylib.h \
	testlocale.cc
testlocale_LDADD = libice.la $(CORE_LIBS) @LIBINTL@ @LIBICONV@

testarray_SOURCES = \
	mstring.h \
	yarray.h \
	base.h \
	ypointer.h \
	testarray.cc
testarray_LDADD = libice.la @LIBINTL@ @LIBICONV@

testpointer_SOURCES = \
	ypointer.h \
	testpointer.cc

nodist_pkgdata_DATA = \
	preferences

preferences: genpref$(EXEEXT)
	$(AM_V_GEN)./genpref$(EXEEXT) -o $@ -s

CLEANFILES = preferences strtest testarray testpointer
