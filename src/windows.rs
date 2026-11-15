#![allow(static_mut_refs, dead_code)]
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
}
#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(dc: H) -> H;
    fn CreateCompatibleBitmap(dc: H, w: i32, h: i32) -> H;
    fn BitBlt(dst: H, x: i32, y: i32, w: i32, h: i32, src: H, sx: i32, sy: i32, rop: u32) -> i32;
    fn DeleteDC(dc: H) -> i32;
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
    fn GdipGraphicsClear(g: H, color: u32) -> i32;
    fn GdipSaveGraphics(g: H, state: *mut u32) -> i32;
    fn GdipRestoreGraphics(g: H, state: u32) -> i32;
    fn GdipScaleWorldTransform(g: H, x: f32, y: f32, order: i32) -> i32;
    fn GdipTranslateWorldTransform(g: H, x: f32, y: f32, order: i32) -> i32;
    fn GdipSetClipPath(g: H, path: H, mode: i32) -> i32;
    fn GdipResetClip(g: H) -> i32;
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
    status: Status,
    prev_status: Status,
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
    status: Status::Off,
    prev_status: Status::Off,
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
const WIN_H: f32 = 620.0;
const BTN: (f32, f32, f32, f32) = (20.0, 404.0, 340.0, 52.0);
const TOG: (f32, f32, f32, f32) = (300.0, 33.0, 56.0, 32.0);
const FIELD: (f32, f32, f32, f32) = (20.0, 340.0, 340.0, 48.0);

static mut DARK: bool = false;
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

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
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
    if im.is_null() || a <= 0.003 {
        return;
    }
    let mut at = null_mut();
    GdipCreateImageAttributes(&mut at);
    let m: [f32; 25] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        a, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
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
    let inst = GetModuleHandleW(null());
    let load = |id: usize, size: i32| -> H {
        let icon = LoadImageW(inst, id as *const u16, 1, size, size, 0);
        let mut bmp = null_mut();
        if !icon.is_null() {
            GdipCreateBitmapFromHICON(icon, &mut bmp);
            DestroyIcon(icon);
        }
        bmp
    };
    IMG_HEAD = load(1, 96);
}

// ───────────────────────────── Status text ─────────────────────────────
fn status_text(s: Status) -> (String, String) {
    unsafe {
        match s {
            Status::Off => (
                "Приём выключен".into(),
                "Нажмите «Включить AirPlay», чтобы начать".into(),
            ),
            Status::Starting => ("Запуск приёмника…".into(), "Это займёт пару секунд".into()),
            Status::Waiting => (
                "Ожидание iPhone".into(),
                format!(
                    "Откройте «Повтор экрана» в Пункте управления и выберите «{}»",
                    UI.name
                ),
            ),
            Status::Connected => (
                "Экран подключён".into(),
                "Изображение и звук передаются на этот компьютер".into(),
            ),
            Status::Failed => {
                if UI.was_ready {
                    (
                        "Приёмник остановлен".into(),
                        "Нажмите «Включить AirPlay», чтобы запустить снова".into(),
                    )
                } else {
                    (
                        "Не удалось запустить приёмник".into(),
                        "Нажмите «Включить AirPlay», чтобы повторить".into(),
                    )
                }
            }
        }
    }
}
unsafe fn set_status(h: H, s: Status) {
    if UI.status != s {
        UI.prev_status = UI.status;
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
    let off = mix(pc(NEUTRAL), 0xFFFFFF, UI.hover_tog * 0.35);
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
unsafe fn draw_status_block(g: H, s: Status, a: f32, dy: f32) {
    if a <= 0.01 {
        return;
    }
    let (title, hint) = status_text(s);
    text(
        g,
        &title,
        20.0,
        214.0 + dy,
        340.0,
        28.0,
        F_HERO,
        argb(pc(TEXT), a),
        FMT_C,
    );
    text(
        g,
        &hint,
        48.0,
        244.0 + dy,
        284.0,
        40.0,
        F_BODY,
        argb(pc(SEC), a),
        FMT_W,
    );
}
unsafe fn draw_hero(g: H, a: f32) {
    let (x, y, w, h) = (20.0, 92.0, 340.0, 212.0);
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
    GdipSetClipPath(g, clip, 0);
    let (cx, cy, r) = (190.0f32, 158.0f32, 32.0f32);
    let act = UI.a_start.max(UI.a_wait);
    let mut col = mix(pc(TER), pc(ACC_TXT), act);
    col = mix(col, pc(GREEN), UI.a_conn);
    col = mix(col, pc(RED), UI.a_fail);
    // ambient glow
    let glow = act * 0.6 + UI.a_conn * 0.8 + UI.a_fail * 0.5;
    for i in 0..16 {
        let rr = 120.0 - i as f32 * 6.0;
        fill_circle(g, cx, cy, rr, argb(col, a * glow * 0.016));
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
        let p = pen(argb(col, 0.95 * UI.a_start * a), 2.6);
        GdipDrawArc(
            g,
            p,
            cx - r - 8.0,
            cy - r - 8.0,
            (r + 8.0) * 2.0,
            (r + 8.0) * 2.0,
            ph * 420.0,
            95.0,
        );
        GdipDeletePen(p);
        stroke_circle(g, cx, cy, r + 8.0, argb(col, 0.14 * UI.a_start * a), 2.0);
    }
    // disk + glyph
    fill_circle(g, cx, cy, r, argb(mix(pc(P_INPUT), col, 0.13), a));
    stroke_circle(g, cx, cy, r - 1.0, argb(col, 0.55 * a), 1.6);
    let k = 1.0 + 0.035 * ((ph * 2.2).sin()) * UI.a_conn;
    glyph(g, cx, cy, k, argb(col, a));
    GdipResetClip(g);
    GdipDeletePath(clip);
    // status text with cross-fade
    let f = ease_out(UI.status_fade);
    if f < 1.0 {
        draw_status_block(g, UI.prev_status, a * (1.0 - f), -8.0 * f);
    }
    draw_status_block(g, UI.status, a * f, 8.0 * (1.0 - f));
}
unsafe fn draw_field(g: H, a: f32) {
    let (x, y, w, h) = FIELD;
    text(
        g,
        "Имя устройства в AirPlay",
        x + 4.0,
        316.0,
        300.0,
        18.0,
        F_SMALL,
        argb(pc(SEC), a),
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
        "Включить AirPlay",
        x,
        y + 14.0,
        w,
        24.0,
        F_BTN,
        argb(0xFFFFFF, a * (1.0 - run)),
        FMT_C,
    );
    text(
        g,
        "Остановить приём",
        x,
        y + 14.0,
        w,
        24.0,
        F_BTN,
        argb(pc(TEXT), a * run),
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
    text(
        g,
        "Как подключиться",
        24.0,
        478.0,
        300.0,
        20.0,
        F_BADGE,
        argb(pc(TEXT), a),
        FMT_L,
    );
    let rows = [
        ("1", "Откройте «Повтор экрана» в Пункте управления", 506.0),
        ("2", "Выберите устройство — имя указано выше", 538.0),
    ];
    for (n, t, y) in rows {
        fill_circle(g, 35.0, y + 11.0, 11.0, argb(pc(ACC_TXT), a * 0.13));
        text(
            g,
            n,
            24.0,
            y + 3.5,
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
    let items = ["Экран и звук", "Локально", "до 60 FPS"];
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
            argb(pc(TER), a),
            FMT_L,
        );
        x += 22.0 + widths[i] + gap;
    }
}
unsafe fn render(g: H) {
    GdipSetSmoothingMode(g, 4);
    GdipSetTextRenderingHint(g, 4);
    GdipSetInterpolationMode(g, 7);
    GdipSetPixelOffsetMode(g, 2);
    GdipGraphicsClear(g, argb(pc(BG), 1.0));
    GdipScaleWorldTransform(g, UI.s, UI.s, 0);
    let (a, st) = sec(g, 0);
    image(g, IMG_HEAD, 24.0, 24.0, 48.0, 48.0, 96.0, a);
    text(
        g,
        "Aireluma",
        84.0,
        22.0,
        200.0,
        30.0,
        F_TITLE,
        argb(pc(TEXT), a),
        FMT_L,
    );
    text(
        g,
        "Повтор экрана с iOS",
        85.0,
        52.0,
        200.0,
        18.0,
        F_BODY,
        argb(pc(SEC), a),
        FMT_L,
    );
    draw_toggle(g, a);
    GdipRestoreGraphics(g, st);
    let (a, st) = sec(g, 1);
    draw_hero(g, a);
    GdipRestoreGraphics(g, st);
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
    if BB_DC.is_null() || BB_W != w || BB_H != hh {
        if !BB_BMP.is_null() {
            DeleteObject(BB_BMP);
        }
        if BB_DC.is_null() {
            BB_DC = CreateCompatibleDC(dc);
        }
        BB_BMP = CreateCompatibleBitmap(dc, w, hh);
        SelectObject(BB_DC, BB_BMP);
        BB_W = w;
        BB_H = hh;
    }
    let mut g = null_mut();
    GdipCreateFromHDC(BB_DC, &mut g);
    render(g);
    GdipDeleteGraphics(g);
    BitBlt(dc, 0, 0, w, hh, BB_DC, 0, 0, 0x00CC0020);
    EndPaint(h, &ps);
}

// ───────────────────────────── Animation engine ─────────────────────────────
unsafe fn kick(h: H) {
    if !UI.timer {
        UI.timer = true;
        UI.last = GetTickCount64();
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
    if visible {
        InvalidateRect(h, null(), 0);
        UpdateWindow(h);
    }
    if !visible || !(moving || continuous) {
        KillTimer(h, 2);
        UI.timer = false;
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
                    wide(&format!("Не удалось запустить приёмник: {e}")).as_ptr(),
                    wide("Ошибка").as_ptr(),
                    0x10,
                );
            }
        }
    }
}
unsafe fn cycle_focus(h: H, back: bool) {
    let order: Vec<i32> = if CHILD.is_some() {
        vec![1, 2]
    } else {
        vec![0, 1, 2]
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
    let tip = wide("Aireluma — приём экрана iPhone");
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
        let title = wide("Aireluma — экран iPhone");
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
            kick(h);
            0
        }
        0x20 => {
            if (l & 0xffff) == 1 && (UI.over_btn || UI.over_tog) {
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
            if UI.over_btn {
                UI.press_item = 1;
                UI.focus = 1;
                SetCapture(h);
            } else if UI.over_tog {
                UI.press_item = 2;
                UI.focus = 2;
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
            if item == 1 && UI.over_btn {
                receiver_toggle(h);
            } else if item == 2 && UI.over_tog {
                theme_toggle(h);
            }
            kick(h);
            0
        }
        0x100 => {
            let repeat = (l >> 30) & 1 != 0;
            if !repeat && (w == 0x20 || w == 0x0d) {
                if UI.focus == 2 {
                    theme_toggle(h);
                } else {
                    receiver_toggle(h);
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
                    AppendMenuW(menu, 0, 101, wide("Открыть Aireluma").as_ptr());
                    AppendMenuW(menu, 0x800, 0, null());
                    AppendMenuW(menu, 0, 102, wide("Выйти").as_ptr());
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
