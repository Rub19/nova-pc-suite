#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nova_engine::{elevation, vitals};
use serde_json::{json, Value};
use std::{thread, time::Duration};
use tauri::Manager;

#[tauri::command]
async fn run(module: String, action: String, payload: Option<Value>) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || nova_engine::run(&module, &action, payload))
        .await
        .map_err(|_| "Opération interrompue".to_string())?
}

#[tauri::command]
fn journal() -> Vec<Value> {
    nova_engine::journal()
}

#[tauri::command]
async fn vitals_sample() -> Value {
    tauri::async_runtime::spawn_blocking(vitals::sample)
        .await
        .unwrap_or(Value::Null)
}

#[tauri::command]
fn is_admin() -> bool {
    elevation::is_admin()
}

#[tauri::command]
fn startup_page() -> String {
    elevation::initial_page()
}

#[tauri::command]
fn app_ready(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
    }
}

#[tauri::command]
fn open_windows(target: String) -> Result<(), String> {
    nova_engine::open_windows(&target)
}

#[tauri::command]
async fn elevate(app: tauri::AppHandle, page: String) -> Result<Value, String> {
    if !elevation::valid_page(&page) {
        return Err("Écran invalide".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        if elevation::is_admin() {
            return Ok(json!({"ok": true, "alreadyAdmin": true}));
        }
        let handoff = elevation::Handoff::new()?;
        let window = app
            .get_webview_window("main")
            .ok_or("Fenêtre Nova indisponible")?;
        window
            .hide()
            .map_err(|_| "Transition de fenêtre impossible")?;
        if elevation::relaunch_as_admin(&handoff, &page) && handoff.wait(60000) {
            app.exit(0);
            Ok(json!({"ok": true}))
        } else {
            let _ = window.show();
            let _ = window.set_focus();
            Ok(json!({"ok": false, "error": "Passage administrateur annulé. Cette fenêtre reste ouverte."}))
        }
    })
    .await
    .map_err(|_| "Transition interrompue".to_string())?
}

fn main() {
    if std::env::args().any(|a| a == "--scheduled-scan") {
        return;
    }
    let replacement = match elevation::requested_token() {
        Ok(token) => token.is_some() && elevation::is_admin(),
        Err(_) => return,
    };
    let _instance = match elevation::Instance::acquire(replacement) {
        Ok(Some(instance)) => instance,
        _ => return,
    };
    tauri::Builder::default()
        .setup(|app| {
            if let Some(token) = elevation::requested_token()? {
                elevation::complete(&token)?;
            }
            #[cfg(debug_assertions)]
            if std::env::var_os("NOVA_OFFSCREEN").is_some() {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_skip_taskbar(true);
                    let _ = window.set_position(tauri::PhysicalPosition::new(-6000, -6000));
                }
            }
            let handle = app.handle().clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(5));
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.show();
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            run,
            journal,
            vitals_sample,
            is_admin,
            startup_page,
            app_ready,
            open_windows,
            elevate
        ])
        .run(tauri::generate_context!())
        .expect("Impossible de démarrer Nova");
}
