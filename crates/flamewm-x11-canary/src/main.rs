//! Xephyr canary harness: locate Flame surfaces, query geometry/map state,
//! send synthetic button/motion events, sample pixels, emit JSON/text.
//!
//! Read-only probe plus synthetic input; never touches product renderers.
//! Scenarios: SR05 first-present, SR07 popup matrix, SR08 black-pixel,
//! SR15 input matrix.

use std::collections::VecDeque;
use std::env;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use x11rb::CURRENT_TIME;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, BUTTON_PRESS_EVENT, ButtonPressEvent, ConnectionExt, EventMask, ImageFormat,
    KeyButMask, MOTION_NOTIFY_EVENT, MapState, Motion, MotionNotifyEvent, Window,
};
use x11rb::rust_connection::RustConnection;

const BUTTON_RELEASE_EVENT: u8 = 5;

struct Canary {
    conn: RustConnection,
    screen: usize,
    root: Window,
}

#[derive(Debug, Clone)]
struct WinInfo {
    id: u32,
    parent: u32,
    x: i16,
    y: i16,
    width: u16,
    height: u16,
    map_state: u8,
    name: String,
    class: String,
}

impl WinInfo {
    fn map_name(&self) -> &'static str {
        match self.map_state {
            0 => "unmapped",
            1 => "unviewable",
            2 => "viewable",
            _ => "unknown",
        }
    }
    fn to_json(&self) -> String {
        format!(
            "{{\"id\":{},\"parent\":{},\"x\":{},\"y\":{},\"width\":{},\"height\":{},\"map_state\":\"{}\",\"name\":{},\"class\":{}}}",
            self.id,
            self.parent,
            self.x,
            self.y,
            self.width,
            self.height,
            self.map_name(),
            json_str(&self.name),
            json_str(&self.class),
        )
    }
}

fn json_str(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn fail(message: &str) -> ! {
    eprintln!("CANARY_FAIL {message}");
    std::process::exit(1);
}

impl Canary {
    fn connect() -> Result<Self, String> {
        let (conn, screen) = x11rb::connect(None).map_err(|e| e.to_string())?;
        let root = conn.setup().roots[screen].root;
        Ok(Self { conn, screen, root })
    }

    fn prop_bytes(&self, window: Window, atom: Atom) -> Vec<u8> {
        self.conn
            .get_property(false, window, atom, AtomEnum::ANY, 0, u32::MAX)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.value)
            .unwrap_or_default()
    }

    fn text_prop(&self, window: Window, names: &[&str]) -> String {
        for name in names {
            let atom = self
                .conn
                .intern_atom(false, name.as_bytes())
                .ok()
                .and_then(|c| c.reply().ok())
                .map(|r| r.atom);
            if let Some(atom) = atom {
                let bytes = self.prop_bytes(window, atom);
                if !bytes.is_empty() {
                    // WM_CLASS is NUL-separated instance\0class\0; use class part.
                    let text = if *name == "WM_CLASS" {
                        bytes
                            .split(|b| *b == 0)
                            .filter(|p| !p.is_empty())
                            .last()
                            .map_or(String::new(), |p| String::from_utf8_lossy(p).into_owned())
                    } else {
                        String::from_utf8_lossy(&bytes)
                            .trim_end_matches('\0')
                            .to_owned()
                    };
                    if !text.is_empty() {
                        return text;
                    }
                }
            }
        }
        String::new()
    }

    fn info(&self, window: Window, parent: Window) -> Option<WinInfo> {
        let geom = self.conn.get_geometry(window).ok()?.reply().ok()?;
        let attrs = self.conn.get_window_attributes(window).ok()?.reply().ok()?;
        let name = self.text_prop(window, &["_NET_WM_NAME", "WM_NAME"]);
        let class = self.text_prop(window, &["WM_CLASS"]);
        Some(WinInfo {
            id: window,
            parent,
            x: geom.x,
            y: geom.y,
            width: geom.width,
            height: geom.height,
            map_state: u8::from(attrs.map_state),
            name,
            class,
        })
    }

    fn walk(&self, parent: Window, out: &mut Vec<WinInfo>) {
        let tree = match self.conn.query_tree(parent) {
            Ok(c) => match c.reply() {
                Ok(t) => t,
                Err(_) => return,
            },
            Err(_) => return,
        };
        for child in tree.children {
            if let Some(info) = self.info(child, parent) {
                out.push(info);
            }
            self.walk(child, out);
        }
    }

    fn all(&self) -> Vec<WinInfo> {
        let mut out = Vec::new();
        self.walk(self.root, &mut out);
        out
    }

    fn find(&self, name: &str) -> Vec<WinInfo> {
        let needle = name.to_lowercase();
        self.all()
            .into_iter()
            .filter(|w| {
                w.name.to_lowercase().contains(&needle) || w.class.to_lowercase().contains(&needle)
            })
            .collect()
    }

    fn by_id(&self, id: u32) -> Option<WinInfo> {
        let mut queue = VecDeque::from([self.root]);
        while let Some(parent) = queue.pop_front() {
            let tree = self.conn.query_tree(parent).ok()?.reply().ok()?;
            for child in tree.children {
                if child == id {
                    return self.info(child, parent);
                }
                queue.push_back(child);
            }
        }
        None
    }

    fn sample(&self, drawable: Window, x: i16, y: i16, w: u16, h: u16) -> Result<Vec<u8>, String> {
        let reply = self
            .conn
            .get_image(ImageFormat::Z_PIXMAP, drawable, x, y, w, h, u32::MAX)
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?;
        Ok(reply.data)
    }

    fn pixel_stats(data: &[u8]) -> (bool, u64, u8, u8, u8) {
        if data.is_empty() {
            return (true, 0, 0, 0, 0);
        }
        // Assume 4 bytes/pixel (24/32-bit ZPixmap); fall back to raw mean.
        let mut sum_r: u64 = 0;
        let mut sum_g: u64 = 0;
        let mut sum_b: u64 = 0;
        let mut n: u64 = 0;
        let mut all_zero = true;
        // Try native-endian 32-bit pixels, BGRX order typical for X.
        let mut i = 0;
        while i + 4 <= data.len() {
            let px = u32::from_ne_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
            if px != 0 {
                all_zero = false;
            }
            sum_b += u64::from((px & 0xff) as u8);
            sum_g += u64::from(((px >> 8) & 0xff) as u8);
            sum_r += u64::from(((px >> 16) & 0xff) as u8);
            n += 1;
            i += 4;
        }
        if n == 0 {
            let sum: u64 = data.iter().map(|b| u64::from(*b)).sum();
            all_zero = sum == 0;
            n = data.len() as u64;
            sum_r = sum;
        }
        let mean_r = u8::try_from(sum_r / n.max(1)).unwrap_or(255);
        let mean_g = u8::try_from(sum_g / n.max(1)).unwrap_or(255);
        let mean_b = u8::try_from(sum_b / n.max(1)).unwrap_or(255);
        (all_zero, n, mean_r, mean_g, mean_b)
    }

    fn send_button(&self, window: Window, x: i16, y: i16, press: bool) -> Result<(), String> {
        let geom = self
            .conn
            .get_geometry(window)
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?;
        let ev = ButtonPressEvent {
            response_type: if press {
                BUTTON_PRESS_EVENT
            } else {
                BUTTON_RELEASE_EVENT
            },
            detail: 1,
            sequence: 0,
            time: CURRENT_TIME,
            root: self.root,
            event: window,
            child: x11rb::NONE,
            root_x: geom.x.saturating_add(x),
            root_y: geom.y.saturating_add(y),
            event_x: x,
            event_y: y,
            state: KeyButMask::default(),
            same_screen: true,
        };
        self.conn
            .send_event(false, window, EventMask::BUTTON_PRESS, ev)
            .map_err(|e| e.to_string())?
            .check()
            .map_err(|e| e.to_string())?;
        self.conn.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn send_motion(&self, window: Window, x: i16, y: i16) -> Result<(), String> {
        let geom = self
            .conn
            .get_geometry(window)
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?;
        let ev = MotionNotifyEvent {
            response_type: MOTION_NOTIFY_EVENT,
            detail: Motion::NORMAL,
            sequence: 0,
            time: CURRENT_TIME,
            root: self.root,
            event: window,
            child: x11rb::NONE,
            root_x: geom.x.saturating_add(x),
            root_y: geom.y.saturating_add(y),
            event_x: x,
            event_y: y,
            state: KeyButMask::default(),
            same_screen: true,
        };
        self.conn
            .send_event(false, window, EventMask::POINTER_MOTION, ev)
            .map_err(|e| e.to_string())?
            .check()
            .map_err(|e| e.to_string())?;
        self.conn.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn reply_str(e: impl ToString) -> String {
    e.to_string()
}

fn arg(args: &[String], flag: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == flag {
            return it.next().cloned();
        }
        if let Some(rest) = a.strip_prefix(&format!("{flag}=")) {
            return Some(rest.to_owned());
        }
    }
    None
}

fn has(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn usage() -> ! {
    eprintln!(
        "flamewm-x11-canary <command> [options]\n\
         commands:\n  \
         list [--name <substr>] [--json]\n  \
         geometry --name <substr> [--json] | --window <id> [--json]\n  \
         map-state --name <substr> [--json] | --window <id>\n  \
         click --name <substr> | --window <id> [--x <n> --y <n>]\n  \
         motion --name <substr> | --window <id> [--x <n> --y <n>]\n  \
         sample --window <id> [--x <n> --y <n> --w <n> --h <n>] [--json]\n  \
         sample-root [--w <n> --h <n>] [--json]\n  \
         wait-mapped --name <substr> [--timeout-secs <n>]\n  \
         sr05 [--name <substr>] [--timeout-secs <n>]\n  \
         sr07 [--parent <substr>] [--json]\n  \
         sr08 [--window <id> | --name <substr>] [--json]\n  \
         sr15 [--name <substr> | --window <id>] [--points x,y;...] [--json]"
    );
    std::process::exit(2);
}

fn resolve_target(canary: &Canary, args: &[String]) -> WinInfo {
    if let Some(id) = arg(args, "--window") {
        let id: u32 = id
            .parse()
            .unwrap_or_else(|_| fail("--window must be a number"));
        return canary
            .by_id(id)
            .unwrap_or_else(|| fail(&format!("window {id} not found")));
    }
    if let Some(name) = arg(args, "--name") {
        let mut hits = canary.find(&name);
        if hits.is_empty() {
            fail(&format!("no window matches {name:?}"));
        }
        hits.sort_by_key(|w| !(w.map_state == u8::from(MapState::VIEWABLE)));
        return hits.remove(0);
    }
    fail("need --window <id> or --name <substr>");
}

fn cmd_list(canary: &Canary, args: &[String]) {
    let wins = match arg(args, "--name") {
        Some(n) => canary.find(&n),
        None => canary.all(),
    };
    if has(args, "--json") {
        let items: Vec<String> = wins.iter().map(WinInfo::to_json).collect();
        println!("{{\"windows\":[{}]}}", items.join(","));
    } else {
        for w in &wins {
            println!(
                "win id={} parent={} geom={}x{}+{}+{} map={} name={:?} class={:?}",
                w.id,
                w.parent,
                w.width,
                w.height,
                w.x,
                w.y,
                w.map_name(),
                w.name,
                w.class
            );
        }
        println!("CANARY_LIST count={}", wins.len());
    }
}

fn cmd_geometry(canary: &Canary, args: &[String]) {
    let target = resolve_target(canary, args);
    if has(args, "--json") {
        println!("{}", target.to_json());
    } else {
        println!(
            "CANARY_GEOMETRY id={} geom={}x{}+{}+{} map={} name={:?}",
            target.id,
            target.width,
            target.height,
            target.x,
            target.y,
            target.map_name(),
            target.name
        );
    }
}

fn cmd_map_state(canary: &Canary, args: &[String]) {
    let target = resolve_target(canary, args);
    if has(args, "--json") {
        println!(
            "{{\"id\":{},\"map_state\":\"{}\",\"viewable\":{}}}",
            target.id,
            target.map_name(),
            target.map_state == u8::from(MapState::VIEWABLE)
        );
    } else {
        println!(
            "CANARY_MAP_STATE id={} map={} viewable={}",
            target.id,
            target.map_name(),
            target.map_state == u8::from(MapState::VIEWABLE)
        );
    }
}

fn cmd_click(canary: &Canary, args: &[String]) {
    let target = resolve_target(canary, args);
    let x: i16 = arg(args, "--x")
        .map(|v| v.parse().unwrap_or_else(|_| fail("--x must be a number")))
        .unwrap_or((target.width / 2) as i16);
    let y: i16 = arg(args, "--y")
        .map(|v| v.parse().unwrap_or_else(|_| fail("--y must be a number")))
        .unwrap_or((target.height / 2) as i16);
    canary
        .send_button(target.id, x, y, true)
        .unwrap_or_else(|e| fail(&e));
    canary
        .send_button(target.id, x, y, false)
        .unwrap_or_else(|e| fail(&e));
    println!("CANARY_CLICK id={} x={x} y={y} ok=true", target.id);
}

fn cmd_motion(canary: &Canary, args: &[String]) {
    let target = resolve_target(canary, args);
    let x: i16 = arg(args, "--x")
        .map(|v| v.parse().unwrap_or_else(|_| fail("--x must be a number")))
        .unwrap_or((target.width / 2) as i16);
    let y: i16 = arg(args, "--y")
        .map(|v| v.parse().unwrap_or_else(|_| fail("--y must be a number")))
        .unwrap_or((target.height / 2) as i16);
    canary
        .send_motion(target.id, x, y)
        .unwrap_or_else(|e| fail(&e));
    println!("CANARY_MOTION id={} x={x} y={y} ok=true", target.id);
}

fn print_sample(window: u32, x: i16, y: i16, w: u16, h: u16, data: &[u8], json: bool) {
    let (all_zero, pixels, mr, mg, mb) = Canary::pixel_stats(data);
    if json {
        println!(
            "{{\"window\":{window},\"x\":{x},\"y\":{y},\"width\":{w},\"height\":{h},\
             \"pixels\":{pixels},\"all_zero\":{all_zero},\
             \"mean_r\":{mr},\"mean_g\":{mg},\"mean_b\":{mb},\"bytes\":{}}}",
            data.len()
        );
    } else {
        println!(
            "CANARY_SAMPLE window={window} rect={w}x{h}+{x}+{y} pixels={pixels} \
             all_zero={all_zero} mean=#{mr:02x}{mg:02x}{mb:02x} bytes={}",
            data.len()
        );
    }
}

fn cmd_sample(canary: &Canary, args: &[String]) {
    let target = resolve_target(canary, args);
    let w: u16 = arg(args, "--w")
        .map(|v| v.parse().unwrap_or_else(|_| fail("--w must be a number")))
        .unwrap_or(target.width.min(64));
    let h: u16 = arg(args, "--h")
        .map(|v| v.parse().unwrap_or_else(|_| fail("--h must be a number")))
        .unwrap_or(target.height.min(64));
    let x: i16 = arg(args, "--x")
        .map(|v| v.parse().unwrap_or_else(|_| fail("--x must be a number")))
        .unwrap_or(0);
    let y: i16 = arg(args, "--y")
        .map(|v| v.parse().unwrap_or_else(|_| fail("--y must be a number")))
        .unwrap_or(0);
    let data = canary
        .sample(target.id, x, y, w.max(1), h.max(1))
        .unwrap_or_else(|e| fail(&e));
    print_sample(target.id, x, y, w, h, &data, has(args, "--json"));
}

fn cmd_sample_root(canary: &Canary, args: &[String]) {
    let setup = &canary.conn.setup().roots[canary.screen];
    let sw = setup.width_in_pixels;
    let sh = setup.height_in_pixels;
    let w: u16 = arg(args, "--w")
        .map(|v| v.parse().unwrap_or_else(|_| fail("--w must be a number")))
        .unwrap_or(sw.min(256));
    let h: u16 = arg(args, "--h")
        .map(|v| v.parse().unwrap_or_else(|_| fail("--h must be a number")))
        .unwrap_or(sh.min(256));
    let data = canary
        .sample(canary.root, 0, 0, w.max(1), h.max(1))
        .unwrap_or_else(|e| fail(&e));
    print_sample(canary.root, 0, 0, w, h, &data, has(args, "--json"));
}

/// SR05 first-present: wait until a named surface is mapped/viewable.
fn cmd_sr05(canary: &Canary, args: &[String]) {
    let name = arg(args, "--name").unwrap_or_else(|| "flame".to_owned());
    let timeout: u64 = arg(args, "--timeout-secs")
        .map(|v| {
            v.parse()
                .unwrap_or_else(|_| fail("--timeout-secs must be a number"))
        })
        .unwrap_or(10);
    let start = Instant::now();
    let limit = Duration::from_secs(timeout.max(1));
    loop {
        let hits = canary.find(&name);
        let viewable = hits
            .iter()
            .filter(|w| w.map_state == u8::from(MapState::VIEWABLE))
            .count();
        if viewable > 0 {
            let first = &hits
                .iter()
                .find(|w| w.map_state == u8::from(MapState::VIEWABLE))
                .cloned()
                .unwrap_or_else(|| hits[0].clone());
            if has(args, "--json") {
                println!(
                    "{{\"scenario\":\"SR05\",\"pass\":true,\"name\":{},\"matched\":{},\
                     \"viewable\":{},\"first\":{},\"elapsed_ms\":{}}}",
                    json_str(&name),
                    hits.len(),
                    viewable,
                    first.to_json(),
                    start.elapsed().as_millis()
                );
            } else {
                println!(
                    "CANARY_SR05 pass=true name={:?} matched={} viewable={} \
                     first_id={} geom={}x{}+{}+{} elapsed_ms={}",
                    name,
                    hits.len(),
                    viewable,
                    first.id,
                    first.width,
                    first.height,
                    first.x,
                    first.y,
                    start.elapsed().as_millis()
                );
            }
            return;
        }
        if start.elapsed() >= limit {
            if has(args, "--json") {
                println!(
                    "{{\"scenario\":\"SR05\",\"pass\":false,\"name\":{},\"matched\":{},\
                     \"viewable\":0,\"elapsed_ms\":{}}}",
                    json_str(&name),
                    hits.len(),
                    start.elapsed().as_millis()
                );
            } else {
                println!(
                    "CANARY_SR05 pass=false name={:?} matched={} viewable=0 elapsed_ms={}",
                    name,
                    hits.len(),
                    start.elapsed().as_millis()
                );
            }
            std::process::exit(1);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// SR07 popup matrix: enumerate transient/override-redirect-style small
/// top-level windows under root and report geometry/map matrix.
fn cmd_sr07(canary: &Canary, args: &[String]) {
    let tree = canary
        .conn
        .query_tree(canary.root)
        .map_err(|e| e.to_string())
        .unwrap_or_else(|e| fail(&e))
        .reply()
        .map_err(|e| e.to_string())
        .unwrap_or_else(|e| fail(&e));
    let mut rows: Vec<WinInfo> = Vec::new();
    for child in tree.children {
        if let Some(info) = canary.info(child, canary.root) {
            let small = info.width <= 600 && info.height <= 600;
            if small || info.map_state != u8::from(MapState::VIEWABLE) {
                rows.push(info);
            }
        }
    }
    if let Some(parent) = arg(args, "--parent") {
        let needle = parent.to_lowercase();
        rows.retain(|w| {
            w.name.to_lowercase().contains(&needle) || w.class.to_lowercase().contains(&needle)
        });
    }
    let viewable = rows
        .iter()
        .filter(|w| w.map_state == u8::from(MapState::VIEWABLE))
        .count();
    if has(args, "--json") {
        let items: Vec<String> = rows.iter().map(WinInfo::to_json).collect();
        println!(
            "{{\"scenario\":\"SR07\",\"popups\":{},\"viewable\":{},\"matrix\":[{}]}}",
            rows.len(),
            viewable,
            items.join(",")
        );
    } else {
        for w in &rows {
            println!(
                "popup id={} geom={}x{}+{}+{} map={} name={:?}",
                w.id,
                w.width,
                w.height,
                w.x,
                w.y,
                w.map_name(),
                w.name
            );
        }
        println!("CANARY_SR07 popups={} viewable={}", rows.len(), viewable);
    }
}

/// SR08 black-pixel: sample root (tiled) or a named window; fail on all-black.
fn cmd_sr08(canary: &Canary, args: &[String]) {
    let json = has(args, "--json");
    let mut samples: Vec<(u32, i16, i16, u16, u16, Vec<u8>)> = Vec::new();
    if let Some(id) = arg(args, "--window") {
        let id: u32 = id
            .parse()
            .unwrap_or_else(|_| fail("--window must be a number"));
        let info = canary
            .by_id(id)
            .unwrap_or_else(|| fail(&format!("window {id} not found")));
        let w = info.width.max(1).min(256);
        let h = info.height.max(1).min(256);
        let data = canary.sample(id, 0, 0, w, h).unwrap_or_else(|e| fail(&e));
        samples.push((id, 0, 0, w, h, data));
    } else if let Some(name) = arg(args, "--name") {
        let target = resolve_target(canary, &vec!["x".to_owned(), "--name".to_owned(), name]);
        let w = target.width.max(1).min(256);
        let h = target.height.max(1).min(256);
        let data = canary
            .sample(target.id, 0, 0, w, h)
            .unwrap_or_else(|e| fail(&e));
        samples.push((target.id, 0, 0, w, h, data));
    } else {
        // Tile root: 4 corners + center at 64x64.
        let setup = &canary.conn.setup().roots[canary.screen];
        let sw = i32::from(setup.width_in_pixels);
        let sh = i32::from(setup.height_in_pixels);
        let tile: i16 = 64;
        let points = [
            (0, 0),
            (sw.saturating_sub(64) as i16, 0),
            (0, sh.saturating_sub(64) as i16),
            (sw.saturating_sub(64) as i16, sh.saturating_sub(64) as i16),
            ((sw / 2 - 32) as i16, (sh / 2 - 32) as i16),
        ];
        for (x, y) in points {
            match canary.sample(canary.root, x.max(0), y.max(0), tile as u16, tile as u16) {
                Ok(data) => samples.push((canary.root, x, y, 64, 64, data)),
                Err(e) => fail(&e),
            }
        }
    }
    let mut all_zero_total = 0;
    for (win, x, y, w, h, data) in &samples {
        let (all_zero, pixels, mr, mg, mb) = Canary::pixel_stats(data);
        if all_zero {
            all_zero_total += 1;
        }
        if json {
            println!(
                "{{\"scenario\":\"SR08\",\"window\":{win},\"x\":{x},\"y\":{y},\
                 \"width\":{w},\"height\":{h},\"pixels\":{pixels},\
                 \"all_zero\":{all_zero},\"mean_r\":{mr},\"mean_g\":{mg},\"mean_b\":{mb}}}"
            );
        } else {
            println!(
                "CANARY_SR08 window={win} rect={w}x{h}+{x}+{y} pixels={pixels} \
                 all_zero={all_zero} mean=#{mr:02x}{mg:02x}{mb:02x}"
            );
        }
    }
    let pass = all_zero_total < samples.len();
    if json {
        println!(
            "{{\"scenario\":\"SR08\",\"pass\":{pass},\"samples\":{},\"all_zero_tiles\":{all_zero_total}}}",
            samples.len()
        );
    } else {
        println!(
            "CANARY_SR08 pass={pass} samples={} all_zero_tiles={all_zero_total}",
            samples.len()
        );
    }
    if !pass {
        std::process::exit(1);
    }
}

/// SR15 input matrix: warp-free synthetic motion grid + click per point on
/// the target window; reports per-point delivery.
fn cmd_sr15(canary: &Canary, args: &[String]) {
    let target = resolve_target(canary, args);
    let points: Vec<(i16, i16)> = match arg(args, "--points") {
        Some(spec) => spec
            .split(';')
            .filter(|p| !p.trim().is_empty())
            .map(|p| {
                let mut it = p.split(',');
                let x: i16 = it
                    .next()
                    .unwrap_or("0")
                    .trim()
                    .parse()
                    .unwrap_or_else(|_| fail("--points must be x,y;..."));
                let y: i16 = it
                    .next()
                    .unwrap_or("0")
                    .trim()
                    .parse()
                    .unwrap_or_else(|_| fail("--points must be x,y;..."));
                (x, y)
            })
            .collect(),
        None => {
            // 3x3 grid over the window.
            let w = i32::from(target.width.max(1));
            let h = i32::from(target.height.max(1));
            let mut pts = Vec::new();
            for ry in [1, 2, 3] {
                for rx in [1, 2, 3] {
                    pts.push(((w * rx / 4) as i16, (h * ry / 4) as i16));
                }
            }
            pts
        }
    };
    let mut ok = 0;
    let mut results: Vec<String> = Vec::new();
    for (x, y) in &points {
        let m = canary.send_motion(target.id, *x, *y);
        let c = if m.is_ok() {
            canary
                .send_button(target.id, *x, *y, true)
                .and_then(|()| canary.send_button(target.id, *x, *y, false))
        } else {
            Err("motion failed".to_owned())
        };
        let pass = c.is_ok();
        if pass {
            ok += 1;
        } else if let Err(e) = c {
            eprintln!("CANARY_SR15 point x={x} y={y} deliver_failed: {e}");
        }
        results.push(format!("{{\"x\":{x},\"y\":{y},\"ok\":{pass}}}"));
    }
    if has(args, "--json") {
        println!(
            "{{\"scenario\":\"SR15\",\"window\":{},\"points\":{},\"delivered\":{},\"matrix\":[{}]}}",
            target.id,
            points.len(),
            ok,
            results.join(",")
        );
    } else {
        println!(
            "CANARY_SR15 window={} points={} delivered={}",
            target.id,
            points.len(),
            ok
        );
    }
    if ok == 0 {
        std::process::exit(1);
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage();
    }
    let cmd = args[1].clone();
    let rest = args[1..].to_vec();
    // wait-mapped is an alias of sr05 with require semantics.
    let canary = match Canary::connect() {
        Ok(c) => c,
        Err(e) => fail(&format!("X connect failed: {e}")),
    };
    match cmd.as_str() {
        "list" => cmd_list(&canary, &rest),
        "geometry" => cmd_geometry(&canary, &rest),
        "map-state" => cmd_map_state(&canary, &rest),
        "click" => cmd_click(&canary, &rest),
        "motion" => cmd_motion(&canary, &rest),
        "sample" => cmd_sample(&canary, &rest),
        "sample-root" => cmd_sample_root(&canary, &rest),
        "wait-mapped" | "sr05" => cmd_sr05(&canary, &rest),
        "sr07" => cmd_sr07(&canary, &rest),
        "sr08" => cmd_sr08(&canary, &rest),
        "sr15" => cmd_sr15(&canary, &rest),
        _ => usage(),
    }
    ExitCode::SUCCESS
}
