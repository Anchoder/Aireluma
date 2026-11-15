use gtk::{cairo, gdk, gio, glib, pango, prelude::*};
use std::{
    cell::RefCell,
    env,
    f64::consts::PI,
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    rc::Rc,
    sync::mpsc,
    thread,
    time::Duration,
};

// ───────────────────────────── Layout (logical pixels) ─────────────────────────────
const WIN_W: i32 = 380;
const WIN_H: i32 = 668;
type Rect = (f64, f64, f64, f64);
const BTN: Rect = (20.0, 404.0, 340.0, 52.0);
const TOG: Rect = (304.0, 33.0, 56.0, 32.0);
const SEG: Rect = (82.0, 628.0, 216.0, 28.0);
const FIELD: Rect = (20.0, 340.0, 340.0, 48.0);
const HERO: Rect = (20.0, 92.0, 340.0, 212.0);

// ───────────────────────────── Palette ─────────────────────────────
const BG: usize = 0;
const CARD: usize = 1;
const FILL: usize = 2;
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

fn mix(a: u32, b: u32, t: f64) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let mut r = 0;
    for sh in [0, 8, 16] {
        let x = ((a >> sh) & 255) as f64;
        let y = ((b >> sh) & 255) as f64;
        r |= ((x + (y - x) * t).round() as u32) << sh;
    }
    r
}
fn ease_out(t: f64) -> f64 {
    let t = 1.0 - t.clamp(0.0, 1.0);
    1.0 - t * t * t
}
fn approach(v: &mut f64, target: f64, dt: f64, k: f64) -> bool {
    *v += (target - *v) * (1.0 - (-k * dt).exp());
    if (target - *v).abs() < 0.003 {
        *v = target;
    }
    *v != target
}
fn rgb_f(c: u32) -> (f64, f64, f64) {
    (
        ((c >> 16) & 255) as f64 / 255.0,
        ((c >> 8) & 255) as f64 / 255.0,
        (c & 255) as f64 / 255.0,
    )
}

// ───────────────────────────── Languages ─────────────────────────────
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
}
/// Translation table: Russian, English, German.
fn tr(lang: Lang, k: Str) -> &'static str {
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
    };
    t[lang as usize]
}

// ───────────────────────────── State ─────────────────────────────
enum Event {
    Ready,
    Stream(bool),
}
#[derive(Clone, Copy, PartialEq)]
enum Status {
    Off,
    Starting,
    Waiting,
    Connected,
    Failed,
}

struct State {
    dark: f64,
    dark_t: f64,
    hover_btn: f64,
    hover_tog: f64,
    press: f64,
    field: f64,
    running: f64,
    a_start: f64,
    a_wait: f64,
    a_conn: f64,
    a_fail: f64,
    enter: f64,
    phase: f64,
    last: Option<i64>,
    skip: bool,
    tick: Option<gtk::TickCallbackId>,
    status: Status,
    status_fade: f64,
    prev_text: (String, String),
    name: String,
    was_ready: bool,
    lang: Lang,
    lang_pos: f64,
    lang_t: f64,
    lang_fade: f64,
    over_btn: bool,
    over_tog: bool,
    over_seg: i32,
    press_item: i32,
    focus: i32,
    kbd: bool,
    edit_focused: bool,
    child: Option<Child>,
    events: Option<mpsc::Receiver<Event>>,
    ready: bool,
    icon: Option<cairo::ImageSurface>,
}

impl State {
    fn new(dark: bool, lang: Lang) -> Self {
        let d = if dark { 1.0 } else { 0.0 };
        State {
            dark: d,
            dark_t: d,
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
            last: None,
            skip: false,
            tick: None,
            status: Status::Off,
            status_fade: 1.0,
            prev_text: (String::new(), String::new()),
            name: String::new(),
            was_ready: false,
            lang,
            lang_pos: lang as i32 as f64,
            lang_t: lang as i32 as f64,
            lang_fade: 1.0,
            over_btn: false,
            over_tog: false,
            over_seg: -1,
            press_item: 0,
            focus: 1,
            kbd: false,
            edit_focused: false,
            child: None,
            events: None,
            ready: false,
            icon: None,
        }
    }
    fn pc(&self, i: usize) -> u32 {
        mix(LIGHT[i], DARK_P[i], self.dark)
    }
    fn tr(&self, k: Str) -> &'static str {
        tr(self.lang, k)
    }
    fn lfade(&self) -> f64 {
        0.2 + 0.8 * ease_out(self.lang_fade)
    }
    fn status_text(&self, s: Status) -> (String, String) {
        let start = self.tr(Str::Start);
        match s {
            Status::Off => (
                self.tr(Str::OffTitle).into(),
                self.tr(Str::OffHint).replace("{}", start),
            ),
            Status::Starting => (
                self.tr(Str::StartingTitle).into(),
                self.tr(Str::StartingHint).into(),
            ),
            Status::Waiting => (
                self.tr(Str::WaitTitle).into(),
                self.tr(Str::WaitHint).replace("{}", &self.name),
            ),
            Status::Connected => (
                self.tr(Str::ConnTitle).into(),
                self.tr(Str::ConnHint).into(),
            ),
            Status::Failed if self.was_ready => (
                self.tr(Str::StoppedTitle).into(),
                self.tr(Str::StoppedHint).replace("{}", start),
            ),
            Status::Failed => (
                self.tr(Str::FailTitle).into(),
                self.tr(Str::FailHint).replace("{}", start),
            ),
        }
    }
    fn set_status(&mut self, s: Status) {
        if self.status != s {
            self.prev_text = self.status_text(self.status);
            self.status = s;
            self.status_fade = 0.0;
        }
    }
    fn stop_child(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.events = None;
        self.ready = false;
    }
}
impl Drop for State {
    fn drop(&mut self) {
        self.stop_child();
    }
}

struct Widgets {
    main: gtk::DrawingArea,
    ambient: gtk::DrawingArea,
    entry: gtk::Entry,
}
#[derive(Clone)]
struct Ctx {
    s: Rc<RefCell<State>>,
    w: Rc<Widgets>,
}

// ───────────────────────────── Settings ─────────────────────────────
fn config_dir() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .filter(|v| !v.is_empty())
                .map(|h| PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(env::temp_dir)
        .join("aireluma")
}
fn save_setting(name: &str, value: &str) {
    let dir = config_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(dir.join(name), value);
}
fn load_dark() -> bool {
    fs::read_to_string(config_dir().join("appearance"))
        .map(|v| v.trim() == "dark")
        .unwrap_or(false)
}
/// Saved choice first, then the session language, otherwise English.
fn load_lang() -> Lang {
    match fs::read_to_string(config_dir().join("language"))
        .ok()
        .as_deref()
        .map(str::trim)
    {
        Some("ru") => return Lang::Ru,
        Some("en") => return Lang::En,
        Some("de") => return Lang::De,
        _ => {}
    }
    for var in ["LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG"] {
        if let Some(v) = env::var_os(var).filter(|v| !v.is_empty()) {
            let v = v.to_string_lossy().to_lowercase();
            return if v.starts_with("ru") {
                Lang::Ru
            } else if v.starts_with("de") {
                Lang::De
            } else {
                Lang::En
            };
        }
    }
    Lang::En
}

// ───────────────────────────── Drawing helpers ─────────────────────────────
fn rgba(cr: &cairo::Context, rgb: u32, a: f64) {
    let (r, g, b) = rgb_f(rgb);
    cr.set_source_rgba(r, g, b, a.clamp(0.0, 1.0));
}
fn rr_path(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.arc(x + r, y + r, r, PI, PI * 1.5);
    cr.close_path();
}
#[allow(clippy::too_many_arguments)]
fn fill_rr(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64, rgb: u32, a: f64) {
    rr_path(cr, x, y, w, h, r);
    rgba(cr, rgb, a);
    let _ = cr.fill();
}
#[allow(clippy::too_many_arguments)]
fn stroke_rr(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    r: f64,
    rgb: u32,
    a: f64,
    width: f64,
) {
    rr_path(cr, x, y, w, h, r);
    rgba(cr, rgb, a);
    cr.set_line_width(width);
    let _ = cr.stroke();
}
fn fill_circle(cr: &cairo::Context, cx: f64, cy: f64, r: f64, rgb: u32, a: f64) {
    cr.new_sub_path();
    cr.arc(cx, cy, r, 0.0, PI * 2.0);
    rgba(cr, rgb, a);
    let _ = cr.fill();
}
fn stroke_circle(cr: &cairo::Context, cx: f64, cy: f64, r: f64, rgb: u32, a: f64, width: f64) {
    cr.new_sub_path();
    cr.arc(cx, cy, r, 0.0, PI * 2.0);
    rgba(cr, rgb, a);
    cr.set_line_width(width);
    let _ = cr.stroke();
}
#[allow(clippy::too_many_arguments)]
fn line(cr: &cairo::Context, x1: f64, y1: f64, x2: f64, y2: f64, rgb: u32, a: f64, width: f64) {
    cr.move_to(x1, y1);
    cr.line_to(x2, y2);
    rgba(cr, rgb, a);
    cr.set_line_width(width);
    let _ = cr.stroke();
}
fn shadow(cr: &cairo::Context, r: Rect, radius: f64, rgb: u32, alpha: f64, dy: f64) {
    for i in 0..9 {
        let e = i as f64 * 2.2;
        let a = alpha * 0.07 * (1.0 - i as f64 / 9.0);
        fill_rr(
            cr,
            r.0 - e,
            r.1 - e + dy,
            r.2 + e * 2.0,
            r.3 + e * 2.0,
            radius + e,
            rgb,
            a,
        );
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Align {
    Left,
    Center,
}
fn layout(
    area: &gtk::DrawingArea,
    s: &str,
    size: f64,
    bold: bool,
    width: f64,
    align: Align,
    wrap: bool,
) -> pango::Layout {
    let l = area.create_pango_layout(Some(s));
    let mut fd = l.context().font_description().unwrap_or_default();
    fd.set_absolute_size(size * pango::SCALE as f64);
    fd.set_weight(if bold {
        pango::Weight::Semibold
    } else {
        pango::Weight::Normal
    });
    l.set_font_description(Some(&fd));
    l.set_width((width * pango::SCALE as f64) as i32);
    l.set_alignment(match align {
        Align::Left => pango::Alignment::Left,
        Align::Center => pango::Alignment::Center,
    });
    if wrap {
        l.set_wrap(pango::WrapMode::WordChar);
    } else {
        l.set_ellipsize(pango::EllipsizeMode::End);
    }
    l
}
/// Single-line text centred vertically on `y`.
#[allow(clippy::too_many_arguments)]
fn text(
    cr: &cairo::Context,
    area: &gtk::DrawingArea,
    s: &str,
    x: f64,
    y: f64,
    w: f64,
    size: f64,
    bold: bool,
    rgb: u32,
    a: f64,
    align: Align,
) {
    if a <= 0.004 {
        return;
    }
    let l = layout(area, s, size, bold, w, align, false);
    let (_, h) = l.pixel_size();
    rgba(cr, rgb, a);
    cr.move_to(x, y - h as f64 / 2.0);
    pangocairo::functions::show_layout(cr, &l);
}
fn text_width(area: &gtk::DrawingArea, s: &str, size: f64) -> f64 {
    let l = layout(area, s, size, false, 10_000.0, Align::Left, false);
    l.pixel_size().0 as f64
}
/// Font size at which `s` fits into `max_w`, never below `min`.
fn fit_size(area: &gtk::DrawingArea, s: &str, size: f64, min: f64, max_w: f64) -> f64 {
    let w = text_width(area, s, size);
    if w <= max_w {
        size
    } else {
        (size * max_w / w).max(min)
    }
}

// ───────────────────────────── Glyph, icons, spinner ─────────────────────────────
fn glyph(cr: &cairo::Context, cx: f64, cy: f64, k: f64, rgb: u32, a: f64) {
    let d = PI / 180.0;
    cr.new_path();
    cr.move_to(cx - 7.0 * k, cy + 6.0 * k);
    cr.line_to(cx - 9.0 * k, cy + 6.0 * k);
    cr.arc(cx - 9.0 * k, cy + 2.0 * k, 4.0 * k, 90.0 * d, 180.0 * d);
    cr.line_to(cx - 13.0 * k, cy - 10.0 * k);
    cr.arc(cx - 9.0 * k, cy - 10.0 * k, 4.0 * k, 180.0 * d, 270.0 * d);
    cr.line_to(cx + 9.0 * k, cy - 14.0 * k);
    cr.arc(cx + 9.0 * k, cy - 10.0 * k, 4.0 * k, 270.0 * d, 360.0 * d);
    cr.line_to(cx + 13.0 * k, cy + 2.0 * k);
    cr.arc(cx + 9.0 * k, cy + 2.0 * k, 4.0 * k, 0.0, 90.0 * d);
    cr.line_to(cx + 7.0 * k, cy + 6.0 * k);
    rgba(cr, rgb, a);
    cr.set_line_width(2.3 * k);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
    let _ = cr.stroke();
    cr.move_to(cx - 8.5 * k, cy + 13.5 * k);
    cr.line_to(cx + 8.5 * k, cy + 13.5 * k);
    cr.line_to(cx, cy + 2.5 * k);
    cr.close_path();
    let _ = cr.fill();
}
fn footer_icon(cr: &cairo::Context, kind: usize, x: f64, y: f64, rgb: u32, a: f64) {
    let d = PI / 180.0;
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
    match kind {
        0 => {
            rr_path(cr, x + 1.0, y + 1.5, 14.0, 9.5, 2.2);
            rgba(cr, rgb, a);
            cr.set_line_width(1.5);
            let _ = cr.stroke();
            line(cr, x + 8.0, y + 11.0, x + 8.0, y + 13.6, rgb, a, 1.5);
            line(cr, x + 5.0, y + 14.2, x + 11.0, y + 14.2, rgb, a, 1.5);
        }
        1 => {
            let (cx, cy) = (x + 8.0, y + 13.0);
            for r in [10.8, 7.0, 3.3] {
                cr.new_sub_path();
                cr.arc(cx, cy, r, 227.0 * d, 313.0 * d);
                rgba(cr, rgb, a);
                cr.set_line_width(1.5);
                let _ = cr.stroke();
            }
            fill_circle(cr, cx, cy + 0.2, 1.3, rgb, a);
        }
        _ => {
            stroke_circle(cr, x + 8.0, y + 8.0, 7.0, rgb, a, 1.5);
            line(cr, x + 8.0, y + 8.0, x + 8.0, y + 4.2, rgb, a, 1.5);
            line(cr, x + 8.0, y + 8.0, x + 11.0, y + 9.8, rgb, a, 1.5);
        }
    }
}
/// Comet-style spinner: a faded tail behind a bright head, eased rotation.
fn spinner(cr: &cairo::Context, cx: f64, cy: f64, rad: f64, rgb: u32, alpha: f64, ph: f64) {
    stroke_circle(cr, cx, cy, rad, rgb, 0.12 * alpha, 2.0);
    let d = PI / 180.0;
    let head = ph * 300.0 + 55.0 * (ph * 2.1).sin();
    let tail = 210.0 + 40.0 * (ph * 1.3).sin();
    let steps = 30;
    let step = tail / steps as f64;
    cr.set_line_cap(cairo::LineCap::Butt);
    for i in 0..steps {
        let t = i as f64 / steps as f64;
        let al = (1.0 - t).powf(1.5) * 0.95 * alpha;
        let start = head - (i + 1) as f64 * step;
        cr.new_sub_path();
        cr.arc(cx, cy, rad, start * d, (start + step + 0.9) * d);
        rgba(cr, rgb, al);
        cr.set_line_width(2.6);
        let _ = cr.stroke();
    }
    let (sn, cs) = (head * d).sin_cos();
    fill_circle(cr, cx + rad * cs, cy + rad * sn, 2.3, rgb, alpha);
}

// ───────────────────────────── Window painting ─────────────────────────────
fn section<F: FnOnce(&cairo::Context, f64)>(cr: &cairo::Context, enter: f64, i: usize, f: F) {
    let p = ease_out((enter - i as f64 * 0.075) / 0.65);
    if p <= 0.0 {
        return;
    }
    if p >= 1.0 {
        f(cr, 1.0);
        return;
    }
    cr.save().ok();
    cr.translate(0.0, (1.0 - p) * 18.0);
    f(cr, p);
    cr.restore().ok();
}

fn draw_toggle(cr: &cairo::Context, st: &State, a: f64) {
    let (x, y, w, h) = TOG;
    let t = st.dark;
    let off = mix(
        mix(st.pc(NEUTRAL), st.pc(TER), 0.22),
        0xFFFFFF,
        st.hover_tog * 0.25,
    );
    let on = mix(st.pc(ACC_B), 0xFFFFFF, st.hover_tog * 0.12);
    fill_rr(cr, x, y, w, h, h / 2.0, mix(off, on, t), a);
    let kx = x + 16.0 + t * (w - 32.0);
    let ky = y + h / 2.0;
    for i in 0..4 {
        fill_circle(cr, kx, ky + 1.4, 13.0 + i as f64 * 1.1, 0x000000, a * 0.05);
    }
    fill_circle(cr, kx, ky, 13.0, 0xFFFFFF, a);
    let sa = (1.0 - t) * a;
    if sa > 0.01 {
        fill_circle(cr, kx, ky, 4.4, 0xF5A524, sa);
        cr.set_line_cap(cairo::LineCap::Round);
        for i in 0..8 {
            let ang = i as f64 * PI / 4.0 + t * 1.6;
            let (s, c) = ang.sin_cos();
            line(
                cr,
                kx + c * 6.9,
                ky + s * 6.9,
                kx + c * 8.7,
                ky + s * 8.7,
                0xF5A524,
                sa,
                1.6,
            );
        }
    }
    let ma = t * a;
    if ma > 0.01 {
        fill_circle(cr, kx, ky, 6.2, 0x5B4BDD, ma);
        fill_circle(cr, kx + 3.0, ky - 2.6, 5.2, 0xFFFFFF, ma);
    }
    if st.kbd && st.focus == 2 {
        stroke_rr(
            cr,
            x - 3.0,
            y - 3.0,
            w + 6.0,
            h + 6.0,
            h / 2.0 + 3.0,
            st.pc(ACC_TXT),
            a * 0.7,
            2.0,
        );
    }
}

fn draw_field(cr: &cairo::Context, area: &gtk::DrawingArea, st: &State, a: f64) {
    let (x, y, w, h) = FIELD;
    text(
        cr,
        area,
        st.tr(Str::FieldLabel),
        x + 4.0,
        324.0,
        300.0,
        12.0,
        false,
        st.pc(SEC),
        a * st.lfade(),
        Align::Left,
    );
    let f = st.field;
    if f > 0.01 {
        stroke_rr(
            cr,
            x - 2.0,
            y - 2.0,
            w + 4.0,
            h + 4.0,
            16.0,
            st.pc(ACC_TXT),
            a * 0.20 * f,
            3.5,
        );
    }
    fill_rr(cr, x, y, w, h, 14.0, st.pc(CARD), a);
    stroke_rr(
        cr,
        x + 0.5,
        y + 0.5,
        w - 1.0,
        h - 1.0,
        14.0,
        mix(st.pc(BORDER), st.pc(ACC_TXT), f),
        a,
        1.0 + 0.5 * f,
    );
}

fn draw_button(cr: &cairo::Context, area: &gtk::DrawingArea, st: &State, a: f64) {
    let (x, y, w, h) = BTN;
    let run = st.running;
    let sc = 1.0 - st.press * 0.022;
    cr.save().ok();
    cr.translate(x + w / 2.0, y + h / 2.0);
    cr.scale(sc, sc);
    cr.translate(-(x + w / 2.0), -(y + h / 2.0));
    let sh = a * (1.0 - run) * (1.0 - st.press * 0.5);
    for i in 0..9 {
        let e = i as f64 * 1.8;
        let al = sh * 0.05 * (1.0 - i as f64 / 9.0);
        fill_rr(
            cr,
            x - e + 6.0,
            y - e + 10.0,
            w + e * 2.0 - 12.0,
            h + e * 2.0 - 4.0,
            16.0 + e,
            st.pc(ACC_B),
            al,
        );
    }
    let lift = st.hover_btn * 0.10 - st.press * 0.06;
    let towards = if lift >= 0.0 { 0xFFFFFF } else { 0x000000 };
    let top = mix(st.pc(ACC_A), towards, lift.abs());
    let bot = mix(st.pc(ACC_B), towards, lift.abs());
    let neutral = mix(st.pc(NEUTRAL), st.pc(TEXT), st.hover_btn * 0.06);
    let (r1, g1, b1) = rgb_f(mix(top, neutral, run));
    let (r2, g2, b2) = rgb_f(mix(bot, neutral, run));
    let grad = cairo::LinearGradient::new(0.0, y, 0.0, y + h);
    grad.add_color_stop_rgba(0.0, r1, g1, b1, a);
    grad.add_color_stop_rgba(1.0, r2, g2, b2, a);
    rr_path(cr, x, y, w, h, 16.0);
    let _ = cr.set_source(&grad);
    let _ = cr.fill();
    cr.set_line_cap(cairo::LineCap::Round);
    line(
        cr,
        x + 16.0,
        y + 0.8,
        x + w - 16.0,
        y + 0.8,
        0xFFFFFF,
        0.28 * a * (1.0 - run),
        1.0,
    );
    let ya = y + h / 2.0;
    text(
        cr,
        area,
        st.tr(Str::Start),
        x,
        ya,
        w,
        16.0,
        true,
        0xFFFFFF,
        a * (1.0 - run) * st.lfade(),
        Align::Center,
    );
    text(
        cr,
        area,
        st.tr(Str::Stop),
        x,
        ya,
        w,
        16.0,
        true,
        st.pc(TEXT),
        a * run * st.lfade(),
        Align::Center,
    );
    cr.restore().ok();
    if st.kbd && st.focus == 1 {
        stroke_rr(
            cr,
            x - 4.0,
            y - 4.0,
            w + 8.0,
            h + 8.0,
            20.0,
            st.pc(ACC_TXT),
            a * 0.7,
            2.0,
        );
    }
}

fn draw_steps(cr: &cairo::Context, area: &gtk::DrawingArea, st: &State, a: f64) {
    let a = a * st.lfade();
    text(
        cr,
        area,
        st.tr(Str::StepsTitle),
        24.0,
        487.0,
        300.0,
        12.0,
        true,
        st.pc(TEXT),
        a,
        Align::Left,
    );
    let rows = [
        ("1", st.tr(Str::Step1), 506.0),
        ("2", st.tr(Str::Step2), 538.0),
    ];
    for (n, t, y) in rows {
        fill_circle(cr, 35.0, y + 11.0, 11.0, st.pc(ACC_TXT), a * 0.13);
        text(
            cr,
            area,
            n,
            24.0,
            y + 11.5,
            22.0,
            12.0,
            true,
            st.pc(ACC_TXT),
            a,
            Align::Center,
        );
        let size = fit_size(area, t, 13.5, 11.0, 306.0);
        text(
            cr,
            area,
            t,
            56.0,
            y + 12.0,
            310.0,
            size,
            false,
            st.pc(SEC),
            a,
            Align::Left,
        );
    }
}

fn draw_footer(cr: &cairo::Context, area: &gtk::DrawingArea, st: &State, a: f64) {
    line(
        cr,
        24.0,
        580.5,
        WIN_W as f64 - 24.0,
        580.5,
        st.pc(BORDER),
        a,
        1.0,
    );
    let at = a * st.lfade();
    let items = [
        st.tr(Str::FeatScreen),
        st.tr(Str::FeatLocal),
        st.tr(Str::FeatFps),
    ];
    let room = WIN_W as f64 - 48.0;
    let mut size = 12.0;
    let mut widths: Vec<f64> = items.iter().map(|s| text_width(area, s, size)).collect();
    let mut total: f64 = widths.iter().map(|w| 22.0 + w).sum::<f64>() + 36.0;
    if total > room {
        size = (size * room / total).max(9.5);
        widths = items.iter().map(|s| text_width(area, s, size)).collect();
        total = widths.iter().map(|w| 22.0 + w).sum::<f64>() + 36.0;
    }
    let gap = ((room - (total - 36.0)) / 2.0).clamp(6.0, 26.0);
    let total = total - 36.0 + gap * 2.0;
    let mut x = (WIN_W as f64 - total) / 2.0;
    for i in 0..3 {
        footer_icon(cr, i, x, 590.5, st.pc(TER), a);
        text(
            cr,
            area,
            items[i],
            x + 22.0,
            599.0,
            widths[i] + 8.0,
            size,
            false,
            st.pc(TER),
            at,
            Align::Left,
        );
        x += 22.0 + widths[i] + gap;
    }
    // segmented language switch with a sliding thumb
    let (sx, sy, sw, sh) = SEG;
    let seg_w = (sw - 4.0) / 3.0;
    fill_rr(
        cr,
        sx,
        sy,
        sw,
        sh,
        sh / 2.0,
        mix(0xE4E4EA, 0x2C2C2E, st.dark),
        a,
    );
    let tx = sx + 2.0 + st.lang_pos * seg_w;
    fill_rr(
        cr,
        tx,
        sy + 3.0,
        seg_w,
        sh - 4.0,
        (sh - 4.0) / 2.0,
        0x000000,
        a * 0.07 * (1.0 - st.dark),
    );
    fill_rr(
        cr,
        tx,
        sy + 2.0,
        seg_w,
        sh - 4.0,
        (sh - 4.0) / 2.0,
        mix(0xFFFFFF, 0x5A5A5F, st.dark),
        a,
    );
    for (i, name) in LANG_NAMES.iter().enumerate() {
        let near = (1.0 - (st.lang_pos - i as f64).abs()).clamp(0.0, 1.0);
        text(
            cr,
            area,
            name,
            sx + 2.0 + i as f64 * seg_w,
            sy + sh / 2.0,
            seg_w,
            12.0,
            true,
            mix(st.pc(SEC), st.pc(TEXT), near),
            a,
            Align::Center,
        );
    }
    if st.kbd && st.focus == 3 {
        stroke_rr(
            cr,
            sx - 3.0,
            sy - 3.0,
            sw + 6.0,
            sh + 6.0,
            sh / 2.0 + 3.0,
            st.pc(ACC_TXT),
            a * 0.7,
            2.0,
        );
    }
}

fn draw_main(cr: &cairo::Context, area: &gtk::DrawingArea, st: &State) {
    rgba(cr, st.pc(BG), 1.0);
    let _ = cr.paint();
    cr.set_line_cap(cairo::LineCap::Round);
    let enter = st.enter;
    section(cr, enter, 0, |cr, a| {
        if let Some(icon) = &st.icon {
            cr.save().ok();
            cr.translate(20.0, 24.0);
            cr.scale(48.0 / icon.width() as f64, 48.0 / icon.height() as f64);
            let _ = cr.set_source_surface(icon, 0.0, 0.0);
            cr.source().set_filter(cairo::Filter::Best);
            let _ = cr.paint_with_alpha(a);
            cr.restore().ok();
        }
        text(
            cr,
            area,
            "Aireluma",
            80.0,
            38.0,
            220.0,
            24.0,
            true,
            st.pc(TEXT),
            a,
            Align::Left,
        );
        text(
            cr,
            area,
            st.tr(Str::Subtitle),
            81.0,
            61.0,
            220.0,
            13.5,
            false,
            st.pc(SEC),
            a * st.lfade(),
            Align::Left,
        );
        draw_toggle(cr, st, a);
    });
    section(cr, enter, 1, |cr, a| {
        shadow(cr, HERO, 24.0, 0x2B2B4A, a * (1.0 - st.dark) * 0.5, 6.0);
        fill_rr(cr, HERO.0, HERO.1, HERO.2, HERO.3, 24.0, st.pc(CARD), a);
        stroke_rr(
            cr,
            HERO.0 + 0.5,
            HERO.1 + 0.5,
            HERO.2 - 1.0,
            HERO.3 - 1.0,
            24.0,
            st.pc(BORDER),
            a,
            1.0,
        );
    });
    section(cr, enter, 2, |cr, a| draw_field(cr, area, st, a));
    section(cr, enter, 3, |cr, a| draw_button(cr, area, st, a));
    section(cr, enter, 4, |cr, a| draw_steps(cr, area, st, a));
    section(cr, enter, 5, |cr, a| draw_footer(cr, area, st, a));
}

fn status_block(
    cr: &cairo::Context,
    area: &gtk::DrawingArea,
    st: &State,
    title: &str,
    hint: &str,
    a: f64,
    dy: f64,
) {
    if a <= 0.01 {
        return;
    }
    text(
        cr,
        area,
        title,
        20.0,
        222.0 + dy,
        340.0,
        19.0,
        true,
        st.pc(TEXT),
        a,
        Align::Center,
    );
    let l = layout(area, hint, 13.5, false, 284.0, Align::Center, true);
    rgba(cr, st.pc(SEC), a);
    cr.move_to(48.0, 239.0 + dy);
    pangocairo::functions::show_layout(cr, &l);
}

/// Everything inside the hero card that animates while the receiver runs.
fn draw_hero(cr: &cairo::Context, area: &gtk::DrawingArea, st: &State) {
    // the widget starts at the card's top-left corner
    cr.translate(-HERO.0, -HERO.1);
    cr.set_line_cap(cairo::LineCap::Round);
    let (cx, cy, r) = (190.0, 153.0, 32.0);
    let act = st.a_start.max(st.a_wait);
    let mut col = mix(st.pc(TER), st.pc(ACC_TXT), act);
    col = mix(col, st.pc(GREEN), st.a_conn);
    col = mix(col, st.pc(RED), st.a_fail);
    cr.save().ok();
    rr_path(cr, HERO.0, HERO.1, HERO.2, HERO.3, 24.0);
    cr.clip();
    let glow = act * 0.6 + st.a_conn * 0.8 + st.a_fail * 0.5;
    if glow > 0.01 {
        let g = cairo::RadialGradient::new(cx, cy, 0.0, cx, cy, 120.0);
        let (rr, gg, bb) = rgb_f(col);
        g.add_color_stop_rgba(0.0, rr, gg, bb, glow * 0.26);
        g.add_color_stop_rgba(1.0, rr, gg, bb, 0.0);
        let _ = cr.set_source(&g);
        let _ = cr.paint();
    }
    let ph = st.phase;
    for k in 0..3 {
        let t = (ph / 3.0 + k as f64 / 3.0).fract();
        let rad = r + 2.0 + t * 38.0;
        let al = (1.0 - t).powf(1.7) * 0.55 * st.a_wait;
        if al > 0.004 {
            stroke_circle(cr, cx, cy, rad, col, al, 1.6);
        }
    }
    if st.a_conn > 0.004 {
        let t = (ph / 4.0).fract();
        stroke_circle(
            cr,
            cx,
            cy,
            r + 4.0 + t * 30.0,
            col,
            (1.0 - t) * 0.4 * st.a_conn,
            1.6,
        );
        stroke_circle(cr, cx, cy, r + 7.0, col, 0.28 * st.a_conn, 2.0);
    }
    if st.a_start > 0.004 {
        spinner(cr, cx, cy, r + 8.0, col, st.a_start, ph);
    }
    fill_circle(cr, cx, cy, r, mix(st.pc(FILL), col, 0.13), 1.0);
    stroke_circle(cr, cx, cy, r - 1.0, col, 0.55, 1.6);
    let k = 1.0 + 0.035 * (ph * 2.2).sin() * st.a_conn;
    glyph(cr, cx, cy, k, col, 1.0);
    cr.restore().ok();
    // status text with a cross-fade: the old text leaves before the new one arrives
    let f = ease_out(st.status_fade);
    if f < 0.5 {
        let (t, h) = (&st.prev_text.0, &st.prev_text.1);
        status_block(cr, area, st, t, h, 1.0 - f * 2.0, -8.0 * f);
    }
    let (title, hint) = st.status_text(st.status);
    let fin = ((f - 0.3) / 0.7).clamp(0.0, 1.0);
    status_block(cr, area, st, &title, &hint, fin, 8.0 * (1.0 - f));
}

// ───────────────────────────── Animation and actions ─────────────────────────────
fn apply_prefer_dark(dark: bool) {
    #[allow(deprecated)]
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(dark);
    }
}

fn kick(ctx: &Ctx) {
    if ctx.s.borrow().tick.is_some() {
        return;
    }
    let c = ctx.clone();
    let id = ctx.w.main.add_tick_callback(move |_, clock| {
        if step(&c, clock.frame_time()) {
            glib::ControlFlow::Continue
        } else {
            c.s.borrow_mut().tick = None;
            glib::ControlFlow::Break
        }
    });
    ctx.s.borrow_mut().tick = Some(id);
}

/// Advance every animation by one frame; returns whether more frames are needed.
fn step(ctx: &Ctx, frame_us: i64) -> bool {
    let mut full = true;
    let keep;
    let dark_flip;
    {
        let mut u = ctx.s.borrow_mut();
        let dt = match u.last {
            Some(l) => ((frame_us - l) as f64 / 1e6).clamp(0.001, 0.05),
            None => 0.016,
        };
        u.last = Some(frame_us);
        u.phase += dt;
        u.enter += dt;
        let mut moving = u.enter < 1.5;
        let hv = |b: bool| if b { 1.0 } else { 0.0 };
        let before = u.dark > 0.5;
        let (dark_t, ob, ot, pr, ef, run) = (
            u.dark_t,
            u.over_btn,
            u.over_tog,
            u.press_item == 1 && u.over_btn,
            u.edit_focused,
            u.child.is_some(),
        );
        moving |= approach(&mut u.dark, dark_t, dt, 9.0);
        moving |= approach(&mut u.hover_btn, hv(ob), dt, 16.0);
        moving |= approach(&mut u.hover_tog, hv(ot), dt, 16.0);
        moving |= approach(&mut u.press, hv(pr), dt, 28.0);
        moving |= approach(&mut u.field, hv(ef), dt, 14.0);
        moving |= approach(&mut u.running, hv(run), dt, 9.0);
        let lt = u.lang_t;
        moving |= approach(&mut u.lang_pos, lt, dt, 14.0);
        if u.lang_fade < 1.0 {
            u.lang_fade = (u.lang_fade + dt / 0.3).min(1.0);
            moving = true;
        }
        let st = u.status;
        moving |= approach(&mut u.a_start, hv(st == Status::Starting), dt, 7.0);
        moving |= approach(&mut u.a_wait, hv(st == Status::Waiting), dt, 7.0);
        moving |= approach(&mut u.a_conn, hv(st == Status::Connected), dt, 7.0);
        moving |= approach(&mut u.a_fail, hv(st == Status::Failed), dt, 7.0);
        if u.status_fade < 1.0 {
            u.status_fade = (u.status_fade + dt / 0.45).min(1.0);
            moving = true;
        }
        // receiver events
        let events: Vec<Event> = u
            .events
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();
        for e in events {
            match e {
                Event::Ready => {
                    u.ready = true;
                    u.was_ready = true;
                    u.set_status(Status::Waiting);
                }
                Event::Stream(true) => {
                    u.ready = true;
                    u.was_ready = true;
                    u.set_status(Status::Connected);
                }
                Event::Stream(false) => {
                    let s = if u.ready {
                        Status::Waiting
                    } else {
                        Status::Starting
                    };
                    u.set_status(s);
                }
            }
        }
        let exited = u
            .child
            .as_mut()
            .is_some_and(|c| matches!(c.try_wait(), Ok(Some(_))));
        if exited {
            u.stop_child();
            u.set_status(Status::Failed);
            moving = true;
        }
        let continuous = matches!(
            u.status,
            Status::Starting | Status::Waiting | Status::Connected
        );
        dark_flip = (u.dark > 0.5) != before;
        // while only the indicator animates, redraw just that widget, at half rate
        if !moving && continuous {
            full = false;
            if u.a_start < 0.01 {
                u.skip = !u.skip;
                if u.skip {
                    return true;
                }
            }
        }
        keep = moving || continuous;
    }
    if dark_flip {
        let dark = ctx.s.borrow().dark > 0.5;
        update_entry_style(ctx, dark);
        apply_prefer_dark(dark);
    }
    let (enter, p1) = {
        let u = ctx.s.borrow();
        (u.enter, ease_out((u.enter - 0.075) / 0.65))
    };
    if full {
        ctx.w.main.queue_draw();
        if enter < 1.5 {
            ctx.w.ambient.set_opacity(p1);
        }
    }
    ctx.w.ambient.queue_draw();
    keep
}

fn update_entry_style(ctx: &Ctx, dark: bool) {
    if dark {
        ctx.w.entry.add_css_class("dark");
    } else {
        ctx.w.entry.remove_css_class("dark");
    }
}

fn theme_toggle(ctx: &Ctx, persist: bool) {
    let dark = {
        let mut u = ctx.s.borrow_mut();
        u.dark_t = if u.dark_t > 0.5 { 0.0 } else { 1.0 };
        u.dark_t > 0.5
    };
    if persist {
        save_setting("appearance", if dark { "dark" } else { "light" });
    }
    kick(ctx);
}

fn set_lang(ctx: &Ctx, l: Lang, persist: bool) {
    {
        let mut u = ctx.s.borrow_mut();
        if u.lang == l {
            return;
        }
        u.prev_text = u.status_text(u.status);
        u.lang = l;
        u.status_fade = 0.0;
        u.lang_fade = 0.0;
        u.lang_t = l as i32 as f64;
    }
    if persist {
        save_setting("language", ["ru", "en", "de"][l as usize]);
    }
    kick(ctx);
}

fn receiver_toggle(ctx: &Ctx) {
    if ctx.s.borrow().child.is_some() {
        {
            let mut u = ctx.s.borrow_mut();
            u.stop_child();
            u.set_status(Status::Off);
        }
        ctx.w.entry.set_sensitive(true);
        kick(ctx);
        return;
    }
    let exe = env::var_os("AIRELUMA_RECEIVER")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/lib/aireluma/uxplay"));
    let typed = ctx.w.entry.text();
    let name = if typed.trim().is_empty() {
        "Aireluma".to_string()
    } else {
        typed.trim().to_string()
    };
    let spawned = Command::new(exe)
        .args([
            "-n",
            &name,
            "-nh",
            "-vs",
            "autovideosink",
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
        .spawn();
    match spawned {
        Ok(mut child) => {
            let (tx, rx) = mpsc::channel();
            if let Some(out) = child.stdout.take() {
                thread::spawn(move || {
                    // Read raw bytes: a line that is not valid UTF-8 must not end the
                    // reader, or the receiver would block on a full pipe.
                    let mut reader = BufReader::new(out);
                    let mut buf = Vec::new();
                    while let Ok(n) = reader.read_until(b'\n', &mut buf) {
                        if n == 0 {
                            break;
                        }
                        let line = String::from_utf8_lossy(&buf);
                        let event = if line.contains("AIRELUMA_RECEIVER_READY") {
                            Some(Event::Ready)
                        } else if line.contains("AIRELUMA_STREAM_STARTED") {
                            Some(Event::Stream(true))
                        } else if line.contains("AIRELUMA_STREAM_STOPPED") {
                            Some(Event::Stream(false))
                        } else {
                            None
                        };
                        if let Some(event) = event {
                            if tx.send(event).is_err() {
                                break;
                            }
                        }
                        buf.clear();
                    }
                });
            }
            {
                let mut u = ctx.s.borrow_mut();
                u.child = Some(child);
                u.events = Some(rx);
                u.ready = false;
                u.was_ready = false;
                u.name = name;
                u.set_status(Status::Starting);
            }
            ctx.w.entry.set_sensitive(false);
        }
        Err(_) => {
            let mut u = ctx.s.borrow_mut();
            u.was_ready = false;
            u.set_status(Status::Failed);
        }
    }
    kick(ctx);
}

fn cycle_focus(ctx: &Ctx, back: bool) {
    let order: Vec<i32> = if ctx.s.borrow().child.is_some() {
        vec![1, 2, 3]
    } else {
        vec![0, 1, 2, 3]
    };
    let next = {
        let mut u = ctx.s.borrow_mut();
        let cur = order.iter().position(|v| *v == u.focus).unwrap_or(0) as i32;
        let n = order.len() as i32;
        let next = order[(((cur + if back { -1 } else { 1 }) % n + n) % n) as usize];
        u.focus = next;
        u.kbd = true;
        next
    };
    if next == 0 {
        ctx.w.entry.grab_focus();
    } else {
        ctx.w.main.grab_focus();
    }
    kick(ctx);
}

fn over(r: Rect, x: f64, y: f64) -> bool {
    x >= r.0 && x < r.0 + r.2 && y >= r.1 && y < r.1 + r.3
}
fn seg_at(x: f64, y: f64) -> i32 {
    if !over(SEG, x, y) {
        return -1;
    }
    (((x - SEG.0) / (SEG.2 / 3.0)) as i32).clamp(0, 2)
}
fn set_hover(ctx: &Ctx, x: f64, y: f64) {
    let hand = {
        let mut u = ctx.s.borrow_mut();
        u.over_btn = over(BTN, x, y);
        u.over_tog = over(TOG, x, y);
        u.over_seg = seg_at(x, y);
        u.over_btn || u.over_tog || u.over_seg >= 0
    };
    ctx.w
        .main
        .set_cursor_from_name(if hand { Some("pointer") } else { None });
}

// ───────────────────────────── Window ─────────────────────────────
fn build_ui(app: &gtk::Application, smoke: bool) {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(include_str!("linux.css"));
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("No display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let dark = load_dark();
    apply_prefer_dark(dark);
    let mut state = State::new(dark, load_lang());
    // the application icon, drawn into the header
    if let Ok(tex) = gdk::Texture::from_bytes(&glib::Bytes::from_static(include_bytes!(
        "../assets/aireluma.png"
    ))) {
        let (w, h) = (tex.width(), tex.height());
        let mut buf = vec![0u8; (w * h * 4) as usize];
        tex.download(&mut buf, (w * 4) as usize);
        state.icon =
            cairo::ImageSurface::create_for_data(buf, cairo::Format::ARgb32, w, h, w * 4).ok();
    }
    let shared = Rc::new(RefCell::new(state));

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Aireluma")
        .resizable(false)
        .icon_name("io.github.Anchoder.Aireluma")
        .build();
    let main = gtk::DrawingArea::new();
    main.set_content_width(WIN_W);
    main.set_content_height(WIN_H);
    main.set_focusable(true);
    let ambient = gtk::DrawingArea::new();
    ambient.set_content_width(HERO.2 as i32);
    ambient.set_content_height(HERO.3 as i32);
    ambient.set_halign(gtk::Align::Start);
    ambient.set_valign(gtk::Align::Start);
    ambient.set_margin_start(HERO.0 as i32);
    ambient.set_margin_top(HERO.1 as i32);
    ambient.set_can_target(false);
    let entry = gtk::Entry::builder()
        .text("Aireluma")
        .has_frame(false)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
        .margin_start(36)
        .margin_top(352)
        .width_request(308)
        .height_request(24)
        .build();
    entry.add_css_class("aireluma-entry");
    if dark {
        entry.add_css_class("dark");
    }
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&main));
    overlay.add_overlay(&ambient);
    overlay.add_overlay(&entry);
    window.set_child(Some(&overlay));

    let ctx = Ctx {
        s: shared.clone(),
        w: Rc::new(Widgets {
            main: main.clone(),
            ambient: ambient.clone(),
            entry: entry.clone(),
        }),
    };

    {
        let s = shared.clone();
        main.set_draw_func(move |area, cr, _, _| draw_main(cr, area, &s.borrow()));
        let s = shared.clone();
        ambient.set_draw_func(move |area, cr, _, _| draw_hero(cr, area, &s.borrow()));
    }

    // pointer
    let motion = gtk::EventControllerMotion::new();
    {
        let c = ctx.clone();
        motion.connect_motion(move |_, x, y| {
            set_hover(&c, x, y);
            kick(&c);
        });
        let c = ctx.clone();
        motion.connect_leave(move |_| {
            set_hover(&c, -1.0, -1.0);
            kick(&c);
        });
    }
    main.add_controller(motion);
    let click = gtk::GestureClick::new();
    click.set_button(1);
    {
        let c = ctx.clone();
        click.connect_pressed(move |_, _, x, y| {
            c.w.main.grab_focus();
            set_hover(&c, x, y);
            {
                let mut u = c.s.borrow_mut();
                u.kbd = false;
                if u.over_btn {
                    u.press_item = 1;
                    u.focus = 1;
                } else if u.over_tog {
                    u.press_item = 2;
                    u.focus = 2;
                } else if u.over_seg >= 0 {
                    u.press_item = 3;
                    u.focus = 3;
                }
            }
            kick(&c);
        });
        let c = ctx.clone();
        click.connect_released(move |_, _, x, y| {
            set_hover(&c, x, y);
            let (item, seg, b, t) = {
                let mut u = c.s.borrow_mut();
                let item = u.press_item;
                u.press_item = 0;
                (item, u.over_seg, u.over_btn, u.over_tog)
            };
            if item == 1 && b {
                receiver_toggle(&c);
            } else if item == 2 && t {
                theme_toggle(&c, true);
            } else if item == 3 && seg >= 0 {
                set_lang(&c, LANGS[seg as usize], true);
            }
            kick(&c);
        });
    }
    main.add_controller(click);

    // keyboard: Tab cycles the controls, Enter/Space activate, arrows pick a language
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let c = ctx.clone();
        keys.connect_key_pressed(move |_, key, _, mods| {
            use gdk::Key;
            let focus = c.s.borrow().focus;
            match key {
                Key::Tab | Key::ISO_Left_Tab => {
                    let back =
                        key == Key::ISO_Left_Tab || mods.contains(gdk::ModifierType::SHIFT_MASK);
                    cycle_focus(&c, back);
                    glib::Propagation::Stop
                }
                _ if focus == 0 => glib::Propagation::Proceed,
                Key::Left | Key::Right if focus == 3 => {
                    let cur = c.s.borrow().lang as i32;
                    let i = (cur + if key == Key::Left { -1 } else { 1 }).clamp(0, 2);
                    set_lang(&c, LANGS[i as usize], true);
                    c.s.borrow_mut().kbd = true;
                    glib::Propagation::Stop
                }
                Key::space | Key::Return | Key::KP_Enter => {
                    c.s.borrow_mut().kbd = true;
                    match focus {
                        2 => theme_toggle(&c, true),
                        3 => {}
                        _ => receiver_toggle(&c),
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
    }
    window.add_controller(keys);
    {
        let c = ctx.clone();
        entry.connect_activate(move |_| receiver_toggle(&c));
        let focus = gtk::EventControllerFocus::new();
        let c1 = ctx.clone();
        focus.connect_enter(move |_| {
            {
                let mut u = c1.s.borrow_mut();
                u.edit_focused = true;
                u.focus = 0;
            }
            kick(&c1);
        });
        let c2 = ctx.clone();
        focus.connect_leave(move |_| {
            c2.s.borrow_mut().edit_focused = false;
            kick(&c2);
        });
        entry.add_controller(focus);
    }
    {
        let s = shared.clone();
        window.connect_close_request(move |_| {
            s.borrow_mut().stop_child();
            glib::Propagation::Proceed
        });
    }

    window.present();
    kick(&ctx);
    if smoke {
        // exercise both themes and every language, then quit
        let c = ctx.clone();
        glib::timeout_add_local_once(Duration::from_millis(200), move || theme_toggle(&c, false));
        let c = ctx.clone();
        glib::timeout_add_local_once(Duration::from_millis(500), move || {
            set_lang(&c, Lang::De, false)
        });
        let c = ctx.clone();
        glib::timeout_add_local_once(Duration::from_millis(700), move || {
            set_lang(&c, Lang::En, false)
        });
        let c = ctx.clone();
        glib::timeout_add_local_once(Duration::from_millis(900), move || theme_toggle(&c, false));
        let app = app.clone();
        glib::timeout_add_local_once(Duration::from_millis(1300), move || {
            println!("AIRELUMA_LINUX_UI_OK");
            app.quit();
        });
    }
}

pub fn run() {
    let smoke = env::args().any(|v| v == "--smoke-test");
    let app = gtk::Application::builder()
        .application_id("io.github.Anchoder.Aireluma")
        .flags(gio::ApplicationFlags::FLAGS_NONE)
        .build();
    app.connect_activate(move |app| build_ui(app, smoke));
    app.run_with_args(&[] as &[&str]);
}
