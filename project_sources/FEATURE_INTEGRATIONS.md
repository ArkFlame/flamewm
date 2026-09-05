# Status Integrations and Calendar — V4

All persistent panel status integrations must be event-driven and non-blocking on the X/UI event loop. Missing/restarting services degrade gracefully; they must not crash WM or create retry storms.

## Shared D-Bus integration

Use one small IceWM-event-loop-compatible D-Bus dispatcher for:

- NetworkManager on system bus;
- MPRIS players on session bus.

Do not add GLib/Qt event loops merely for IPC. Map D-Bus watches/timeouts into IceWM poll/timer ownership; unregister before destruction and use service-generation tokens so stale callbacks after owner/reconnect changes are ignored. Never block the X loop with synchronous D-Bus requests.

## Media / MPRIS

Track available players, active player, playback status, metadata and capability flags. Subscribe to state changes; no recurring `playerctl` polling.

Current V4 visual state rule is explicit even though unusual:

```text
Playing -> Play glyph
Paused  -> Pause glyph
```

Treat it as a state glyph, not an inferred action glyph. Player appearance/disappearance updates taskbar immediately; irrelevant media surface collapses.

## Audio

Use asynchronous libpulse APIs compatible with PulseAudio/PipeWire-Pulse:

- default sink;
- volume/mute;
- sink/server subscriptions;
- set volume/mute;
- reconnect when server restarts using bounded backoff and callback-generation guards.

No periodic `pactl`/`wpctl` process spawning in final panel. If backend is unavailable, hide/disable cleanly without invisible dead slot.

## Network

Existing IceWM net applet is throughput monitoring, not Wi-Fi control. V4 network integration uses NetworkManager system D-Bus.

Model at minimum:

```text
unavailable / disconnected / wired / Wi-Fi connecting / Wi-Fi connected
SSID / signal / visible access points
```

Operations: appropriate Wi-Fi enable/disable, connect saved profile, disconnect, AP refresh. New secrets require a secure NetworkManager-compatible path/agent or host-settings handoff; never put password in logs/process args or shell strings. Service owner loss clears stale object paths before reconnect.

Use semantic `Network` icon role with tested light/dark fallback and symbolic/currentColor behavior so an icon cannot silently render dark-on-dark.

## Clock/calendar

Keep IceWM time/date formatting engine where sufficient. V4 requires centered visible time+date and a month calendar popup.

Calendar uses normal local date arithmetic, no resident calendar daemon. Today's highlight is drawn in a **square cell** and then rounded 50% so it is a circle rather than a text-width oval.

## Tray/battery

XEmbed tray stays IceWM-owned. Battery remains existing foundation plus Flame visual treatment; V4 does not justify a new power-management subsystem.

## Verification

- NetworkManager missing -> graceful hidden/disabled control.
- Pulse server restart -> reconnect without WM crash.
- MPRIS player appears/disappears -> media slot updates immediately.
- Playing/paused glyph follows exact V4 contract.
- No recurring subprocess polling loop.
- Calendar today marker circular; time/date/popup anchors correctly on every panel edge.
