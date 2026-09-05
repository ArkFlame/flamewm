# IceWM Rust Port Post-Mortem — Why SourcePort Won

**Project:** IceWM 4.1.0 → Rust
**Failed track:** `icewm-rust` through 0.0.9
**Winning track:** `icewm-rust-sourceport` through 0.1.4
**Purpose:** convert the failure into a durable porting methodology for future whole-project and feature ports.
**Verdict:** the SourcePort methodology is the default strategy for parity-oriented ports going forward.

---

## 0. Executive verdict

The original `icewm-rust` track lost because it treated the task as **reimplement IceWM's architecture and behavior in Rust**.

SourcePort won because it treated the task as **preserve IceWM first, change only the language**.

That distinction dominates every other factor.

The failed approach was effectively:

```text
study IceWM
-> infer architecture
-> design Rust modules
-> recreate behavior
-> add parity features
-> discover mismatches
-> patch mismatches
-> repeat
```

The successful SourcePort approach is:

```text
freeze exact IceWM source
-> copy every source unit exactly
-> rename into target-language staging units
-> mechanically replace language semantics
-> preserve unresolved semantics explicitly
-> promote/wire exact upstream owners into executable Rust
-> compile
-> compare against the exact original
-> fix the first observed divergence at its original owner
-> only later refactor
```

The durable rule is:

> **For a parity port, architecture is evidence, not an opportunity for redesign. Copy the exact owner first; translate its semantics; repair imports/types/FFI; wire it; compile it; differential-test it; refactor only after parity.**

This is consistent with the philosophy documented by C2Rust: the first translation should closely mirror the source and prioritize functional equivalence, while safer/idiomatic refactoring is a later incremental phase. C2Rust also treats cross-checking the source and translated programs as a first-class correctness tool.

---

# 1. Evidence audited

The analysis compared the actual supplied archives:

```text
icewm-rust-0.0.9.tar.gz
icewm-rust-sourceport-0.1.4.tar.gz
```

and the embedded IceWM 4.1.0 source/provenance in SourcePort.

## 1.1 Quantitative comparison

| Metric | Failed `icewm-rust` 0.0.9 | SourcePort 0.1.4 |
|---|---:|---:|
| Upstream IceWM `.cc/.h` units represented 1:1 | No | **249 / 249** |
| Exact Stage-1 upstream lines preserved | No equivalent | **92,704** |
| Stage-2 semantic units | No equivalent | **249 / 249** |
| Stage-2 structural conversion coverage | N/A | **56,095 / 92,704 = 60.51%** |
| Compiled Rust modules | 21 | **18** |
| Compiled Rust LOC | 7,525 | **5,240** |
| Rust unit tests declared | 76 | 10 |
| Exact per-unit SHA/source map | No; coarse manual map | **Yes, generated for all 249 units** |
| Exact embedded upstream tree readily addressable | Archived reference tar | **Expanded full source tree** |
| Side-by-side stock-vs-Rust Xephyr harness | Late/semantic differential | **First-class visual differential** |
| Stock visual resources reused | Mostly no | **Yes, XPM/theme/taskbar resources** |
| Cargo required before `CHECK PASS` | Script yes, release process bypassed it | **Yes, operationally enforced** |
| User-observed result | 9 turns, still compile failure | **5 turns, incomplete but visually close to stock** |

The important result is counterintuitive:

> SourcePort did **not** win by writing more runnable Rust. Its executable Rust is smaller. It won by carrying much more **source identity and behavioral evidence**.

---

# 2. The central failure: structural entropy

The failed port deliberately stated:

> “The port follows IceWM's state ownership instead of mechanically translating C++ file-for-file.”

For a greenfield redesign, that can be reasonable.

For a strict parity port, it was the wrong optimization.

IceWM 4.1.0 has 249 top-level `.cc/.h` source units. The failed track collapsed large parts of that mature decomposition into a small novel module set such as:

```text
wm.rs
shell.rs
types.rs
policy.rs
config.rs
theme.rs
fdo.rs
keys.rs
layout.rs
...
```

`wm.rs` alone reached 2,555 lines and absorbed responsibilities originating from multiple IceWM owners such as `wmmgr`, `wmframe`, `wmclient`, `movesize`, focus, workspaces, EWMH interactions and lifecycle.

This created **structural entropy**: the distance between “where IceWM defines a behavior” and “where the Rust port implements it.”

Every parity bug then required a translation exercise before the bug could even be fixed:

```text
observed difference
-> infer which new Rust abstraction owns it
-> rediscover which IceWM class actually defined it
-> reconstruct hidden dependencies/defaults/resources
-> adapt the reconstructed behavior to the new architecture
-> hope no adjacent semantics were omitted
```

SourcePort makes that lookup almost trivial:

```text
observed difference
-> identify stock IceWM owner
-> open exact Stage 1 / Stage 2 owner
-> translate/promote that owner or exact behavior
-> wire it into Stage 3
-> compare again
```

The second loop is dramatically cheaper and has fewer opportunities for invention.

---

# 3. Why the copy → translate → wire methodology won

## 3.1 It made the original source the canonical data model

SourcePort Stage 1 contains all 249 upstream units as byte-identical renamed `.rs` files.

The verifier proves:

```text
STAGE VERIFY PASS:
units=249
cc=122
h=127
lines=92704
```

Each row records:

```text
original path
source kind
Stage-1 path
Stage-2 path
SHA-256
source line count
converted-line count
```

That is much stronger than a prose architecture report.

A prose map says what an agent currently believes.

A source map says exactly what exists and whether it has drifted.

## 3.2 It separated preservation from semantic conversion

Stage 1 answers:

> What did upstream actually contain?

Stage 2 answers:

> What does this unit roughly become in the target language?

Stage 3 answers:

> Which translated semantics are actually compiled and behaviorally promoted today?

Those questions were conflated in the failed track.

SourcePort's Stage 2 also has an important honesty mechanism:

```text
CXX_UNCONVERTED
```

Unsupported semantics are explicitly left unresolved instead of silently replaced with an invented Rust design.

That is a major safety property.

## 3.3 It preserved names, constants and control-flow landmarks

A porting agent can search the target corpus for an upstream symbol and normally find a recognizable counterpart.

This preserves cognitive locality:

```text
upstream class/function/constant
≈ same named translated class/function/constant
```

The failed track frequently translated concepts rather than source units. Conceptual translation sounds elegant, but it loses the exact branch ordering, defaults, side effects, and “weird” legacy behavior that defines compatibility.

## 3.4 It promoted behavior from exact owners

SourcePort 0.1.4 explicitly traces visible behavior to these owners:

```text
src/wmframe.cc       frame geometry/state
src/wmclient.cc      client icon acquisition / WM_HINTS
src/wmtitle.cc       titlebar/system-button layout
src/wmbutton.cc      title-control resources
src/wmtaskbar.cc     taskbar composition
src/atasks.cc        task-button state/icon behavior
src/aworkspaces.cc   workspace preview projection
src/acpustatus.cc    CPU graph semantics
src/aapm.cc          battery/APM behavior
src/wmprog.cc        root/Start menu composition
src/wmconfig.cc      preferences/theme precedence
src/themable.h       classic dimensions/colors
src/ypaint.cc        Win95 bevel drawing
```

The static audit reads anchors from those exact upstream files and rejects drift.

This transforms “source-grounded” from a documentation goal into executable repository policy.

## 3.5 It reused stock resources instead of approximating appearance

The failed port spent effort creating a functional shell and flat fallback renderer.

SourcePort asked a better question:

> Which exact artwork and drawing semantics does IceWM use here?

Then it consumed IceWM's XPM resources and transcribed classic rendering behavior.

This is why visual convergence accelerated.

A pixel/resource mismatch is not solved by designing a nicer Rust renderer. It is solved by using the same source of pixels/metrics until parity exists.

## 3.6 It used visual differential testing early

SourcePort's `compare` path:

1. verifies the source corpus;
2. builds SourcePort;
3. builds the byte-identical embedded IceWM 4.1.0 reference;
4. starts both under separate Xephyr displays;
5. launches the same fixture with the same geometry/config;
6. asks the developer to compare them directly.

That is the correct oracle for “looks exactly like IceWM.”

The failed track had differential tooling, but it arrived after a large custom implementation already existed and concentrated heavily on protocol/root-window state. That is useful, but it did not constrain the renderer/taskbar/titlebar architecture early enough.

## 3.7 It optimized for parity delta, not feature count

The failed 0.0.9 project had more Rust tests and a broader protocol surface.

SourcePort had fewer tests and fewer implemented total features, yet looked much closer to the target.

This demonstrates the correct progress metric:

```text
progress != LOC
progress != number of tests
progress != number of EWMH atoms supported
progress != number of features added

progress = reduction in observable/source-contract parity delta
```

For a parity port, the next task should almost always be selected by the largest proven delta against upstream—not by which subsystem is interesting to implement.

---

# 4. Why my `icewm-rust` approach lost

## 4.1 I redesigned before I had parity

I treated Rust as an opportunity to produce a cleaner module organization.

That reversed the correct order.

The correct order is:

```text
parity first
idiomatic refactor second
architecture improvement third
```

My order was effectively:

```text
architecture first
parity repair forever
```

Once architecture diverged, every missing IceWM behavior became an integration problem instead of a translation problem.

## 4.2 I compressed many upstream authorities into a few new authorities

Examples from the failed source map:

```text
wmmgr.*                 -> wm.rs + ewmh.rs
wmclient.*              -> icccm.rs + ewmh.rs + types.rs + policy.rs
wmframe.* + movesize.cc -> wm.rs + layout.rs
wmtaskbar.* + atasks.*  -> shell.rs
wmprog.* + fdomenu.cc   -> fdo.rs + launcher.rs + shell.rs + config.rs
```

This is elegant only if compatibility does not matter.

For parity it creates ambiguity about:

- exact ownership;
- call order;
- which state is cached where;
- lifecycle ordering;
- default values;
- side effects;
- which resource path belongs to which behavior;
- which unusual branches are compatibility-critical.

## 4.3 I ported descriptions of behavior instead of enough implementation detail

The reports were useful, but I allowed reports to become a substitute for direct transliteration.

A report might say:

```text
IceWM has half/quarter tiling.
```

The source defines:

```text
exact coordinate math
exact work-area source
exact state transitions
exact restoration behavior
exact constraints
exact event path
```

Those are not interchangeable levels of information.

For ports, documentation should route the agent to source, not replace source.

## 4.4 I chased breadth before visual/default fidelity

The failed track spent releases on:

- EWMH surface expansion;
- gravity/moveresize;
- RandR;
- fullscreen monitor hints;
- shaded state;
- show-desktop;
- winoptions subset;
- modifier mapping;
- Freedesktop launcher semantics;
- signalfd lifecycle;
- differential tooling.

These are legitimate features.

But while they were being added, the fundamental visible IceWM identity still differed.

SourcePort 0.1.3/0.1.4 instead attacked the visible fixture directly:

```text
frame origin
classic colors
border/title metrics
bevel semantics
title controls
real application icons
taskbar composition
workspace projection
battery indicator
root menu
theme resources
```

That produced much higher user-visible ROI per turn.

## 4.5 I introduced new behavior that upstream did not require

Any custom convenience behavior in a parity phase is suspicious.

SourcePort explicitly removed its previous right-drag-titlebar resize shortcut after proving stock IceWM uses titlebar button 3 for the system menu.

That is exactly the discipline a port needs:

> A SourcePort-specific replacement is a defect when the source already defines the behavior.

The failed track was more willing to retain custom aliases or simplified shell behavior as transitional architecture.

Transitional behavior compounds divergence.

## 4.6 I underused upstream resources and rendering code

Visual compatibility depends heavily on assets, metrics and small drawing rules.

The failed line initially hand-built shell visuals.

SourcePort copied the behavior of `Graphics::drawBorderW`, loaded stock XPM resources, used source-derived metrics, and applied the correct `preferences -> theme -> prefoverride` precedence.

That is why it could visually converge without recreating IceWM's entire renderer first.

## 4.7 I treated static audits as too meaningful

The final 0.0.9 static audit currently reports:

```text
STATIC X11 PASS
STATIC RANDR PASS
STATIC ICEWM 4.1.0 ALIGNMENT PASS
STATIC SOURCE PASS
```

Yet the user reports that the archive still has a Rust compile failure.

This proves the central limitation:

> Static source heuristics cannot be a compiler surrogate.

The failure is more serious because I knew the delivery environment lacked Cargo/Rust and still sealed a release described as fixed.

That violated the required evidence standard.

## 4.8 I shipped without the mandatory compile gate

This is the clearest process failure.

The `0.0.9` project's own `scripts/check.sh` correctly requires `cargo` before it can run tests/build.

But the release process bypassed that truth because the artifact environment lacked Rust.

The correct response should have been:

```text
UNVERIFIED SOURCE SNAPSHOT
not a fixed/working release
```

or no release artifact at all until a compiler-backed gate was available.

Environment limitation explains why verification could not run.

It does not justify changing the definition of “working.”

## 4.9 I overvalued test quantity relative to test placement

0.0.9 declares 76 Rust tests; SourcePort Stage 3 declares 10.

The winning project still converged faster.

The lesson is not “write fewer tests.”

The lesson is:

> Tests must sit on the parity boundary that matters.

A source-port test hierarchy should be:

1. compiler;
2. exact source/provenance checks;
3. owner-level pure behavior tests;
4. runtime protocol tests;
5. stock-vs-port differential behavior;
6. stock-vs-port visual capture;
7. only then broader target-language internal tests.

A large suite around a divergent architecture can protect the divergence very effectively.

## 4.10 I let “minimal clean Rust architecture” compete with the actual objective

The project objective was not:

> design a nice small Rust WM inspired by IceWM.

It was:

> port IceWM.

That objective distinction must dominate architecture decisions from turn 1.

---

# 5. What the failed track did right and should be retained

The post-mortem should not discard useful engineering.

These parts were directionally correct and belong in the improved SourcePort process.

## 5.1 Exact target-version oracle

Correctly rejecting distro IceWM 4.0.0 as a strict oracle for an IceWM 4.1.0 port was right.

Keep exact version pinning.

## 5.2 One authoritative managed-client state

Avoiding multiple independent window registries was correct.

A translated architecture should still preserve the original authority model and avoid accidental duplicate state.

## 5.3 FFI symbol auditing

Checking that declared Xlib/XRandR/XRender symbols exist is useful.

Keep it as a preflight, never as compile proof.

## 5.4 Xvfb semantic smoke testing

Headless runtime tests for EWMH/ICCCM behavior are valuable.

Keep them after the compile gate.

## 5.5 Exact differential fixtures

Comparing the same clients/config against stock and Rust is correct.

Move this much earlier in development and add visual evidence.

## 5.6 Explicit parity boundaries

Both projects correctly avoid claiming full IceWM parity.

Keep explicit `MISSING/PARTIAL/PROMOTED` status.

## 5.7 Minimal external dependency policy

Direct system FFI is appropriate for this low-level port when it matches upstream's dependency surface.

But dependency minimization must not become a reason to redesign semantics.

---

# 6. The new canonical methodology: Identity-Preserving SourcePort

This methodology applies to whole-project ports and to individual feature ports from another project.

## Phase 0 — Freeze the oracle

Record:

```text
upstream repository/archive
exact version/tag/commit
archive/tree SHA-256
license
build command
runtime fixture
reference binary provenance
```

No coding before the oracle is immutable.

## Phase 1 — Exact source mirror

For every source unit in scope:

```text
copy exact file bytes
preserve relative path identity
rename only extension/location required by staging
record SHA-256
```

For C/C++ → Rust:

```text
Foo.h / Foo.cc
-> stage1/Foo_h.rs / Foo_cc.rs
```

or another deterministic mapping.

Do not redesign class boundaries.

## Phase 2 — Mechanical semantic translation

Translate syntax while preserving structure.

Preserve by default:

```text
class/struct names
method names
function names
constants/enums
branch order
loop order
call order
error ordering
state fields
initial values
resource names
configuration keys
side effects
comments that explain quirks
```

Replace language mechanics only:

```text
pointer/reference representation
constructors/destructors
RAII/drop
containers
casts
strings
function pointers/callbacks
preprocessor branches
inheritance/traits/composition
FFI types
```

If semantics are not yet known, write an explicit noncompiled marker such as:

```text
CXX_UNCONVERTED: <original statement>
```

Do not invent behavior to make the translator look complete.

## Phase 3 — Dependency-closure promotion

Choose one upstream owner or tightly coupled source slice.

Determine its exact dependency closure.

Promote that closure into compiled target-language code.

Rules:

```text
original owner remains named in target code/provenance
one source behavior -> one translated authority
no feature redesign
no unrelated refactor
small compatibility shim allowed only at language/ABI boundary
```

## Phase 4 — Compile immediately

After every promoted slice:

```text
target compiler
unit tests for translated pure logic
linker
```

No next feature while compilation is red.

No packaging while compilation is unexecuted.

## Phase 5 — Differential runtime

Run the original and translated implementations with the same:

```text
input
configuration
environment geometry
test clients
resources
```

Compare observable state.

For a WM:

```text
root properties
client properties
frame geometry
focus
stacking
workspaces
window actions
process lifecycle
```

## Phase 6 — Visual differential

For visual software, screenshot comparison is mandatory.

Use identical:

```text
screen dimensions
fonts
theme
client windows
window geometry
configuration
```

Classify every visible delta and map it to the exact upstream owner.

Do not “polish” the port independently during parity.

## Phase 7 — Repeat by largest parity delta

Pick the next tranche by observed divergence, not by novelty.

Priority:

```text
compile/runtime blockers
-> default visible behavior
-> default interaction behavior
-> protocol/edge behavior
-> optional subsystems
-> auxiliary tools
```

## Phase 8 — Parity freeze

When the required surface matches:

```text
freeze differential fixtures
freeze screenshots/hashes where appropriate
freeze source map
freeze compatibility tests
```

## Phase 9 — Idiomatic refactor only now

Refactor one behavior-preserving area at a time.

Every refactor must pass the same differential oracle.

Only here may the port:

```text
merge modules
rename target-language APIs
introduce safer ownership abstractions
replace raw FFI wrappers
simplify duplicated structure
adopt idiomatic target-language patterns
```

The source structure stops being mandatory only after parity is protected by evidence.

---

# 7. Feature-port variant: copying one feature from another project

The same rule applies when not porting a whole project.

Bad feature-port workflow:

```text
read feature
understand concept
reimplement concept in our architecture
```

Canonical workflow:

```text
find exact source owner
-> find direct callers
-> find transitive helpers/types/constants
-> find resources/config/defaults
-> find lifecycle registration
-> find tests
-> copy that minimum complete closure into a staging/reference area
-> translate semantics
-> adapt only platform-specific edges
-> wire to our lifecycle
-> compare source behavior
-> then refactor into our architecture if desired
```

The copied closure must include hidden contracts such as:

```text
defaults
error cases
ordering
state reset
reload
shutdown
resource lookup
thread/event-loop ownership
persistence
```

“Copy the class” therefore means **copy the behavioral closure**, not only the visible method.

---

# 8. Compile/release law learned from 0.0.9

The following is now non-negotiable.

## 8.1 Release status vocabulary

```text
SOURCE_DRAFT
COMPILES
TESTED
RUNTIME_VERIFIED
DIFFERENTIAL_VERIFIED
RELEASE
```

A project may only claim a state whose gate actually executed.

## 8.2 No compiler, no release

If target compiler is unavailable:

```text
allowed: SOURCE_DRAFT / UNVERIFIED artifact
forbidden: FIXED / WORKING / RELEASE
```

Static audits may detect:

```text
missing symbols
stubs
version drift
obvious delimiter damage
source-anchor drift
```

They cannot establish:

```text
Rust grammar correctness
type correctness
borrow/lifetime correctness
trait resolution
link correctness
warnings under #![deny(warnings)]
```

## 8.3 Same-gate repair

When Cargo fails:

```text
capture exact compiler output
-> repair only proven error
-> rerun same Cargo command
-> repeat until green
```

Do not broaden feature work while compilation is red.

---

# 9. Improved SourcePort architecture for future projects

SourcePort 0.1.4 is the winning direction, but it can be made even stronger.

## 9.1 Preserve 1:1 compiled units longer

Current SourcePort Stage 3 still consolidates many promoted behaviors into `wm.rs` and `shell.rs`.

For larger future ports, prefer compiling translated modules closer to the original owners:

```text
wmframe.rs
wmclient.rs
wmmgr.rs
wmtaskbar.rs
atasks.rs
aworkspaces.rs
...
```

A compatibility facade can later merge APIs if useful.

This reduces the same structural entropy risk before it returns.

## 9.2 Generate symbol maps, not only file maps

Extend the source map with:

```text
original class/function
source line/span or stable anchor
target symbol
translation status
runtime status
test/differential proof
```

Example:

```text
src/ypaint.cc :: Graphics::drawBorderW
-> classic.rs :: ClassicLook::draw_border_w
STATUS=RUNTIME_PROMOTED
PROOF=visual/classic-border
```

## 9.3 Use compile errors as translation tasks

Compile failure should be treated as a deterministic queue:

```text
missing import
missing type
ownership mismatch
ABI mismatch
language semantic mismatch
unconverted dependency
```

Each error should resolve toward the original source contract, not trigger redesign.

## 9.4 Expand differential automation

For nonvisual behavior, compare machine-readable traces/properties.

For visual behavior, add screenshot image-diff thresholds plus human review for allowed nondeterminism.

The C2Rust cross-checking model validates this general approach: translated and original variants should be compared directly because translation/refactoring can change semantics even when source looks reasonable.

## 9.5 Keep explicit staging

Do not delete the exact source mirror after translation.

It is valuable for:

```text
future parity work
upstream version upgrades
regression archaeology
agent context
source-owner lookup
license/provenance
```

---

# 10. Anti-pattern checklist — automatically reject these during a parity port

Reject a proposed patch if it does any of the following before parity freeze:

- Replaces multiple mature upstream classes with a “cleaner” new abstraction without necessity.
- Implements a feature from a report instead of reading its exact source owner.
- Renames core domain concepts simply to be more idiomatic.
- Changes defaults because the target-language design seems better.
- Reorders side effects/error handling without proof.
- Draws replacement UI assets when upstream assets can be reused.
- Adds convenience interaction that stock does not have.
- Adds broad new protocol/features while the default visible fixture still differs.
- Counts tests/LOC as parity progress.
- Calls static source lint a compile gate.
- Packages a release that has not been compiled.
- Calls an unexecuted runtime path “should work.”
- Removes the original source corpus after translation.
- Lets an agent decide architecture from memory when exact source is available.

---

# 11. New decision heuristic

When asked to port X from Project A to Project B/language C, classify the task first.

## PARITY PORT

Use SourcePort by default when:

- the user wants a copy/port/replacement;
- exact behavior matters;
- source is available and legally reusable;
- the source is mature/large/quirky;
- hidden compatibility behavior matters;
- visual sameness matters.

Default rule:

```text
COPY -> TRANSLATE -> WIRE -> COMPILE -> DIFFERENTIAL -> REFACTOR
```

## PRODUCT REIMPLEMENTATION

Architecture-first design is acceptable only when the user explicitly wants a new behavior/model rather than source parity, or source cannot legally/technically be carried over.

Even then, exact source remains evidence for edge behavior where compatibility is required.

---

# 12. Revised success metric for future porting turns

Every turn should report a parity delta ledger.

Example:

```text
UPSTREAM OWNER       TARGET OWNER       BEFORE         AFTER           PROOF
wmframe.cc           wmframe.rs         wrong origin   exact origin    differential geometry
wmclient.cc          wmclient.rs        no app icon    WM_HINTS icon   screenshot + property
aworkspaces.cc       aworkspaces.rs     wrong scale    exact integer   screenshot
```

Turn quality is measured by:

```text
number/severity of proven deltas eliminated
÷
amount of new divergence introduced
```

This is a much better objective than “features implemented this release.”

---

# 13. Final self-assessment

## What I did wrong

1. I optimized the Rust architecture before preserving IceWM architecture.
2. I collapsed source ownership and increased lookup/translation cost.
3. I used source reports as too much of an implementation intermediary.
4. I implemented breadth before default visual parity.
5. I recreated resources/semantics that should have been translated or reused.
6. I allowed static audits to create false confidence.
7. I knowingly packaged an artifact without a compiler-backed gate.
8. I protected my rewritten architecture with many tests instead of first protecting source equivalence.
9. I treated each parity gap as a feature to implement rather than a source behavior to translate.

## What was right and remains useful

1. Exact IceWM version pinning.
2. Direct X11 ABI attention.
3. One WM state authority.
4. Runtime Xvfb testing.
5. Exact differential oracle work.
6. Explicit parity-gap documentation.
7. Minimal dependency preference.

## What changes permanently

For parity-oriented ports, my default reasoning is now:

```text
Do not ask: “How would I design this in the target language?”
Ask:        “What is the exact source owner, and what is the smallest faithful target-language transcription?”
```

The target language is initially an implementation substrate, not permission to reinterpret the product.

---

# 14. Canonical SourcePort law

Use this as the compact memory/skill rule:

> **SourcePort Law:** When porting an existing project or feature and parity matters, preserve identity before improving architecture. Freeze the exact source; mirror the exact owning files/classes; mechanically translate syntax and language semantics while retaining names, control flow, constants, defaults, side effects, resources and ordering; explicitly mark unresolved semantics; wire the smallest dependency closure until it compiles; compare it against the exact original using the same runtime fixture and visual/state oracle; repair deltas at their source owner; only after parity is protected may target-language refactoring change structure. Never ship a “working” port without executing the target compiler and required runtime gate.

---

# 15. External validation

This post-mortem is primarily grounded in the two supplied project archives. Independent tooling literature supports the same core migration order:

- **C2Rust Manual — Introduction:** C2Rust's initial translation intentionally closely mirrors the C source and prioritizes functional identity rather than safe/idiomatic Rust; gradual refactoring comes later. `https://c2rust.com/manual/`
- **C2Rust Cross-checking Tutorial:** translated and original variants are compared because both translation and later refactoring can alter semantics. `https://c2rust.com/manual/docs/cross-check-tutorial.html`
- **Rust FFI documentation:** cross-language/platform boundaries require ABI-compatible types and explicit unsafe contracts; this supports isolating FFI adaptation as a translation boundary rather than redesigning product behavior around it. `https://doc.rust-lang.org/core/ffi/`

C2Rust targets C99 rather than C++, so it is not a drop-in IceWM translator. Its relevance here is methodological: **preserve semantics first, refactor later, and cross-check original against translation.**

---

# 16. Status of the winning track

SourcePort 0.1.4 is not complete IceWM 4.1.0 parity, and its own documentation correctly says so.

Its current explicit remaining families include:

- complete theme/font/image machinery;
- full menu/programs semantics;
- Xft/Unicode/BiDi;
- XIM/XKB;
- full WM_NORMAL_HINTS/focus/transient/group behavior;
- full EWMH/window-state surface;
- RandR multi-output/workarea parity;
- QuickSwitch previews;
- XEmbed tray and remaining applets;
- wallpaper/session/helpers/i18n;
- full preferences/keys/toolbar/winoptions compatibility.

That incompleteness does not weaken the post-mortem conclusion.

It strengthens it:

> A smaller incomplete implementation that preserves upstream identity and already matches the visible original is a substantially better base for completing the port than a broader reimplementation that has accumulated architectural divergence and compile instability.

