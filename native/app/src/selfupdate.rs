use crate::{ss, toast, work, App, Nova};
use nova_engine::selfupdate::{self, Available};
use slint::{ComponentHandle, Timer, TimerMode};
use std::cell::RefCell;
use std::time::Duration;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
struct State {
    found: Option<Available>,
    skipped: String,
    timer: Option<Timer>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn show(ui: &App) {
    let nova = ui.global::<Nova>();
    let found = STATE.with(|s| {
        let s = s.borrow();
        s.found.clone().filter(|found| found.version != s.skipped)
    });
    nova.set_update_version(ss(found.as_ref().map(|f| f.version.clone()).unwrap_or_default()));
    nova.set_update_notes(ss(found.map(|f| f.notes.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default().chars().take(120).collect::<String>()).unwrap_or_default()));
}

fn check(ui: &App) {
    work(ui, || selfupdate::check(VERSION), |ui, result| {
        let info = match &result {
            Ok(Some(found)) => format!("Mises à jour automatiques : Nova {} est disponible.", found.version),
            Ok(None) => format!("Mises à jour automatiques : Nova est à jour."),
            Err(error) => format!("Mises à jour automatiques : {error}"),
        };
        ui.global::<Nova>().set_update_info(ss(info));
        if let Ok(found) = result {
            STATE.with(|s| s.borrow_mut().found = found);
        }
        show(&ui);
    });
}

pub fn wire(ui: &App, online: bool) {
    let nova = ui.global::<Nova>();
    nova.set_update_info(ss(if !selfupdate::configured() {
        "Mises à jour automatiques pas encore configurées : la clé de signature de Nova n'existe pas encore."
    } else if online {
        "Mises à jour automatiques : vérification…"
    } else {
        "Mises à jour automatiques désactivées pour cette session."
    }));
    let weak = ui.as_weak();
    nova.on_update_later(move || {
        let Some(ui) = weak.upgrade() else { return };
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.skipped = s.found.as_ref().map(|f| f.version.clone()).unwrap_or_default();
        });
        show(&ui);
    });
    let weak = ui.as_weak();
    nova.on_update_go(move || {
        let Some(ui) = weak.upgrade() else { return };
        let Some(found) = STATE.with(|s| s.borrow().found.clone()) else { return };
        ui.global::<Nova>().set_update_busy(true);
        work(&ui, move || selfupdate::download(&found).and_then(|path| selfupdate::launch(&path)), |ui, result| match result {
            Ok(()) => {
                let _ = slint::quit_event_loop();
            }
            Err(error) => {
                ui.global::<Nova>().set_update_busy(false);
                toast(&ui, &format!("Mise à jour impossible : {error}"), true, None);
            }
        });
    });
    if !online || !selfupdate::configured() {
        return;
    }
    check(ui);
    let timer = Timer::default();
    let weak = ui.as_weak();
    timer.start(TimerMode::Repeated, Duration::from_secs(6 * 3600), move || {
        if let Some(ui) = weak.upgrade() {
            check(&ui);
        }
    });
    STATE.with(|s| s.borrow_mut().timer = Some(timer));
}
