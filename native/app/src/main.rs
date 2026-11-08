#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nova_engine::{elevation, report, vitals};
use serde_json::{json, Value};
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::{cell::RefCell, panic::AssertUnwindSafe, rc::Rc, time::Duration};

slint::include_modules!();

#[cfg(feature = "headless")]
mod headless;
mod pages;
mod tour;

pub fn ss(text: impl AsRef<str>) -> SharedString {
    SharedString::from(text.as_ref())
}

pub fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(items))
}

pub fn call(module: &str, action: &str, payload: Value) -> Result<Value, String> {
    let raw = std::panic::catch_unwind(AssertUnwindSafe(|| {
        nova_engine::run(module, action, Some(payload))
    }))
    .map_err(|_| "Cet écran a reçu des données illisibles. Le reste de Nova fonctionne : changez d'écran ou réessayez.".to_string())??;
    if raw["ok"] == true {
        Ok(raw["data"].clone())
    } else {
        Err(raw["error"]
            .as_str()
            .filter(|e| !e.is_empty())
            .unwrap_or("Windows a refusé cette opération.")
            .to_string())
    }
}

pub fn work<R, J, D>(ui: &App, job: J, done: D)
where
    R: Send + 'static,
    J: FnOnce() -> R + Send + 'static,
    D: FnOnce(App, R) + Send + 'static,
{
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let result = job();
        let _ = weak.upgrade_in_event_loop(move |ui| done(ui, result));
    });
}

pub fn measures_of(value: &Value) -> ModelRc<Measure> {
    let list: Vec<Measure> = report::arr(value.clone())
        .iter()
        .map(|m| {
            let unit = m["unit"].as_str().unwrap_or("number");
            Measure {
                label: ss(m["label"].as_str().unwrap_or("")),
                before: ss(report::format_measure_value(&m["before"], unit)),
                after: ss(report::format_measure_value(&m["after"], unit)),
                delta: ss(report::measure_delta(&m["before"], &m["after"], unit).unwrap_or_default()),
            }
        })
        .collect();
    model(list)
}

struct Toast {
    id: i32,
    undo: Option<Value>,
}

#[derive(Default)]
struct Shared {
    toasts: Vec<Toast>,
    next_toast: i32,
    pending: Option<Box<dyn FnOnce(&App)>>,
    timers: Vec<Timer>,
    traces: [Vec<f64>; 3],
}

thread_local! {
    static SHARED: RefCell<Shared> = RefCell::new(Shared::default());
}

fn with_toasts(ui: &App, change: impl FnOnce(&mut Vec<ToastItem>)) {
    let mut list: Vec<ToastItem> = ui.global::<Nova>().get_toasts().iter().collect();
    change(&mut list);
    let holder = ui.global::<Nova>().get_toasts();
    let Some(existing) = holder.as_any().downcast_ref::<VecModel<ToastItem>>() else {
        return;
    };
    while existing.row_count() > list.len() {
        existing.remove(existing.row_count() - 1);
    }
    for (i, item) in list.into_iter().enumerate() {
        if i >= existing.row_count() {
            existing.push(item);
        } else if existing.row_data(i).is_none_or(|row| row != item) {
            existing.set_row_data(i, item);
        }
    }
}

pub fn toast(ui: &App, text: &str, alert: bool, undo: Option<Value>) {
    let id = SHARED.with(|s| {
        let mut s = s.borrow_mut();
        s.next_toast += 1;
        let id = s.next_toast;
        s.toasts.push(Toast { id, undo: undo.clone() });
        id
    });
    let undo_label = undo
        .as_ref()
        .and_then(|u| u["label"].as_str())
        .unwrap_or("")
        .to_string();
    with_toasts(ui, |list| {
        if list.len() >= 3 {
            list.remove(0);
        }
        list.push(ToastItem {
            id,
            text: ss(text),
            alert,
            undo_label: ss(undo_label),
            leaving: false,
        });
    });
    if !alert {
        let weak = ui.as_weak();
        let wait = if undo.is_some() { 12000 } else { 5000 };
        Timer::single_shot(Duration::from_millis(wait), move || {
            if let Some(ui) = weak.upgrade() {
                dismiss(&ui, id);
            }
        });
    }
}

fn dismiss(ui: &App, id: i32) {
    with_toasts(ui, |list| {
        if let Some(item) = list.iter_mut().find(|t| t.id == id) {
            item.leaving = true;
        }
    });
    let weak = ui.as_weak();
    Timer::single_shot(Duration::from_millis(170), move || {
        if let Some(ui) = weak.upgrade() {
            with_toasts(&ui, |list| list.retain(|t| t.id != id));
            SHARED.with(|s| s.borrow_mut().toasts.retain(|t| t.id != id));
        }
    });
}

pub struct Ask {
    pub title: String,
    pub body: String,
    pub confirm: &'static str,
    pub danger: bool,
    pub lines: Vec<String>,
    pub licenses: Vec<(String, String)>,
}

pub fn ask(ui: &App, sheet: Ask, then: impl FnOnce(&App) + 'static) {
    SHARED.with(|s| s.borrow_mut().pending = Some(Box::new(then)));
    let nova = ui.global::<Nova>();
    nova.set_ask_title(ss(sheet.title));
    nova.set_ask_body(ss(sheet.body));
    nova.set_ask_confirm(ss(sheet.confirm));
    nova.set_ask_danger(sheet.danger);
    nova.set_ask_lines(model(sheet.lines.iter().map(ss).collect()));
    nova.set_ask_licenses(model(
        sheet
            .licenses
            .iter()
            .map(|(title, text)| License { title: ss(title), text: ss(text) })
            .collect(),
    ));
    ui.global::<Shell>().set_modal(true);
    ui.set_ask_visible(true);
    nova.set_ask_open(true);
}

fn answer(ui: &App, yes: bool) {
    let then = SHARED.with(|s| s.borrow_mut().pending.take());
    ui.global::<Nova>().set_ask_open(false);
    ui.global::<Shell>().set_modal(false);
    let weak = ui.as_weak();
    let wait = if ui.global::<Theme>().get_quick() == 0 { 0 } else { 120 };
    Timer::single_shot(Duration::from_millis(wait), move || {
        if let Some(ui) = weak.upgrade() {
            if !ui.global::<Nova>().get_ask_open() {
                ui.set_ask_visible(false);
                ui.invoke_restore_focus();
            }
        }
    });
    if yes {
        if let Some(then) = then {
            then(ui);
        }
    }
}

pub fn refresh_module(ui: &App, module: &str) {
    match module {
        "startup" => pages::demarrage_load(ui),
        "perf" => pages::perf_load(ui),
        "repair" => pages::reparer_load(ui),
        _ => {}
    }
}

pub fn run_undo(ui: &App, undo: Value, then: impl FnOnce(&App) + Send + 'static) {
    let module = undo["module"].as_str().unwrap_or("").to_string();
    let action = undo["action"].as_str().unwrap_or("").to_string();
    let payload = if undo["payload"].is_object() { undo["payload"].clone() } else { json!({}) };
    work(ui, move || (module.clone(), call(&module, &action, payload)), move |ui, (module, result)| {
        match result {
            Ok(data) => {
                toast(&ui, data["summary"].as_str().unwrap_or("Annulé"), false, None);
                refresh_module(&ui, &module);
                then(&ui);
            }
            Err(error) => toast(&ui, &error, true, None),
        }
    });
}

fn elevate(ui: &App) {
    let page = report::PAGES[ui.global::<Nova>().get_page().clamp(0, 7) as usize];
    ask(
        ui,
        Ask {
            title: "Relancer Nova en administrateur".into(),
            body: "Cette action touche à Windows. Nova va se fermer et se rouvrir sur cette page. Windows vous demandera votre accord.".into(),
            confirm: "Relancer",
            danger: false,
            lines: vec![],
            licenses: vec![],
        },
        move |ui| {
            let handoff = match elevation::Handoff::new() {
                Ok(handoff) => handoff,
                Err(error) => return toast(ui, &error, true, None),
            };
            let _ = ui.window().hide();
            work(
                ui,
                move || elevation::relaunch_as_admin(&handoff, page) && handoff.wait(60000),
                |ui, replaced| {
                    if replaced {
                        let _ = slint::quit_event_loop();
                    } else {
                        let _ = ui.window().show();
                        toast(&ui, "Passage administrateur annulé. Cette fenêtre reste ouverte.", true, None);
                    }
                },
            );
        },
    );
}

fn trace_path(points: &[f64]) -> String {
    if points.len() < 2 {
        return String::new();
    }
    let last = points.len() - 1;
    let mut path = String::with_capacity(points.len() * 14);
    for (i, value) in points.iter().enumerate() {
        let x = 172.0 - (last - i) as f64 * 172.0 / 59.0;
        let y = 25.0 - value.clamp(0.0, 100.0) * 0.23;
        path.push_str(&format!("{} {x:.1} {y:.1} ", if i == 0 { "M" } else { "L" }));
    }
    path
}

fn push_trace(list: &mut Vec<f64>, value: Option<f64>) {
    if let Some(value) = value {
        if list.len() >= 60 {
            list.remove(0);
        }
        list.push(value);
    }
}

fn tick_vitals(ui: &App) {
    if ui.window().is_minimized() {
        return;
    }
    work(ui, vitals::sample, |ui, sample| {
        let cpu = sample["cpu"].as_f64();
        let disk = sample["disk"].as_f64();
        let used = sample["memoryUsedBytes"].as_f64().unwrap_or(0.0);
        let total = sample["memoryTotalBytes"].as_f64().unwrap_or(0.0);
        let percent = |v: Option<f64>| v.map(|v| format!("{} %", v.round())).unwrap_or_else(|| "n/d".into());
        let nova = ui.global::<Nova>();
        nova.set_cpu_text(ss(percent(cpu)));
        nova.set_disk_text(ss(percent(disk)));
        nova.set_ram_text(ss(report::format_bytes(used)));
        SHARED.with(|s| {
            let mut s = s.borrow_mut();
            push_trace(&mut s.traces[0], cpu);
            push_trace(&mut s.traces[1], (total > 0.0).then(|| used / total * 100.0));
            push_trace(&mut s.traces[2], disk);
            nova.set_cpu_path(ss(trace_path(&s.traces[0])));
            nova.set_ram_path(ss(trace_path(&s.traces[1])));
            nova.set_disk_path(ss(trace_path(&s.traces[2])));
        });
    });
}

fn wire_shell(ui: &App) {
    let nova = ui.global::<Nova>();
    nova.set_toasts(ModelRc::from(Rc::new(VecModel::<ToastItem>::default())));
    let weak = ui.as_weak();
    nova.on_go(move |page| {
        if let Some(ui) = weak.upgrade() {
            pages::open(&ui, page);
        }
    });
    let weak = ui.as_weak();
    nova.on_answer(move |yes| {
        if let Some(ui) = weak.upgrade() {
            answer(&ui, yes);
        }
    });
    let weak = ui.as_weak();
    nova.on_toast_close(move |id| {
        if let Some(ui) = weak.upgrade() {
            dismiss(&ui, id);
        }
    });
    let weak = ui.as_weak();
    nova.on_toast_undo(move |id| {
        if let Some(ui) = weak.upgrade() {
            let undo = SHARED.with(|s| s.borrow().toasts.iter().find(|t| t.id == id).and_then(|t| t.undo.clone()));
            dismiss(&ui, id);
            if let Some(undo) = undo {
                run_undo(&ui, undo, |_| {});
            }
        }
    });
    let weak = ui.as_weak();
    ui.global::<Shell>().on_elevate(move || {
        if let Some(ui) = weak.upgrade() {
            elevate(&ui);
        }
    });
    ui.global::<Shell>().set_is_admin(elevation::is_admin());
    ui.global::<Theme>().set_reduced(!nova_engine::win::animations_enabled());
    let timer = Timer::default();
    let weak = ui.as_weak();
    timer.start(TimerMode::Repeated, Duration::from_millis(1500), move || {
        if let Some(ui) = weak.upgrade() {
            tick_vitals(&ui);
        }
    });
    SHARED.with(|s| s.borrow_mut().timers.push(timer));
    tick_vitals(ui);
}

fn main() {
    if std::env::args().any(|a| a == "--scheduled-scan") {
        return;
    }
    let tour_dir = std::env::var_os("NOVA_TOUR").map(std::path::PathBuf::from);
    let replacement = match elevation::requested_token() {
        Ok(token) => token.is_some() && elevation::is_admin(),
        Err(_) => return,
    };
    let _instance = if tour_dir.is_some() && cfg!(feature = "headless") {
        None
    } else {
        match elevation::Instance::acquire(replacement) {
            Ok(Some(instance)) => Some(instance),
            _ => return,
        }
    };
    #[cfg(feature = "headless")]
    headless::install();
    let ui = App::new().expect("Impossible de démarrer Nova");
    if let Ok(Some(token)) = elevation::requested_token() {
        if elevation::complete(&token).is_err() {
            return;
        }
    }
    wire_shell(&ui);
    pages::wire(&ui);
    let start = report::PAGES
        .iter()
        .position(|p| *p == elevation::initial_page())
        .unwrap_or(0) as i32;
    pages::open(&ui, start);
    ui.invoke_restore_focus();
    if let Some(dir) = tour_dir {
        tour::start(&ui, dir);
    }
    ui.run().expect("Impossible de démarrer Nova");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traces_are_aligned_on_the_right_and_stay_in_the_box() {
        assert!(trace_path(&[5.0]).is_empty());
        assert_eq!(trace_path(&[0.0, 100.0]), "M 169.1 25.0 L 172.0 2.0 ");
        assert_eq!(trace_path(&[250.0, -5.0]), "M 169.1 2.0 L 172.0 25.0 ");
    }

    #[test]
    fn the_trace_keeps_sixty_points() {
        let mut list = Vec::new();
        for i in 0..70 {
            push_trace(&mut list, Some(i as f64));
        }
        push_trace(&mut list, None);
        assert_eq!(list.len(), 60);
        assert_eq!(list[0], 10.0);
    }
}
