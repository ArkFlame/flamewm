# src/flamewm/Makefile.am.inc — Automake fragment for FlameWM canonical graph
# Included from src/Makefile.am via `include flamewm/Makefile.am.inc`
# Wired via FLAMEWM-BRIDGE-HOOK(C-BUILD-01) in src/Makefile.am.
# All objects built with -std=c++11 (AM_CXXFLAGS append).

# ---------------------------------------------------------------------------
# flamewm_api — header-only INTERFACE (no archive); list for EXTRA_DIST
# ---------------------------------------------------------------------------
flamewm_api_headers = \
    flamewm/api/ids.h \
    flamewm/api/geometry.h \
    flamewm/api/window.h \
    flamewm/api/workspace.h \
    flamewm/api/display.h \
    flamewm/api/shortcuts.h \
    flamewm/api/input.h \
    flamewm/api/background.h \
    flamewm/api/session.h \
    flamewm/api/applications.h \
    flamewm/api/errors.h \
    flamewm/api/events.h \
    flamewm/api/ports.h \
    flamewm/api/panels.h \
    flamewm/api/capabilities.h \
    flamewm/api/settings.h

EXTRA_DIST += $(flamewm_api_headers)

# ---------------------------------------------------------------------------
# flamewm_ui — header-only facade (no X11 in facade)
# ---------------------------------------------------------------------------
flamewm_ui_headers = \
    flamewm/ui/window.h \
    flamewm/ui/button.h \
    flamewm/ui/label.h \
    flamewm/ui/slider.h \
    flamewm/ui/dialog.h \
    flamewm/ui/textfield.h \
    flamewm/ui/list.h \
    flamewm/ui/popover.h \
    flamewm/ui/backend.h \
    flamewm/ui/icon.h \
    flamewm/ui/layout.h \
    flamewm/ui/iconroles.h \
    flamewm/ui/palette.h \
    flamewm/ui/metrics.h \
    flamewm/ui/iconresolver.h \
    flamewm/ui/popoveranchor.h \
    flamewm/ui/assets/asset.h \
    flamewm/ui/image.h \
    flamewm/ui/menu/model.h \
    flamewm/ui/style/box.h \
    flamewm/ui/style/theme.h \
    flamewm/ui/style/typography.h \
    flamewm/ui/style/visual.h

EXTRA_DIST += $(flamewm_ui_headers)

# ---------------------------------------------------------------------------
# flamewm_platform — noinst static (platform/**/*.cc)
# ---------------------------------------------------------------------------
noinst_LIBRARIES = libflamewm_platform.a
libflamewm_platform_a_SOURCES = \
    flamewm/platform/host.cc \
    flamewm/platform/host.h \
    flamewm/platform/applications/service.cc \
    flamewm/platform/applications/service.h \
    flamewm/platform/background/service.cc \
    flamewm/platform/background/service.h \
    flamewm/platform/chrome/policy.cc \
    flamewm/platform/chrome/policy.h \
    flamewm/platform/displays/service.cc \
    flamewm/platform/displays/service.h \
    flamewm/platform/panels/service.cc \
    flamewm/platform/panels/service.h \
    flamewm/platform/reactor/service.cc \
    flamewm/platform/reactor/service.h \
    flamewm/platform/scale/service.cc \
    flamewm/platform/scale/service.h \
    flamewm/platform/session/service.cc \
    flamewm/platform/session/service.h \
    flamewm/platform/settings/service.cc \
    flamewm/platform/settings/service.h \
    flamewm/platform/settings/effect.h \
    flamewm/platform/shortcuts/service.cc \
    flamewm/platform/shortcuts/service.h \
    flamewm/platform/snap/service.cc \
    flamewm/platform/snap/service.h \
    flamewm/platform/system/service.cc \
    flamewm/platform/system/service.h \
    flamewm/platform/windows/service.cc \
    flamewm/platform/windows/service.h \
    flamewm/platform/workspaces/service.cc \
    flamewm/platform/workspaces/service.h
libflamewm_platform_a_CXXFLAGS = $(AM_CXXFLAGS) -std=c++11 -Wall -Wextra

# ---------------------------------------------------------------------------
# flamewm_engine_icewm — noinst static (engine/icewm/**/*.cc incl. ui/backend)
# ---------------------------------------------------------------------------
noinst_LIBRARIES += libflamewm_engine_icewm.a
libflamewm_engine_icewm_a_SOURCES = \
    flamewm/engine/icewm/access.cc \
    flamewm/engine/icewm/access.h \
    flamewm/engine/icewm/application_adapter.cc \
    flamewm/engine/icewm/application_adapter.h \
    flamewm/engine/icewm/background_adapter.cc \
    flamewm/engine/icewm/background_adapter.h \
    flamewm/engine/icewm/bootstrap.cc \
    flamewm/engine/icewm/bootstrap.h \
    flamewm/engine/icewm/bridge.cc \
    flamewm/engine/icewm/bridge.h \
    flamewm/engine/icewm/snap_bridge.cc \
    flamewm/engine/icewm/snap_bridge.h \
    flamewm/engine/icewm/display_adapter.cc \
    flamewm/engine/icewm/display_adapter.h \
    flamewm/engine/icewm/hooks.cc \
    flamewm/engine/icewm/hooks.h \
    flamewm/engine/icewm/input_adapter.cc \
    flamewm/engine/icewm/input_adapter.h \
    flamewm/engine/icewm/mainloop_adapter.cc \
    flamewm/engine/icewm/mainloop_adapter.h \
    flamewm/engine/icewm/session_adapter.cc \
    flamewm/engine/icewm/session_adapter.h \
    flamewm/engine/icewm/shortcut_adapter.cc \
    flamewm/engine/icewm/shortcut_adapter.h \
    flamewm/engine/icewm/tray_adapter.cc \
    flamewm/engine/icewm/tray_adapter.h \
    flamewm/engine/icewm/window_adapter.cc \
    flamewm/engine/icewm/window_adapter.h \
    flamewm/engine/icewm/workarea_adapter.cc \
    flamewm/engine/icewm/workarea_adapter.h \
    flamewm/engine/icewm/workspace_adapter.cc \
    flamewm/engine/icewm/workspace_adapter.h \
    flamewm/engine/icewm/ui/backend.cc \
    flamewm/engine/icewm/ui/backend.h \
    flamewm/engine/icewm/ui/material_painter.cc \
    flamewm/engine/icewm/ui/native_paint.cc \
    flamewm/engine/icewm/ui/native_paint.h \
    flamewm/engine/icewm/ui/native_theme_resources.cc \
    flamewm/engine/icewm/ui/native_theme_resources.h \
    flamewm/engine/icewm/ui/panel_registry.cc \
    flamewm/engine/icewm/ui/panel_registry.h \
    flamewm/engine/icewm/native_overlay.cc \
    flamewm/engine/icewm/native_overlay.h
libflamewm_engine_icewm_a_CXXFLAGS = $(AM_CXXFLAGS) -std=c++11 -Wall -Wextra -DFLAMEWM_PRODUCT_BUILD=1

# ---------------------------------------------------------------------------
# flamewm_control_common — noinst static (control/protocol.*)
# ---------------------------------------------------------------------------
noinst_LIBRARIES += libflamewm_control_common.a
libflamewm_control_common_a_SOURCES = \
    flamewm/control/protocol.cc \
    flamewm/control/protocol.h
libflamewm_control_common_a_CXXFLAGS = $(AM_CXXFLAGS) -std=c++11 -Wall -Wextra

# ---------------------------------------------------------------------------
# flamewm_control_server / flamewm_control_client — noinst static
# ---------------------------------------------------------------------------
noinst_LIBRARIES += libflamewm_control_server.a libflamewm_control_client.a
libflamewm_control_server_a_SOURCES = \
    flamewm/control/server.cc \
    flamewm/control/server.h
libflamewm_control_server_a_CXXFLAGS = $(AM_CXXFLAGS) -std=c++11 -Wall -Wextra

libflamewm_control_client_a_SOURCES = \
    flamewm/control/client.cc \
    flamewm/control/client.h
libflamewm_control_client_a_CXXFLAGS = $(AM_CXXFLAGS) -std=c++11 -Wall -Wextra

# ---------------------------------------------------------------------------
# flamewm_shell — noinst static (shell/**/*.cc — one Shell object graph)
# ---------------------------------------------------------------------------
noinst_LIBRARIES += libflamewm_shell.a
libflamewm_shell_a_SOURCES = \
    flamewm/shell/shell.cc \
    flamewm/shell/shell.h \
    flamewm/shell/panel_host.cc \
    flamewm/shell/panel_host.h \
    flamewm/shell/panel_surface.cc \
    flamewm/shell/panel_surface.h \
    flamewm/shell/task_model.cc \
    flamewm/shell/task_model.h \
    flamewm/shell/task_view.cc \
    flamewm/shell/task_view.h \
    flamewm/shell/start_view.cc \
    flamewm/shell/start_view.h \
    flamewm/shell/workspace_view.cc \
    flamewm/shell/workspace_view.h \
    flamewm/shell/status_area.cc \
    flamewm/shell/status_area.h \
    flamewm/shell/clock_view.cc \
    flamewm/shell/clock_view.h \
    flamewm/shell/tray_host.cc \
    flamewm/shell/tray_host.h \
    flamewm/shell/popovers.cc \
    flamewm/shell/popovers.h
libflamewm_shell_a_CXXFLAGS = $(AM_CXXFLAGS) -std=c++11 -Wall -Wextra

# ---------------------------------------------------------------------------
# flamewm_ui_icewm — header-only alias (transitively ui + engine)
# ---------------------------------------------------------------------------
flamewm_ui_icewm_headers = \
    flamewm/engine/icewm/ui/backend.h \
    flamewm/engine/icewm/ui/panel_registry.h
EXTRA_DIST += $(flamewm_ui_icewm_headers)
