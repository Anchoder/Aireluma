#![allow(static_mut_refs)]
use std::{
    env,
    ffi::OsStr,
    fs,
    io::{BufRead, BufReader},
    os::windows::ffi::OsStrExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    ptr::{null, null_mut},
    thread,
};
type H = *mut core::ffi::c_void;
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
struct Size {
    cx: i32,
    cy: i32,
}
#[repr(C)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[repr(C)]
struct DrawItem {
    ctl_type: u32,
    ctl_id: u32,
    item_id: u32,
    action: u32,
    state: u32,
    hwnd: H,
    dc: H,
    rect: Rect,
    data: usize,
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
    fn AdjustWindowRectEx(rect: *mut Rect, style: u32, menu: i32, ex: u32) -> i32;
    fn RegisterClassW(c: *const WndClass) -> u16;
    fn IsWindowVisible(h: H) -> i32;
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
    fn ScreenToClient(h: H, p: *mut Point) -> i32;
    fn GetParent(h: H) -> H;
    fn GetFocus() -> H;
    fn SetClassLongPtrW(h: H, index: i32, value: isize) -> isize;
    fn RedrawWindow(h: H, rect: *const Rect, region: H, flags: u32) -> i32;
    fn DestroyIcon(icon: H) -> i32;
    fn GetDC(h: H) -> H;
    fn ReleaseDC(h: H, dc: H) -> i32;
    fn DrawIconEx(
        dc: H,
        x: i32,
        y: i32,
        icon: H,
        cx: i32,
        cy: i32,
        step: u32,
        brush: H,
        flags: u32,
    ) -> i32;
    fn InvalidateRect(h: H, rect: *const Rect, erase: i32) -> i32;
    fn DrawTextW(dc: H, text: *const u16, len: i32, rect: *mut Rect, format: u32) -> i32;
    fn DrawFocusRect(dc: H, rect: *const Rect) -> i32;
    fn FillRect(dc: H, rect: *const Rect, brush: H) -> i32;
    fn IsDialogMessageW(h: H, m: *mut Msg) -> i32;
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
    fn SetWindowTextW(h: H, s: *const u16) -> i32;
    fn GetWindowTextW(h: H, s: *mut u16, max: i32) -> i32;
    fn MessageBoxW(h: H, s: *const u16, t: *const u16, flags: u32) -> i32;
    fn KillTimer(h: H, id: usize) -> i32;
    fn SystemParametersInfoW(action: u32, param: u32, value: *mut i32, flags: u32) -> i32;
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
    fn GetTextExtentPoint32W(dc: H, text: *const u16, len: i32, size: *mut Size) -> i32;
    fn Ellipse(dc: H, left: i32, top: i32, right: i32, bottom: i32) -> i32;
    fn MoveToEx(dc: H, x: i32, y: i32, old: *mut Point) -> i32;
    fn LineTo(dc: H, x: i32, y: i32) -> i32;
    fn CreateSolidBrush(color: u32) -> H;
    fn CreatePen(style: i32, width: i32, color: u32) -> H;
    fn SelectObject(dc: H, obj: H) -> H;
    fn DeleteObject(obj: H) -> i32;
    fn RoundRect(dc: H, l: i32, t: i32, r: i32, b: i32, w: i32, h: i32) -> i32;
    fn SetTextColor(dc: H, color: u32) -> u32;
    fn SetBkColor(dc: H, color: u32) -> u32;
    fn SetBkMode(dc: H, mode: i32) -> i32;
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
#[derive(Clone, Copy)]
struct Palette {
    bg: u32,
    card: u32,
    border: u32,
    input: u32,
    input_border: u32,
    step: u32,
    text: u32,
    secondary: u32,
    subtle: u32,
    accent: u32,
    hover: u32,
    pressed: u32,
    running: u32,
    status: u32,
    active: u32,
    theme_button: u32,
}
static mut DARK: bool = false;
static mut THEME_BUTTON: H = null_mut();
fn appearance_path() -> PathBuf {
    PathBuf::from(env::var_os("APPDATA").unwrap_or_default())
        .join("Aireluma")
        .join("appearance")
}
fn theme_palette(dark: bool) -> Palette {
    if dark {
        Palette {
            bg: 0x00141010,
            card: 0x001e1818,
            border: 0x00312b2b,
            input: 0x00161010,
            input_border: 0x003b3434,
            step: 0x002b2020,
            text: 0x00f6f3f3,
            secondary: 0x00c9c1c2,
            subtle: 0x00a79d9f,
            accent: 0x00ed6477,
            hover: 0x00f37688,
            pressed: 0x00ce5663,
            running: 0x004e3e39,
            status: 0x008d8588,
            active: 0x00c7dca5,
            theme_button: 0x00332828,
        }
    } else {
        Palette {
            bg: 0x00faf7f7,
            card: 0x00ffffff,
            border: 0x00e6dddd,
            input: 0x00faf7f7,
            input_border: 0x00dfd4d4,
            step: 0x00f9ecea,
            text: 0x00332924,
            secondary: 0x006a605c,
            subtle: 0x00756b66,
            accent: 0x00dd5367,
            hover: 0x00e76578,
            pressed: 0x00c94459,
            running: 0x00f4e8e5,
            status: 0x00988c86,
            active: 0x006b7932,
            theme_button: 0x00efe8e6,
        }
    }
}
static mut ANIMATION_FROM: Option<Palette> = None;
static mut ANIMATION_START: u64 = 0;
static mut ICON_THEME: Option<bool> = None;
static mut RECEIVER_READY: bool = false;
fn blend(a: u32, b: u32, t: f32) -> u32 {
    let mut result = 0;
    for shift in [0, 8, 16] {
        let x = ((a >> shift) & 255) as f32;
        let y = ((b >> shift) & 255) as f32;
        result |= ((x + (y - x) * t).round() as u32) << shift;
    }
    result
}
unsafe fn palette() -> Palette {
    let target = theme_palette(DARK);
    if let Some(from) = ANIMATION_FROM {
        let t = ((GetTickCount64() - ANIMATION_START) as f32 / 240.0).clamp(0.0, 1.0);
        let t = t * t * (3.0 - 2.0 * t);
        Palette {
            bg: blend(from.bg, target.bg, t),
            card: blend(from.card, target.card, t),
            border: blend(from.border, target.border, t),
            input: blend(from.input, target.input, t),
            input_border: blend(from.input_border, target.input_border, t),
            step: blend(from.step, target.step, t),
            text: blend(from.text, target.text, t),
            secondary: blend(from.secondary, target.secondary, t),
            subtle: blend(from.subtle, target.subtle, t),
            accent: blend(from.accent, target.accent, t),
            hover: blend(from.hover, target.hover, t),
            pressed: blend(from.pressed, target.pressed, t),
            running: blend(from.running, target.running, t),
            status: blend(from.status, target.status, t),
            active: blend(from.active, target.active, t),
            theme_button: blend(from.theme_button, target.theme_button, t),
        }
    } else {
        target
    }
}
unsafe fn set_brushes() {
    let p = palette();
    for slot in [
        &raw mut BG,
        &raw mut CARD,
        &raw mut STEP_BG,
        &raw mut EDIT_BG,
    ]
    .into_iter()
    .zip([p.bg, p.card, p.step, p.input])
    {
        let old = *slot.0;
        *slot.0 = CreateSolidBrush(slot.1);
        if !old.is_null() {
            DeleteObject(old);
        }
    }
}
unsafe fn apply_theme(h: H) {
    set_brushes();
    SetClassLongPtrW(h, -10, BG as isize);
    let p = palette();
    let dark = (((p.bg & 255) + ((p.bg >> 8) & 255) + ((p.bg >> 16) & 255)) / 3 < 128) as i32;
    DwmSetWindowAttribute(h, 20, &dark, 4);
    DwmSetWindowAttribute(h, 35, &(p.bg as i32), 4);
    DwmSetWindowAttribute(h, 34, &(p.border as i32), 4);
    if ICON_THEME != Some(DARK) {
        for i in 0..3 {
            let old = FOOTER_ICONS[i];
            FOOTER_ICONS[i] = LoadImageW(
                GetModuleHandleW(null()),
                (if DARK { i + 2 } else { i + 5 }) as *const u16,
                1,
                16,
                16,
                0,
            );
            if !old.is_null() {
                DestroyIcon(old);
            }
        }
    }
    ICON_THEME = Some(DARK);
    label(
        THEME_BUTTON,
        if DARK {
            "Светлая тема"
        } else {
            "Тёмная тема"
        },
    );
    RedrawWindow(h, null(), null_mut(), 0x1 | 0x4 | 0x80 | 0x100);
}
static mut FIELD_LABEL: H = null_mut();
static mut STEP_ONE: H = null_mut();
static mut STEP_TWO: H = null_mut();
static mut SECTION_TITLE: H = null_mut();
static mut SECTION_FONT: H = null_mut();
static mut FEATURE: H = null_mut();
static mut STEP_ONE_NOTE: H = null_mut();
static mut STEP_TWO_NOTE: H = null_mut();
static mut EDIT_FOCUS: bool = false;
static mut HOVER: bool = false;
static mut ACTIVE: bool = false;
static mut HEADER_HINT: H = null_mut();
static mut VIDEO_WINDOW: H = null_mut();
static mut GENERATION: usize = 0;
static mut ICON: H = null_mut();
static mut TRAY_READY: bool = false;
static mut TASKBAR_CREATED: u32 = 0;
static mut CHILD: Option<Child> = None;
static mut STATUS: H = null_mut();
static mut INPUT: H = null_mut();
static mut BUTTON: H = null_mut();
static mut BG: H = null_mut();
static mut CARD: H = null_mut();
static mut STEP_BG: H = null_mut();
static mut HEADER_TITLE: H = null_mut();
static mut HEADER_TAG: H = null_mut();
static mut EDIT_BG: H = null_mut();
static mut FOOTER_ICONS: [H; 3] = [null_mut(); 3];
static mut FOOTER_X: [i32; 3] = [24, 160, 268];
static mut INPUT_FONT: H = null_mut();
static mut FONT: H = null_mut();
static mut TITLE_FONT: H = null_mut();
static mut MUTED_FONT: H = null_mut();
static mut BUTTON_FONT: H = null_mut();
fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}
unsafe fn label(h: H, s: &str) {
    SetWindowTextW(h, wide(s).as_ptr());
    if h == STATUS {
        let r = Rect {
            left: 42,
            top: 198,
            right: 54,
            bottom: 214,
        };
        InvalidateRect(GetParent(h), &r, 0);
    }
}
unsafe fn control(
    parent: H,
    class: &str,
    text: &str,
    style: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: usize,
) -> H {
    let c = CreateWindowExW(
        0,
        wide(class).as_ptr(),
        wide(text).as_ptr(),
        0x40000000 | 0x10000000 | style,
        x,
        y,
        w,
        h,
        parent,
        id as H,
        GetModuleHandleW(null()),
        null_mut(),
    );
    SendMessageW(c, 0x30, FONT as usize, 1);
    c
}
unsafe fn surface(dc: H, l: i32, t: i32, r: i32, b: i32, fill: u32, border: u32, radius: i32) {
    let brush = CreateSolidBrush(fill);
    let pen = CreatePen(0, 1, border);
    let ob = SelectObject(dc, brush);
    let op = SelectObject(dc, pen);
    RoundRect(dc, l, t, r, b, radius, radius);
    SelectObject(dc, ob);
    SelectObject(dc, op);
    DeleteObject(brush);
    DeleteObject(pen);
}
unsafe fn create_footer(h: H) {
    let texts = ["Экран и звук", "Локально", "до 60 FPS"];
    let dc = GetDC(h);
    let old = SelectObject(dc, MUTED_FONT);
    let mut widths = [0; 3];
    for i in 0..3 {
        let text = wide(texts[i]);
        let mut size = Size { cx: 0, cy: 0 };
        GetTextExtentPoint32W(dc, text.as_ptr(), text.len() as i32 - 1, &mut size);
        widths[i] = size.cx + 1;
    }
    SelectObject(dc, old);
    ReleaseDC(h, dc);
    let spare = 328 - widths.iter().sum::<i32>() - 72;
    let gaps = [spare / 2, spare - spare / 2];
    let mut x = 24;
    for i in 0..3 {
        FOOTER_X[i] = x;
        let text = control(h, "STATIC", texts[i], 0x200, x + 24, 488, widths[i], 24, 0);
        SendMessageW(text, 0x30, MUTED_FONT as usize, 1);
        if i == 0 {
            FEATURE = text;
        }
        x += 24 + widths[i] + if i < 2 { gaps[i] } else { 0 };
    }
}

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
fn paths() -> (PathBuf, PathBuf) {
    let root = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    (
        root.join("runtime").join("uxplay.exe"),
        root.join("runtime"),
    )
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
unsafe extern "system" fn wnd(h: H, m: u32, w: usize, l: isize) -> isize {
    match m {
        1 => {
            let mark = control(h, "STATIC", "", 3, 24, 24, 48, 48, 0);
            SendMessageW(mark, 0x170, ICON as usize, 0);
            HEADER_TITLE = control(h, "STATIC", "Aireluma", 0, 88, 20, 158, 36, 0);
            SendMessageW(HEADER_TITLE, 0x30, TITLE_FONT as usize, 1);
            HEADER_TAG = control(h, "STATIC", "Повтор экрана с iOS", 0, 89, 56, 158, 22, 0);
            SendMessageW(HEADER_TAG, 0x30, MUTED_FONT as usize, 1);
            THEME_BUTTON = control(
                h,
                "BUTTON",
                if DARK {
                    "Светлая тема"
                } else {
                    "Тёмная тема"
                },
                0x10000 | 0x0b,
                254,
                30,
                98,
                28,
                4,
            );
            FIELD_LABEL = control(
                h,
                "STATIC",
                "Имя устройства в AirPlay",
                0,
                40,
                116,
                296,
                20,
                0,
            );
            SendMessageW(FIELD_LABEL, 0x30, MUTED_FONT as usize, 1);
            INPUT = control(h, "EDIT", "Aireluma", 0x80 | 0x10000, 42, 149, 292, 23, 2);
            SendMessageW(INPUT, 0x30, INPUT_FONT as usize, 1);
            SendMessageW(INPUT, 0xd3, 3, 10 | (10 << 16));
            STATUS = control(h, "STATIC", "Приём выключен", 0, 60, 193, 276, 24, 0);
            BUTTON = control(
                h,
                "BUTTON",
                "Включить AirPlay",
                0x10000 | 0x0b,
                40,
                232,
                296,
                40,
                1,
            );
            SECTION_TITLE = control(h, "STATIC", "Как подключиться", 0, 24, 316, 328, 26, 0);
            SendMessageW(SECTION_TITLE, 0x30, SECTION_FONT as usize, 1);
            STEP_ONE = control(h, "STATIC", "1", 1, 25, 363, 26, 20, 0);
            SendMessageW(STEP_ONE, 0x30, MUTED_FONT as usize, 1);
            HEADER_HINT = control(
                h,
                "STATIC",
                "Откройте «Повтор экрана»",
                0,
                68,
                350,
                284,
                24,
                0,
            );
            STEP_ONE_NOTE = control(
                h,
                "STATIC",
                "В Пункте управления iPhone",
                0,
                68,
                375,
                284,
                20,
                0,
            );
            SendMessageW(STEP_ONE_NOTE, 0x30, MUTED_FONT as usize, 1);
            STEP_TWO = control(h, "STATIC", "2", 1, 25, 421, 26, 20, 0);
            SendMessageW(STEP_TWO, 0x30, MUTED_FONT as usize, 1);
            control(h, "STATIC", "Выберите устройство", 0, 68, 408, 284, 24, 3);
            STEP_TWO_NOTE = control(
                h,
                "STATIC",
                "Имя указано в поле выше",
                0,
                68,
                433,
                284,
                20,
                0,
            );
            SendMessageW(STEP_TWO_NOTE, 0x30, MUTED_FONT as usize, 1);
            create_footer(h);
            SetTimer(h, 1, 1000, null_mut());
            SetTimer(h, 2, 50, null_mut());
            TRAY_READY = tray(h, 0);
            0
        }
        0x0f => {
            let mut ps: Paint = std::mem::zeroed();
            let dc = BeginPaint(h, &mut ps);
            let p = palette();
            surface(dc, 24, 100, 352, 292, p.card, p.border, 20);
            surface(
                dc,
                40,
                140,
                336,
                180,
                p.input,
                if EDIT_FOCUS { p.accent } else { p.input_border },
                12,
            );
            surface(dc, 24, 358, 52, 386, p.step, p.step, 10);
            surface(dc, 24, 416, 52, 444, p.step, p.step, 10);
            let pen = CreatePen(0, 1, p.border);
            let op = SelectObject(dc, pen);
            MoveToEx(dc, 24, 474, null_mut());
            LineTo(dc, 352, 474);
            SelectObject(dc, op);
            DeleteObject(pen);
            let color = if ACTIVE {
                p.active
            } else if CHILD.is_some() {
                p.accent
            } else {
                p.status
            };
            let brush = CreateSolidBrush(color);
            let dp = CreatePen(0, 1, color);
            let ob = SelectObject(dc, brush);
            let oldp = SelectObject(dc, dp);
            Ellipse(dc, 44, 202, 51, 209);
            SelectObject(dc, ob);
            SelectObject(dc, oldp);
            DeleteObject(brush);
            DeleteObject(dp);
            for i in 0..3 {
                DrawIconEx(
                    dc,
                    FOOTER_X[i],
                    492,
                    FOOTER_ICONS[i],
                    16,
                    16,
                    0,
                    null_mut(),
                    3,
                );
            }
            EndPaint(h, &ps);
            0
        }
        0x2b => {
            let item = &mut *(l as *mut DrawItem);
            if item.ctl_id == 1 || item.ctl_id == 4 {
                let p = palette();
                let theme = item.ctl_id == 4;
                FillRect(item.dc, &item.rect, if theme { BG } else { CARD });
                let pressed = item.state & 1 != 0;
                let running = CHILD.is_some();
                let color = if theme {
                    p.theme_button
                } else if running {
                    p.running
                } else if pressed {
                    p.pressed
                } else if HOVER {
                    p.hover
                } else {
                    p.accent
                };
                let brush = CreateSolidBrush(color);
                let pen = CreatePen(0, 1, color);
                let op = SelectObject(item.dc, pen);
                let ob = SelectObject(item.dc, brush);
                RoundRect(
                    item.dc,
                    item.rect.left,
                    item.rect.top,
                    item.rect.right,
                    item.rect.bottom,
                    12,
                    12,
                );
                let oldfont = SelectObject(item.dc, if theme { MUTED_FONT } else { BUTTON_FONT });
                SetBkMode(item.dc, 1);
                SetTextColor(item.dc, if theme || running { p.text } else { 0x00ffffff });
                let mut buf = [0u16; 128];
                let len = GetWindowTextW(item.hwnd, buf.as_mut_ptr(), 128);
                DrawTextW(item.dc, buf.as_ptr(), len, &mut item.rect, 0x25);
                if item.state & 0x10 != 0 {
                    let r = Rect {
                        left: item.rect.left + 4,
                        top: item.rect.top + 4,
                        right: item.rect.right - 4,
                        bottom: item.rect.bottom - 4,
                    };
                    DrawFocusRect(item.dc, &r);
                }
                SelectObject(item.dc, oldfont);
                SelectObject(item.dc, op);
                SelectObject(item.dc, ob);
                DeleteObject(pen);
                DeleteObject(brush);
                1
            } else {
                0
            }
        }
        0x113 if w == 3 => {
            if GetTickCount64() - ANIMATION_START >= 240 {
                ANIMATION_FROM = None;
                KillTimer(h, 3);
            }
            apply_theme(h);
            0
        }
        0x113 if w == 2 => {
            let mut p = Point { x: 0, y: 0 };
            GetCursorPos(&mut p);
            ScreenToClient(h, &mut p);
            let over = p.x >= 40 && p.x < 336 && p.y >= 232 && p.y < 272;
            if over != HOVER {
                HOVER = over;
                InvalidateRect(BUTTON, null(), 0);
            }
            0
        }
        0x8012 => {
            if w == GENERATION && CHILD.is_some() {
                if l == 2 {
                    RECEIVER_READY = true;
                    label(STATUS, "Ожидание iPhone");
                } else if l == 1 {
                    RECEIVER_READY = true;
                    ACTIVE = true;
                    label(STATUS, "Экран подключён");
                } else {
                    ACTIVE = false;
                    label(
                        STATUS,
                        if RECEIVER_READY {
                            "Ожидание iPhone"
                        } else {
                            "Запуск приёмника…"
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
        0x138 if l as H == INPUT => {
            let p = palette();
            SetTextColor(w as H, p.subtle);
            SetBkColor(w as H, p.input);
            EDIT_BG as isize
        }
        0x138 => {
            let p = palette();
            let child = l as H;
            let step = child == STEP_ONE || child == STEP_TWO;
            let in_panel = child == FIELD_LABEL || child == STATUS;
            SetTextColor(
                w as H,
                if child == HEADER_TITLE || child == SECTION_TITLE {
                    p.text
                } else if child == STATUS && ACTIVE {
                    p.active
                } else if step {
                    p.accent
                } else if child == STEP_ONE_NOTE || child == STEP_TWO_NOTE || child == HEADER_TAG {
                    p.subtle
                } else {
                    p.secondary
                },
            );
            SetBkColor(
                w as H,
                if step {
                    p.step
                } else if in_panel {
                    p.card
                } else {
                    p.bg
                },
            );
            SetBkMode(w as H, 2);
            if step {
                STEP_BG as isize
            } else if in_panel {
                CARD as isize
            } else {
                BG as isize
            }
        }
        0x111 if w & 0xffff == 2 && ((w >> 16) == 0x100 || (w >> 16) == 0x200) => {
            EDIT_FOCUS = GetFocus() == INPUT;
            let r = Rect {
                left: 39,
                top: 139,
                right: 337,
                bottom: 181,
            };
            InvalidateRect(h, &r, 0);
            0
        }
        0x133 => {
            let p = palette();
            SetTextColor(w as H, p.text);
            SetBkColor(w as H, p.input);
            EDIT_BG as isize
        }
        0x111 if w & 0xffff == 4 => {
            let from = palette();
            DARK = !DARK;
            let mut animations = 1;
            SystemParametersInfoW(0x1042, 0, &mut animations, 0);
            if animations != 0 {
                ANIMATION_FROM = Some(from);
                ANIMATION_START = GetTickCount64();
                SetTimer(h, 3, 16, null_mut());
            } else {
                ANIMATION_FROM = None;
            }
            let path = appearance_path();
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(path, if DARK { "dark" } else { "light" });
            apply_theme(h);
            0
        }
        0x111 if w & 0xffff == 1 => {
            if let Some(mut c) = CHILD.take() {
                GENERATION = GENERATION.wrapping_add(1);
                let _ = c.kill();
                let _ = c.wait();
                ACTIVE = false;
                label(STATUS, "Приём выключен");
                label(BUTTON, "Включить AirPlay");
                EnableWindow(INPUT, 1);
            } else {
                let mut b = [0u16; 256];
                let n = GetWindowTextW(INPUT, b.as_mut_ptr(), 256);
                let name = String::from_utf16_lossy(&b[..n.max(0) as usize]);
                let (exe, dir) = paths();
                let path = format!("{};{}", dir.display(), env::var("PATH").unwrap_or_default());
                match Command::new(exe)
                    .env("PATH", path)
                    .env_remove("AIRELUMA_VIDEO_HWND")
                    .env("GST_PLUGIN_SYSTEM_PATH_1_0", dir.join("plugins"))
                    .env("GST_PLUGIN_SCANNER_1_0", dir.join("gst-plugin-scanner.exe"))
                    .args([
                        "-n",
                        if name.trim().is_empty() {
                            "Aireluma"
                        } else {
                            &name
                        },
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
                                    let state = if line
                                        .windows(23)
                                        .any(|w| w == b"AIRELUMA_RECEIVER_READY")
                                    {
                                        Some(2)
                                    } else if line
                                        .windows(23)
                                        .any(|w| w == b"AIRELUMA_STREAM_STARTED")
                                    {
                                        Some(1)
                                    } else if line
                                        .windows(23)
                                        .any(|w| w == b"AIRELUMA_STREAM_STOPPED")
                                    {
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
                        label(STATUS, "Запуск приёмника…");
                        label(BUTTON, "Остановить приём");
                        EnableWindow(INPUT, 0);
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
            0
        }
        0x113 => {
            if let Some(c) = CHILD.as_mut() {
                EnumWindows(Some(brand_video), c.id() as isize);
                if matches!(c.try_wait(), Ok(Some(_))) {
                    CHILD = None;
                    ACTIVE = false;
                    label(STATUS, "Не удалось запустить приёмник");
                    label(BUTTON, "Включить AirPlay");
                    EnableWindow(INPUT, 1);
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
use std::os::windows::process::CommandExt;
pub fn run() {
    unsafe {
        SetCurrentProcessExplicitAppUserModelID(wide("Aireluma.Desktop").as_ptr());
        let prior = FindWindowW(wide("AirPlayWindow").as_ptr(), null());
        if !prior.is_null() {
            PostMessageW(prior, 0x8011, 0, 0);
            return;
        }
        let instance = GetModuleHandleW(null());
        ICON = LoadImageW(instance, 1usize as *const u16, 1, 48, 48, 0);
        for i in 0..3 {
            FOOTER_ICONS[i] = LoadImageW(instance, (i + 2) as *const u16, 1, 16, 16, 0);
        }
        TASKBAR_CREATED = RegisterWindowMessageW(wide("TaskbarCreated").as_ptr());
        DARK = fs::read_to_string(appearance_path())
            .map(|v| v.trim() == "dark")
            .unwrap_or(false);
        set_brushes();
        INPUT_FONT = CreateFontW(
            -14,
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
        FONT = CreateFontW(
            -14,
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
        MUTED_FONT = CreateFontW(
            -13,
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
        BUTTON_FONT = CreateFontW(
            -14,
            0,
            0,
            0,
            600,
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
        SECTION_FONT = CreateFontW(
            -16,
            0,
            0,
            0,
            600,
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
        TITLE_FONT = CreateFontW(
            -27,
            0,
            0,
            0,
            600,
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
        let class = wide("AirPlayWindow");
        let wc = WndClass {
            style: 0,
            proc: Some(wnd),
            cls_extra: 0,
            wnd_extra: 0,
            instance,
            icon: ICON,
            cursor: LoadCursorW(null_mut(), 32512usize as *const u16),
            background: BG,
            menu: null(),
            name: class.as_ptr(),
        };
        RegisterClassW(&wc);
        let mut bounds = Rect {
            left: 0,
            top: 0,
            right: 376,
            bottom: 528,
        };
        AdjustWindowRectEx(&mut bounds, 0x00ca0000, 0, 0);
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
        apply_theme(h);
        ShowWindow(h, 1);
        UpdateWindow(h);
        let mut msg: Msg = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            if IsDialogMessageW(h, &mut msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
