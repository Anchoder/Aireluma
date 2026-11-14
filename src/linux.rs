use gtk::{gio, glib, prelude::*};
use std::{
    cell::RefCell,
    env, fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    rc::Rc,
    sync::mpsc,
    thread,
    time::Duration,
};

enum Event {
    Ready,
    Stream(bool),
}
#[derive(Default)]
struct Receiver {
    child: Option<Child>,
    events: Option<mpsc::Receiver<Event>>,
    ready: bool,
}
impl Receiver {
    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.events = None;
        self.ready = false;
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        self.stop();
    }
}

fn appearance_path() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".config"))
        .join("aireluma/appearance")
}
fn label(text: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.add_css_class(class);
    label
}
fn picture(bytes: &'static [u8], size: i32) -> gtk::Image {
    let texture = gtk::gdk::Texture::from_bytes(&glib::Bytes::from_static(bytes))
        .expect("Invalid application image");
    let image = gtk::Image::from_paintable(Some(&texture));
    image.set_pixel_size(size);
    image
}
fn apply_theme(root: &gtk::Box, button: &gtk::Button, dark: bool) {
    if dark {
        root.add_css_class("dark");
        button.set_label("Светлая тема");
    } else {
        root.remove_css_class("dark");
        button.set_label("Тёмная тема");
    }
}

fn build_ui(app: &gtk::Application, smoke: bool) {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(include_str!("linux.css"));
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("No display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Aireluma")
        .default_width(376)
        .resizable(false)
        .icon_name("io.github.Anchoder.Aireluma")
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 24);
    root.add_css_class("aireluma");
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    header.append(&picture(include_bytes!("../assets/aireluma.png"), 48));
    let brand = gtk::Box::new(gtk::Orientation::Vertical, 2);
    brand.set_hexpand(true);
    brand.append(&label("Aireluma", "title"));
    brand.append(&label("Повтор экрана с iOS", "subtitle"));
    header.append(&brand);
    let theme = gtk::Button::with_label("Тёмная тема");
    theme.add_css_class("secondary");
    theme.set_valign(gtk::Align::Center);
    header.append(&theme);
    root.append(&header);
    let card = gtk::Box::new(gtk::Orientation::Vertical, 12);
    card.add_css_class("card");
    card.append(&label("Имя устройства в AirPlay", "field-label"));
    let entry = gtk::Entry::builder()
        .text("Aireluma")
        .height_request(40)
        .build();
    card.append(&entry);
    let status = label("●  Приём выключен", "status");
    card.append(&status);
    let start = gtk::Button::with_label("Включить AirPlay");
    start.add_css_class("primary");
    start.set_height_request(40);
    start.set_margin_top(4);
    card.append(&start);
    root.append(&card);
    let guide = gtk::Box::new(gtk::Orientation::Vertical, 18);
    guide.append(&label("Как подключиться", "section-title"));
    for (number, title, note) in [
        (
            "1",
            "Откройте «Повтор экрана»",
            "В Пункте управления iPhone",
        ),
        ("2", "Выберите устройство", "Имя указано в поле выше"),
    ] {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        let step = gtk::Label::new(Some(number));
        step.add_css_class("step");
        step.set_size_request(28, 28);
        step.set_valign(gtk::Align::Center);
        row.append(&step);
        let words = gtk::Box::new(gtk::Orientation::Vertical, 6);
        words.append(&label(title, "body"));
        words.append(&label(note, "subtitle"));
        row.append(&words);
        guide.append(&row);
    }
    root.append(&guide);
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 24);
    footer.add_css_class("footer");
    footer.set_halign(gtk::Align::Center);
    for (light, dark, text) in [
        (
            include_bytes!("../assets/footer-screen-light.png").as_slice(),
            include_bytes!("../assets/footer-screen.png").as_slice(),
            "Экран и звук",
        ),
        (
            include_bytes!("../assets/footer-wifi-light.png").as_slice(),
            include_bytes!("../assets/footer-wifi.png").as_slice(),
            "Локально",
        ),
        (
            include_bytes!("../assets/footer-clock-light.png").as_slice(),
            include_bytes!("../assets/footer-clock.png").as_slice(),
            "до 60 FPS",
        ),
    ] {
        let item = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let icons = gtk::Stack::new();
        icons.set_transition_type(gtk::StackTransitionType::Crossfade);
        icons.set_transition_duration(240);
        icons.add_named(&picture(light, 16), Some("light"));
        icons.add_named(&picture(dark, 16), Some("dark"));
        let copy = icons.clone();
        root.connect_notify_local(Some("css-classes"), move |root, _| {
            copy.set_visible_child_name(if root.has_css_class("dark") {
                "dark"
            } else {
                "light"
            })
        });
        item.append(&icons);
        item.append(&label(text, "footer-label"));
        footer.append(&item);
    }
    root.append(&footer);
    window.set_child(Some(&root));
    let dark = Rc::new(RefCell::new(
        fs::read_to_string(appearance_path())
            .map(|v| v.trim() == "dark")
            .unwrap_or(false),
    ));
    apply_theme(&root, &theme, *dark.borrow());
    {
        let root = root.clone();
        let dark = dark.clone();
        theme.connect_clicked(move |button| {
            let mut dark = dark.borrow_mut();
            *dark = !*dark;
            apply_theme(&root, button, *dark);
            let path = appearance_path();
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(path, if *dark { "dark" } else { "light" });
        });
    }
    let receiver = Rc::new(RefCell::new(Receiver::default()));
    {
        let receiver = receiver.clone();
        let entry = entry.clone();
        let status = status.clone();
        start.connect_clicked(move |button| {
            let mut state = receiver.borrow_mut();
            if state.child.is_some() {
                state.stop();
                entry.set_sensitive(true);
                button.set_label("Включить AirPlay");
                status.set_text("●  Приём выключен");
                return;
            }
            let exe = env::var_os("AIRELUMA_RECEIVER")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/usr/lib/aireluma/uxplay"));
            let name = entry.text();
            let name = if name.trim().is_empty() {
                "Aireluma"
            } else {
                name.trim()
            };
            match Command::new(exe)
                .args([
                    "-n",
                    name,
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
                .spawn()
            {
                Ok(mut child) => {
                    let (tx, rx) = mpsc::channel();
                    if let Some(stdout) = child.stdout.take() {
                        thread::spawn(move || {
                            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
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
                            }
                        });
                    }
                    state.child = Some(child);
                    state.events = Some(rx);
                    state.ready = false;
                    entry.set_sensitive(false);
                    button.set_label("Остановить приём");
                    status.set_text("●  Запуск приёмника…");
                }
                Err(_) => {
                    status.set_text("Не удалось запустить приёмник");
                }
            }
        });
    }
    {
        let receiver = receiver.clone();
        let status = status.clone();
        let entry = entry.clone();
        let start = start.clone();
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let mut state = receiver.borrow_mut();
            let events: Vec<_> = state
                .events
                .as_ref()
                .map(|rx| rx.try_iter().collect())
                .unwrap_or_default();
            for event in events {
                match event {
                    Event::Ready => {
                        state.ready = true;
                        status.set_text("●  Ожидание iPhone");
                    }
                    Event::Stream(true) => {
                        status.set_text("●  Экран подключён");
                    }
                    Event::Stream(false) => {
                        status.set_text("●  Ожидание iPhone");
                    }
                }
            }
            if state
                .child
                .as_mut()
                .is_some_and(|child| matches!(child.try_wait(), Ok(Some(_))))
            {
                state.stop();
                entry.set_sensitive(true);
                start.set_label("Включить AirPlay");
                status.set_text("Приёмник остановлен. Повторите запуск.");
            }
            glib::ControlFlow::Continue
        });
    }
    {
        let receiver = receiver.clone();
        window.connect_close_request(move |_| {
            receiver.borrow_mut().stop();
            glib::Propagation::Proceed
        });
    }
    window.present();
    if smoke {
        let root_copy = root.clone();
        let theme_copy = theme.clone();
        glib::timeout_add_local_once(Duration::from_millis(200), move || {
            apply_theme(&root_copy, &theme_copy, true)
        });
        let root_copy = root.clone();
        let theme_copy = theme.clone();
        glib::timeout_add_local_once(Duration::from_millis(600), move || {
            apply_theme(&root_copy, &theme_copy, false)
        });
        let app = app.clone();
        glib::timeout_add_local_once(Duration::from_millis(1000), move || {
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
