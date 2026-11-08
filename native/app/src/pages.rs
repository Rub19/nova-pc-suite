use crate::{
    ask, call, measures_of, model, run_undo, ss, toast, work, AdapterRow, App, Ask, BigFolder, BilanState, BootBar, CleanRow,
    DemarrageState, DiskUse, DriverRow, InstalledRow, Fact, FindingRow, HistoryRow, JournalRow, JournalState, MajState,
    Nova, NettoyageState, PerfState, PilotesState, Plan, Probe, Problem, ProcRow, ReparerState,
    RestoreRow, Stat, StartupRow, Upgrade, Vendor, VolumeRow, WuItem, WuLine, WuState,
};
use nova_engine::report::{self, arr, format_bytes, format_date, format_date_no_year, format_datetime, plural, Finding, Level, Scan, ScanMemory};
use serde_json::{json, Value};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::{cell::RefCell, collections::HashSet, rc::Rc};

#[derive(Default)]
struct Wu {
    selected: Vec<String>,
    items: Vec<Value>,
    state: WuState,
    rows: Option<Rc<VecModel<WuItem>>>,
}

#[derive(Default)]
struct State {
    loaded: [bool; 8],
    scan: Option<Scan>,
    findings: Vec<Finding>,
    drivers: Vec<Value>,
    wu: [Wu; 2],
    upgrades: Vec<Value>,
    clean_items: Vec<Value>,
    chosen: Vec<String>,
    touched: bool,
    clean_rows: Option<Rc<VecModel<CleanRow>>>,
    startup: Vec<Value>,
    boots: Option<Value>,
    changes: Vec<i64>,
    overview: Option<Value>,
    journal: Vec<Value>,
    undone: Vec<i64>,
    installed: Vec<Value>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

fn memory_path() -> Option<std::path::PathBuf> {
    nova_engine::data_dir().ok().map(|d| d.join("last-scan.json"))
}

fn page_index(page: Option<&str>) -> i32 {
    page.and_then(|p| report::PAGES.iter().position(|x| *x == p)).map(|i| i as i32).unwrap_or(-1)
}

pub fn open(ui: &App, page: i32) {
    ui.global::<Nova>().set_page(page);
    let index = page.clamp(0, 7) as usize;
    let first = with(|s| !std::mem::replace(&mut s.loaded[index], true));
    match page {
        0 if first => bilan_load(ui),
        1 if first => pilotes_load(ui),
        2 if first => {
            winget_load(ui);
            history_load(ui);
            installed_load(ui);
        }
        3 if first => nettoyage_load(ui),
        4 if first => demarrage_load(ui),
        5 if first => {
            perf_load(ui);
            procs_load(ui);
            adapters_load(ui);
        }
        6 if first => reparer_load(ui),
        7 => journal_load(ui),
        _ => {}
    }
}

pub fn wire(ui: &App) {
    wire_bilan(ui);
    wire_pilotes(ui);
    wire_maj(ui);
    wire_nettoyage(ui);
    wire_demarrage(ui);
    wire_perf(ui);
    wire_reparer(ui);
    wire_journal(ui);
    for idx in 0..2 {
        wu_push(ui, idx);
    }
}

macro_rules! bind {
    ($ui:expr, $global:ty, $setter:ident, |$app:ident $(, $arg:ident)*| $body:expr) => {{
        let weak = $ui.as_weak();
        $ui.global::<$global>().$setter(move |$($arg),*| {
            if let Some($app) = weak.upgrade() {
                let $app = &$app;
                $body
            }
        });
    }};
}

fn level_name(level: Level) -> &'static str {
    match level {
        Level::Alert => "alert",
        Level::Warn => "warn",
        Level::Ok => "ok",
    }
}

fn bilan_load(ui: &App) {
    let state = ui.global::<BilanState>();
    state.set_loading(true);
    state.set_error(ss(""));
    work(ui, || call("diag", "scan", json!({})), |ui, result| {
        let state = ui.global::<BilanState>();
        state.set_loading(false);
        let parsed = result.and_then(|data| {
            serde_json::from_value::<Scan>(data).map_err(|_| {
                "Cet écran a reçu des données illisibles. Le reste de Nova fonctionne : changez d'écran ou réessayez.".to_string()
            })
        });
        match parsed {
            Ok(scan) => bilan_show(&ui, scan),
            Err(error) => state.set_error(ss(error)),
        }
    });
}

fn bilan_show(ui: &App, scan: Scan) {
    let items = report::findings(&scan);
    let saved: Option<ScanMemory> = memory_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok());
    let memory = report::remember(saved, &scan.timestamp, &items);
    if let (Some(path), Ok(text)) = (memory_path(), serde_json::to_vec(&memory)) {
        let _ = nova_engine::winpaths::write_owned(&path, &text, false);
    }
    let row = |f: &Finding| FindingRow {
        id: ss(f.id),
        level: ss(level_name(f.level)),
        title: ss(&f.title),
        detail: ss(&f.detail),
        page: page_index(f.page),
        cta: ss(f.cta.unwrap_or("")),
        fresh: memory.fresh.iter().any(|id| id == f.id),
    };
    let state = ui.global::<BilanState>();
    let issues: Vec<FindingRow> = items.iter().filter(|f| f.level != Level::Ok).map(row).collect();
    let fine: Vec<FindingRow> = items.iter().filter(|f| f.level == Level::Ok).map(row).collect();
    ui.global::<Nova>().set_issues(issues.len() as i32);
    state.set_issues(model(issues));
    state.set_fine(model(fine));
    state.set_headline(ss(report::headline(&items)));
    let solved = if memory.solved > 0 {
        format!(". {} depuis la précédente.", plural(memory.solved as u64, "point réglé", "points réglés"))
    } else {
        ".".to_string()
    };
    state.set_meta(ss(format!("Analyse du {}{solved}", format_datetime(&scan.timestamp))));
    state.set_admin_note(!scan.boots.available);
    state.set_facts(model(
        report::machine_facts(&scan)
            .into_iter()
            .map(|(key, value)| Fact { key: ss(key), value: ss(value) })
            .collect(),
    ));
    state.set_disks(model(
        scan.volumes
            .iter()
            .map(|v| {
                let (ratio, level) = report::volume_usage(v);
                let label = if v.label.is_empty() { String::new() } else { format!(" {}", v.label) };
                DiskUse {
                    title: ss(format!("{}{label}", v.drive)),
                    detail: ss(format!("{} libres sur {}", format_bytes(v.free_bytes), format_bytes(v.size_bytes))),
                    ratio: ratio as f32,
                    level: ss(level_name(level)),
                }
            })
            .collect(),
    ));
    state.set_stability(model(
        report::stability_stats(&scan)
            .into_iter()
            .map(|(label, value)| Stat { label: ss(label), value: ss(value) })
            .collect(),
    ));
    state.set_last_incident(ss(scan
        .stability
        .last_incident
        .as_deref()
        .map(|d| format!("Dernier incident système : {}.", format_datetime(d)))
        .unwrap_or_default()));
    state.set_crashes(model(
        report::crash_lines(&scan)
            .into_iter()
            .map(|(key, value)| Fact { key: ss(key), value: ss(value) })
            .collect(),
    ));
    state.set_has_data(true);
    with(|s| {
        s.scan = Some(scan);
        s.findings = items;
    });
}

fn copy_text(ui: &App, text: &str, done: &str) {
    match nova_engine::win::set_clipboard(text) {
        Ok(()) => toast(ui, done, false, None),
        Err(error) => toast(ui, &error, true, None),
    }
}

fn wire_bilan(ui: &App) {
    bind!(ui, BilanState, on_rescan, |ui| bilan_load(ui));
    bind!(ui, BilanState, on_go, |ui, page| open(ui, page));
    bind!(ui, BilanState, on_copy, |ui| {
        let text = with(|s| s.scan.as_ref().map(|scan| report::report_text(scan, &s.findings)));
        if let Some(text) = text {
            copy_text(ui, &text, "Bilan copié");
        }
    });
}

fn wu_kind(idx: usize) -> &'static str {
    if idx == 0 { "Driver" } else { "Software" }
}

fn wu_push(ui: &App, idx: usize) {
    with(|s| {
        let wu = &mut s.wu[idx];
        wu.state.selected_count = wu.selected.len() as i32;
        wu.state.all_selected = !wu.items.is_empty() && wu.selected.len() == wu.items.len();
        let rows = wu.rows.get_or_insert_with(|| Rc::new(VecModel::default())).clone();
        if idx == 0 {
            let g = ui.global::<PilotesState>();
            g.set_wu(wu.state.clone());
            if g.get_wu_items().row_count() == 0 && rows.row_count() == 0 {
                g.set_wu_items(ModelRc::from(rows));
            }
        } else {
            let g = ui.global::<MajState>();
            g.set_wu(wu.state.clone());
            if g.get_wu_items().row_count() == 0 && rows.row_count() == 0 {
                g.set_wu_items(ModelRc::from(rows));
            }
        }
    });
}

fn wu_detail(item: &Value) -> String {
    let date = item["driverDate"].as_str().filter(|d| !d.is_empty()).map(|d| format!("pilote du {}", format_date(Some(d)))).unwrap_or_default();
    let kb: Vec<String> = arr(item["kb"].clone()).iter().filter_map(|k| k.as_str().map(str::to_string)).collect();
    let size = item["sizeBytes"].as_f64().filter(|s| *s > 0.0).map(format_bytes).unwrap_or_default();
    let parts = [
        item["driverProvider"].as_str().unwrap_or("").to_string(),
        date,
        kb.join(", "),
        item["severity"].as_str().unwrap_or("").to_string(),
        size,
        if item["rebootLikely"] == true { "redémarrage probable".to_string() } else { String::new() },
    ];
    parts.into_iter().filter(|p| !p.is_empty()).collect::<Vec<_>>().join(", ")
}

fn wu_search(ui: &App, idx: usize) {
    with(|s| {
        s.wu[idx].state.loading = true;
        s.wu[idx].state.error = ss("");
    });
    wu_push(ui, idx);
    let kind = wu_kind(idx);
    work(ui, move || call("updates", "wu-search", json!({ "kind": kind })), move |ui, result| {
        with(|s| {
            let wu = &mut s.wu[idx];
            wu.state.loading = false;
            match result {
                Ok(data) => {
                    wu.state.searched = true;
                    wu.items = arr(data["items"].clone());
                    let ids: Vec<String> = wu.items.iter().filter_map(|i| i["id"].as_str().map(str::to_string)).collect();
                    wu.selected.retain(|id| ids.contains(id));
                    if let Some(rows) = &wu.rows {
                        rows.set_vec(
                            wu.items
                                .iter()
                                .map(|i| WuItem {
                                    id: ss(i["id"].as_str().unwrap_or("")),
                                    title: ss(i["title"].as_str().unwrap_or("")),
                                    detail: ss(wu_detail(i)),
                                    selected: wu.selected.iter().any(|x| Some(x.as_str()) == i["id"].as_str()),
                                })
                                .collect::<Vec<_>>(),
                        );
                    }
                }
                Err(error) => wu.state.error = ss(error),
            }
        });
        wu_push(&ui, idx);
    });
}

fn wu_sync_checks(idx: usize) {
    with(|s| {
        let wu = &s.wu[idx];
        if let Some(rows) = &wu.rows {
            for i in 0..rows.row_count() {
                if let Some(mut row) = rows.row_data(i) {
                    let selected = wu.selected.iter().any(|id| id.as_str() == row.id.as_str());
                    if row.selected != selected {
                        row.selected = selected;
                        rows.set_row_data(i, row);
                    }
                }
            }
        }
    });
}

fn wu_toggle(ui: &App, idx: usize, id: &str) {
    with(|s| {
        let selected = &mut s.wu[idx].selected;
        if let Some(at) = selected.iter().position(|x| x == id) {
            selected.remove(at);
        } else {
            selected.push(id.to_string());
        }
    });
    wu_sync_checks(idx);
    wu_push(ui, idx);
}

fn wu_toggle_all(ui: &App, idx: usize) {
    with(|s| {
        let wu = &mut s.wu[idx];
        wu.selected = if wu.selected.len() == wu.items.len() {
            Vec::new()
        } else {
            wu.items.iter().filter_map(|i| i["id"].as_str().map(str::to_string)).collect()
        };
    });
    wu_sync_checks(idx);
    wu_push(ui, idx);
}

fn wu_install(ui: &App, idx: usize) {
    let (count, licenses, ids) = with(|s| {
        let wu = &s.wu[idx];
        let chosen: Vec<&Value> = wu.items.iter().filter(|i| wu.selected.iter().any(|id| Some(id.as_str()) == i["id"].as_str())).collect();
        let licenses: Vec<(String, String)> = chosen
            .iter()
            .filter(|i| i["eulaAccepted"] != true && i["eula"].as_str().is_some_and(|e| !e.is_empty()))
            .map(|i| (i["title"].as_str().unwrap_or("").to_string(), i["eula"].as_str().unwrap_or("").to_string()))
            .collect();
        (chosen.len() as u64, licenses, wu.selected.clone())
    });
    let accept = !licenses.is_empty();
    ask(
        ui,
        Ask {
            title: format!("Installer {}", plural(count, "mise à jour", "mises à jour")),
            body: "Nova crée d'abord un point de restauration, pour pouvoir revenir en arrière. Le PC peut ensuite demander un redémarrage.".into(),
            confirm: if accept { "Accepter et installer" } else { "Installer" },
            danger: false,
            lines: vec![],
            licenses,
        },
        move |ui| {
            with(|s| s.wu[idx].state.busy = true);
            wu_push(ui, idx);
            let payload = json!({"kind": wu_kind(idx), "ids": ids, "acceptEulas": accept});
            work(ui, move || call("updates", "wu-install", payload), move |ui, result| {
                with(|s| s.wu[idx].state.busy = false);
                match result {
                    Ok(data) => {
                        if let Some(summary) = data["summary"].as_str() {
                            toast(&ui, summary, false, None);
                        }
                        wu_outcome(&ui, idx, &data);
                        with(|s| s.wu[idx].selected.clear());
                        wu_search(&ui, idx);
                    }
                    Err(error) => {
                        toast(&ui, &error, true, None);
                        wu_push(&ui, idx);
                    }
                }
            });
        },
    );
}

fn wu_outcome(ui: &App, idx: usize, data: &Value) {
    let mut lines: Vec<WuLine> = arr(data["results"].clone())
        .iter()
        .map(|r| {
            let ok = r["ok"] == true;
            let label = if ok { "Installé".to_string() } else { format!("Échec {}", r["hresult"].as_str().unwrap_or("")) };
            WuLine { ok, text: ss(format!("{label} : {}", r["title"].as_str().unwrap_or(""))) }
        })
        .collect();
    for d in arr(data["changedDrivers"].clone()) {
        lines.push(WuLine {
            ok: true,
            text: ss(format!("{} : {} › {}", d["name"].as_str().unwrap_or(""), d["before"].as_str().unwrap_or(""), d["after"].as_str().unwrap_or(""))),
        });
    }
    let point = &data["restorePoint"];
    let note = if point.is_object() {
        format!(
            "{} : {}.",
            if point["created"] == true { "Point de restauration créé" } else { "Point de restauration récent conservé" },
            point["description"].as_str().unwrap_or("")
        )
    } else {
        String::new()
    };
    with(|s| {
        s.wu[idx].state.restore_note = ss(note);
        s.wu[idx].state.reboot = data["rebootRequired"] == true;
    });
    let measures = measures_of(&data["measures"]);
    if idx == 0 {
        let g = ui.global::<PilotesState>();
        g.set_wu_measures(measures);
        g.set_wu_lines(model(lines));
    } else {
        let g = ui.global::<MajState>();
        g.set_wu_measures(measures);
        g.set_wu_lines(model(lines));
    }
}

fn pilotes_load(ui: &App) {
    let g = ui.global::<PilotesState>();
    g.set_loading(true);
    g.set_error(ss(""));
    work(ui, || call("updates", "devices", json!({})), |ui, result| {
        let g = ui.global::<PilotesState>();
        g.set_loading(false);
        match result {
            Ok(data) => {
                g.set_problems(model(
                    arr(data["problems"].clone())
                        .iter()
                        .map(|p| Problem {
                            name: ss(p["name"].as_str().unwrap_or("")),
                            advice: ss(report::device_code_text(p["code"].as_u64().unwrap_or(p["code"].as_f64().unwrap_or(0.0) as u64) as u32)),
                        })
                        .collect(),
                ));
                g.set_vendors(model(
                    arr(data["vendors"].clone())
                        .iter()
                        .map(|v| Vendor { id: ss(v["id"].as_str().unwrap_or("")), name: ss(v["name"].as_str().unwrap_or("")) })
                        .collect(),
                ));
                with(|s| s.drivers = arr(data["drivers"].clone()));
                g.set_has_data(true);
                pilotes_filter(&ui);
            }
            Err(error) => g.set_error(ss(error)),
        }
    });
}

pub fn filter_drivers(drivers: &[Value], query: &str, include_microsoft: bool) -> Vec<Value> {
    let q = query.trim().to_lowercase();
    let mut seen = HashSet::new();
    let mut rows: Vec<Value> = drivers
        .iter()
        .filter(|d| include_microsoft || !report::invert_driver_filter(d["manufacturer"].as_str().unwrap_or("")))
        .filter(|d| {
            q.is_empty()
                || format!("{} {} {}", d["name"].as_str().unwrap_or(""), d["manufacturer"].as_str().unwrap_or(""), d["version"].as_str().unwrap_or(""))
                    .to_lowercase()
                    .contains(&q)
        })
        .filter(|d| seen.insert(format!("{}|{}", d["name"].as_str().unwrap_or(""), d["version"].as_str().unwrap_or(""))))
        .cloned()
        .collect();
    rows.sort_by(|a, b| a["date"].as_str().unwrap_or("9").cmp(b["date"].as_str().unwrap_or("9")));
    rows
}

fn pilotes_filter(ui: &App) {
    let g = ui.global::<PilotesState>();
    let rows = with(|s| filter_drivers(&s.drivers, &g.get_query(), g.get_include_microsoft()));
    g.set_drivers(model(
        rows.iter()
            .map(|d| DriverRow {
                name: ss(d["name"].as_str().unwrap_or("")),
                manufacturer: ss(d["manufacturer"].as_str().unwrap_or("")),
                version: ss(d["version"].as_str().unwrap_or("")),
                date: ss(format_date(d["date"].as_str())),
                signed: d["signed"] == true,
            })
            .collect(),
    ));
}

fn wire_pilotes(ui: &App) {
    bind!(ui, PilotesState, on_reload, |ui| pilotes_load(ui));
    bind!(ui, PilotesState, on_filter, |ui| pilotes_filter(ui));
    bind!(ui, PilotesState, on_open_vendor, |ui, id| {
        let id = id.to_string();
        work(ui, move || call("updates", "open-vendor", json!({ "id": id })), |ui, result| {
            if let Err(error) = result {
                toast(&ui, &error, true, None);
            }
        });
    });
    bind!(ui, PilotesState, on_wu_search, |ui| wu_search(ui, 0));
    bind!(ui, PilotesState, on_wu_install, |ui| wu_install(ui, 0));
    bind!(ui, PilotesState, on_wu_toggle_all, |ui| wu_toggle_all(ui, 0));
    bind!(ui, PilotesState, on_wu_toggle, |ui, id| wu_toggle(ui, 0, &id));
}

fn winget_load(ui: &App) {
    let g = ui.global::<MajState>();
    g.set_winget_loading(true);
    g.set_winget_error(ss(""));
    work(ui, || call("updates", "winget-list", json!({})), |ui, result| {
        let g = ui.global::<MajState>();
        g.set_winget_loading(false);
        match result {
            Ok(data) => {
                let items = arr(data["items"].clone());
                g.set_upgrades(model(
                    items
                        .iter()
                        .map(|i| Upgrade {
                            id: ss(i["id"].as_str().unwrap_or("")),
                            name: ss(i["name"].as_str().unwrap_or("")),
                            versions: ss(format!("{} › {}", i["version"].as_str().unwrap_or(""), i["available"].as_str().unwrap_or(""))),
                        })
                        .collect(),
                ));
                with(|s| s.upgrades = items);
                g.set_winget_loaded(true);
            }
            Err(error) => g.set_winget_error(ss(error)),
        }
    });
}

fn history_load(ui: &App) {
    let g = ui.global::<MajState>();
    g.set_history_error(ss(""));
    work(ui, || call("updates", "wu-history", json!({})), |ui, result| {
        let g = ui.global::<MajState>();
        match result {
            Ok(data) => g.set_history(model(
                arr(data["items"].clone())
                    .iter()
                    .map(|h| HistoryRow {
                        title: ss(h["title"].as_str().unwrap_or("")),
                        detail: ss(format!("{}{}", format_date(h["date"].as_str()), if h["ok"] == true { "" } else { ", échec" })),
                    })
                    .collect(),
            )),
            Err(error) => g.set_history_error(ss(error)),
        }
    });
}

fn installed_load(ui: &App) {
    let g = ui.global::<MajState>();
    g.set_installed_loading(true);
    g.set_installed_error(ss(""));
    work(ui, || call("updates", "installed", json!({})), |ui, result| {
        let g = ui.global::<MajState>();
        g.set_installed_loading(false);
        match result {
            Ok(data) => {
                with(|s| s.installed = arr(data["items"].clone()));
                installed_show(&ui);
            }
            Err(error) => g.set_installed_error(ss(error)),
        }
    });
}

pub fn filter_installed(list: &[Value], query: &str, sort: i32) -> Vec<Value> {
    let q = query.trim().to_lowercase();
    let mut rows: Vec<Value> = list
        .iter()
        .filter(|a| {
            q.is_empty()
                || format!("{} {}", a["name"].as_str().unwrap_or(""), a["publisher"].as_str().unwrap_or(""))
                    .to_lowercase()
                    .contains(&q)
        })
        .cloned()
        .collect();
    match sort {
        1 => rows.sort_by(|a, b| b["bytes"].as_f64().unwrap_or(0.0).total_cmp(&a["bytes"].as_f64().unwrap_or(0.0))),
        2 => rows.sort_by(|a, b| b["date"].as_str().unwrap_or("").cmp(a["date"].as_str().unwrap_or(""))),
        _ => {}
    }
    rows
}

fn installed_show(ui: &App) {
    let g = ui.global::<MajState>();
    let (rows, total) = with(|s| (filter_installed(&s.installed, &g.get_installed_query(), g.get_installed_sort()), s.installed.len()));
    g.set_installed_title(ss(format!("{} sur ce PC", plural(total as u64, "logiciel installé", "logiciels installés"))));
    g.set_installed(model(
        rows.iter()
            .map(|a| {
                let parts = [
                    a["publisher"].as_str().unwrap_or("").to_string(),
                    a["version"].as_str().filter(|v| !v.is_empty()).map(|v| format!("version {v}")).unwrap_or_default(),
                    a["date"].as_str().map(|d| format!("installé le {}", format_date(Some(d)))).unwrap_or_default(),
                ];
                let bytes = a["bytes"].as_f64().unwrap_or(0.0);
                InstalledRow {
                    name: ss(a["name"].as_str().unwrap_or("")),
                    detail: ss(parts.into_iter().filter(|p| !p.is_empty()).collect::<Vec<_>>().join(", ")),
                    size: ss(if bytes > 0.0 { format_bytes(bytes) } else { String::new() }),
                }
            })
            .collect(),
    ));
}

fn upgrade_one(ui: &App, id: &str) {
    let name = with(|s| s.upgrades.iter().find(|u| u["id"].as_str() == Some(id)).and_then(|u| u["name"].as_str()).unwrap_or(id).to_string());
    ui.global::<MajState>().set_queue(ss(name));
    let id = id.to_string();
    work(ui, move || call("updates", "winget-upgrade", json!({ "id": id })), |ui, result| {
        let g = ui.global::<MajState>();
        g.set_queue(ss(""));
        match result {
            Ok(data) => {
                g.set_winget_measures(measures_of(&data["measures"]));
                if let Some(summary) = data["summary"].as_str() {
                    toast(&ui, summary, false, None);
                }
            }
            Err(error) => toast(&ui, &error, true, None),
        }
        winget_load(&ui);
    });
}

fn upgrade_next(ui: &App, mut left: Vec<(String, String)>, total: usize, done: usize) {
    if left.is_empty() {
        ui.global::<MajState>().set_queue(ss(""));
        toast(ui, &format!("{} sur {total}", plural(done as u64, "logiciel mis à jour", "logiciels mis à jour")), false, None);
        winget_load(ui);
        return;
    }
    let (id, name) = left.remove(0);
    ui.global::<MajState>().set_queue(ss(name));
    work(ui, move || call("updates", "winget-upgrade", json!({ "id": id })), move |ui, result| {
        let done = done + if result.is_ok() { 1 } else { 0 };
        if let Err(error) = result {
            toast(&ui, &error, true, None);
        }
        upgrade_next(&ui, left, total, done);
    });
}

fn wire_maj(ui: &App) {
    bind!(ui, MajState, on_wu_search, |ui| wu_search(ui, 1));
    bind!(ui, MajState, on_wu_install, |ui| wu_install(ui, 1));
    bind!(ui, MajState, on_wu_toggle_all, |ui| wu_toggle_all(ui, 1));
    bind!(ui, MajState, on_wu_toggle, |ui, id| wu_toggle(ui, 1, &id));
    bind!(ui, MajState, on_winget_reload, |ui| winget_load(ui));
    bind!(ui, MajState, on_history_reload, |ui| history_load(ui));
    bind!(ui, MajState, on_installed_reload, |ui| installed_load(ui));
    bind!(ui, MajState, on_installed_filter, |ui| installed_show(ui));
    bind!(ui, MajState, on_open_apps, |ui| {
        if let Err(error) = nova_engine::open_windows("apps") {
            toast(ui, &error, true, None);
        }
    });
    bind!(ui, MajState, on_upgrade, |ui, id| upgrade_one(ui, &id));
    bind!(ui, MajState, on_upgrade_all, |ui| {
        let list: Vec<(String, String)> = with(|s| {
            s.upgrades
                .iter()
                .map(|u| (u["id"].as_str().unwrap_or("").to_string(), u["name"].as_str().unwrap_or("").to_string()))
                .collect()
        });
        let total = list.len();
        upgrade_next(ui, list, total, 0);
    });
    bind!(ui, MajState, on_reboot, |ui| {
        ask(
            ui,
            Ask {
                title: "Redémarrer le PC".into(),
                body: "Windows redémarre dans 60 secondes. Enregistrez votre travail. Vous pourrez annuler pendant ce délai.".into(),
                confirm: "Redémarrer dans 60 s",
                danger: true,
                lines: vec![],
                licenses: vec![],
            },
            |ui| {
                ui.global::<MajState>().set_rebooting(true);
                work(ui, || call("updates", "reboot", json!({})), |ui, result| {
                    ui.global::<MajState>().set_rebooting(false);
                    match result {
                        Ok(data) => {
                            let undo = data["undo"].is_object().then(|| data["undo"].clone());
                            toast(&ui, data["summary"].as_str().unwrap_or("Fait"), false, undo);
                        }
                        Err(error) => toast(&ui, &error, true, None),
                    }
                });
            },
        );
    });
}

fn usable(item: &Value) -> bool {
    item["needsAdmin"] != true && item["blocked"] != true && item["bytes"].as_f64().unwrap_or(0.0) > 0.0
}

fn nettoyage_load(ui: &App) {
    let g = ui.global::<NettoyageState>();
    g.set_loading(true);
    g.set_error(ss(""));
    work(ui, || call("cleanup", "scan", json!({})), |ui, result| {
        let g = ui.global::<NettoyageState>();
        g.set_loading(false);
        match result {
            Ok(data) => {
                let mut items = arr(data["items"].clone());
                report::by_size_then_usable(&mut items, |i| {
                    (i["bytes"].as_f64().unwrap_or(0.0), i["needsAdmin"] == true, i["blocked"] == true)
                });
                g.set_free(ss(format_bytes(data["freeBytes"].as_f64().unwrap_or(0.0))));
                g.set_locked(items.iter().any(|i| i["needsAdmin"] == true));
                g.set_tools(items.iter().filter(|i| usable(i)).count() > 1);
                with(|s| {
                    if !s.touched {
                        s.chosen = items
                            .iter()
                            .filter(|i| usable(i) && i["selected"] == true)
                            .filter_map(|i| i["id"].as_str().map(str::to_string))
                            .collect();
                    }
                    s.clean_items = items;
                    let rows: Vec<CleanRow> = s.clean_items.iter().map(|i| clean_row(i, &s.chosen)).collect();
                    let target = s.clean_rows.get_or_insert_with(|| Rc::new(VecModel::default())).clone();
                    target.set_vec(rows);
                    g.set_rows(ModelRc::from(target));
                });
                g.set_has_data(true);
                nettoyage_totals(&ui);
            }
            Err(error) => g.set_error(ss(error)),
        }
    });
}

fn clean_row(item: &Value, chosen: &[String]) -> CleanRow {
    let bytes = item["bytes"].as_f64().unwrap_or(0.0);
    let id = item["id"].as_str().unwrap_or("");
    let blocked = item["blocked"] == true;
    let admin = item["needsAdmin"] == true;
    CleanRow {
        id: ss(id),
        label: ss(item["label"].as_str().unwrap_or("")),
        detail: ss(if blocked { "Reporté : une mise à jour attend un redémarrage." } else { item["detail"].as_str().unwrap_or("") }),
        size: ss(if admin { "droits requis".to_string() } else if bytes > 0.0 { format_bytes(bytes) } else { "vide".to_string() }),
        off: !usable(item),
        checked: usable(item) && chosen.iter().any(|c| c == id),
    }
}

fn picked(s: &State) -> Vec<Value> {
    s.clean_items
        .iter()
        .filter(|i| usable(i) && s.chosen.iter().any(|c| Some(c.as_str()) == i["id"].as_str()))
        .cloned()
        .collect()
}

fn nettoyage_totals(ui: &App) {
    let g = ui.global::<NettoyageState>();
    with(|s| {
        let list = picked(s);
        let total: f64 = list.iter().filter_map(|i| i["bytes"].as_f64()).sum();
        g.set_total(ss(format_bytes(total)));
        g.set_can_clean(!list.is_empty());
        g.set_none_picked(list.is_empty());
        g.set_all_picked(list.len() == s.clean_items.iter().filter(|i| usable(i)).count());
        if let Some(rows) = &s.clean_rows {
            for i in 0..rows.row_count() {
                if let Some(mut row) = rows.row_data(i) {
                    let checked = !row.off && s.chosen.iter().any(|c| c.as_str() == row.id.as_str());
                    if row.checked != checked {
                        row.checked = checked;
                        rows.set_row_data(i, row);
                    }
                }
            }
        }
    });
}

fn largest_load(ui: &App) {
    let g = ui.global::<NettoyageState>();
    g.set_largest_loading(true);
    g.set_largest_error(ss(""));
    work(ui, || call("cleanup", "largest", json!({})), |ui, result| {
        let g = ui.global::<NettoyageState>();
        g.set_largest_loading(false);
        match result {
            Ok(data) => {
                let items = arr(data["items"].clone());
                let top = items.iter().filter_map(|i| i["bytes"].as_f64()).fold(1.0, f64::max);
                g.set_largest(model(
                    items
                        .iter()
                        .map(|i| {
                            let bytes = i["bytes"].as_f64().unwrap_or(0.0);
                            BigFolder {
                                name: ss(i["name"].as_str().unwrap_or("")),
                                path: ss(i["path"].as_str().unwrap_or("")),
                                size: ss(format_bytes(bytes)),
                                ratio: (bytes / top) as f32,
                            }
                        })
                        .collect(),
                ));
                g.set_largest_loaded(true);
            }
            Err(error) => g.set_largest_error(ss(error)),
        }
    });
}

fn reveal(ui: &App, path: &str) {
    if let Err(error) = nova_engine::win::reveal(path) {
        toast(ui, &error, true, None);
    }
}

fn wire_nettoyage(ui: &App) {
    bind!(ui, NettoyageState, on_measure_largest, |ui| largest_load(ui));
    bind!(ui, NettoyageState, on_open_folder, |ui, path| reveal(ui, &path));
    bind!(ui, NettoyageState, on_rescan, |ui| nettoyage_load(ui));
    bind!(ui, NettoyageState, on_toggle, |ui, id| {
        with(|s| {
            s.touched = true;
            if let Some(at) = s.chosen.iter().position(|c| c.as_str() == id.as_str()) {
                s.chosen.remove(at);
            } else {
                s.chosen.push(id.to_string());
            }
        });
        nettoyage_totals(ui);
    });
    bind!(ui, NettoyageState, on_pick_all, |ui| {
        with(|s| {
            s.touched = true;
            s.chosen = s.clean_items.iter().filter(|i| usable(i)).filter_map(|i| i["id"].as_str().map(str::to_string)).collect();
        });
        nettoyage_totals(ui);
    });
    bind!(ui, NettoyageState, on_pick_none, |ui| {
        with(|s| {
            s.touched = true;
            s.chosen.clear();
        });
        nettoyage_totals(ui);
    });
    bind!(ui, NettoyageState, on_clean, |ui| {
        let list = with(|s| picked(s));
        let total: f64 = list.iter().filter_map(|i| i["bytes"].as_f64()).sum();
        let ids: Vec<String> = list.iter().filter_map(|i| i["id"].as_str().map(str::to_string)).collect();
        ask(
            ui,
            Ask {
                title: format!("Supprimer {}", format_bytes(total)),
                body: "Ces fichiers seront supprimés définitivement. Il n'y a pas d'annulation, mais aucun document personnel n'est concerné.".into(),
                confirm: "Supprimer",
                danger: true,
                lines: list
                    .iter()
                    .map(|i| format!("{} : {}", i["label"].as_str().unwrap_or(""), format_bytes(i["bytes"].as_f64().unwrap_or(0.0))))
                    .collect(),
                licenses: vec![],
            },
            move |ui| {
                let g = ui.global::<NettoyageState>();
                g.set_busy(true);
                g.set_done_text(ss(""));
                work(ui, move || call("cleanup", "clean", json!({ "ids": ids })), |ui, result| {
                    let g = ui.global::<NettoyageState>();
                    g.set_busy(false);
                    match result {
                        Ok(data) => {
                            if let Some(summary) = data["summary"].as_str() {
                                toast(&ui, summary, false, None);
                            }
                            let skipped = data["skippedFiles"].as_u64().unwrap_or(0);
                            let extra = if skipped > 0 {
                                format!(" {} en place.", plural(skipped, "fichier ouvert laissé", "fichiers ouverts laissés"))
                            } else {
                                String::new()
                            };
                            g.set_done_text(ss(format!("{} supprimés.{extra}", format_bytes(data["removedBytes"].as_f64().unwrap_or(0.0)))));
                            g.set_done_measures(measures_of(&data["measures"]));
                            g.set_done_notes(model(
                                arr(data["results"].clone())
                                    .iter()
                                    .filter(|r| r["note"].is_string())
                                    .map(|r| ss(format!("{} : {}", r["label"].as_str().unwrap_or(""), r["note"].as_str().unwrap_or(""))))
                                    .collect(),
                            ));
                            nettoyage_load(&ui);
                        }
                        Err(error) => toast(&ui, &error, true, None),
                    }
                });
            },
        );
    });
}

pub fn demarrage_load(ui: &App) {
    let g = ui.global::<DemarrageState>();
    g.set_loading(true);
    g.set_error(ss(""));
    work(
        ui,
        || (call("startup", "list", json!({})), call("diag", "boots", json!({})), nova_engine::journal()),
        |ui, (list, boots, journal)| {
            let g = ui.global::<DemarrageState>();
            g.set_loading(false);
            let changes: Vec<i64> = journal
                .iter()
                .filter(|e| e["module"] == "startup" && e["ok"] == true)
                .filter_map(|e| e["timestamp"].as_i64())
                .collect();
            if let Ok(boots) = boots {
                show_boots(&ui, &boots, &changes);
                with(|s| s.boots = Some(boots));
            }
            with(|s| s.changes = changes);
            match list {
                Ok(data) => {
                    let items = arr(data["items"].clone());
                    let active = items.iter().filter(|i| i["enabled"] == true).count() as u64;
                    g.set_count_title(ss(format!("{} au démarrage", plural(active, "programme lancé", "programmes lancés"))));
                    g.set_rows(model(items.iter().map(startup_row).collect()));
                    with(|s| s.startup = items);
                    g.set_has_data(true);
                }
                Err(error) => g.set_error(ss(error)),
            }
        },
    );
}

fn startup_row(i: &Value) -> StartupRow {
    let name = i["name"].as_str().unwrap_or("");
    let display = name.rsplit('\\').next().unwrap_or(name);
    let mut parts = vec![i["publisher"].as_str().filter(|p| !p.is_empty()).unwrap_or("Éditeur inconnu").to_string()];
    if i["running"] == true {
        parts.push(format!("{} de mémoire en ce moment", format_bytes(i["ramBytes"].as_f64().unwrap_or(0.0))));
    }
    parts.push(i["sourceLabel"].as_str().unwrap_or("").to_string());
    StartupRow {
        source: ss(i["source"].as_str().unwrap_or("")),
        name: ss(name),
        display: ss(display),
        detail: ss(parts.join(", ")),
        path: ss(i["path"].as_str().unwrap_or("")),
        missing: i["exists"] != true,
        enabled: i["enabled"] == true,
        protected: i["protected"] == true,
        manageable: i["manageable"] == true,
    }
}

fn show_boots(ui: &App, data: &Value, changes: &[i64]) {
    let g = ui.global::<DemarrageState>();
    let parsed: report::Boots = serde_json::from_value(data.clone()).unwrap_or_default();
    g.set_boots_known(true);
    g.set_boots_available(parsed.available);
    let top = parsed.items.iter().map(|b| b.ms).fold(0.0, f64::max).max(1.0);
    g.set_bars(model(
        parsed
            .items
            .iter()
            .rev()
            .map(|b| BootBar {
                value: ss(format!("{} s", (b.ms / 1000.0).round())),
                date: ss(format_date_no_year(&b.date)),
                ratio: (b.ms / top) as f32,
            })
            .collect(),
    ));
    let compare = report::boot_comparison(&parsed.items, changes)
        .map(|(before, after)| {
            format!(
                "Depuis votre dernier changement : {} s › {} s en moyenne.",
                (before / 1000.0).round(),
                (after / 1000.0).round()
            )
        })
        .unwrap_or_default();
    g.set_compare(ss(compare));
}

fn wire_demarrage(ui: &App) {
    bind!(ui, DemarrageState, on_reload, |ui| demarrage_load(ui));
    bind!(ui, DemarrageState, on_reveal, |ui, path| reveal(ui, &path));
    bind!(ui, DemarrageState, on_toggle, |ui, source, name| {
        let enabled = with(|s| {
            s.startup
                .iter()
                .find(|i| i["source"].as_str() == Some(source.as_str()) && i["name"].as_str() == Some(name.as_str()))
                .map(|i| i["enabled"] == true)
        });
        let Some(enabled) = enabled else { return };
        let g = ui.global::<DemarrageState>();
        g.set_pending(name.clone());
        let payload = json!({"source": source.as_str(), "name": name.as_str(), "enabled": !enabled});
        work(ui, move || call("startup", "set", payload), |ui, result| {
            ui.global::<DemarrageState>().set_pending(ss(""));
            match result {
                Ok(data) => {
                    let undo = data["undo"].is_object().then(|| data["undo"].clone());
                    toast(&ui, data["summary"].as_str().unwrap_or("Fait"), false, undo);
                }
                Err(error) => toast(&ui, &error, true, None),
            }
            demarrage_load(&ui);
        });
    });
}

pub fn perf_load(ui: &App) {
    let g = ui.global::<PerfState>();
    g.set_loading(true);
    g.set_error(ss(""));
    work(ui, || call("perf", "overview", json!({})), |ui, result| {
        let g = ui.global::<PerfState>();
        g.set_loading(false);
        match result {
            Ok(data) => {
                g.set_plans(model(
                    arr(data["plans"].clone())
                        .iter()
                        .map(|p| Plan { guid: ss(p["guid"].as_str().unwrap_or("")), name: ss(p["name"].as_str().unwrap_or("")), active: p["active"] == true })
                        .collect(),
                ));
                let gpu = &data["gpu"];
                if gpu.is_object() {
                    g.set_gpu_title(ss(gpu["name"].as_str().unwrap_or("")));
                    g.set_gpu_note(ss(format!("Pilote {}. Relevé au moment de l'ouverture de cette page.", gpu["driver"].as_str().unwrap_or(""))));
                    let mib = |key: &str| format_bytes(gpu[key].as_f64().unwrap_or(0.0) * 1048576.0);
                    let mut stats = vec![
                        Stat { label: ss("Température"), value: ss(format!("{} °C", gpu["temperature"].as_f64().unwrap_or(0.0).round())) },
                        Stat { label: ss("Utilisation"), value: ss(format!("{} %", gpu["usePercent"].as_f64().unwrap_or(0.0).round())) },
                        Stat { label: ss("Mémoire vidéo"), value: ss(format!("{} sur {}", mib("memoryUsedMiB"), mib("memoryTotalMiB"))) },
                    ];
                    if let Some(power) = gpu["powerW"].as_f64() {
                        stats.push(Stat { label: ss("Puissance"), value: ss(format!("{} W", power.round())) });
                    }
                    g.set_gpu_stats(model(stats));
                }
                g.set_volumes(model(
                    arr(data["volumes"].clone())
                        .iter()
                        .map(|v| {
                            let letter = v["letter"].as_str().unwrap_or("");
                            let label = v["label"].as_str().filter(|l| !l.is_empty()).map(|l| format!(" {l}")).unwrap_or_default();
                            VolumeRow {
                                letter: ss(letter),
                                title: ss(format!("{letter}:{label}")),
                                detail: ss(format!(
                                    "{}, {} libres sur {}, dernière optimisation : {}",
                                    v["media"].as_str().unwrap_or("Support inconnu"),
                                    format_bytes(v["freeBytes"].as_f64().unwrap_or(0.0)),
                                    format_bytes(v["sizeBytes"].as_f64().unwrap_or(0.0)),
                                    format_date(v["lastOptimized"].as_str())
                                )),
                            }
                        })
                        .collect(),
                ));
                let enabled = data["hibernation"]["enabled"] == true;
                g.set_hibernation_on(enabled);
                let bytes = format_bytes(data["hibernation"]["fileBytes"].as_f64().unwrap_or(0.0));
                g.set_hibernation_text(ss(if enabled {
                    format!("Le fichier hiberfil.sys occupe {bytes}.")
                } else {
                    "Le démarrage rapide de Windows est aussi désactivé.".to_string()
                }));
                with(|s| s.overview = Some(data));
                g.set_has_data(true);
            }
            Err(error) => g.set_error(ss(error)),
        }
    });
}

fn procs_load(ui: &App) {
    let g = ui.global::<PerfState>();
    g.set_procs_loading(true);
    g.set_procs_error(ss(""));
    work(ui, || call("perf", "processes", json!({})), |ui, result| {
        let g = ui.global::<PerfState>();
        g.set_procs_loading(false);
        match result {
            Ok(data) => {
                g.set_procs(model(
                    arr(data["items"].clone())
                        .iter()
                        .map(|p| {
                            let count = p["count"].as_u64().unwrap_or(1);
                            ProcRow {
                                name: ss(p["name"].as_str().unwrap_or("")),
                                cpu: ss(format!("{} %", report::fr_number(p["cpuPercent"].as_f64().unwrap_or(0.0), 1))),
                                ram: ss(format_bytes(p["ramBytes"].as_f64().unwrap_or(0.0))),
                                closable: p["closable"] == true,
                                muted: ss(if count > 1 { format!("({count} processus)") } else { String::new() }),
                            }
                        })
                        .collect(),
                ));
                g.set_procs_loaded(true);
            }
            Err(error) => g.set_procs_error(ss(error)),
        }
    });
}

fn adapters_load(ui: &App) {
    work(ui, || call("perf", "adapters", json!({})), |ui, result| {
        if let Ok(data) = result {
            ui.global::<PerfState>().set_adapters(model(
                arr(data["items"].clone())
                    .iter()
                    .map(|a| {
                        let addresses: Vec<String> = arr(a["addresses"].clone()).iter().filter_map(|v| v.as_str().map(str::to_string)).collect();
                        let parts = [
                            addresses.into_iter().next().unwrap_or_default(),
                            a["gateway"].as_str().map(|g| format!("passerelle {g}")).unwrap_or_default(),
                            a["speed"].as_str().unwrap_or("").to_string(),
                            a["mac"].as_str().filter(|m| !m.is_empty()).map(|m| format!("MAC {m}")).unwrap_or_default(),
                        ];
                        AdapterRow {
                            name: ss(a["name"].as_str().unwrap_or("")),
                            detail: ss(parts.into_iter().filter(|p| !p.is_empty()).collect::<Vec<_>>().join(", ")),
                        }
                    })
                    .collect(),
            ));
        }
    });
}

fn perf_action(ui: &App, action: &'static str, payload: Value, key: String) {
    let g = ui.global::<PerfState>();
    g.set_busy(ss(key));
    g.set_measures(model(vec![]));
    work(ui, move || call("perf", action, payload), |ui, result| {
        let g = ui.global::<PerfState>();
        g.set_busy(ss(""));
        match result {
            Ok(data) => {
                g.set_measures(measures_of(&data["measures"]));
                if let Some(summary) = data["summary"].as_str() {
                    let undo = data["undo"].is_object().then(|| data["undo"].clone());
                    toast(&ui, summary, false, undo);
                }
                perf_load(&ui);
            }
            Err(error) => toast(&ui, &error, true, None),
        }
    });
}

fn net_test(ui: &App) {
    let g = ui.global::<PerfState>();
    g.set_net_loading(true);
    g.set_net_error(ss(""));
    work(ui, || call("perf", "network-test", json!({})), |ui, result| {
        let g = ui.global::<PerfState>();
        g.set_net_loading(false);
        match result {
            Ok(data) => {
                let mut rows: Vec<Probe> = arr(data["probes"].clone())
                    .iter()
                    .map(|p| Probe {
                        role: ss(p["role"].as_str().unwrap_or("")),
                        result: ss(match p["averageMs"].as_f64() {
                            None => "aucune réponse".to_string(),
                            Some(ms) => format!("{} ms, {} % de perte", report::fr_number(ms, 1), p["lossPercent"].as_f64().unwrap_or(0.0)),
                        }),
                    })
                    .collect();
                rows.push(Probe {
                    role: ss("Résolution des noms (DNS)"),
                    result: ss(if data["dnsOk"] == true { format!("{} ms", data["dnsMs"].as_u64().unwrap_or(0)) } else { "échec".to_string() }),
                });
                g.set_probes(model(rows));
                g.set_net_tested(true);
            }
            Err(error) => g.set_net_error(ss(error)),
        }
    });
}

fn wire_perf(ui: &App) {
    bind!(ui, PerfState, on_reload, |ui| perf_load(ui));
    bind!(ui, PerfState, on_procs_reload, |ui| procs_load(ui));
    bind!(ui, PerfState, on_net_test, |ui| net_test(ui));
    bind!(ui, PerfState, on_set_plan, |ui, guid| {
        perf_action(ui, "power-apply", json!({"guid": guid.as_str()}), guid.to_string());
    });
    bind!(ui, PerfState, on_optimize, |ui, letter| {
        perf_action(ui, "volume-optimize", json!({"letter": letter.as_str()}), letter.to_string());
    });
    bind!(ui, PerfState, on_hibernate, |ui, enabled| {
        if enabled {
            perf_action(ui, "hibernate-set", json!({"enabled": true}), "hibernate".into());
            return;
        }
        let bytes = with(|s| s.overview.as_ref().and_then(|o| o["hibernation"]["fileBytes"].as_f64()).unwrap_or(0.0));
        ask(
            ui,
            Ask {
                title: "Désactiver la veille prolongée".into(),
                body: format!("Libère {} sur le disque. Le démarrage rapide de Windows sera aussi désactivé : le PC peut démarrer un peu plus lentement. Vous pourrez la réactiver ici.", format_bytes(bytes)),
                confirm: "Désactiver",
                danger: false,
                lines: vec![],
                licenses: vec![],
            },
            |ui| perf_action(ui, "hibernate-set", json!({"enabled": false}), "hibernate".into()),
        );
    });
    bind!(ui, PerfState, on_close_process, |ui, name| {
        let g = ui.global::<PerfState>();
        g.set_proc_busy(name.clone());
        g.set_proc_measures(model(vec![]));
        let payload = json!({"name": name.as_str()});
        work(ui, move || call("perf", "process-close", payload), |ui, result| {
            let g = ui.global::<PerfState>();
            g.set_proc_busy(ss(""));
            match result {
                Ok(data) => {
                    g.set_proc_measures(measures_of(&data["measures"]));
                    toast(&ui, data["summary"].as_str().unwrap_or("Fait"), false, None);
                }
                Err(error) => toast(&ui, &error, true, None),
            }
            procs_load(&ui);
        });
    });
    bind!(ui, PerfState, on_dns_flush, |ui| {
        ui.global::<PerfState>().set_net_busy(ss("dns"));
        ui.global::<PerfState>().set_net_measures(model(vec![]));
        work(ui, || call("perf", "dns-flush", json!({})), |ui, result| {
            let g = ui.global::<PerfState>();
            g.set_net_busy(ss(""));
            match result {
                Ok(data) => {
                    g.set_net_measures(measures_of(&data["measures"]));
                    toast(&ui, data["summary"].as_str().unwrap_or("Fait"), false, None);
                }
                Err(error) => toast(&ui, &error, true, None),
            }
        });
    });
}

pub fn reparer_load(ui: &App) {
    let g = ui.global::<ReparerState>();
    g.set_restore_error(ss(""));
    work(ui, || call("repair", "restore-list", json!({})), |ui, result| {
        let g = ui.global::<ReparerState>();
        g.set_restore_loaded(true);
        match result {
            Ok(data) => {
                g.set_needs_admin(data["needsAdmin"] == true);
                g.set_restore(model(
                    arr(data["items"].clone())
                        .iter()
                        .map(|p| RestoreRow {
                            title: ss(p["description"].as_str().unwrap_or("")),
                            date: ss(format_datetime(p["date"].as_str().unwrap_or(""))),
                        })
                        .collect(),
                ));
            }
            Err(error) => g.set_restore_error(ss(error)),
        }
    });
}

fn verdict_text(verdict: &str) -> Option<&'static str> {
    Some(match verdict {
        "ok" => "Aucun problème trouvé.",
        "repaired" => "Problèmes trouvés et réparés.",
        "problem" => "Problème signalé. Lisez le détail ci-dessous.",
        _ => return None,
    })
}

fn repair_launch(ui: &App, action: String, label: String) {
    let g = ui.global::<ReparerState>();
    g.set_outcome_title(ss(""));
    g.set_outcome_log(ss(""));
    g.set_outcome_measures(model(vec![]));
    g.set_busy(true);
    let name = action.clone();
    work(ui, move || call("repair", &name, json!({})), move |ui, result| {
        let g = ui.global::<ReparerState>();
        g.set_busy(false);
        match result {
            Ok(data) => {
                let silent = action.ends_with("-check") || action == "chkdsk-scan";
                if !silent {
                    if let Some(summary) = data["summary"].as_str() {
                        let undo = data["undo"].is_object().then(|| data["undo"].clone());
                        toast(&ui, summary, false, undo);
                    }
                }
                let text = verdict_text(data["verdict"].as_str().unwrap_or("")).map(str::to_string).unwrap_or_else(|| data["summary"].as_str().unwrap_or("").to_string());
                g.set_outcome_title(ss(format!("{label} : {text}")));
                g.set_outcome_measures(measures_of(&data["measures"]));
                let log: Vec<String> = arr(data["output"].clone()).iter().filter_map(|l| l.as_str().map(str::to_string)).collect();
                g.set_outcome_log(ss(log.join("\n")));
                if matches!(action.as_str(), "restore-create" | "dism-repair" | "sfc-repair" | "protection-enable") {
                    reparer_load(&ui);
                }
            }
            Err(error) => toast(&ui, &error, true, None),
        }
    });
}

fn wire_reparer(ui: &App) {
    bind!(ui, ReparerState, on_restore_reload, |ui| reparer_load(ui));
    bind!(ui, ReparerState, on_check, |ui, action, title| {
        repair_launch(ui, action.to_string(), title.to_string());
    });
    bind!(ui, ReparerState, on_repair, |ui, action, title, time| {
        let (action, title_owned) = (action.to_string(), title.to_string());
        ask(
            ui,
            Ask {
                title: format!("Réparer : {title}"),
                body: format!("Nova crée d'abord un point de restauration. Comptez {time}. Vous pouvez utiliser le PC pendant ce temps, sans l'éteindre."),
                confirm: "Réparer",
                danger: false,
                lines: vec![],
                licenses: vec![],
            },
            move |ui| repair_launch(ui, action, title_owned),
        );
    });
    bind!(ui, ReparerState, on_clean_store, |ui| {
        ask(
            ui,
            Ask {
                title: "Supprimer les anciens composants".into(),
                body: "Windows retire les anciennes versions remplacées par des mises à jour. Elles ne pourront plus être restaurées par désinstallation d'une mise à jour déjà ancienne. Comptez plusieurs minutes.".into(),
                confirm: "Supprimer",
                danger: false,
                lines: vec![],
                licenses: vec![],
            },
            |ui| repair_launch(ui, "store-cleanup".into(), "Anciens composants".into()),
        );
    });
    bind!(ui, ReparerState, on_open_restore, |ui| {
        work(ui, || call("repair", "open-restore", json!({})), |ui, result| {
            if let Err(error) = result {
                toast(&ui, &error, true, None);
            }
        });
    });
    bind!(ui, ReparerState, on_create_point, |ui| repair_launch(ui, "restore-create".into(), "Point de restauration".into()));
    bind!(ui, ReparerState, on_enable_protection, |ui| repair_launch(ui, "protection-enable".into(), "Protection du système".into()));
}

fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn can_undo(s: &State, index: usize) -> bool {
    report::still_undoable(&s.journal, index, now_seconds())
        && !s.undone.contains(&s.journal[index]["timestamp"].as_i64().unwrap_or(0))
}

fn journal_visible(s: &State, filter: i32) -> Vec<usize> {
    (0..s.journal.len())
        .filter(|&i| match filter {
            1 => s.journal[i]["ok"] != true,
            2 => can_undo(s, i),
            _ => true,
        })
        .collect()
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn entry_title(entry: &Value) -> String {
    let text = entry["summary"].as_str().or(entry["action"].as_str()).unwrap_or("");
    if entry["ok"] == true { text.to_string() } else { format!("Échec : {text}") }
}

fn journal_show(ui: &App) {
    let g = ui.global::<JournalState>();
    let filter = g.get_filter();
    let rows = with(|s| {
        let mut previous = String::new();
        journal_visible(s, filter)
            .into_iter()
            .map(|i| {
                let entry = &s.journal[i];
                let stamp = entry["timestamp"].as_i64().unwrap_or(0);
                let day = report::journal_day(stamp);
                let header = if day != previous { capitalized(&day) } else { String::new() };
                previous = day;
                JournalRow {
                    day: ss(header),
                    time: ss(report::format_timestamp(stamp)),
                    title: ss(entry_title(entry)),
                    failed: entry["ok"] != true,
                    restore: ss(entry["restorePoint"]["description"].as_str().unwrap_or("")),
                    measures: measures_of(&entry["measures"]),
                    undo_label: ss(if can_undo(s, i) { entry["undo"]["label"].as_str().unwrap_or("") } else { "" }),
                    index: i as i32,
                }
            })
            .collect::<Vec<_>>()
    });
    g.set_has_entries(with(|s| !s.journal.is_empty()));
    g.set_rows(model(rows));
}

fn journal_load(ui: &App) {
    work(ui, nova_engine::journal, |ui, entries| {
        with(|s| s.journal = entries);
        journal_show(&ui);
    });
}

fn wire_journal(ui: &App) {
    bind!(ui, JournalState, on_set_filter, |ui, filter| {
        ui.global::<JournalState>().set_filter(filter);
        journal_show(ui);
    });
    bind!(ui, JournalState, on_copy, |ui| {
        let filter = ui.global::<JournalState>().get_filter();
        let text = with(|s| {
            journal_visible(s, filter)
                .into_iter()
                .map(|i| {
                    let entry = &s.journal[i];
                    format!("{} : {}", report::format_timestamp(entry["timestamp"].as_i64().unwrap_or(0)), entry_title(entry))
                })
                .collect::<Vec<_>>()
                .join("\n")
        });
        copy_text(ui, &text, "Journal copié");
    });
    bind!(ui, JournalState, on_undo, |ui, index| {
        let entry = with(|s| s.journal.get(index as usize).cloned());
        let Some(entry) = entry else { return };
        let stamp = entry["timestamp"].as_i64().unwrap_or(0);
        run_undo(ui, entry["undo"].clone(), move |ui| {
            with(|s| s.undone.push(stamp));
            journal_load(ui);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_inventory_hides_microsoft_and_duplicates_then_sorts_by_date() {
        let drivers = vec![
            json!({"name": "A", "manufacturer": "NVIDIA", "version": "1", "date": "2024-01-01"}),
            json!({"name": "A", "manufacturer": "NVIDIA", "version": "1", "date": "2024-01-01"}),
            json!({"name": "B", "manufacturer": "Microsoft", "version": "2", "date": "2020-01-01"}),
            json!({"name": "C", "manufacturer": "(Standard system devices)", "version": "2", "date": "2021-01-01"}),
            json!({"name": "D", "manufacturer": "Intel", "version": "3", "date": "2019-01-01"}),
        ];
        let names = |rows: Vec<Value>| rows.iter().map(|r| r["name"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        assert_eq!(names(filter_drivers(&drivers, "", false)), ["D", "A"]);
        assert_eq!(names(filter_drivers(&drivers, "", true)), ["D", "B", "C", "A"]);
        assert_eq!(names(filter_drivers(&drivers, "intel", true)), ["D"]);
    }

    #[test]
    fn journal_filters_failures_and_undoable_rows() {
        let undo = json!({"module": "startup", "action": "set", "label": "Réactiver", "payload": {"source": "s", "name": "n"}});
        let mut state = State::default();
        state.journal = vec![
            json!({"ok": true, "timestamp": 300, "summary": "a", "undo": undo}),
            json!({"ok": true, "timestamp": 200, "summary": "b", "undo": undo}),
            json!({"ok": false, "timestamp": 100, "summary": "c"}),
        ];
        assert_eq!(journal_visible(&state, 0), [0, 1, 2]);
        assert_eq!(journal_visible(&state, 1), [2]);
        assert_eq!(journal_visible(&state, 2), [0]);
        state.undone.push(300);
        assert!(journal_visible(&state, 2).is_empty());
        assert_eq!(entry_title(&state.journal[2]), "Échec : c");
    }

    #[test]
    fn clean_rows_never_tick_what_cannot_be_cleaned() {
        let item = json!({"id": "x", "label": "X", "detail": "d", "bytes": 2048, "needsAdmin": true, "blocked": false});
        let row = clean_row(&item, &["x".to_string()]);
        assert!(row.off && !row.checked);
        assert_eq!(row.size.as_str(), "droits requis");
        let free = json!({"id": "y", "label": "Y", "detail": "d", "bytes": 0, "needsAdmin": false, "blocked": false});
        assert_eq!(clean_row(&free, &[]).size.as_str(), "vide");
    }

    #[test]
    fn installed_software_filters_and_sorts() {
        let list = vec![
            json!({"name": "Alpha", "publisher": "X", "bytes": 10.0, "date": "2024-01-01"}),
            json!({"name": "Beta", "publisher": "Yoyo", "bytes": 30.0, "date": "2025-01-01"}),
        ];
        let names = |rows: Vec<Value>| rows.iter().map(|r| r["name"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        assert_eq!(names(filter_installed(&list, "yoyo", 0)), ["Beta"]);
        assert_eq!(names(filter_installed(&list, "", 1)), ["Beta", "Alpha"]);
        assert_eq!(names(filter_installed(&list, "", 2)), ["Beta", "Alpha"]);
        assert_eq!(names(filter_installed(&list, "", 0)), ["Alpha", "Beta"]);
    }

    #[test]
    fn days_start_with_a_capital() {
        assert_eq!(capitalized("mercredi 4 novembre"), "Mercredi 4 novembre");
    }
}
