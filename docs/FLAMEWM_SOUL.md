# FlameWM Soul

FlameWM is an independent Rust X11 window manager and desktop product. Its
authority is explicit, typed, and owned by FlameWM crates rather than inherited
from a historical engine.

- One mechanism, one owner.
- Product behavior crosses feature facades and typed contracts.
- The WM owns managed-window truth; UI is a projection, not a second WM.
- Reactors own native event sources and their lifetimes.
- Renderer crates own rendering and native renderer access.
- Historical and RWR vendor material is reference evidence only.
- A pattern becomes architecture only through evidence, review, and promotion.

These principles describe the 0.0.7 architecture contract. They do not claim
that every planned surface or migration is complete.
