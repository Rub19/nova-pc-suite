use crate::{App, BilanState, DemarrageState, MajState, NettoyageState, Nova, PerfState, PilotesState, ReparerState};
use slint::{
    platform::{Key, WindowEvent},
    ComponentHandle, LogicalSize, SharedString, Timer, TimerMode,
};
use std::{cell::Cell, fs::File, io::BufWriter, path::PathBuf, rc::Rc, time::{Duration, Instant}};

fn snap(ui: &App, path: PathBuf) {
    let Ok(shot) = ui.window().take_snapshot() else {
        println!("capture impossible : {}", path.display());
        return;
    };
    let Ok(file) = File::create(&path) else { return };
    let mut encoder = png::Encoder::new(BufWriter::new(file), shot.width(), shot.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let Ok(mut writer) = encoder.write_header() else { return };
    let _ = writer.write_image_data(shot.as_bytes());
    println!("capture {}", path.display());
}

fn press(ui: &App, text: &str) {
    let window = ui.window();
    let control: SharedString = Key::Control.into();
    window.dispatch_event(WindowEvent::KeyPressed { text: control.clone() });
    window.dispatch_event(WindowEvent::KeyPressed { text: text.into() });
    window.dispatch_event(WindowEvent::KeyReleased { text: text.into() });
    window.dispatch_event(WindowEvent::KeyReleased { text: control });
}

fn idle(ui: &App, page: i32) -> bool {
    match page {
        0 => !ui.global::<BilanState>().get_loading(),
        1 => !ui.global::<PilotesState>().get_loading() && !ui.global::<PilotesState>().get_wu().loading,
        2 => !ui.global::<MajState>().get_winget_loading(),
        3 => !ui.global::<NettoyageState>().get_loading() && !ui.global::<NettoyageState>().get_largest_loading(),
        4 => !ui.global::<DemarrageState>().get_loading(),
        5 => !ui.global::<PerfState>().get_loading() && !ui.global::<PerfState>().get_procs_loading() && !ui.global::<PerfState>().get_net_loading(),
        6 => ui.global::<ReparerState>().get_restore_loaded(),
        _ => true,
    }
}

pub fn start(ui: &App, dir: PathBuf) {
    let _ = std::fs::create_dir_all(&dir);
    let tall: f32 = std::env::var("NOVA_TOUR_HEIGHT").ok().and_then(|v| v.parse().ok()).unwrap_or(2200.0);
    ui.window().set_size(LogicalSize::new(1180.0, 780.0));
    let _ = ui.show();
    let step = Rc::new(Cell::new(0u32));
    let page = Rc::new(Cell::new(0i32));
    let since = Rc::new(Cell::new(Instant::now()));
    let timer = Timer::default();
    let weak = ui.as_weak();
    timer.start(TimerMode::Repeated, Duration::from_millis(100), move || {
        let Some(ui) = weak.upgrade() else { return };
        let elapsed = since.get().elapsed();
        let current = page.get();
        let advance = |next: u32| {
            step.set(next);
            since.set(Instant::now());
        };
        match step.get() {
            0 => {
                press(&ui, &(current + 1).to_string());
                let moved = ui.global::<Nova>().get_page() == current;
                println!("Ctrl+{} : {}", current + 1, if moved { "ok" } else { "ÉCHEC" });
                match current {
                    1 => ui.global::<PilotesState>().invoke_wu_search(),
                    3 => ui.global::<NettoyageState>().invoke_measure_largest(),
                    5 => ui.global::<PerfState>().invoke_net_test(),
                    _ => {}
                }
                advance(1);
            }
            1 => {
                if idle(&ui, current) || elapsed > Duration::from_secs(420) {
                    println!("page {} prête en {:?}, idle={}", current, elapsed, idle(&ui, current));
                    advance(2);
                }
            }
            2 => {
                if elapsed > Duration::from_millis(900) {
                    snap(&ui, dir.join(format!("p{current}.png")));
                    ui.window().set_size(LogicalSize::new(1180.0, tall));
                    advance(3);
                }
            }
            3 => {
                if elapsed > Duration::from_millis(500) {
                    snap(&ui, dir.join(format!("p{current}-long.png")));
                    ui.window().set_size(LogicalSize::new(1180.0, 780.0));
                    if current >= 7 {
                        advance(4);
                    } else {
                        page.set(current + 1);
                        advance(0);
                    }
                }
            }
            4 => {
                crate::toast(&ui, "Message de test de la visite", false, None);
                crate::ask(
                    &ui,
                    crate::Ask {
                        title: "Fenêtre de confirmation".into(),
                        body: "Vue d'essai pendant la visite. Rien n'est lancé.".into(),
                        confirm: "Confirmer",
                        danger: true,
                        lines: vec!["Ligne d'exemple de la visite".into()],
                        licenses: vec![],
                    },
                    |_| {},
                );
                advance(5);
            }
            5 => {
                if elapsed > Duration::from_millis(500) {
                    snap(&ui, dir.join("dialogue.png"));
                    ui.global::<Nova>().invoke_answer(false);
                    ui.set_about_visible(true);
                    ui.global::<Nova>().set_about_open(true);
                    advance(6);
                }
            }
            6 => {
                if elapsed > Duration::from_millis(500) {
                    snap(&ui, dir.join("apropos.png"));
                    println!("visite terminée");
                    let _ = slint::quit_event_loop();
                    advance(7);
                }
            }
            _ => {}
        }
    });
    std::mem::forget(timer);
}
