#![allow(static_mut_refs)]
use std::{
    env,
    ffi::OsStr,
    fs,
    io::{BufRead, BufReader},
    os::windows::{ffi::OsStrExt, process::CommandExt},
    path::PathBuf,
    process::{Child, Command, Stdio},
    ptr::{null, null_mut},
    thread,
};
type H = *mut core::ffi::c_void;

// ───────────────────────────── Win32 / GDI+ FFI ─────────────────────────────
#[repr(C)]
struct WndClass {
    style: u32,
    proc: Option<unsafe extern "system" fn(H, u32, usize, isize) -> isize>,
    cls_extra: i32,
    wnd_extra: i32,
    instance: H,
    icon: H,
    cursor: H,
    background: H,
    menu: *const u16,
    name: *const u16,
}
#[repr(C)]
struct Msg {
    hwnd: H,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    x: i32,
    y: i32,
    private: u32,
}
#[repr(C)]
struct Guid {
    a: u32,
    b: u16,
    c: u16,
    d: [u8; 8],
}
#[repr(C)]
struct NotifyIcon {
    size: u32,
    hwnd: H,
    id: u32,
    flags: u32,
    callback: u32,
    icon: H,
    tip: [u16; 128],
    state: u32,
    state_mask: u32,
    info: [u16; 256],
    timeout: u32,
    info_title: [u16; 64],
    info_flags: u32,
    guid: Guid,
    balloon_icon: H,
}
#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[repr(C)]
struct Paint {
    dc: H,
    erase: i32,
    rect: Rect,
    restore: i32,
    inc_update: i32,
    reserved: [u8; 32],
}
#[repr(C)]
struct BmiHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bpp: u16,
    compression: u32,
    size_image: u32,
    xppm: i32,
    yppm: i32,
    used: u32,
    important: u32,
}
#[repr(C)]
struct TrackMouse {
    size: u32,
    flags: u32,
    hwnd: H,
    hover: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct PointF {
    x: f32,
    y: f32,
}
#[repr(C)]
struct RectF {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
#[repr(C)]
struct GdipInput {
    version: u32,
    callback: *const core::ffi::c_void,
    no_thread: i32,
    no_codecs: i32,
}

#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(h: H, attribute: u32, value: *const i32, size: u32) -> i32;
}
#[link(name = "shell32")]
extern "system" {
    fn Shell_NotifyIconW(message: u32, data: *mut NotifyIcon) -> i32;
    fn SetCurrentProcessExplicitAppUserModelID(id: *const u16) -> i32;
}
#[link(name = "user32")]
extern "system" {
    fn AdjustWindowRectExForDpi(rect: *mut Rect, style: u32, menu: i32, ex: u32, dpi: u32) -> i32;
    fn GetDpiForWindow(h: H) -> u32;
    fn GetDpiForSystem() -> u32;
    fn SetProcessDpiAwarenessContext(v: isize) -> i32;
    fn RegisterClassW(c: *const WndClass) -> u16;
    fn IsWindowVisible(h: H) -> i32;
    fn IsIconic(h: H) -> i32;
    fn EnumWindows(
        callback: Option<unsafe extern "system" fn(H, isize) -> i32>,
        data: isize,
    ) -> i32;
    fn GetWindowThreadProcessId(h: H, pid: *mut u32) -> u32;
    fn SendMessageTimeoutW(
        h: H,
        msg: u32,
        w: usize,
        l: isize,
        flags: u32,
        timeout: u32,
        result: *mut usize,
    ) -> isize;
    fn EnableWindow(h: H, enable: i32) -> i32;
    fn SetFocus(h: H) -> H;
    fn SetCapture(h: H) -> H;
    fn ReleaseCapture() -> i32;
    fn GetKeyState(key: i32) -> i16;
    fn TrackMouseEvent(t: *mut TrackMouse) -> i32;
    fn SetCursor(c: H) -> H;
    fn SetWindowPos(h: H, after: H, x: i32, y: i32, w: i32, hh: i32, flags: u32) -> i32;
    fn GetClientRect(h: H, r: *mut Rect) -> i32;
    fn DestroyIcon(icon: H) -> i32;
    fn InvalidateRect(h: H, rect: *const Rect, erase: i32) -> i32;
    fn BeginPaint(h: H, p: *mut Paint) -> H;
    fn EndPaint(h: H, p: *const Paint) -> i32;
    fn FindWindowW(class: *const u16, title: *const u16) -> H;
    fn PostMessageW(h: H, m: u32, w: usize, l: isize) -> i32;
    fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: H,
        menu: H,
        inst: H,
        param: H,
    ) -> H;
    fn DefWindowProcW(h: H, m: u32, w: usize, l: isize) -> isize;
    fn ShowWindow(h: H, n: i32) -> i32;
    fn UpdateWindow(h: H) -> i32;
    fn GetMessageW(m: *mut Msg, h: H, min: u32, max: u32) -> i32;
    fn TranslateMessage(m: *const Msg) -> i32;
    fn DispatchMessageW(m: *const Msg) -> isize;
    fn PostQuitMessage(n: i32);
    fn GetWindowTextW(h: H, s: *mut u16, max: i32) -> i32;
    fn MessageBoxW(h: H, s: *const u16, t: *const u16, flags: u32) -> i32;
    fn KillTimer(h: H, id: usize) -> i32;
    fn SetTimer(h: H, id: usize, ms: u32, cb: H) -> usize;
    fn SendMessageW(h: H, m: u32, w: usize, l: isize) -> isize;
    fn LoadCursorW(h: H, id: *const u16) -> H;
    fn LoadImageW(h: H, name: *const u16, kind: u32, cx: i32, cy: i32, flags: u32) -> H;
    fn DestroyWindow(h: H) -> i32;
    fn SetForegroundWindow(h: H) -> i32;
    fn GetCursorPos(p: *mut Point) -> i32;
    fn CreatePopupMenu() -> H;
    fn AppendMenuW(menu: H, flags: u32, id: usize, text: *const u16) -> i32;
    fn TrackPopupMenu(
        menu: H,
        flags: u32,
        x: i32,
        y: i32,
        reserved: i32,
        window: H,
        rect: H,
    ) -> i32;
    fn DestroyMenu(menu: H) -> i32;
    fn RegisterWindowMessageW(name: *const u16) -> u32;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> H;
    fn GetTickCount64() -> u64;
    fn GetUserDefaultUILanguage() -> u16;
}
#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(dc: H) -> H;
    fn CreateDIBSection(
        dc: H,
        bmi: *const BmiHeader,
        usage: u32,
        bits: *mut *mut core::ffi::c_void,
        section: H,
        offset: u32,
    ) -> H;
    fn BitBlt(dst: H, x: i32, y: i32, w: i32, h: i32, src: H, sx: i32, sy: i32, rop: u32) -> i32;
    fn CreateSolidBrush(color: u32) -> H;
    fn SelectObject(dc: H, obj: H) -> H;
    fn DeleteObject(obj: H) -> i32;
    fn SetTextColor(dc: H, color: u32) -> u32;
    fn SetBkColor(dc: H, color: u32) -> u32;
    fn CreateFontW(
        height: i32,
        width: i32,
        esc: i32,
        orient: i32,
        weight: i32,
        italic: u32,
        under: u32,
        strike: u32,
        charset: u32,
        out: u32,
        clip: u32,
        quality: u32,
        pitch: u32,
        face: *const u16,
    ) -> H;
}
#[link(name = "gdiplus")]
extern "system" {
    fn GdiplusStartup(token: *mut usize, input: *const GdipInput, output: *mut H) -> i32;
    fn GdipCreateFromHDC(dc: H, g: *mut H) -> i32;
    fn GdipDeleteGraphics(g: H) -> i32;
    fn GdipSetSmoothingMode(g: H, m: i32) -> i32;
    fn GdipSetTextRenderingHint(g: H, m: i32) -> i32;
    fn GdipSetInterpolationMode(g: H, m: i32) -> i32;
    fn GdipSetPixelOffsetMode(g: H, m: i32) -> i32;
    fn GdipSetClipRect(g: H, x: f32, y: f32, w: f32, h: f32, mode: i32) -> i32;
    fn GdipFillRectangle(g: H, brush: H, x: f32, y: f32, w: f32, h: f32) -> i32;
    fn GdipSaveGraphics(g: H, state: *mut u32) -> i32;
    fn GdipRestoreGraphics(g: H, state: u32) -> i32;
    fn GdipScaleWorldTransform(g: H, x: f32, y: f32, order: i32) -> i32;
    fn GdipTranslateWorldTransform(g: H, x: f32, y: f32, order: i32) -> i32;
    fn GdipSetClipPath(g: H, path: H, mode: i32) -> i32;
    fn GdipCreateSolidFill(color: u32, brush: *mut H) -> i32;
    fn GdipDeleteBrush(brush: H) -> i32;
    fn GdipCreateLineBrush(
        p1: *const PointF,
        p2: *const PointF,
        c1: u32,
        c2: u32,
        wrap: i32,
        brush: *mut H,
    ) -> i32;
    fn GdipCreatePen1(color: u32, width: f32, unit: i32, pen: *mut H) -> i32;
    fn GdipDeletePen(pen: H) -> i32;
    fn GdipSetPenLineCap197819(pen: H, start: i32, end: i32, dash: i32) -> i32;
    fn GdipSetPenLineJoin(pen: H, join: i32) -> i32;
    fn GdipCreatePath(mode: i32, path: *mut H) -> i32;
    fn GdipDeletePath(path: H) -> i32;
    fn GdipAddPathArc(path: H, x: f32, y: f32, w: f32, h: f32, start: f32, sweep: f32) -> i32;
    fn GdipAddPathLine(path: H, x1: f32, y1: f32, x2: f32, y2: f32) -> i32;
    fn GdipClosePathFigure(path: H) -> i32;
    fn GdipFillPath(g: H, brush: H, path: H) -> i32;
    fn GdipDrawPath(g: H, pen: H, path: H) -> i32;
    fn GdipFillEllipse(g: H, brush: H, x: f32, y: f32, w: f32, h: f32) -> i32;
    fn GdipDrawEllipse(g: H, pen: H, x: f32, y: f32, w: f32, h: f32) -> i32;
    fn GdipDrawArc(g: H, pen: H, x: f32, y: f32, w: f32, h: f32, start: f32, sweep: f32) -> i32;
    fn GdipDrawLine(g: H, pen: H, x1: f32, y1: f32, x2: f32, y2: f32) -> i32;
    fn GdipFillPolygon(g: H, brush: H, pts: *const PointF, n: i32, mode: i32) -> i32;
    fn GdipCreateFontFamilyFromName(name: *const u16, coll: H, family: *mut H) -> i32;
    fn GdipCreateFont(family: H, size: f32, style: i32, unit: i32, font: *mut H) -> i32;
    fn GdipStringFormatGetGenericTypographic(fmt: *mut H) -> i32;
    fn GdipCloneStringFormat(fmt: H, out: *mut H) -> i32;
    fn GdipSetStringFormatAlign(fmt: H, a: i32) -> i32;
    fn GdipSetStringFormatLineAlign(fmt: H, a: i32) -> i32;
    fn GdipSetStringFormatFlags(fmt: H, flags: i32) -> i32;
    fn GdipDrawString(
        g: H,
        s: *const u16,
        len: i32,
        font: H,
        rect: *const RectF,
        fmt: H,
        brush: H,
    ) -> i32;
    fn GdipMeasureString(
        g: H,
        s: *const u16,
        len: i32,
        font: H,
        rect: *const RectF,
        fmt: H,
        bounds: *mut RectF,
        cp: *mut i32,
        lines: *mut i32,
    ) -> i32;
    fn GdipCreateBitmapFromHICON(icon: H, bmp: *mut H) -> i32;
    fn GdipCreateBitmapFromScan0(
        w: i32,
        h: i32,
        stride: i32,
        fmt: i32,
        scan0: *mut u8,
        bmp: *mut H,
    ) -> i32;
    fn GdipCreateImageAttributes(a: *mut H) -> i32;
    fn GdipDisposeImageAttributes(a: H) -> i32;
    fn GdipSetImageAttributesColorMatrix(
        a: H,
        kind: i32,
        enable: i32,
        m: *const f32,
        gray: *const f32,
        flags: i32,
    ) -> i32;
    fn GdipDrawImageRectRect(
        g: H,
        img: H,
        dx: f32,
        dy: f32,
        dw: f32,
        dh: f32,
        sx: f32,
        sy: f32,
        sw: f32,
        sh: f32,
        unit: i32,
        attrs: H,
        cb: H,
        data: H,
    ) -> i32;
}

// ───────────────────────────── Theme palette ─────────────────────────────
const BG: usize = 0;
const CARD: usize = 1;
const P_INPUT: usize = 2;
const BORDER: usize = 3;
const TEXT: usize = 4;
const SEC: usize = 5;
const TER: usize = 6;
const ACC_A: usize = 7;
const ACC_B: usize = 8;
const GREEN: usize = 9;
const RED: usize = 10;
const NEUTRAL: usize = 11;
const ACC_TXT: usize = 12;
const LIGHT: [u32; 13] = [
    0xF5F5F7, 0xFFFFFF, 0xF2F2F7, 0xE3E3E8, 0x1D1D1F, 0x6E6E73, 0xA1A1A6, 0x7D6FF5, 0x5B4BDD,
    0x34C759, 0xFF3B30, 0xDCDCE3, 0x5B4BDD,
];
const DARK_P: [u32; 13] = [
    0x0B0B0D, 0x1C1C1E, 0x2C2C2E, 0x2F2F33, 0xF5F5F7, 0xA1A1A6, 0x6E6E73, 0x9A8EFF, 0x6C5CF0,
    0x30D158, 0xFF453A, 0x3A3A3F, 0xA79DFF,
];

// ───────────────────────────── UI state ─────────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum Status {
    Off,
    Starting,
    Waiting,
    Connected,
    Failed,
}
struct Ui {
    s: f32,
    dark: f32,
    dark_t: f32,
    hover_btn: f32,
    hover_tog: f32,
    press: f32,
    field: f32,
    running: f32,
    a_start: f32,
    a_wait: f32,
    a_conn: f32,
    a_fail: f32,
    enter: f32,
    phase: f64,
    last: u64,
    timer: bool,
    interval: u32,
    partial: bool,
    status: Status,
    prev_text: (String, String),
    lang_pos: f32,
    lang_t: f32,
    lang_fade: f32,
    over_seg: i32,
    status_fade: f32,
    over_btn: bool,
    over_tog: bool,
    press_item: i32,
    tracking: bool,
    focus: i32,
    kbd: bool,
    edit_shown: bool,
    name: String,
    was_ready: bool,
}
static mut UI: Ui = Ui {
    s: 1.0,
    dark: 0.0,
    dark_t: 0.0,
    hover_btn: 0.0,
    hover_tog: 0.0,
    press: 0.0,
    field: 0.0,
    running: 0.0,
    a_start: 0.0,
    a_wait: 0.0,
    a_conn: 0.0,
    a_fail: 0.0,
    enter: 0.0,
    phase: 0.0,
    last: 0,
    timer: false,
    interval: 16,
    partial: false,
    status: Status::Off,
    prev_text: (String::new(), String::new()),
    lang_pos: 0.0,
    lang_t: 0.0,
    lang_fade: 1.0,
    over_seg: -1,
    status_fade: 1.0,
    over_btn: false,
    over_tog: false,
    press_item: 0,
    tracking: false,
    focus: 1,
    kbd: false,
    edit_shown: false,
    name: String::new(),
    was_ready: false,
};

const WIN_W: f32 = 380.0;
const WIN_H: f32 = 668.0;
const BTN: (f32, f32, f32, f32) = (20.0, 404.0, 340.0, 52.0);
const TOG: (f32, f32, f32, f32) = (304.0, 33.0, 56.0, 32.0);
// Region around the status indicator that changes while the receiver is running.
const AMBIENT: (f32, f32, f32, f32) = (114.0, 92.0, 152.0, 144.0);
const SEG: (f32, f32, f32, f32) = (82.0, 628.0, 216.0, 28.0);
const FIELD: (f32, f32, f32, f32) = (20.0, 340.0, 340.0, 48.0);

static mut DARK: bool = false;
static mut PARTIAL: bool = false;
static mut GENERATION: usize = 0;
static mut RECEIVER_READY: bool = false;
static mut VIDEO_WINDOW: H = null_mut();
static mut ICON: H = null_mut();
static mut TRAY_READY: bool = false;
static mut TASKBAR_CREATED: u32 = 0;
static mut CHILD: Option<Child> = None;
static mut INPUT: H = null_mut();
static mut EDIT_FONT: H = null_mut();
static mut EDIT_BRUSH: H = null_mut();
static mut EDIT_BRUSH_COLOR: u32 = 0xFFFFFFFF;
static mut BB_DC: H = null_mut();
static mut BB_BMP: H = null_mut();
static mut BB_W: i32 = 0;
static mut BB_H: i32 = 0;
// GDI+ resources
static mut F_TITLE: H = null_mut();
static mut F_HERO: H = null_mut();
static mut F_BTN: H = null_mut();
static mut F_BODY: H = null_mut();
static mut F_SMALL: H = null_mut();
static mut F_BADGE: H = null_mut();
static mut FMT_L: H = null_mut();
static mut FMT_C: H = null_mut();
static mut FMT_R: H = null_mut();
static mut FMT_W: H = null_mut();
static mut IMG_HEAD: H = null_mut();
static mut IMG_GLOW: H = null_mut();

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}
fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let mut r = 0;
    for sh in [0, 8, 16] {
        let x = ((a >> sh) & 255) as f32;
        let y = ((b >> sh) & 255) as f32;
        r |= ((x + (y - x) * t).round() as u32) << sh;
    }
    r
}
fn argb(rgb: u32, a: f32) -> u32 {
    (((a.clamp(0.0, 1.0) * 255.0).round() as u32) << 24) | (rgb & 0xFFFFFF)
}
fn bgr(rgb: u32) -> u32 {
    ((rgb & 255) << 16) | (rgb & 0xFF00) | ((rgb >> 16) & 255)
}
fn ease_out(t: f32) -> f32 {
    let t = 1.0 - t.clamp(0.0, 1.0);
    1.0 - t * t * t
}
unsafe fn pc(i: usize) -> u32 {
    mix(LIGHT[i], DARK_P[i], UI.dark)
}
fn approach(v: &mut f32, target: f32, dt: f32, k: f32) -> bool {
    *v += (target - *v) * (1.0 - (-k * dt).exp());
    if (target - *v).abs() < 0.003 {
        *v = target;
    }
    *v != target
}

// ───────────────────────────── GDI+ helpers ─────────────────────────────
unsafe fn rr_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> H {
    let r = r.min(w / 2.0).min(h / 2.0);
    let d = r * 2.0;
    let mut p = null_mut();
    GdipCreatePath(0, &mut p);
    GdipAddPathArc(p, x, y, d, d, 180.0, 90.0);
    GdipAddPathArc(p, x + w - d, y, d, d, 270.0, 90.0);
    GdipAddPathArc(p, x + w - d, y + h - d, d, d, 0.0, 90.0);
    GdipAddPathArc(p, x, y + h - d, d, d, 90.0, 90.0);
    GdipClosePathFigure(p);
    p
}
unsafe fn fill_rr(g: H, x: f32, y: f32, w: f32, h: f32, r: f32, color: u32) {
    let p = rr_path(x, y, w, h, r);
    let mut b = null_mut();
    GdipCreateSolidFill(color, &mut b);
    GdipFillPath(g, b, p);
    GdipDeleteBrush(b);
    GdipDeletePath(p);
}
unsafe fn grad_rr(g: H, x: f32, y: f32, w: f32, h: f32, r: f32, top: u32, bottom: u32) {
    let p = rr_path(x, y, w, h, r);
    let (a, b2) = (PointF { x, y }, PointF { x, y: y + h });
    let mut b = null_mut();
    GdipCreateLineBrush(&a, &b2, top, bottom, 0, &mut b);
    GdipFillPath(g, b, p);
    GdipDeleteBrush(b);
    GdipDeletePath(p);
}
unsafe fn stroke_rr(g: H, x: f32, y: f32, w: f32, h: f32, r: f32, color: u32, width: f32) {
    let p = rr_path(x, y, w, h, r);
    let mut pen = null_mut();
    GdipCreatePen1(color, width, 0, &mut pen);
    GdipDrawPath(g, pen, p);
    GdipDeletePen(pen);
    GdipDeletePath(p);
}
unsafe fn fill_circle(g: H, cx: f32, cy: f32, r: f32, color: u32) {
    let mut b = null_mut();
    GdipCreateSolidFill(color, &mut b);
    GdipFillEllipse(g, b, cx - r, cy - r, r * 2.0, r * 2.0);
    GdipDeleteBrush(b);
}
unsafe fn pen(color: u32, width: f32) -> H {
    let mut p = null_mut();
    GdipCreatePen1(color, width, 0, &mut p);
    GdipSetPenLineCap197819(p, 2, 2, 2);
    GdipSetPenLineJoin(p, 2);
    p
}
unsafe fn stroke_circle(g: H, cx: f32, cy: f32, r: f32, color: u32, width: f32) {
    let p = pen(color, width);
    GdipDrawEllipse(g, p, cx - r, cy - r, r * 2.0, r * 2.0);
    GdipDeletePen(p);
}
unsafe fn shadow(g: H, x: f32, y: f32, w: f32, h: f32, r: f32, rgb: u32, alpha: f32, dy: f32) {
    for i in 0..9 {
        let e = i as f32 * 2.2;
        let a = alpha * 0.07 * (1.0 - i as f32 / 9.0);
        fill_rr(
            g,
            x - e,
            y - e + dy,
            w + e * 2.0,
            h + e * 2.0,
            r + e,
            argb(rgb, a),
        );
    }
}
unsafe fn text(g: H, s: &str, x: f32, y: f32, w: f32, h: f32, font: H, color: u32, fmt: H) {
    let mut b = null_mut();
    GdipCreateSolidFill(color, &mut b);
    let t = wide(s);
    let r = RectF { x, y, w, h };
    GdipDrawString(g, t.as_ptr(), (t.len() - 1) as i32, font, &r, fmt, b);
    GdipDeleteBrush(b);
}
unsafe fn measure(g: H, s: &str, font: H) -> f32 {
    let t = wide(s);
    let r = RectF {
        x: 0.0,
        y: 0.0,
        w: 1000.0,
        h: 100.0,
    };
    let mut o = RectF {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    };
    GdipMeasureString(
        g,
        t.as_ptr(),
        (t.len() - 1) as i32,
        font,
        &r,
        FMT_L,
        &mut o,
        null_mut(),
        null_mut(),
    );
    o.w
}
unsafe fn image(g: H, im: H, x: f32, y: f32, w: f32, h: f32, size: f32, a: f32) {
    image_tint(g, im, x, y, w, h, size, a, 0xFFFFFF);
}
unsafe fn image_tint(g: H, im: H, x: f32, y: f32, w: f32, h: f32, size: f32, a: f32, rgb: u32) {
    if im.is_null() || a <= 0.003 {
        return;
    }
    let mut at = null_mut();
    GdipCreateImageAttributes(&mut at);
    let (r, gr, b) = (
        ((rgb >> 16) & 255) as f32 / 255.0,
        ((rgb >> 8) & 255) as f32 / 255.0,
        (rgb & 255) as f32 / 255.0,
    );
    let m: [f32; 25] = [
        r, 0.0, 0.0, 0.0, 0.0, 0.0, gr, 0.0, 0.0, 0.0, 0.0, 0.0, b, 0.0, 0.0, 0.0, 0.0, 0.0, a,
        0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    GdipSetImageAttributesColorMatrix(at, 0, 1, m.as_ptr(), null(), 0);
    GdipDrawImageRectRect(
        g,
        im,
        x,
        y,
        w,
        h,
        0.0,
        0.0,
        size,
        size,
        2,
        at,
        null_mut(),
        null_mut(),
    );
    GdipDisposeImageAttributes(at);
}
unsafe fn font(name: &str, size: f32) -> H {
    let mut fam = null_mut();
    GdipCreateFontFamilyFromName(wide(name).as_ptr(), null_mut(), &mut fam);
    let mut f = null_mut();
    GdipCreateFont(fam, size, 0, 2, &mut f);
    f
}
unsafe fn format(align: i32, wrap: bool) -> H {
    let mut base = null_mut();
    GdipStringFormatGetGenericTypographic(&mut base);
    let mut f = null_mut();
    GdipCloneStringFormat(base, &mut f);
    GdipSetStringFormatAlign(f, align);
    GdipSetStringFormatLineAlign(f, 0);
    GdipSetStringFormatFlags(f, if wrap { 0x4000 } else { 0x4000 | 0x1000 });
    f
}
unsafe fn init_gdip() {
    let mut token = 0usize;
    let input = GdipInput {
        version: 1,
        callback: null(),
        no_thread: 0,
        no_codecs: 0,
    };
    GdiplusStartup(&mut token, &input, null_mut());
    F_TITLE = font("Segoe UI Semibold", 24.0);
    F_HERO = font("Segoe UI Semibold", 19.0);
    F_BTN = font("Segoe UI Semibold", 16.0);
    F_BODY = font("Segoe UI", 13.5);
    F_SMALL = font("Segoe UI", 12.0);
    F_BADGE = font("Segoe UI Semibold", 12.0);
    FMT_L = format(0, false);
    FMT_C = format(1, false);
    FMT_R = format(2, false);
    FMT_W = format(1, true);
    let icon = LoadImageW(GetModuleHandleW(null()), 1usize as *const u16, 1, 96, 96, 0);
    if !icon.is_null() {
        GdipCreateBitmapFromHICON(icon, &mut IMG_HEAD);
        DestroyIcon(icon);
    }
    // Radial falloff mask for the ambient glow, tinted at draw time.
    let n = 240usize;
    let mut px = vec![0u32; n * n];
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (x as f32 + 0.5 - 120.0, y as f32 + 0.5 - 120.0);
            let a =
                ((1.0 - (dx * dx + dy * dy).sqrt() / 120.0).clamp(0.0, 1.0) * 255.0).round() as u32;
            px[y * n + x] = (a << 24) | (a << 16) | (a << 8) | a;
        }
    }
    let scan0 = Box::leak(px.into_boxed_slice()).as_mut_ptr() as *mut u8;
    GdipCreateBitmapFromScan0(
        n as i32,
        n as i32,
        (n * 4) as i32,
        0x000E200B,
        scan0,
        &mut IMG_GLOW,
    );
}

#[derive(Clone, Copy, PartialEq)]
enum Lang {
    Ru = 0,
    En = 1,
    De = 2,
}
const LANGS: [Lang; 3] = [Lang::Ru, Lang::En, Lang::De];
const LANG_NAMES: [&str; 3] = ["Русский", "English", "Deutsch"];
#[derive(Clone, Copy)]
enum Str {
    Subtitle,
    FieldLabel,
    Start,
    Stop,
    StepsTitle,
    Step1,
    Step2,
    FeatScreen,
    FeatLocal,
    FeatFps,
    OffTitle,
    OffHint,
    StartingTitle,
    StartingHint,
    WaitTitle,
    WaitHint,
    ConnTitle,
    ConnHint,
    StoppedTitle,
    StoppedHint,
    FailTitle,
    FailHint,
    ErrTitle,
    ErrText,
    TrayTip,
    VideoTitle,
    TrayOpen,
    TrayExit,
}
static mut LANG: Lang = Lang::Ru;
/// Translation table: Russian, English, German.
fn tr(k: Str) -> &'static str {
    let t: [&str; 3] = match k {
        Str::Subtitle => [
            "Повтор экрана с iOS",
            "iOS screen mirroring",
            "iOS-Bildschirmspiegelung",
        ],
        Str::FieldLabel => [
            "Имя устройства в AirPlay",
            "Device name in AirPlay",
            "Gerätename in AirPlay",
        ],
        Str::Start => ["Включить AirPlay", "Start AirPlay", "AirPlay starten"],
        Str::Stop => ["Остановить приём", "Stop receiving", "Empfang beenden"],
        Str::StepsTitle => ["Как подключиться", "How to connect", "So verbinden Sie"],
        Str::Step1 => [
            "Откройте «Повтор экрана» в Пункте управления",
            "Open Screen Mirroring in Control Center",
            "Kontrollzentrum → Bildschirmsynchronisierung",
        ],
        Str::Step2 => [
            "Выберите устройство — имя указано выше",
            "Pick the device — its name is shown above",
            "Gerät wählen – der Name steht oben",
        ],
        Str::FeatScreen => ["Экран и звук", "Screen & audio", "Bild & Ton"],
        Str::FeatLocal => ["Локально", "Local network", "Lokal"],
        Str::FeatFps => ["до 60 FPS", "up to 60 FPS", "bis 60 FPS"],
        Str::OffTitle => ["Приём выключен", "Receiver off", "Empfang aus"],
        Str::OffHint => [
            "Нажмите «{}», чтобы начать",
            "Click “{}” to begin",
            "Klicken Sie auf „{}“, um zu beginnen",
        ],
        Str::StartingTitle => [
            "Запуск приёмника…",
            "Starting receiver…",
            "Empfänger startet…",
        ],
        Str::StartingHint => [
            "Это займёт пару секунд",
            "This takes a few seconds",
            "Das dauert einen Moment",
        ],
        Str::WaitTitle => ["Ожидание iPhone", "Waiting for iPhone", "Warte auf iPhone"],
        Str::WaitHint => [
            "Откройте «Повтор экрана» в Пункте управления и выберите «{}»",
            "Open Screen Mirroring in Control Center and choose “{}”",
            "Kontrollzentrum → Bildschirmsynchronisierung → „{}“",
        ],
        Str::ConnTitle => [
            "Экран подключён",
            "Screen connected",
            "Bildschirm verbunden",
        ],
        Str::ConnHint => [
            "Изображение и звук передаются на этот компьютер",
            "Picture and sound are streaming to this computer",
            "Bild und Ton werden auf diesen Computer übertragen",
        ],
        Str::StoppedTitle => [
            "Приёмник остановлен",
            "Receiver stopped",
            "Empfänger angehalten",
        ],
        Str::StoppedHint => [
            "Нажмите «{}», чтобы запустить снова",
            "Click “{}” to start again",
            "Klicken Sie auf „{}“, um neu zu starten",
        ],
        Str::FailTitle => [
            "Не удалось запустить приёмник",
            "Could not start the receiver",
            "Empfänger konnte nicht starten",
        ],
        Str::FailHint => [
            "Нажмите «{}», чтобы повторить",
            "Click “{}” to try again",
            "Klicken Sie auf „{}“, um es erneut zu versuchen",
        ],
        Str::ErrTitle => ["Ошибка", "Error", "Fehler"],
        Str::ErrText => [
            "Не удалось запустить приёмник: {}",
            "Could not start the receiver: {}",
            "Empfänger konnte nicht gestartet werden: {}",
        ],
        Str::TrayTip => [
            "Aireluma — приём экрана iPhone",
            "Aireluma — iPhone screen receiver",
            "Aireluma — iPhone-Bildschirmempfang",
        ],
        Str::VideoTitle => [
            "Aireluma — экран iPhone",
            "Aireluma — iPhone screen",
            "Aireluma — iPhone-Bildschirm",
        ],
        Str::TrayOpen => ["Открыть Aireluma", "Open Aireluma", "Aireluma öffnen"],
        Str::TrayExit => ["Выйти", "Quit", "Beenden"],
    };
    t[unsafe { LANG } as usize]
}
fn language_path() -> PathBuf {
    PathBuf::from(env::var_os("APPDATA").unwrap_or_default())
        .join("Aireluma")
        .join("language")
}
/// Saved choice first, then the Windows display language, otherwise English.
fn load_lang() -> Lang {
    match fs::read_to_string(language_path())
        .ok()
        .as_deref()
        .map(str::trim)
    {
        Some("ru") => Lang::Ru,
        Some("en") => Lang::En,
        Some("de") => Lang::De,
        _ => match unsafe { GetUserDefaultUILanguage() } & 0x3ff {
            0x19 => Lang::Ru,
            0x07 => Lang::De,
            _ => Lang::En,
        },
    }
}
unsafe fn set_lang(h: H, l: Lang) {
    if LANG == l {
        return;
    }
    UI.prev_text = status_text(UI.status);
    LANG = l;
    UI.status_fade = 0.0;
    UI.lang_fade = 0.0;
    UI.lang_t = l as i32 as f32;
    let path = language_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, ["ru", "en", "de"][l as usize]);
    VIDEO_WINDOW = null_mut();
    if TRAY_READY {
        tray(h, 1);
    }
    kick(h);
}
// Localized text fades back in after a language switch.
unsafe fn lfade() -> f32 {
    0.2 + 0.8 * ease_out(UI.lang_fade)
}

// ───────────────────────────── Status text ─────────────────────────────
fn status_text(s: Status) -> (String, String) {
    unsafe {
        let start = tr(Str::Start);
        match s {
            Status::Off => (
                tr(Str::OffTitle).into(),
                tr(Str::OffHint).replace("{}", start),
            ),
            Status::Starting => (tr(Str::StartingTitle).into(), tr(Str::StartingHint).into()),
            Status::Waiting => (
                tr(Str::WaitTitle).into(),
                tr(Str::WaitHint).replace("{}", &UI.name),
            ),
            Status::Connected => (tr(Str::ConnTitle).into(), tr(Str::ConnHint).into()),
            Status::Failed if UI.was_ready => (
                tr(Str::StoppedTitle).into(),
                tr(Str::StoppedHint).replace("{}", start),
            ),
            Status::Failed => (
                tr(Str::FailTitle).into(),
                tr(Str::FailHint).replace("{}", start),
            ),
        }
    }
}
unsafe fn set_status(h: H, s: Status) {
    if UI.status != s {
        UI.prev_text = status_text(UI.status);
        UI.status = s;
        UI.status_fade = 0.0;
    }
    kick(h);
}

// ───────────────────────────── Rendering ─────────────────────────────
unsafe fn sec(g: H, i: usize) -> (f32, u32) {
    let p = ease_out((UI.enter - i as f32 * 0.075) / 0.65);
    let mut st = 0;
    GdipSaveGraphics(g, &mut st);
    GdipTranslateWorldTransform(g, 0.0, (1.0 - p) * 18.0, 0);
    (p, st)
}
unsafe fn glyph(g: H, cx: f32, cy: f32, k: f32, color: u32) {
    let mut p = null_mut();
    GdipCreatePath(0, &mut p);
    GdipAddPathLine(p, cx - 7.0 * k, cy + 6.0 * k, cx - 9.0 * k, cy + 6.0 * k);
    GdipAddPathArc(p, cx - 13.0 * k, cy - 2.0 * k, 8.0 * k, 8.0 * k, 90.0, 90.0);
    GdipAddPathLine(p, cx - 13.0 * k, cy + 2.0 * k, cx - 13.0 * k, cy - 10.0 * k);
    GdipAddPathArc(
        p,
        cx - 13.0 * k,
        cy - 14.0 * k,
        8.0 * k,
        8.0 * k,
        180.0,
        90.0,
    );
    GdipAddPathLine(p, cx - 9.0 * k, cy - 14.0 * k, cx + 9.0 * k, cy - 14.0 * k);
    GdipAddPathArc(
        p,
        cx + 5.0 * k,
        cy - 14.0 * k,
        8.0 * k,
        8.0 * k,
        270.0,
        90.0,
    );
    GdipAddPathLine(p, cx + 13.0 * k, cy - 10.0 * k, cx + 13.0 * k, cy + 2.0 * k);
    GdipAddPathArc(p, cx + 5.0 * k, cy - 2.0 * k, 8.0 * k, 8.0 * k, 0.0, 90.0);
    GdipAddPathLine(p, cx + 9.0 * k, cy + 6.0 * k, cx + 7.0 * k, cy + 6.0 * k);
    let pn = pen(color, 2.3 * k);
    GdipDrawPath(g, pn, p);
    GdipDeletePen(pn);
    GdipDeletePath(p);
    let tri = [
        PointF {
            x: cx - 8.5 * k,
            y: cy + 13.5 * k,
        },
        PointF {
            x: cx + 8.5 * k,
            y: cy + 13.5 * k,
        },
        PointF {
            x: cx,
            y: cy + 2.5 * k,
        },
    ];
    let mut b = null_mut();
    GdipCreateSolidFill(color, &mut b);
    GdipFillPolygon(g, b, tri.as_ptr(), 3, 0);
    GdipDeleteBrush(b);
}
unsafe fn draw_toggle(g: H, a: f32) {
    let (x, y, w, h) = TOG;
    let t = UI.dark;
    let off = mix(
        mix(pc(NEUTRAL), pc(TER), 0.22),
        0xFFFFFF,
        UI.hover_tog * 0.25,
    );
    let on = mix(pc(ACC_B), 0xFFFFFF, UI.hover_tog * 0.12);
    fill_rr(g, x, y, w, h, h / 2.0, argb(mix(off, on, t), a));
    let kx = x + 16.0 + t * (w - 32.0);
    let ky = y + h / 2.0;
    for i in 0..4 {
        fill_circle(
            g,
            kx,
            ky + 1.4,
            13.0 + i as f32 * 1.1,
            argb(0x000000, a * 0.05),
        );
    }
    fill_circle(g, kx, ky, 13.0, argb(0xFFFFFF, a));
    // sun
    let sa = (1.0 - t) * a;
    if sa > 0.01 {
        let sun = argb(0xF5A524, sa);
        fill_circle(g, kx, ky, 4.4, sun);
        let p = pen(sun, 1.6);
        for i in 0..8 {
            let ang = i as f32 * std::f32::consts::FRAC_PI_4 + t * 1.6;
            let (s, c) = ang.sin_cos();
            GdipDrawLine(g, p, kx + c * 6.9, ky + s * 6.9, kx + c * 8.7, ky + s * 8.7);
        }
        GdipDeletePen(p);
    }
    // moon
    let ma = t * a;
    if ma > 0.01 {
        fill_circle(g, kx, ky, 6.2, argb(0x5B4BDD, ma));
        fill_circle(g, kx + 3.0, ky - 2.6, 5.2, argb(0xFFFFFF, ma));
    }
    if UI.kbd && UI.focus == 2 {
        stroke_rr(
            g,
            x - 3.0,
            y - 3.0,
            w + 6.0,
            h + 6.0,
            h / 2.0 + 3.0,
            argb(pc(ACC_TXT), a * 0.7),
            2.0,
        );
    }
}
unsafe fn draw_status_block(g: H, title: &str, hint: &str, a: f32, dy: f32) {
    if a <= 0.01 {
        return;
    }
    text(
        g,
        title,
        20.0,
        209.0 + dy,
        340.0,
        28.0,
        F_HERO,
        argb(pc(TEXT), a),
        FMT_C,
    );
    text(
        g,
        hint,
        48.0,
        239.0 + dy,
        284.0,
        40.0,
        F_BODY,
        argb(pc(SEC), a),
        FMT_W,
    );
}
// Comet-style spinner: a faded tail behind a bright head, eased rotation.
unsafe fn spinner(g: H, cx: f32, cy: f32, rad: f32, col: u32, alpha: f32, ph: f32) {
    stroke_circle(g, cx, cy, rad, argb(col, 0.12 * alpha), 2.0);
    let head = ph * 300.0 + 55.0 * (ph * 2.1).sin();
    let tail = 210.0 + 40.0 * (ph * 1.3).sin();
    let steps = 30;
    let step = tail / steps as f32;
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let al = (1.0 - t).powf(1.5) * 0.95 * alpha;
        let mut p = null_mut();
        GdipCreatePen1(argb(col, al), 2.6, 0, &mut p);
        let start = head - (i + 1) as f32 * step;
        GdipDrawArc(
            g,
            p,
            cx - rad,
            cy - rad,
            rad * 2.0,
            rad * 2.0,
            start,
            step + 0.9,
        );
        GdipDeletePen(p);
    }
    let (sn, cs) = head.to_radians().sin_cos();
    fill_circle(g, cx + rad * cs, cy + rad * sn, 2.3, argb(col, alpha));
}
unsafe fn draw_hero(g: H, a: f32) {
    let (x, y, w, h) = (20.0, 92.0, 340.0, 212.0);
    if !PARTIAL {
        shadow(
            g,
            x,
            y,
            w,
            h,
            24.0,
            0x2B2B4A,
            a * (1.0 - UI.dark) * 0.5,
            6.0,
        );
    }
    fill_rr(g, x, y, w, h, 24.0, argb(pc(CARD), a));
    stroke_rr(
        g,
        x + 0.5,
        y + 0.5,
        w - 1.0,
        h - 1.0,
        24.0,
        argb(pc(BORDER), a),
        1.0,
    );
    let clip = rr_path(x, y, w, h, 24.0);
    let mut saved = 0;
    GdipSaveGraphics(g, &mut saved);
    GdipSetClipPath(g, clip, 1);
    let (cx, cy, r) = (190.0f32, 153.0f32, 32.0f32);
    let act = UI.a_start.max(UI.a_wait);
    let mut col = mix(pc(TER), pc(ACC_TXT), act);
    col = mix(col, pc(GREEN), UI.a_conn);
    col = mix(col, pc(RED), UI.a_fail);
    // ambient glow
    let glow = act * 0.6 + UI.a_conn * 0.8 + UI.a_fail * 0.5;
    if glow > 0.01 {
        GdipSetInterpolationMode(g, 3);
        image_tint(
            g,
            IMG_GLOW,
            cx - 120.0,
            cy - 120.0,
            240.0,
            240.0,
            240.0,
            a * glow * 0.26,
            col,
        );
        GdipSetInterpolationMode(g, 7);
    }
    // ripples while waiting / connected
    let ph = UI.phase as f32;
    for k in 0..3 {
        let t = (ph / 3.0 + k as f32 / 3.0).fract();
        let rr = r + 2.0 + t * 38.0;
        let al = (1.0 - t).powf(1.7) * 0.55 * UI.a_wait * a;
        if al > 0.004 {
            stroke_circle(g, cx, cy, rr, argb(col, al), 1.6);
        }
    }
    if UI.a_conn > 0.004 {
        let t = (ph / 4.0).fract();
        stroke_circle(
            g,
            cx,
            cy,
            r + 4.0 + t * 30.0,
            argb(col, (1.0 - t) * 0.4 * UI.a_conn * a),
            1.6,
        );
        stroke_circle(g, cx, cy, r + 7.0, argb(col, 0.28 * UI.a_conn * a), 2.0);
    }
    if UI.a_start > 0.004 {
        spinner(g, cx, cy, r + 8.0, col, UI.a_start * a, ph);
    }
    // disk + glyph
    fill_circle(g, cx, cy, r, argb(mix(pc(P_INPUT), col, 0.13), a));
    stroke_circle(g, cx, cy, r - 1.0, argb(col, 0.55 * a), 1.6);
    let k = 1.0 + 0.035 * ((ph * 2.2).sin()) * UI.a_conn;
    glyph(g, cx, cy, k, argb(col, a));
    GdipRestoreGraphics(g, saved);
    GdipDeletePath(clip);
    // status text with cross-fade
    let f = ease_out(UI.status_fade);
    // The old text leaves before the new one arrives, so the two never overlap.
    if f < 0.5 {
        let (t, h) = (&UI.prev_text.0, &UI.prev_text.1);
        draw_status_block(g, t, h, a * (1.0 - f * 2.0), -8.0 * f);
    }
    let (title, hint) = status_text(UI.status);
    let fin = ((f - 0.3) / 0.7).clamp(0.0, 1.0);
    draw_status_block(g, &title, &hint, a * fin, 8.0 * (1.0 - f));
}
unsafe fn draw_field(g: H, a: f32) {
    let (x, y, w, h) = FIELD;
    text(
        g,
        tr(Str::FieldLabel),
        x + 4.0,
        316.0,
        300.0,
        18.0,
        F_SMALL,
        argb(pc(SEC), a * lfade()),
        FMT_L,
    );
    let f = UI.field;
    if f > 0.01 {
        stroke_rr(
            g,
            x - 2.0,
            y - 2.0,
            w + 4.0,
            h + 4.0,
            16.0,
            argb(pc(ACC_TXT), a * 0.20 * f),
            3.5,
        );
    }
    fill_rr(g, x, y, w, h, 14.0, argb(pc(CARD), a));
    stroke_rr(
        g,
        x + 0.5,
        y + 0.5,
        w - 1.0,
        h - 1.0,
        14.0,
        argb(mix(pc(BORDER), pc(ACC_TXT), f), a),
        1.0 + 0.5 * f,
    );
    if !UI.edit_shown && a > 0.6 {
        UI.edit_shown = true;
        ShowWindow(INPUT, 5);
    }
}
unsafe fn draw_button(g: H, a: f32) {
    let (x, y, w, h) = BTN;
    let run = UI.running;
    let sc = 1.0 - UI.press * 0.022;
    let mut st = 0;
    GdipSaveGraphics(g, &mut st);
    GdipTranslateWorldTransform(g, x + w / 2.0, y + h / 2.0, 0);
    GdipScaleWorldTransform(g, sc, sc, 0);
    GdipTranslateWorldTransform(g, -(x + w / 2.0), -(y + h / 2.0), 0);
    // colored drop shadow fades out when running
    let sh = a * (1.0 - run) * (1.0 - UI.press * 0.5);
    for i in 0..9 {
        let e = i as f32 * 1.8;
        let al = sh * 0.05 * (1.0 - i as f32 / 9.0);
        fill_rr(
            g,
            x - e + 6.0,
            y - e + 10.0,
            w + e * 2.0 - 12.0,
            h + e * 2.0 - 4.0,
            16.0 + e,
            argb(pc(ACC_B), al),
        );
    }
    let lift = UI.hover_btn * 0.10 - UI.press * 0.06;
    let top = mix(
        pc(ACC_A),
        if lift >= 0.0 { 0xFFFFFF } else { 0x000000 },
        lift.abs(),
    );
    let bot = mix(
        pc(ACC_B),
        if lift >= 0.0 { 0xFFFFFF } else { 0x000000 },
        lift.abs(),
    );
    let neutral = mix(pc(NEUTRAL), pc(TEXT), UI.hover_btn * 0.06);
    grad_rr(
        g,
        x,
        y,
        w,
        h,
        16.0,
        argb(mix(top, neutral, run), a),
        argb(mix(bot, neutral, run), a),
    );
    // top highlight
    let p = pen(argb(0xFFFFFF, 0.28 * a * (1.0 - run)), 1.0);
    GdipDrawLine(g, p, x + 16.0, y + 0.8, x + w - 16.0, y + 0.8);
    GdipDeletePen(p);
    text(
        g,
        tr(Str::Start),
        x,
        y + 14.0,
        w,
        24.0,
        F_BTN,
        argb(0xFFFFFF, a * (1.0 - run) * lfade()),
        FMT_C,
    );
    text(
        g,
        tr(Str::Stop),
        x,
        y + 14.0,
        w,
        24.0,
        F_BTN,
        argb(pc(TEXT), a * run * lfade()),
        FMT_C,
    );
    GdipRestoreGraphics(g, st);
    if UI.kbd && UI.focus == 1 {
        stroke_rr(
            g,
            x - 4.0,
            y - 4.0,
            w + 8.0,
            h + 8.0,
            20.0,
            argb(pc(ACC_TXT), a * 0.7),
            2.0,
        );
    }
}
unsafe fn draw_steps(g: H, a: f32) {
    let a = a * lfade();
    text(
        g,
        tr(Str::StepsTitle),
        24.0,
        478.0,
        300.0,
        20.0,
        F_BADGE,
        argb(pc(TEXT), a),
        FMT_L,
    );
    let rows = [("1", tr(Str::Step1), 506.0), ("2", tr(Str::Step2), 538.0)];
    for (n, t, y) in rows {
        fill_circle(g, 35.0, y + 11.0, 11.0, argb(pc(ACC_TXT), a * 0.13));
        text(
            g,
            n,
            24.0,
            y + 4.5,
            22.0,
            18.0,
            F_BADGE,
            argb(pc(ACC_TXT), a),
            FMT_C,
        );
        text(
            g,
            t,
            56.0,
            y + 3.0,
            310.0,
            20.0,
            F_BODY,
            argb(pc(SEC), a),
            FMT_L,
        );
    }
}
unsafe fn footer_icon(g: H, kind: usize, x: f32, y: f32, color: u32) {
    let p = pen(color, 1.5);
    match kind {
        0 => {
            // monitor
            let path = rr_path(x + 1.0, y + 1.5, 14.0, 9.5, 2.2);
            GdipDrawPath(g, p, path);
            GdipDeletePath(path);
            GdipDrawLine(g, p, x + 8.0, y + 11.0, x + 8.0, y + 13.6);
            GdipDrawLine(g, p, x + 5.0, y + 14.2, x + 11.0, y + 14.2);
        }
        1 => {
            // wi-fi: three arcs and a dot
            let (cx, cy) = (x + 8.0, y + 13.0);
            for r in [10.8f32, 7.0, 3.3] {
                GdipDrawArc(g, p, cx - r, cy - r, r * 2.0, r * 2.0, 227.0, 86.0);
            }
            fill_circle(g, cx, cy + 0.2, 1.3, color);
        }
        _ => {
            // clock
            GdipDrawEllipse(g, p, x + 1.0, y + 1.0, 14.0, 14.0);
            GdipDrawLine(g, p, x + 8.0, y + 8.0, x + 8.0, y + 4.2);
            GdipDrawLine(g, p, x + 8.0, y + 8.0, x + 11.0, y + 9.8);
        }
    }
    GdipDeletePen(p);
}
unsafe fn draw_footer(g: H, a: f32) {
    let p = pen(argb(pc(BORDER), a), 1.0);
    GdipDrawLine(g, p, 24.0, 580.0, WIN_W - 24.0, 580.0);
    GdipDeletePen(p);
    let at = a * lfade();
    let items = [tr(Str::FeatScreen), tr(Str::FeatLocal), tr(Str::FeatFps)];
    let mut widths = [0.0f32; 3];
    let mut total = 0.0;
    for i in 0..3 {
        widths[i] = measure(g, items[i], F_SMALL);
        total += 22.0 + widths[i];
    }
    let gap = 26.0;
    total += gap * 2.0;
    let mut x = (WIN_W - total) / 2.0;
    for i in 0..3 {
        footer_icon(g, i, x, 590.5, argb(pc(TER), a));
        text(
            g,
            items[i],
            x + 22.0,
            591.0,
            widths[i] + 8.0,
            20.0,
            F_SMALL,
            argb(pc(TER), at),
            FMT_L,
        );
        x += 22.0 + widths[i] + gap;
    }
    draw_lang(g, a);
}
unsafe fn seg_at(px: f32, py: f32) -> i32 {
    if !over(SEG, px, py) {
        return -1;
    }
    (((px - SEG.0) / (SEG.2 / 3.0)) as i32).clamp(0, 2)
}
// Segmented language switch with a sliding thumb.
unsafe fn draw_lang(g: H, a: f32) {
    let (x, y, w, h) = SEG;
    let sw = (w - 4.0) / 3.0;
    fill_rr(
        g,
        x,
        y,
        w,
        h,
        h / 2.0,
        argb(mix(0xE4E4EA, 0x2C2C2E, UI.dark), a),
    );
    let tx = x + 2.0 + UI.lang_pos * sw;
    fill_rr(
        g,
        tx,
        y + 3.0,
        sw,
        h - 4.0,
        (h - 4.0) / 2.0,
        argb(0x000000, a * 0.07 * (1.0 - UI.dark)),
    );
    fill_rr(
        g,
        tx,
        y + 2.0,
        sw,
        h - 4.0,
        (h - 4.0) / 2.0,
        argb(mix(0xFFFFFF, 0x5A5A5F, UI.dark), a),
    );
    for (i, name) in LANG_NAMES.iter().enumerate() {
        let near = (1.0 - (UI.lang_pos - i as f32).abs()).clamp(0.0, 1.0);
        let col = mix(pc(SEC), pc(TEXT), near);
        text(
            g,
            name,
            x + 2.0 + i as f32 * sw,
            y + 6.5,
            sw,
            18.0,
            F_BADGE,
            argb(col, a),
            FMT_C,
        );
    }
    if UI.kbd && UI.focus == 3 {
        stroke_rr(
            g,
            x - 3.0,
            y - 3.0,
            w + 6.0,
            h + 6.0,
            h / 2.0 + 3.0,
            argb(pc(ACC_TXT), a * 0.7),
            2.0,
        );
    }
}
unsafe fn render(g: H, clip: Option<(f32, f32, f32, f32)>) {
    GdipSetSmoothingMode(g, 4);
    GdipSetTextRenderingHint(g, 5);
    GdipSetInterpolationMode(g, 7);
    GdipSetPixelOffsetMode(g, 2);
    PARTIAL = clip.is_some();
    if let Some(c) = clip {
        let s = UI.s;
        GdipSetClipRect(g, c.0 * s, c.1 * s, c.2 * s, c.3 * s, 0);
    }
    let mut bg = null_mut();
    GdipCreateSolidFill(argb(pc(BG), 1.0), &mut bg);
    GdipFillRectangle(g, bg, 0.0, 0.0, BB_W as f32, BB_H as f32);
    GdipDeleteBrush(bg);
    GdipScaleWorldTransform(g, UI.s, UI.s, 0);
    if clip.is_none() {
        let (a, st) = sec(g, 0);
        image(g, IMG_HEAD, 20.0, 24.0, 48.0, 48.0, 96.0, a);
        text(
            g,
            "Aireluma",
            80.0,
            22.0,
            200.0,
            30.0,
            F_TITLE,
            argb(pc(TEXT), a),
            FMT_L,
        );
        text(
            g,
            tr(Str::Subtitle),
            81.0,
            52.0,
            220.0,
            18.0,
            F_BODY,
            argb(pc(SEC), a * lfade()),
            FMT_L,
        );
        draw_toggle(g, a);
        GdipRestoreGraphics(g, st);
    }
    let (a, st) = sec(g, 1);
    draw_hero(g, a);
    GdipRestoreGraphics(g, st);
    if clip.is_some() {
        return;
    }
    let (a, st) = sec(g, 2);
    draw_field(g, a);
    GdipRestoreGraphics(g, st);
    let (a, st) = sec(g, 3);
    draw_button(g, a);
    GdipRestoreGraphics(g, st);
    let (a, st) = sec(g, 4);
    draw_steps(g, a);
    GdipRestoreGraphics(g, st);
    let (a, st) = sec(g, 5);
    draw_footer(g, a);
    GdipRestoreGraphics(g, st);
}
unsafe fn paint(h: H) {
    let mut ps: Paint = std::mem::zeroed();
    let dc = BeginPaint(h, &mut ps);
    let mut rc = Rect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    GetClientRect(h, &mut rc);
    let (w, hh) = (rc.right.max(1), rc.bottom.max(1));
    let mut fresh = false;
    if BB_DC.is_null() || BB_W != w || BB_H != hh {
        if !BB_BMP.is_null() {
            DeleteObject(BB_BMP);
        }
        if BB_DC.is_null() {
            BB_DC = CreateCompatibleDC(dc);
        }
        let bmi = BmiHeader {
            size: std::mem::size_of::<BmiHeader>() as u32,
            width: w,
            height: -hh,
            planes: 1,
            bpp: 32,
            compression: 0,
            size_image: 0,
            xppm: 0,
            yppm: 0,
            used: 0,
            important: 0,
        };
        let mut bits = null_mut();
        BB_BMP = CreateDIBSection(dc, &bmi, 0, &mut bits, null_mut(), 0);
        SelectObject(BB_DC, BB_BMP);
        BB_W = w;
        BB_H = hh;
        fresh = true;
    }
    // Only the indicator region is redrawn while nothing else is animating.
    let r = ps.rect;
    let (a, s) = (AMBIENT, UI.s);
    let partial = UI.partial
        && !fresh
        && r.left >= (a.0 * s).floor() as i32
        && r.top >= (a.1 * s).floor() as i32
        && r.right <= ((a.0 + a.2) * s).ceil() as i32
        && r.bottom <= ((a.1 + a.3) * s).ceil() as i32;
    let mut g = null_mut();
    GdipCreateFromHDC(BB_DC, &mut g);
    render(g, if partial { Some(AMBIENT) } else { None });
    GdipDeleteGraphics(g);
    BitBlt(
        dc,
        r.left,
        r.top,
        r.right - r.left,
        r.bottom - r.top,
        BB_DC,
        r.left,
        r.top,
        0x00CC0020,
    );
    EndPaint(h, &ps);
}

// ───────────────────────────── Animation engine ─────────────────────────────
unsafe fn kick(h: H) {
    if !UI.timer {
        UI.timer = true;
        UI.last = GetTickCount64();
        UI.interval = 16;
        SetTimer(h, 2, 16, null_mut());
    }
}
unsafe fn set_edit_brush() {
    let c = bgr(pc(CARD));
    if c != EDIT_BRUSH_COLOR {
        let old = EDIT_BRUSH;
        EDIT_BRUSH = CreateSolidBrush(c);
        EDIT_BRUSH_COLOR = c;
        if !old.is_null() {
            DeleteObject(old);
        }
    }
}
unsafe fn apply_dwm(h: H) {
    let bg = pc(BG);
    let dark = (UI.dark > 0.5) as i32;
    DwmSetWindowAttribute(h, 20, &dark, 4);
    DwmSetWindowAttribute(h, 35, &(bgr(bg) as i32), 4);
    DwmSetWindowAttribute(h, 34, &(bgr(pc(BORDER)) as i32), 4);
}
unsafe fn tick(h: H) {
    let now = GetTickCount64();
    let dt = ((now.saturating_sub(UI.last)) as f32 / 1000.0).clamp(0.001, 0.05);
    UI.last = now;
    UI.phase += dt as f64;
    UI.enter += dt;
    let mut moving = UI.enter < 1.5;
    let hv = |b: bool| if b { 1.0 } else { 0.0 };
    let before_dark = UI.dark;
    moving |= approach(&mut UI.dark, UI.dark_t, dt, 9.0);
    moving |= approach(&mut UI.hover_btn, hv(UI.over_btn), dt, 16.0);
    moving |= approach(&mut UI.hover_tog, hv(UI.over_tog), dt, 16.0);
    moving |= approach(
        &mut UI.press,
        hv(UI.press_item == 1 && UI.over_btn),
        dt,
        28.0,
    );
    moving |= approach(&mut UI.field, hv(edit_focused()), dt, 14.0);
    moving |= approach(&mut UI.running, hv(CHILD.is_some()), dt, 9.0);
    moving |= approach(&mut UI.lang_pos, UI.lang_t, dt, 14.0);
    if UI.lang_fade < 1.0 {
        UI.lang_fade = (UI.lang_fade + dt / 0.3).min(1.0);
        moving = true;
    }
    let st = UI.status;
    moving |= approach(&mut UI.a_start, hv(st == Status::Starting), dt, 7.0);
    moving |= approach(&mut UI.a_wait, hv(st == Status::Waiting), dt, 7.0);
    moving |= approach(&mut UI.a_conn, hv(st == Status::Connected), dt, 7.0);
    moving |= approach(&mut UI.a_fail, hv(st == Status::Failed), dt, 7.0);
    if UI.status_fade < 1.0 {
        UI.status_fade = (UI.status_fade + dt / 0.45).min(1.0);
        moving = true;
    }
    let continuous = matches!(st, Status::Starting | Status::Waiting | Status::Connected);
    if UI.dark != before_dark {
        set_edit_brush();
        apply_dwm(h);
        InvalidateRect(INPUT, null(), 1);
    }
    let visible = IsWindowVisible(h) != 0 && IsIconic(h) == 0;
    UI.partial = visible && !moving && continuous;
    if visible {
        if UI.partial {
            let (a, s) = (AMBIENT, UI.s);
            let dirty = Rect {
                left: (a.0 * s).floor() as i32,
                top: (a.1 * s).floor() as i32,
                right: ((a.0 + a.2) * s).ceil() as i32,
                bottom: ((a.1 + a.3) * s).ceil() as i32,
            };
            InvalidateRect(h, &dirty, 0);
        } else {
            InvalidateRect(h, null(), 0);
        }
        UpdateWindow(h);
    }
    if !visible || !(moving || continuous) {
        KillTimer(h, 2);
        UI.timer = false;
    } else {
        // Ambient animation only needs half the frame rate.
        let want = if UI.partial && UI.a_start < 0.01 {
            33
        } else {
            16
        };
        if UI.interval != want {
            UI.interval = want;
            SetTimer(h, 2, want, null_mut());
        }
    }
}
static mut EDIT_HAS_FOCUS: bool = false;
unsafe fn edit_focused() -> bool {
    EDIT_HAS_FOCUS
}

// ───────────────────────────── Actions ─────────────────────────────
unsafe fn theme_toggle(h: H) {
    DARK = !DARK;
    UI.dark_t = if DARK { 1.0 } else { 0.0 };
    let path = appearance_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, if DARK { "dark" } else { "light" });
    kick(h);
}
unsafe fn receiver_toggle(h: H) {
    if let Some(mut c) = CHILD.take() {
        GENERATION = GENERATION.wrapping_add(1);
        let _ = c.kill();
        let _ = c.wait();
        EnableWindow(INPUT, 1);
        set_status(h, Status::Off);
    } else {
        let mut b = [0u16; 256];
        let n = GetWindowTextW(INPUT, b.as_mut_ptr(), 256);
        let raw = String::from_utf16_lossy(&b[..n.max(0) as usize]);
        let name = if raw.trim().is_empty() {
            "Aireluma".to_string()
        } else {
            raw.trim().to_string()
        };
        let (exe, dir) = paths();
        let path = format!("{};{}", dir.display(), env::var("PATH").unwrap_or_default());
        match Command::new(exe)
            .env("PATH", path)
            .env_remove("AIRELUMA_VIDEO_HWND")
            .env("GST_PLUGIN_SYSTEM_PATH_1_0", dir.join("plugins"))
            .env("GST_PLUGIN_SCANNER_1_0", dir.join("gst-plugin-scanner.exe"))
            .args([
                "-n",
                &name,
                "-nh",
                "-vs",
                "d3d12videosink",
                "-p",
                "35000",
                "-h265",
                "-s",
                "3840x2160@60",
                "-fps",
                "60",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .spawn()
        {
            Ok(mut c) => {
                GENERATION = GENERATION.wrapping_add(1);
                VIDEO_WINDOW = null_mut();
                let generation = GENERATION;
                let target = h as usize;
                if let Some(out) = c.stdout.take() {
                    thread::spawn(move || {
                        let mut reader = BufReader::new(out);
                        let mut line = Vec::new();
                        while let Ok(n) = reader.read_until(b'\n', &mut line) {
                            if n == 0 {
                                break;
                            }
                            let state = if line.windows(23).any(|w| w == b"AIRELUMA_RECEIVER_READY")
                            {
                                Some(2)
                            } else if line.windows(23).any(|w| w == b"AIRELUMA_STREAM_STARTED") {
                                Some(1)
                            } else if line.windows(23).any(|w| w == b"AIRELUMA_STREAM_STOPPED") {
                                Some(0)
                            } else {
                                None
                            };
                            if let Some(state) = state {
                                unsafe {
                                    PostMessageW(target as H, 0x8012, generation, state);
                                }
                            }
                            line.clear();
                        }
                    });
                }
                CHILD = Some(c);
                RECEIVER_READY = false;
                UI.was_ready = false;
                UI.name = name;
                EnableWindow(INPUT, 0);
                set_status(h, Status::Starting);
            }
            Err(e) => {
                MessageBoxW(
                    h,
                    wide(&tr(Str::ErrText).replace("{}", &e.to_string())).as_ptr(),
                    wide(tr(Str::ErrTitle)).as_ptr(),
                    0x10,
                );
            }
        }
    }
}
unsafe fn cycle_focus(h: H, back: bool) {
    let order: Vec<i32> = if CHILD.is_some() {
        vec![1, 2, 3]
    } else {
        vec![0, 1, 2, 3]
    };
    let cur = order.iter().position(|v| *v == UI.focus).unwrap_or(0) as i32;
    let n = order.len() as i32;
    let next = order[(((cur + if back { -1 } else { 1 }) % n + n) % n) as usize];
    UI.focus = next;
    UI.kbd = true;
    if next == 0 {
        SetFocus(INPUT);
    } else {
        SetFocus(h);
    }
    kick(h);
}
unsafe fn over(r: (f32, f32, f32, f32), px: f32, py: f32) -> bool {
    px >= r.0 && px < r.0 + r.2 && py >= r.1 && py < r.1 + r.3
}
unsafe fn layout_edit() {
    let s = UI.s;
    let r = |v: f32| (v * s).round() as i32;
    let old = EDIT_FONT;
    EDIT_FONT = CreateFontW(
        -r(15.0),
        0,
        0,
        0,
        400,
        0,
        0,
        0,
        1,
        0,
        0,
        5,
        0,
        wide("Segoe UI").as_ptr(),
    );
    SendMessageW(INPUT, 0x30, EDIT_FONT as usize, 1);
    if !old.is_null() {
        DeleteObject(old);
    }
    SetWindowPos(
        INPUT,
        null_mut(),
        r(36.0),
        r(353.0),
        r(308.0),
        r(22.0),
        0x14,
    );
}

fn appearance_path() -> PathBuf {
    PathBuf::from(env::var_os("APPDATA").unwrap_or_default())
        .join("Aireluma")
        .join("appearance")
}
fn paths() -> (PathBuf, PathBuf) {
    let root = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    (
        root.join("runtime").join("uxplay.exe"),
        root.join("runtime"),
    )
}

// ───────────────────────────── Tray & misc ─────────────────────────────
unsafe fn tray(h: H, operation: u32) -> bool {
    let mut n: NotifyIcon = std::mem::zeroed();
    n.size = std::mem::size_of::<NotifyIcon>() as u32;
    n.hwnd = h;
    n.id = 1;
    n.flags = 1 | 2 | 4;
    n.callback = 0x8001;
    n.icon = ICON;
    let tip = wide(tr(Str::TrayTip));
    n.tip[..tip.len()].copy_from_slice(&tip);
    Shell_NotifyIconW(operation, &mut n) != 0
}
unsafe fn show_app(h: H) {
    ShowWindow(h, 9);
    SetForegroundWindow(h);
}
unsafe extern "system" fn brand_video(window: H, data: isize) -> i32 {
    let mut pid = 0;
    GetWindowThreadProcessId(window, &mut pid);
    if pid == data as u32 && IsWindowVisible(window) != 0 && VIDEO_WINDOW != window {
        let mut result = 0;
        let title = wide(tr(Str::VideoTitle));
        if SendMessageTimeoutW(
            window,
            0x0c,
            0,
            title.as_ptr() as isize,
            2,
            100,
            &mut result,
        ) != 0
        {
            SendMessageTimeoutW(window, 0x80, 1, ICON as isize, 2, 100, &mut result);
            SendMessageTimeoutW(window, 0x80, 0, ICON as isize, 2, 100, &mut result);
            VIDEO_WINDOW = window;
        }
    }
    1
}

// ───────────────────────────── Window procedure ─────────────────────────────
unsafe extern "system" fn wnd(h: H, m: u32, w: usize, l: isize) -> isize {
    match m {
        1 => {
            UI.s = GetDpiForWindow(h) as f32 / 96.0;
            INPUT = CreateWindowExW(
                0,
                wide("EDIT").as_ptr(),
                wide("Aireluma").as_ptr(),
                0x40000000 | 0x80 | 0x10000,
                0,
                0,
                10,
                10,
                h,
                2usize as H,
                GetModuleHandleW(null()),
                null_mut(),
            );
            SendMessageW(INPUT, 0xd3, 3, 0);
            layout_edit();
            set_edit_brush();
            SetTimer(h, 1, 1000, null_mut());
            TRAY_READY = tray(h, 0);
            kick(h);
            0
        }
        0x0f => {
            paint(h);
            0
        }
        0x14 => 1,
        0x05 | 0x18 => {
            if m == 0x18 && w == 0 {
                return DefWindowProcW(h, m, w, l);
            }
            if m == 0x05 && w == 1 {
                return DefWindowProcW(h, m, w, l);
            }
            kick(h);
            DefWindowProcW(h, m, w, l)
        }
        0x2e0 => {
            UI.s = (w & 0xffff) as f32 / 96.0;
            let r = &*(l as *const Rect);
            SetWindowPos(
                h,
                null_mut(),
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                0x14,
            );
            layout_edit();
            kick(h);
            0
        }
        0x200 => {
            let px = (l & 0xffff) as i16 as f32 / UI.s;
            let py = ((l >> 16) & 0xffff) as i16 as f32 / UI.s;
            UI.over_btn = over(BTN, px, py);
            UI.over_tog = over(TOG, px, py);
            UI.over_seg = seg_at(px, py);
            if !UI.tracking {
                let mut t = TrackMouse {
                    size: std::mem::size_of::<TrackMouse>() as u32,
                    flags: 2,
                    hwnd: h,
                    hover: 0,
                };
                TrackMouseEvent(&mut t);
                UI.tracking = true;
            }
            kick(h);
            0
        }
        0x2a3 => {
            UI.tracking = false;
            UI.over_btn = false;
            UI.over_tog = false;
            UI.over_seg = -1;
            kick(h);
            0
        }
        0x20 => {
            if (l & 0xffff) == 1 && (UI.over_btn || UI.over_tog || UI.over_seg >= 0) {
                SetCursor(LoadCursorW(null_mut(), 32649usize as *const u16));
                1
            } else {
                DefWindowProcW(h, m, w, l)
            }
        }
        0x201 => {
            SetFocus(h);
            UI.kbd = false;
            let px = (l & 0xffff) as i16 as f32 / UI.s;
            let py = ((l >> 16) & 0xffff) as i16 as f32 / UI.s;
            UI.over_btn = over(BTN, px, py);
            UI.over_tog = over(TOG, px, py);
            UI.over_seg = seg_at(px, py);
            if UI.over_btn {
                UI.press_item = 1;
                UI.focus = 1;
                SetCapture(h);
            } else if UI.over_tog {
                UI.press_item = 2;
                UI.focus = 2;
                SetCapture(h);
            } else if UI.over_seg >= 0 {
                UI.press_item = 3;
                UI.focus = 3;
                SetCapture(h);
            }
            kick(h);
            0
        }
        0x202 => {
            let item = UI.press_item;
            UI.press_item = 0;
            ReleaseCapture();
            let px = (l & 0xffff) as i16 as f32 / UI.s;
            let py = ((l >> 16) & 0xffff) as i16 as f32 / UI.s;
            UI.over_btn = over(BTN, px, py);
            UI.over_tog = over(TOG, px, py);
            UI.over_seg = seg_at(px, py);
            if item == 1 && UI.over_btn {
                receiver_toggle(h);
            } else if item == 2 && UI.over_tog {
                theme_toggle(h);
            } else if item == 3 && UI.over_seg >= 0 {
                set_lang(h, LANGS[UI.over_seg as usize]);
            }
            kick(h);
            0
        }
        0x100 => {
            let repeat = (l >> 30) & 1 != 0;
            if !repeat && UI.focus == 3 && (w == 0x25 || w == 0x27) {
                let i = (LANG as i32 + if w == 0x25 { -1 } else { 1 }).clamp(0, 2);
                set_lang(h, LANGS[i as usize]);
                UI.kbd = true;
            } else if !repeat && (w == 0x20 || w == 0x0d) {
                match UI.focus {
                    2 => theme_toggle(h),
                    3 => {}
                    _ => receiver_toggle(h),
                }
                UI.kbd = true;
            }
            0
        }
        0x113 if w == 2 => {
            tick(h);
            0
        }
        0x8012 => {
            if w == GENERATION && CHILD.is_some() {
                if l == 2 {
                    RECEIVER_READY = true;
                    UI.was_ready = true;
                    set_status(h, Status::Waiting);
                } else if l == 1 {
                    RECEIVER_READY = true;
                    UI.was_ready = true;
                    set_status(h, Status::Connected);
                } else {
                    set_status(
                        h,
                        if RECEIVER_READY {
                            Status::Waiting
                        } else {
                            Status::Starting
                        },
                    );
                }
            }
            0
        }
        0x8010 => {
            DestroyWindow(h);
            0
        }
        0x8011 => {
            show_app(h);
            0
        }
        0x112 if w & 0xfff0 == 0xf020 && TRAY_READY => {
            ShowWindow(h, 0);
            0
        }
        0x10 => {
            if TRAY_READY {
                ShowWindow(h, 0);
            } else {
                DestroyWindow(h);
            }
            0
        }
        0x8001 => {
            match l as u32 {
                0x202 | 0x203 => show_app(h),
                0x205 | 0x7b => {
                    let mut pt = Point { x: 0, y: 0 };
                    GetCursorPos(&mut pt);
                    let menu = CreatePopupMenu();
                    AppendMenuW(menu, 0, 101, wide(tr(Str::TrayOpen)).as_ptr());
                    AppendMenuW(menu, 0x800, 0, null());
                    AppendMenuW(menu, 0, 102, wide(tr(Str::TrayExit)).as_ptr());
                    SetForegroundWindow(h);
                    TrackPopupMenu(menu, 0x2, pt.x, pt.y, 0, h, null_mut());
                    DestroyMenu(menu);
                }
                _ => {}
            }
            0
        }
        0x111 if w & 0xffff == 101 => {
            show_app(h);
            0
        }
        0x111 if w & 0xffff == 102 => {
            DestroyWindow(h);
            0
        }
        0x111 if w & 0xffff == 2 && ((w >> 16) == 0x100 || (w >> 16) == 0x200) => {
            EDIT_HAS_FOCUS = (w >> 16) == 0x100;
            if EDIT_HAS_FOCUS {
                UI.focus = 0;
            }
            kick(h);
            0
        }
        0x133 | 0x138 if l as H == INPUT => {
            let disabled = m == 0x138;
            SetTextColor(w as H, bgr(if disabled { pc(TER) } else { pc(TEXT) }));
            SetBkColor(w as H, bgr(pc(CARD)));
            EDIT_BRUSH as isize
        }
        0x113 => {
            if let Some(c) = CHILD.as_mut() {
                EnumWindows(Some(brand_video), c.id() as isize);
                if matches!(c.try_wait(), Ok(Some(_))) {
                    CHILD = None;
                    EnableWindow(INPUT, 1);
                    set_status(h, Status::Failed);
                }
            }
            0
        }
        2 => {
            if TRAY_READY {
                tray(h, 2);
                TRAY_READY = false;
            }
            if let Some(mut c) = CHILD.take() {
                let _ = c.kill();
                let _ = c.wait();
            }
            PostQuitMessage(0);
            0
        }
        _ => {
            if m == TASKBAR_CREATED && m != 0 {
                TRAY_READY = tray(h, 0);
                0
            } else {
                DefWindowProcW(h, m, w, l)
            }
        }
    }
}

pub fn run() {
    unsafe {
        SetProcessDpiAwarenessContext(-4);
        SetCurrentProcessExplicitAppUserModelID(wide("Aireluma.Desktop").as_ptr());
        let prior = FindWindowW(wide("AirPlayWindow").as_ptr(), null());
        if !prior.is_null() {
            PostMessageW(prior, 0x8011, 0, 0);
            return;
        }
        let instance = GetModuleHandleW(null());
        ICON = LoadImageW(instance, 1usize as *const u16, 1, 48, 48, 0);
        TASKBAR_CREATED = RegisterWindowMessageW(wide("TaskbarCreated").as_ptr());
        DARK = fs::read_to_string(appearance_path())
            .map(|v| v.trim() == "dark")
            .unwrap_or(false);
        LANG = load_lang();
        UI.lang_pos = LANG as i32 as f32;
        UI.lang_t = UI.lang_pos;
        UI.dark = if DARK { 1.0 } else { 0.0 };
        UI.dark_t = UI.dark;
        init_gdip();
        let class = wide("AirPlayWindow");
        let wc = WndClass {
            style: 0,
            proc: Some(wnd),
            cls_extra: 0,
            wnd_extra: 0,
            instance,
            icon: ICON,
            cursor: LoadCursorW(null_mut(), 32512usize as *const u16),
            background: null_mut(),
            menu: null(),
            name: class.as_ptr(),
        };
        RegisterClassW(&wc);
        let dpi = GetDpiForSystem();
        let s = dpi as f32 / 96.0;
        let mut bounds = Rect {
            left: 0,
            top: 0,
            right: (WIN_W * s).round() as i32,
            bottom: (WIN_H * s).round() as i32,
        };
        AdjustWindowRectExForDpi(&mut bounds, 0x00ca0000, 0, 0x02000000, dpi);
        let h = CreateWindowExW(
            0x02000000,
            class.as_ptr(),
            wide("Aireluma").as_ptr(),
            0x00ca0000,
            0x80000000u32 as i32,
            0x80000000u32 as i32,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        );
        apply_dwm(h);
        ShowWindow(h, 1);
        UpdateWindow(h);
        let mut msg: Msg = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            if msg.message == 0x100 {
                if msg.wparam == 9 {
                    cycle_focus(h, GetKeyState(0x10) < 0);
                    continue;
                }
                if msg.wparam == 0x0d && msg.hwnd == INPUT {
                    receiver_toggle(h);
                    continue;
                }
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
