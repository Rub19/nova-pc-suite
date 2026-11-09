#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;

use engine::{Choices, Target};
use slint::winit_030::WinitWindowAccessor;
use slint::ComponentHandle;
use std::rc::Rc;

slint::include_modules!();

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Install,
    Uninstall,
}

#[derive(Clone)]
struct Args {
    mode: Mode,
    silent: bool,
    passive: bool,
    relaunch: bool,
    app_args: Vec<String>,
}

fn parse(args: &[String], exe_name: &str) -> Args {
    let split = args.iter().position(|arg| arg.eq_ignore_ascii_case("/ARGS")).unwrap_or(args.len());
    let (own, rest) = args.split_at(split);
    let has = |flag: &str| own.iter().any(|arg| arg.eq_ignore_ascii_case(flag));
    let uninstall = has("--uninstall") || exe_name.eq_ignore_ascii_case("uninstall.exe");
    Args {
        mode: if uninstall { Mode::Uninstall } else { Mode::Install },
        silent: has("/S"),
        passive: has("/P") || has("/UPDATE"),
        relaunch: has("/R"),
        app_args: rest.iter().skip(1).map(|arg| arg.trim_matches('"').to_string()).filter(|arg| !arg.is_empty()).collect(),
    }
}

fn reduced_motion() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS};
    let mut enabled = windows::core::BOOL(1);
    let ok = unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, Some(&mut enabled as *mut _ as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)) };
    ok.is_ok() && !enabled.as_bool()
}

fn status_text(step: &str, mode: Mode) -> &'static str {
    match step {
        "stop" => "Fermeture de Nova…",
        "copy" => "Copie de l'application…",
        "register" if mode == Mode::Uninstall => "Nettoyage de Windows…",
        "register" => "Inscription dans Windows…",
        "remove" => "Suppression de l'application…",
        "shortcut" => "Création des raccourcis…",
        "legacy" => "Rangement de l'ancienne version…",
        _ => "Terminé",
    }
}

fn run_headless(args: &Args, target: &Target) -> i32 {
    let result = match args.mode {
        Mode::Install => engine::install(target, &Choices { desktop: target.desktop_link(), autostart: None }, &|_, _| {}),
        Mode::Uninstall => engine::uninstall(target, &|_, _| {}),
    };
    match result {
        Ok(()) => {
            if args.mode == Mode::Install && args.relaunch {
                engine::launch(target, &args.app_args);
            }
            if args.mode == Mode::Uninstall {
                engine::remove_self_later(target);
            }
            0
        }
        Err(_) => 1,
    }
}

fn offscreen() -> bool {
    std::env::var_os("NOVA_SETUP_SNAP").is_some() || std::env::var_os("NOVA_SETUP_OFFSCREEN").is_some()
}

fn snapshot(ui: &SetupWindow, name: &str) {
    let Some(dir) = std::env::var_os("NOVA_SETUP_SNAP") else { return };
    let Ok(buffer) = ui.window().take_snapshot() else { return };
    let path = std::path::Path::new(&dir).join(format!("{name}.png"));
    let _ = write_png(&path, buffer.width(), buffer.height(), buffer.as_bytes());
}

fn write_png(path: &std::path::Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend(kind);
        out.extend(data);
        let mut crc = 0xffff_ffffu32;
        for byte in &out[start..] {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 { 0xedb8_8320 ^ (crc >> 1) } else { crc >> 1 };
            }
        }
        out.extend((!crc).to_be_bytes());
    }
    let mut raw = Vec::with_capacity(rgba.len() + height as usize);
    for row in rgba.chunks(width as usize * 4) {
        raw.push(0);
        raw.extend(row);
    }
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(&raw)?;
    let mut header = Vec::new();
    header.extend(width.to_be_bytes());
    header.extend(height.to_be_bytes());
    header.extend([8, 6, 0, 0, 0]);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &encoder.finish()?);
    chunk(&mut out, b"IEND", &[]);
    std::fs::write(path, out)
}

fn fill_texts(ui: &SetupWindow, mode: Mode, target: &Target) {
    let installed = target.installed_version();
    let update = installed.is_some() && mode == Mode::Install;
    ui.set_window_title(if mode == Mode::Uninstall { "Désinstaller Nova PC Suite" } else { "Installer Nova PC Suite" }.into());
    ui.set_version_label(format!("Nova PC Suite {}", engine::VERSION).into());
    ui.set_path(target.dir.display().to_string().into());
    ui.set_reduced(reduced_motion());
    match mode {
        Mode::Install => {
            ui.set_screen("welcome".into());
            let (heading, detail, primary) = match &installed {
                Some(version) => ("Mettre à jour Nova".to_string(), format!("{} sera remplacée par la version {}, plus légère et sans WebView. Votre historique et vos réglages restent en place.", if version.is_empty() { "La version installée".to_string() } else { format!("La version {version}") }, engine::VERSION), "Mettre à jour"),
                None => ("Installer Nova PC Suite".to_string(), "Le bilan, l'entretien et la réparation de Windows, sans rien envoyer en ligne.".to_string(), "Installer"),
            };
            ui.set_heading(heading.into());
            ui.set_detail(detail.into());
            ui.set_primary_label(primary.into());
            ui.set_show_options(true);
            ui.set_desktop(target.desktop_link() || installed.is_none());
            ui.set_autostart(target.autostart());
            ui.set_done_heading(if update { "Nova est à jour" } else { "Nova est installé" }.into());
            ui.set_done_detail("Vous le retrouvez dans le menu Démarrer.".into());
            ui.set_can_open(true);
        }
        Mode::Uninstall => {
            ui.set_screen("uninstall".into());
            ui.set_heading("Désinstaller Nova ?".into());
            ui.set_detail("L'application, ses raccourcis et son audit planifié seront supprimés. Votre historique reste sur ce PC.".into());
            ui.set_primary_label("Désinstaller".into());
            ui.set_show_options(false);
            ui.set_done_heading("Nova est désinstallé".into());
            ui.set_done_detail("Votre historique est gardé si vous le réinstallez.".into());
            ui.set_can_open(false);
        }
    }
}

fn start_work(ui: &SetupWindow, args: &Args, target: Rc<Target>, auto_close: bool) {
    let mode = args.mode;
    ui.set_screen("progress".into());
    ui.set_progress(0.0);
    ui.set_heading(if mode == Mode::Uninstall { "Désinstallation…" } else { "Installation…" }.into());
    let choices = Choices { desktop: ui.get_desktop(), autostart: if ui.get_show_options() { Some(ui.get_autostart()) } else { None } };
    let weak = ui.as_weak();
    let job = (*target).clone();
    let (relaunch, app_args) = (args.relaunch, args.app_args.clone());
    std::thread::spawn(move || {
        let report = |step: &str, part: f32| {
            let (weak, step) = (weak.clone(), step.to_string());
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    ui.set_progress(part);
                    ui.set_status(status_text(&step, mode).into());
                }
            });
        };
        let result = match mode {
            Mode::Install => engine::install(&job, &choices, &report),
            Mode::Uninstall => engine::uninstall(&job, &report),
        };
        let weak = weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(ui) = weak.upgrade() else { return };
            match result {
                Ok(()) => {
                    ui.set_screen("done".into());
                    if mode == Mode::Uninstall {
                        engine::remove_self_later(&job);
                    }
                    if mode == Mode::Install && relaunch {
                        engine::launch(&job, &app_args);
                    }
                    if auto_close {
                        let weak = ui.as_weak();
                        slint::Timer::single_shot(std::time::Duration::from_millis(900), move || {
                            if let Some(ui) = weak.upgrade() {
                                let _ = ui.hide();
                            }
                        });
                    }
                }
                Err(message) => {
                    ui.set_heading("Quelque chose a bloqué".into());
                    ui.set_error_text(message.into());
                    ui.set_screen("error".into());
                }
            }
        });
    });
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exe_name = std::env::current_exe().ok().and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned())).unwrap_or_default();
    if args.first().map(String::as_str) == Some("--pack") {
        let (Some(app), Some(out)) = (args.get(1), args.get(2)) else {
            eprintln!("nova-setup --pack <nova-native.exe> <Nova-PC-Suite_x.y.z_x64-setup.exe>");
            std::process::exit(2);
        };
        match engine::pack(std::path::Path::new(app), std::path::Path::new(out)) {
            Ok(()) => std::process::exit(0),
            Err(message) => {
                eprintln!("{message}");
                std::process::exit(1);
            }
        }
    }
    let parsed = parse(&args, &exe_name);
    let target = Rc::new(Target::current());
    if parsed.silent {
        std::process::exit(run_headless(&parsed, &target));
    }
    let _ = slint::BackendSelector::new()
        .with_winit_window_attributes_hook(|attrs| {
            use slint::winit_030::winit::platform::windows::{IconExtWindows, WindowAttributesExtWindows};
            let icon = slint::winit_030::winit::window::Icon::from_resource(1, None).ok();
            let attrs = attrs.with_undecorated_shadow(true).with_window_icon(icon.clone()).with_taskbar_icon(icon);
            if offscreen() {
                attrs.with_position(slint::winit_030::winit::dpi::PhysicalPosition::new(-10000, -10000)).with_active(false)
            } else {
                attrs
            }
        })
        .select();
    let ui = SetupWindow::new().expect("fenêtre");
    fill_texts(&ui, parsed.mode, &target);
    if parsed.mode == Mode::Install {
        if let Err(message) = engine::own_payload() {
            ui.set_heading("Installeur incomplet".into());
            ui.set_error_text(message.into());
            ui.set_screen("error".into());
        }
    }
    let weak = ui.as_weak();
    ui.on_close_window(move || {
        if let Some(ui) = weak.upgrade() {
            let _ = ui.hide();
        }
    });
    let weak = ui.as_weak();
    ui.on_minimize(move || {
        if let Some(ui) = weak.upgrade() {
            ui.window().with_winit_window(|window| window.set_minimized(true));
        }
    });
    let weak = ui.as_weak();
    ui.on_drag(move || {
        if let Some(ui) = weak.upgrade() {
            ui.window().with_winit_window(|window| {
                let _ = window.drag_window();
            });
        }
    });
    let passive = parsed.passive;
    let (weak, job, run) = (ui.as_weak(), target.clone(), parsed.clone());
    ui.on_primary(move || {
        if let Some(ui) = weak.upgrade() {
            start_work(&ui, &run, job.clone(), passive);
        }
    });
    let (weak, job, run) = (ui.as_weak(), target.clone(), parsed.clone());
    ui.on_retry(move || {
        if let Some(ui) = weak.upgrade() {
            start_work(&ui, &run, job.clone(), passive);
        }
    });
    let (weak, job) = (ui.as_weak(), target.clone());
    ui.on_open_app(move || {
        engine::launch(&job, &[]);
        if let Some(ui) = weak.upgrade() {
            let _ = ui.hide();
        }
    });
    if passive && ui.get_screen() != "error" {
        start_work(&ui, &parsed, target.clone(), true);
    }
    if std::env::var_os("NOVA_SETUP_SNAP").is_some() {
        let weak = ui.as_weak();
        let mode = parsed.mode;
        slint::Timer::single_shot(std::time::Duration::from_millis(900), move || {
            let Some(ui) = weak.upgrade() else { return };
            let first = ui.get_screen();
            if first == "welcome" || first == "uninstall" {
                snapshot(&ui, &first);
                ui.invoke_primary();
            }
            let weak = ui.as_weak();
            let timer = Rc::new(slint::Timer::default());
            let keep = timer.clone();
            let mut took_progress = false;
            timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(40), move || {
                let Some(ui) = weak.upgrade() else { return };
                let screen = ui.get_screen();
                if !took_progress && screen == "progress" && ui.get_progress() > 0.3 {
                    took_progress = true;
                    snapshot(&ui, "progress");
                }
                if screen == "done" || screen == "error" {
                    let weak = ui.as_weak();
                    let name = if mode == Mode::Uninstall { format!("uninstall-{screen}") } else { screen.to_string() };
                    slint::Timer::single_shot(std::time::Duration::from_millis(600), move || {
                        if let Some(ui) = weak.upgrade() {
                            snapshot(&ui, &name);
                            let _ = ui.hide();
                        }
                    });
                    keep.stop();
                }
            });
            std::mem::forget(timer);
        });
    }
    ui.run().expect("boucle");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tauri_update_arguments_are_understood() {
        let args: Vec<String> = ["/P", "/UPDATE", "/R", "/ARGS", "\"--background\""].iter().map(|s| s.to_string()).collect();
        let parsed = parse(&args, "Nova-PC-Suite_0.4.0_x64-setup.exe");
        assert!(parsed.mode == Mode::Install && parsed.passive && parsed.relaunch && !parsed.silent);
        assert_eq!(parsed.app_args, vec!["--background".to_string()]);
        let parsed = parse(&["/S".to_string()], "uninstall.exe");
        assert!(parsed.mode == Mode::Uninstall && parsed.silent && parsed.app_args.is_empty());
        assert!(parse(&["--uninstall".to_string()], "nova-setup.exe").mode == Mode::Uninstall);
        let parsed = parse(&["/P".to_string(), "/ARGS".to_string(), "/S".to_string()], "setup.exe");
        assert!(!parsed.silent);
    }
}
