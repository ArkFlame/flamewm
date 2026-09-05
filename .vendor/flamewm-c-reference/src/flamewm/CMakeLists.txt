# src/flamewm/CMakeLists.txt — FlameWM canonical build graph (fragment)
# Included from src/CMakeLists.txt via add_subdirectory(flamewm).
# All targets are C++11. Paths are relative to this directory (src/flamewm).

cmake_minimum_required(VERSION 3.5)

# Guard: when executed via `cmake -P` (script mode) this file is not a
# standalone project — skip target definitions so `cmake -P` parse check
# does not error on add_library. Normal configure include runs with
# CMAKE_SCRIPT_MODE_FILE empty.
if(CMAKE_SCRIPT_MODE_FILE)
    return()
endif()

# ---------------------------------------------------------------------------
# flamewm_api — INTERFACE, header-only contract (api/*.h)
# ---------------------------------------------------------------------------
add_library(flamewm_api INTERFACE)
target_include_directories(flamewm_api INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/..
    ${CMAKE_CURRENT_SOURCE_DIR}
)
# Consumers get C++11 requirement transitively
target_compile_features(flamewm_api INTERFACE cxx_std_11)

# ---------------------------------------------------------------------------
# flamewm_platform — STATIC, platform/**/*.cc
# ---------------------------------------------------------------------------
set(FLAMEWM_PLATFORM_SOURCES
    platform/host.cc
    platform/applications/service.cc
    platform/background/service.cc
    platform/chrome/policy.cc
    platform/displays/service.cc
    platform/panels/service.cc
    platform/reactor/service.cc
    platform/scale/service.cc
    platform/session/service.cc
    platform/settings/service.cc
    platform/shortcuts/service.cc
    platform/snap/service.cc
    platform/system/service.cc
    platform/windows/service.cc
    platform/workspaces/service.cc
)

add_library(flamewm_platform STATIC ${FLAMEWM_PLATFORM_SOURCES})
target_include_directories(flamewm_platform PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/..
    ${CMAKE_CURRENT_SOURCE_DIR}
)
target_link_libraries(flamewm_platform PUBLIC flamewm_api)
if(FLAMEWM_DBUS_FOUND)
    target_compile_definitions(flamewm_platform PUBLIC HAVE_FLAMEWM_DBUS=1)
    target_include_directories(flamewm_platform PUBLIC ${FLAMEWM_DBUS_INCLUDE_DIRS})
endif()
set_target_properties(flamewm_platform PROPERTIES
    CXX_STANDARD 11
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
)

# ---------------------------------------------------------------------------
# flamewm_engine_icewm — STATIC, engine/icewm/**/*.cc (incl. ui/backend.cc)
# ---------------------------------------------------------------------------
set(FLAMEWM_ENGINE_ICEWM_SOURCES
    engine/icewm/access.cc
    engine/icewm/application_adapter.cc
    engine/icewm/background_adapter.cc
    engine/icewm/bootstrap.cc
    engine/icewm/bridge.cc
    engine/icewm/snap_bridge.cc
    engine/icewm/snap_bridge.h
    engine/icewm/display_adapter.cc
    engine/icewm/hooks.cc
    engine/icewm/input_adapter.cc
    engine/icewm/mainloop_adapter.cc
    engine/icewm/session_adapter.cc
    engine/icewm/shortcut_adapter.cc
    engine/icewm/tray_adapter.cc
    engine/icewm/window_adapter.cc
    engine/icewm/workarea_adapter.cc
    engine/icewm/workspace_adapter.cc
    engine/icewm/ui/backend.cc
    engine/icewm/ui/material_painter.cc
    engine/icewm/ui/native_paint.cc
    engine/icewm/ui/native_paint.h
    engine/icewm/ui/native_theme_resources.cc
    engine/icewm/ui/native_theme_resources.h
    engine/icewm/ui/panel_registry.cc
    engine/icewm/native_overlay.cc
    engine/icewm/native_overlay.h
)

add_library(flamewm_engine_icewm STATIC ${FLAMEWM_ENGINE_ICEWM_SOURCES})
target_include_directories(flamewm_engine_icewm PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/..
    ${CMAKE_CURRENT_SOURCE_DIR}
)
target_link_libraries(flamewm_engine_icewm PUBLIC flamewm_api flamewm_platform itk ice ${xrandr_LDFLAGS})
target_compile_definitions(flamewm_engine_icewm PRIVATE FLAMEWM_PRODUCT_BUILD=1)
set_target_properties(flamewm_engine_icewm PROPERTIES
    CXX_STANDARD 11
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
)
# X11 linkage is wired by the parent scope (CORE/X11) when available.
# Stub backend is X11-free; FLAMEWM_WITH_X11 enables real YWindow impl.
# target_compile_definitions(flamewm_engine_icewm PRIVATE FLAMEWM_WITH_X11)

# ---------------------------------------------------------------------------
# flamewm_control_common — STATIC (control/protocol.*) — parity with Autotools
# ---------------------------------------------------------------------------
add_library(flamewm_control_common STATIC
    control/protocol.cc
    control/protocol.h
)
# Override to empty stub once real sources exist; keep deps valid.
target_include_directories(flamewm_control_common PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/..
    ${CMAKE_CURRENT_SOURCE_DIR}
)
target_link_libraries(flamewm_control_common PUBLIC flamewm_api)
set_target_properties(flamewm_control_common PROPERTIES
    CXX_STANDARD 11
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
)

# ---------------------------------------------------------------------------
# flamewm_ui — INTERFACE (header-only facade: ui/*.h, no X11)
# ---------------------------------------------------------------------------
add_library(flamewm_ui INTERFACE)
target_include_directories(flamewm_ui INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/..
    ${CMAKE_CURRENT_SOURCE_DIR}
)
target_sources(flamewm_ui INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/ui/assets/asset.h
    ${CMAKE_CURRENT_SOURCE_DIR}/ui/image.h
    ${CMAKE_CURRENT_SOURCE_DIR}/ui/layout.h
    ${CMAKE_CURRENT_SOURCE_DIR}/ui/menu/model.h
    ${CMAKE_CURRENT_SOURCE_DIR}/ui/style/box.h
    ${CMAKE_CURRENT_SOURCE_DIR}/ui/style/theme.h
    ${CMAKE_CURRENT_SOURCE_DIR}/ui/style/typography.h
    ${CMAKE_CURRENT_SOURCE_DIR}/ui/style/visual.h
)
target_link_libraries(flamewm_ui INTERFACE flamewm_api)
target_compile_features(flamewm_ui INTERFACE cxx_std_11)

# ---------------------------------------------------------------------------
# flamewm_control_server / flamewm_control_client — STATIC (control/*.cc)
# ---------------------------------------------------------------------------
add_library(flamewm_control_server STATIC
    control/server.cc
)
target_include_directories(flamewm_control_server PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/..
    ${CMAKE_CURRENT_SOURCE_DIR}
)
target_link_libraries(flamewm_control_server PUBLIC flamewm_control_common flamewm_api)
set_target_properties(flamewm_control_server PROPERTIES
    CXX_STANDARD 11
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
)
if(FLAMEWM_DBUS_FOUND)
    target_compile_definitions(flamewm_control_server PUBLIC HAVE_FLAMEWM_DBUS=1 FLAMEWM_HAS_DBUS=1)
    target_include_directories(flamewm_control_server PUBLIC ${FLAMEWM_DBUS_INCLUDE_DIRS})
    target_link_libraries(flamewm_control_server PUBLIC ${FLAMEWM_DBUS_LIBRARIES})
endif()

add_library(flamewm_control_client STATIC
    control/client.cc
)
target_include_directories(flamewm_control_client PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/..
    ${CMAKE_CURRENT_SOURCE_DIR}
)
target_link_libraries(flamewm_control_client PUBLIC flamewm_control_common flamewm_api)
set_target_properties(flamewm_control_client PROPERTIES
    CXX_STANDARD 11
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
)
if(FLAMEWM_DBUS_FOUND)
    target_compile_definitions(flamewm_control_client PUBLIC HAVE_FLAMEWM_DBUS=1)
    target_include_directories(flamewm_control_client PUBLIC ${FLAMEWM_DBUS_INCLUDE_DIRS})
    target_link_libraries(flamewm_control_client PUBLIC ${FLAMEWM_DBUS_LIBRARIES})
endif()

# ---------------------------------------------------------------------------
# flamewm_shell — STATIC, shell/**/*.cc (one Flame Shell object graph)
# ---------------------------------------------------------------------------
set(FLAMEWM_SHELL_SOURCES
    shell/shell.cc
    shell/panel_host.cc
    shell/panel_surface.cc
    shell/task_model.cc
    shell/task_view.cc
    shell/start_view.cc
    shell/workspace_view.cc
    shell/status_area.cc
    shell/clock_view.cc
    shell/tray_host.cc
    shell/popovers.cc
)

add_library(flamewm_shell STATIC ${FLAMEWM_SHELL_SOURCES})
target_include_directories(flamewm_shell PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/..
    ${CMAKE_CURRENT_SOURCE_DIR}
)
target_link_libraries(flamewm_shell PUBLIC flamewm_platform flamewm_engine_icewm flamewm_control_client flamewm_ui)
set_target_properties(flamewm_shell PROPERTIES
    CXX_STANDARD 11
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
)

# ---------------------------------------------------------------------------
# flamewm_ui_icewm — INTERFACE, IceWM-backed ui facade (links ui + engine)
# ---------------------------------------------------------------------------
add_library(flamewm_ui_icewm INTERFACE)
target_include_directories(flamewm_ui_icewm INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/..
    ${CMAKE_CURRENT_SOURCE_DIR}
)
target_link_libraries(flamewm_ui_icewm INTERFACE flamewm_ui flamewm_engine_icewm)
target_compile_features(flamewm_ui_icewm INTERFACE cxx_std_11)
