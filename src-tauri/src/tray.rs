use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};

const LINKS: &[(&str, &str)] = &[
    ("overview", "Ouvrir Nova"),
    ("about-pc", "À propos de ce PC"),
    ("optimize", "Optimisation et maintenance…"),
    ("memory", "Mémoire intelligente…"),
    ("cleanup", "Nettoyage récupérable…"),
    ("startup", "Programmes au démarrage…"),
    ("network", "Diagnostic réseau…"),
    ("history", "Historique des opérations"),
];
struct MemoryStatus {
    label: MenuItem<tauri::Wry>,
    stop: MenuItem<tauri::Wry>,
}

#[tauri::command]
pub fn set_tray_memory_status(app: tauri::AppHandle, active: bool) -> Result<(), String> {
    if let Some(status) = app.try_state::<MemoryStatus>() {
        status
            .label
            .set_text(if active {
                "Surveillance RAM : activée"
            } else {
                "Surveillance RAM : désactivée"
            })
            .map_err(|e| e.to_string())?;
        status.stop.set_enabled(active).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn route(id: &str) -> Option<&'static str> {
    LINKS
        .iter()
        .find(|(page, _)| *page == id)
        .map(|(page, _)| *page)
}

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let menu = Menu::new(app)?;
    let title = MenuItem::with_id(
        app,
        "title",
        "Nova PC Suite · accès rapide",
        false,
        None::<&str>,
    )?;
    menu.append(&title)?;
    let label = MenuItem::with_id(
        app,
        "ram-status",
        "Surveillance RAM : désactivée",
        false,
        None::<&str>,
    )?;
    let stop = MenuItem::with_id(
        app,
        "ram-stop",
        "Suspendre la surveillance RAM",
        false,
        None::<&str>,
    )?;
    menu.append(&label)?;
    menu.append(&stop)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    for (id, title) in LINKS {
        menu.append(&MenuItem::with_id(app, *id, *title, true, None::<&str>)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "hide",
        "Masquer la fenêtre dans la zone de notification",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        "Quitter Nova et sa surveillance mémoire",
        true,
        None::<&str>,
    )?)?;
    let mut tray = TrayIconBuilder::with_id("nova-main")
        .tooltip("Nova PC Suite — clic pour les accès rapides")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let id = event.id.as_ref();
            if id == "quit" {
                app.exit(0);
                return;
            }
            if id == "ram-stop" {
                let _ = app.emit("nova:memory-stop", ());
                return;
            }
            if let Some(window) = app.get_webview_window("main") {
                if id == "hide" {
                    let _ = window.hide();
                    return;
                }
                if let Some(page) = route(id) {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                    let _ = app.emit("nova:navigate", page);
                }
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    app.manage(MemoryStatus { label, stop });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tray_links_only_navigate_and_never_run_maintenance() {
        for (id, _) in LINKS {
            assert_eq!(route(id), Some(*id));
        }
        for action in [
            "memory-trim",
            "process-close",
            "storage-optimize",
            "install-drivers",
            "quit",
            "hide",
            "cmd.exe",
        ] {
            assert_eq!(route(action), None);
        }
    }
}
