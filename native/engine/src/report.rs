use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Timelike, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

pub fn one_or_many<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = Value::deserialize(deserializer)?;
    Ok(arr(value)
        .into_iter()
        .map(serde_json::from_value)
        .collect::<Result<Vec<T>, _>>()
        .map_err(serde::de::Error::custom)?)
}

pub fn arr(value: Value) -> Vec<Value> {
    match value {
        Value::Null => Vec::new(),
        Value::Array(list) => list,
        other => vec![other],
    }
}

#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct Os {
    pub name: String,
    pub release: String,
    pub build: String,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct Computer {
    pub manufacturer: String,
    pub model: String,
    pub board: String,
    pub bios: String,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct Cpu {
    pub name: String,
    pub cores: u32,
    pub threads: u32,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Memory {
    pub total_bytes: f64,
    pub free_bytes: f64,
    pub modules: u32,
    #[serde(rename = "speedMHz")]
    pub speed_mhz: Option<u32>,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Gpu {
    pub name: String,
    pub driver_version: String,
    pub driver_date: Option<String>,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Volume {
    pub drive: String,
    pub label: String,
    pub size_bytes: f64,
    pub free_bytes: f64,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Disk {
    pub name: String,
    pub media: String,
    pub bus: String,
    pub size_bytes: f64,
    pub health: String,
    pub temperature: Option<f64>,
    pub wear: Option<f64>,
    pub errors: Option<f64>,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Defender {
    pub running: bool,
    pub realtime: bool,
    pub signature_age_days: i64,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Security {
    pub defender: Option<Defender>,
    #[serde(deserialize_with = "one_or_many")]
    pub other_antivirus: Vec<String>,
    pub firewall_off: Option<Vec<String>>,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct CodeCount {
    pub code: String,
    pub count: u32,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct NameCount {
    pub name: String,
    pub count: u32,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Stability {
    pub unexpected_shutdowns: u32,
    pub blue_screens: u32,
    pub gpu_hangs: u32,
    #[serde(deserialize_with = "one_or_many")]
    pub gpu_codes: Vec<CodeCount>,
    pub app_crashes: u32,
    #[serde(deserialize_with = "one_or_many")]
    pub top_crashes: Vec<NameCount>,
    pub last_incident: Option<String>,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct Boot {
    pub date: String,
    pub ms: f64,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct Boots {
    pub available: bool,
    #[serde(deserialize_with = "one_or_many")]
    pub items: Vec<Boot>,
}
#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Scan {
    pub is_admin: bool,
    pub timestamp: String,
    pub os: Os,
    pub computer: Computer,
    pub cpu: Cpu,
    pub memory: Memory,
    #[serde(deserialize_with = "one_or_many")]
    pub gpus: Vec<Gpu>,
    pub uptime_seconds: f64,
    pub pending_reboot: bool,
    pub system_drive: String,
    #[serde(deserialize_with = "one_or_many")]
    pub volumes: Vec<Volume>,
    #[serde(deserialize_with = "one_or_many")]
    pub disks: Vec<Disk>,
    pub device_problems: u32,
    pub security: Security,
    pub stability: Stability,
    pub last_update: Option<String>,
    pub boots: Boots,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Alert,
    Warn,
    Ok,
}

#[derive(Clone, Debug)]
pub struct Finding {
    pub id: &'static str,
    pub level: Level,
    pub title: String,
    pub detail: String,
    pub page: Option<&'static str>,
    pub cta: Option<&'static str>,
}

pub const PAGES: [&str; 8] = [
    "bilan",
    "pilotes",
    "miseajour",
    "nettoyage",
    "demarrage",
    "performances",
    "reparer",
    "journal",
];

const THIN: char = '\u{202f}';

pub fn plural(count: u64, one: &str, many: &str) -> String {
    format!("{count} {}", if count > 1 { many } else { one })
}

pub fn fr_number(value: f64, digits: u32) -> String {
    let factor = 10f64.powi(digits as i32);
    let rounded = (value.abs() * factor).round() / factor;
    let negative = value < 0.0 && rounded != 0.0;
    let whole = rounded.trunc() as u64;
    let fraction = ((rounded - rounded.trunc()) * factor).round() as u64;
    let plain = whole.to_string();
    let mut grouped = String::new();
    for (i, c) in plain.chars().enumerate() {
        if i > 0 && (plain.len() - i) % 3 == 0 {
            grouped.push(THIN);
        }
        grouped.push(c);
    }
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str(&grouped);
    if fraction > 0 && digits > 0 {
        let text = format!("{fraction:0width$}", width = digits as usize);
        out.push(',');
        out.push_str(text.trim_end_matches('0'));
    }
    out
}

pub fn format_bytes(value: f64) -> String {
    if !value.is_finite() {
        return "inconnu".into();
    }
    let units = ["o", "Ko", "Mo", "Go", "To"];
    let mut size = value.abs();
    let mut unit = 0;
    while size >= 1024.0 && unit < units.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    let digits = if unit >= 3 && size < 100.0 { 1 } else { 0 };
    format!("{} {}", fr_number(size, digits), units[unit])
}

pub fn format_duration(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let days = total / 86400;
    let hours = (total % 86400) / 3600;
    let minutes = (total % 3600) / 60;
    if days > 0 {
        format!("{days} j {hours} h")
    } else if hours > 0 {
        format!("{hours} h {minutes} min")
    } else {
        format!("{minutes} min")
    }
}

const MONTHS_SHORT: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
];
const MONTHS: [&str; 12] = [
    "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre",
    "novembre", "décembre",
];
const WEEKDAYS: [&str; 7] = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];

pub fn parse_moment(iso: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(iso)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

fn parse_day(iso: &str) -> Option<NaiveDate> {
    parse_moment(iso)
        .map(|d| d.with_timezone(&Local).date_naive())
        .or_else(|| NaiveDate::parse_from_str(iso.get(..10)?, "%Y-%m-%d").ok())
}

pub fn format_date(iso: Option<&str>) -> String {
    let Some(iso) = iso.filter(|s| !s.is_empty()) else {
        return "inconnue".into();
    };
    match parse_day(iso) {
        Some(d) => format!("{} {} {}", d.day(), MONTHS_SHORT[d.month0() as usize], d.year()),
        None => iso.to_string(),
    }
}

pub fn format_date_no_year(iso: &str) -> String {
    match parse_day(iso) {
        Some(d) => format!("{} {}", d.day(), MONTHS_SHORT[d.month0() as usize]),
        None => iso.to_string(),
    }
}

fn format_local(moment: DateTime<Local>) -> String {
    format!(
        "{} {} à {:02}:{:02}",
        moment.day(),
        MONTHS_SHORT[moment.month0() as usize],
        moment.hour(),
        moment.minute()
    )
}

pub fn format_datetime(iso: &str) -> String {
    match parse_moment(iso) {
        Some(d) => format_local(d.with_timezone(&Local)),
        None => iso.to_string(),
    }
}

pub fn format_timestamp(seconds: i64) -> String {
    Local
        .timestamp_opt(seconds, 0)
        .single()
        .map(format_local)
        .unwrap_or_default()
}

pub fn journal_day(seconds: i64) -> String {
    Local
        .timestamp_opt(seconds, 0)
        .single()
        .map(|d| {
            format!(
                "{} {} {}",
                WEEKDAYS[d.weekday().num_days_from_monday() as usize],
                d.day(),
                MONTHS[d.month0() as usize]
            )
        })
        .unwrap_or_default()
}

pub fn format_measure_value(value: &Value, unit: &str) -> String {
    match value {
        Value::Null => "inconnu".into(),
        Value::String(text) if unit != "date" => text.clone(),
        Value::String(text) => {
            if text.as_bytes().get(4) == Some(&b'-') {
                format_datetime(text)
            } else {
                text.clone()
            }
        }
        other => {
            let number = other.as_f64().unwrap_or(0.0);
            match unit {
                "bytes" => format_bytes(number),
                "percent" => format!("{} %", fr_number(number, 1)),
                _ => fr_number(number, 1),
            }
        }
    }
}

pub fn measure_delta(before: &Value, after: &Value, unit: &str) -> Option<String> {
    let (before, after) = (before.as_f64()?, after.as_f64()?);
    if unit == "text" || unit == "date" {
        return None;
    }
    let delta = after - before;
    let threshold = if unit == "bytes" { 1048576.0 } else { 0.5 };
    if delta.abs() < threshold {
        return Some("sans changement".into());
    }
    let sign = if delta > 0.0 { "+" } else { "−" };
    Some(format!(
        "{sign}{}",
        format_measure_value(&serde_json::json!(delta.abs()), unit)
    ))
}

pub fn gpu_code_text(code: &str) -> Option<&'static str> {
    Some(match code {
        "117" => "pilote relancé après un blocage (TDR)",
        "141" => "moteur graphique bloqué",
        "142" => "délai dépassé par le pilote",
        "193" => "capture de diagnostic graphique",
        "1a1" => "blocage graphique en cours de fonctionnement",
        "1a8" => "écran noir détecté par Windows",
        "1b8" => "écran noir détecté dans le pilote",
        _ => return None,
    })
}

pub fn device_code_text(code: u32) -> String {
    match code {
        1 => "Mal configuré. Réinstallez le pilote.",
        3 => "Le pilote est peut-être endommagé, ou la mémoire manque.",
        10 => "L'appareil ne démarre pas. Éteignez complètement le PC 30 secondes. Si l'erreur revient, mettez à jour son pilote.",
        12 => "Pas assez de ressources libres pour cet appareil.",
        14 => "Redémarrez le PC pour terminer l'installation.",
        18 => "Réinstallez le pilote de cet appareil.",
        19 => "Sa configuration dans le registre est endommagée.",
        21 => "Windows est en train de retirer cet appareil. Redémarrez.",
        24 => "Appareil absent, ou mal branché.",
        28 => "Aucun pilote n'est installé pour cet appareil.",
        31 => "Windows ne parvient pas à charger le pilote.",
        32 => "Le service de cet appareil est désactivé.",
        37 => "Le pilote a refusé de démarrer.",
        39 => "Le pilote est absent ou endommagé.",
        41 => "Le pilote est chargé mais l'appareil est introuvable.",
        43 => "Windows a arrêté cet appareil après un problème signalé.",
        45 => "Appareil débranché.",
        48 => "Pilote bloqué car incompatible avec Windows.",
        52 => "La signature du pilote n'a pas pu être vérifiée.",
        other => return format!("Windows signale l'erreur {other}."),
    }
    .to_string()
}

pub fn findings(scan: &Scan) -> Vec<Finding> {
    findings_at(scan, Utc::now())
}

pub fn findings_at(scan: &Scan, now: DateTime<Utc>) -> Vec<Finding> {
    let mut list: Vec<Finding> = Vec::new();
    let mut push = |id: &'static str, level: Level, title: String, detail: String, target: Option<(&'static str, &'static str)>| {
        list.push(Finding {
            id,
            level,
            title,
            detail,
            page: target.map(|t| t.0),
            cta: target.map(|t| t.1),
        });
    };

    if let Some(system) = scan.volumes.iter().find(|v| v.drive == scan.system_drive && v.size_bytes > 0.0) {
        let ratio = system.free_bytes / system.size_bytes;
        let text = format!("{} libres sur {}.", format_bytes(system.free_bytes), format_bytes(system.size_bytes));
        if ratio < 0.1 {
            push("space", Level::Alert, format!("Le disque {} est presque plein", system.drive), format!("{text} Windows ralentit et les mises à jour peuvent échouer."), Some(("nettoyage", "Nettoyer")));
        } else if ratio < 0.2 {
            push("space", Level::Warn, format!("Le disque {} se remplit", system.drive), text, Some(("nettoyage", "Nettoyer")));
        } else {
            push("space", Level::Ok, format!("Espace disponible sur {}", system.drive), text, None);
        }
    }

    let bad = scan.disks.iter().find(|d| !d.health.is_empty() && d.health != "Healthy");
    let errored = scan.disks.iter().find(|d| d.errors.unwrap_or(0.0) > 0.0);
    let hot = scan.disks.iter().find(|d| d.temperature.unwrap_or(0.0) >= 70.0);
    let worn = scan.disks.iter().find(|d| d.wear.unwrap_or(0.0) >= 90.0);
    if let Some(d) = bad {
        push("disk", Level::Alert, format!("{} signale un problème", d.name), "Windows ne le considère plus comme sain. Sauvegardez vos fichiers maintenant.".into(), Some(("reparer", "Analyser")));
    } else if let Some(d) = errored {
        push("disk", Level::Alert, format!("{} a des erreurs de lecture", d.name), format!("{} erreurs non corrigées. Sauvegardez vos fichiers.", d.errors.unwrap_or(0.0)), Some(("reparer", "Analyser")));
    } else if let Some(d) = hot {
        push("disk", Level::Warn, format!("{} chauffe", d.name), format!("{} °C. Vérifiez la ventilation du boîtier.", d.temperature.unwrap_or(0.0)), None);
    } else if let Some(d) = worn {
        push("disk", Level::Warn, format!("{} est usé", d.name), format!("{} % de sa durée de vie consommée. Prévoyez son remplacement.", d.wear.unwrap_or(0.0)), None);
    } else if !scan.disks.is_empty() {
        push("disk", Level::Ok, "Disques sains".into(), format!("{} par Windows.", plural(scan.disks.len() as u64, "disque déclaré sain", "disques déclarés sains")), None);
    }

    let s = &scan.stability;
    if s.gpu_hangs > 0 {
        let named = s.gpu_codes.first().map(|top| {
            let text = gpu_code_text(&top.code).map(str::to_string).unwrap_or_else(|| format!("code {}", top.code));
            format!(" Le plus fréquent : {text}.")
        }).unwrap_or_default();
        push("gpu", if s.gpu_hangs >= 5 { Level::Alert } else { Level::Warn }, format!("La carte graphique a décroché {} fois en 30 jours", s.gpu_hangs), format!("Windows a enregistré ces blocages.{named} Essayez d'abord le dernier pilote du fabricant."), Some(("pilotes", "Voir les pilotes")));
    }
    if s.blue_screens > 0 {
        push("bsod", Level::Alert, format!("{} en 30 jours", plural(s.blue_screens as u64, "écran bleu", "écrans bleus")), "Vérifiez les pilotes et les fichiers système.".into(), Some(("reparer", "Vérifier Windows")));
    }
    if s.unexpected_shutdowns > 0 {
        push("power", if s.unexpected_shutdowns >= 3 { Level::Alert } else { Level::Warn }, format!("{} en 30 jours", plural(s.unexpected_shutdowns as u64, "arrêt brutal", "arrêts brutaux")), "Le PC s'est éteint sans passer par l'arrêt normal, à cause d'un plantage ou d'une coupure.".into(), Some(("reparer", "Vérifier Windows")));
    }
    if s.gpu_hangs == 0 && s.blue_screens == 0 && s.unexpected_shutdowns == 0 {
        push("stable", Level::Ok, "Aucun plantage système".into(), "Aucun écran bleu ni arrêt brutal en 30 jours.".into(), None);
    }
    if s.app_crashes >= 10 {
        let detail = s.top_crashes.first().map(|top| format!("{} revient le plus souvent ({} fois).", top.name, top.count)).unwrap_or_default();
        push("apps", Level::Warn, format!("{} plantages d'applications en 30 jours", s.app_crashes), detail, None);
    }

    if scan.device_problems > 0 {
        push("devices", Level::Warn, format!("{} en erreur", plural(scan.device_problems as u64, "appareil", "appareils")), "Windows ne les fait pas fonctionner correctement.".into(), Some(("pilotes", "Voir les appareils")));
    } else {
        push("devices", Level::Ok, "Tous les appareils fonctionnent".into(), "Aucun appareil signalé en erreur.".into(), None);
    }

    if scan.pending_reboot {
        push("reboot", Level::Warn, "Un redémarrage est en attente".into(), "Une mise à jour ne sera terminée qu'après le redémarrage.".into(), Some(("miseajour", "Voir les mises à jour")));
    } else if scan.uptime_seconds > 14.0 * 86400.0 {
        push("uptime", Level::Warn, format!("Allumé depuis {}", format_duration(scan.uptime_seconds)), "Un redémarrage libère la mémoire et termine les opérations en attente.".into(), None);
    }

    if let Some(last) = scan.last_update.as_deref().filter(|d| !d.is_empty()) {
        let days = parse_moment(last).map(|d| (now - d).num_seconds() as f64 / 86400.0).unwrap_or(0.0);
        if days > 30.0 {
            push("updates", Level::Warn, "Windows n'a rien installé depuis plus d'un mois".into(), format!("Dernière installation : {}.", format_date(Some(last))), Some(("miseajour", "Chercher")));
        } else {
            push("updates", Level::Ok, "Windows à jour récemment".into(), format!("Dernière installation : {}.", format_date(Some(last))), None);
        }
    }

    let third = !scan.security.other_antivirus.is_empty();
    let defender = scan.security.defender.as_ref();
    if !third && defender.map(|d| !d.running || !d.realtime).unwrap_or(true) {
        push("av", Level::Alert, "Protection antivirus inactive".into(), "Aucun antivirus ne surveille ce PC en temps réel.".into(), None);
    } else if let Some(d) = defender.filter(|d| d.realtime && d.signature_age_days > 7) {
        push("av", Level::Warn, "Définitions antivirus anciennes".into(), format!("Dernière mise à jour il y a {} jours.", d.signature_age_days), Some(("miseajour", "Chercher")));
    } else {
        push("av", Level::Ok, "Antivirus actif".into(), if third { scan.security.other_antivirus.join(", ") } else { "Microsoft Defender surveille ce PC.".into() }, None);
    }
    if let Some(off) = scan.security.firewall_off.as_ref().filter(|o| !o.is_empty()) {
        push("firewall", Level::Warn, "Pare-feu partiellement coupé".into(), format!("Profil désactivé : {}.", off.join(", ")), None);
    }

    if scan.memory.total_bytes > 0.0 {
        let used = 1.0 - scan.memory.free_bytes / scan.memory.total_bytes;
        if used >= 0.9 {
            push("memory", Level::Warn, format!("Mémoire utilisée à {} %", (used * 100.0).round()), "Fermez ce qui ne sert pas pour éviter les ralentissements.".into(), Some(("performances", "Voir les programmes")));
        }
    }

    if let Some(boot) = scan.boots.items.first().filter(|b| b.ms > 60000.0) {
        push("boot", Level::Warn, format!("Démarrage en {} s", (boot.ms / 1000.0).round()), "Réduisez les programmes lancés au démarrage.".into(), Some(("demarrage", "Voir le démarrage")));
    }

    let rank = |level: Level| match level {
        Level::Alert => 0,
        Level::Warn => 1,
        Level::Ok => 2,
    };
    list.sort_by_key(|f| rank(f.level));
    list
}

pub fn headline(items: &[Finding]) -> String {
    match items.iter().filter(|f| f.level != Level::Ok).count() {
        0 => "Rien à signaler sur ce PC.".into(),
        1 => "Un point à traiter sur ce PC.".into(),
        n => format!("{n} points à traiter sur ce PC."),
    }
}

pub fn report_text(scan: &Scan, items: &[Finding]) -> String {
    let mut lines = vec![
        format!("Nova PC Suite, bilan du {}", format_datetime(&scan.timestamp)),
        format!("{} {}", scan.computer.manufacturer, scan.computer.model),
        format!("{} {} (build {})", scan.os.name, scan.os.release, scan.os.build),
        format!("{}, {} de mémoire", scan.cpu.name, format_bytes(scan.memory.total_bytes)),
    ];
    lines.extend(scan.gpus.iter().map(|g| format!("{}, pilote {}", g.name, g.driver_version)));
    lines.push(String::new());
    lines.extend(items.iter().map(|f| {
        let tag = match f.level {
            Level::Ok => "OK",
            Level::Warn => "À voir",
            Level::Alert => "Urgent",
        };
        format!("[{tag}] {}. {}", f.title, f.detail)
    }));
    lines.join("\n")
}

pub fn machine_facts(scan: &Scan) -> Vec<(String, String)> {
    let mut facts = vec![
        ("Système".to_string(), format!("{} {}, build {}", scan.os.name.replace("Microsoft ", ""), scan.os.release, scan.os.build)),
        ("Carte mère".to_string(), format!("{}, BIOS {}", scan.computer.board, scan.computer.bios)),
        ("Processeur".to_string(), format!("{}, {} cœurs, {} threads", scan.cpu.name, scan.cpu.cores, scan.cpu.threads)),
        ("Mémoire".to_string(), format!(
            "{}, {}{}",
            format_bytes(scan.memory.total_bytes),
            plural(scan.memory.modules as u64, "barrette", "barrettes"),
            scan.memory.speed_mhz.filter(|s| *s > 0).map(|s| format!(" à {s} MT/s")).unwrap_or_default()
        )),
    ];
    for g in &scan.gpus {
        let date = g.driver_date.as_deref().map(|d| format!(" du {}", format_date(Some(d)))).unwrap_or_default();
        facts.push(("Graphique".to_string(), format!("{}, pilote {}{}", g.name, g.driver_version, date)));
    }
    for d in &scan.disks {
        let heat = d.temperature.filter(|t| *t > 0.0).map(|t| format!(", {t} °C")).unwrap_or_default();
        facts.push(("Disque".to_string(), format!("{}, {}, {} {}{}", d.name, format_bytes(d.size_bytes), d.bus, d.media, heat)));
    }
    facts.push(("Allumé depuis".to_string(), format_duration(scan.uptime_seconds)));
    facts
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ScanMemory {
    pub timestamp: String,
    pub ids: Vec<String>,
    pub fresh: Vec<String>,
    pub solved: usize,
}

pub fn compare_findings(previous: Option<&[String]>, items: &[Finding]) -> (Vec<String>, usize) {
    let now: Vec<String> = items.iter().filter(|f| f.level != Level::Ok).map(|f| f.id.to_string()).collect();
    let Some(previous) = previous else {
        return (Vec::new(), 0);
    };
    let fresh = now.iter().filter(|id| !previous.contains(id)).cloned().collect();
    let solved = previous.iter().filter(|id| !now.contains(id)).count();
    (fresh, solved)
}

pub fn remember(saved: Option<ScanMemory>, timestamp: &str, items: &[Finding]) -> ScanMemory {
    if let Some(memory) = saved.as_ref().filter(|m| m.timestamp == timestamp) {
        return memory.clone();
    }
    let (fresh, solved) = compare_findings(saved.as_ref().map(|m| m.ids.as_slice()), items);
    ScanMemory {
        timestamp: timestamp.to_string(),
        ids: items.iter().filter(|f| f.level != Level::Ok).map(|f| f.id.to_string()).collect(),
        fresh,
        solved,
    }
}

pub fn by_size_then_usable<T>(list: &mut [T], facts: impl Fn(&T) -> (f64, bool, bool)) {
    let rank = |item: &T| {
        let (bytes, needs_admin, blocked) = facts(item);
        (needs_admin || blocked || bytes == 0.0, bytes)
    };
    list.sort_by(|a, b| {
        let (ra, ba) = rank(a);
        let (rb, bb) = rank(b);
        ra.cmp(&rb).then(bb.total_cmp(&ba))
    });
}

pub fn boot_comparison(boots: &[Boot], changes: &[i64]) -> Option<(f64, f64)> {
    let change = *changes.iter().max()? * 1000;
    let stamp = |b: &Boot| parse_moment(&b.date).map(|d| d.timestamp_millis());
    let average = |list: Vec<f64>| list.iter().sum::<f64>() / list.len() as f64;
    let before: Vec<f64> = boots.iter().filter(|b| stamp(b).is_some_and(|t| t < change)).map(|b| b.ms).collect();
    let after: Vec<f64> = boots.iter().filter(|b| stamp(b).is_some_and(|t| t > change)).map(|b| b.ms).collect();
    if before.is_empty() || after.is_empty() {
        return None;
    }
    Some((average(before), average(after)))
}

fn undo_target(entry: &Value) -> String {
    let undo = &entry["undo"];
    format!(
        "{}|{}|{}|{}",
        undo["module"], undo["action"], undo["payload"]["source"], undo["payload"]["name"]
    )
}

pub fn still_undoable(entries: &[Value], index: usize, now_seconds: i64) -> bool {
    let entry = &entries[index];
    if entry["ok"] != true || !entry["undo"].is_object() {
        return false;
    }
    if entry["undo"]["action"] == "reboot-cancel" && now_seconds - entry["timestamp"].as_i64().unwrap_or(0) > 60 {
        return false;
    }
    let target = undo_target(entry);
    !entries[..index].iter().any(|e| e["ok"] == true && e["undo"].is_object() && undo_target(e) == target)
}

pub fn invert_driver_filter(manufacturer: &str) -> bool {
    let lower = manufacturer.to_lowercase();
    ["microsoft", "(standard", "standard", "(générique", "generic"]
        .iter()
        .any(|p| lower.starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn healthy() -> Scan {
        serde_json::from_value(json!({
            "isAdmin": true,
            "timestamp": "2026-11-03T10:00:00Z",
            "os": {"name": "Windows 11", "release": "26H2", "build": "26300"},
            "computer": {"manufacturer": "X", "model": "Y", "board": "Z", "bios": "1"},
            "cpu": {"name": "CPU", "cores": 8, "threads": 16},
            "memory": {"totalBytes": 100, "freeBytes": 60, "modules": 2, "speedMHz": 6000},
            "gpus": [],
            "uptimeSeconds": 3600,
            "pendingReboot": false,
            "systemDrive": "C:",
            "volumes": [{"drive": "C:", "label": "", "sizeBytes": 1000, "freeBytes": 500}],
            "disks": [{"name": "SSD", "media": "SSD", "bus": "NVMe", "sizeBytes": 1000, "health": "Healthy", "temperature": 40, "wear": 2, "errors": 0}],
            "deviceProblems": 0,
            "security": {"defender": {"running": true, "realtime": true, "signatureAgeDays": 0}, "otherAntivirus": [], "firewallOff": []},
            "stability": {"unexpectedShutdowns": 0, "blueScreens": 0, "gpuHangs": 0, "gpuCodes": [], "appCrashes": 0, "topCrashes": [], "lastIncident": null},
            "lastUpdate": Utc::now().to_rfc3339(),
            "boots": {"available": true, "items": [{"date": "2026-11-03T09:00:00Z", "ms": 20000}]},
        }))
        .unwrap()
    }

    fn issues(scan: &Scan) -> Vec<Finding> {
        findings(scan).into_iter().filter(|f| f.level != Level::Ok).collect()
    }

    fn finding(id: &'static str, level: Level) -> Finding {
        Finding { id, level, title: id.into(), detail: String::new(), page: None, cta: None }
    }

    #[test]
    fn the_real_native_scan_reads_as_a_scan() {
        let scan: Scan = serde_json::from_value(crate::diag::scan().unwrap()).unwrap();
        assert!(scan.memory.total_bytes > 0.0);
        assert!(!findings(&scan).is_empty());
        assert!(!machine_facts(&scan).is_empty());
    }

    #[test]
    fn always_returns_a_list() {
        assert!(arr(Value::Null).is_empty());
        assert_eq!(arr(json!({"a": 1})), vec![json!({"a": 1})]);
        assert_eq!(arr(json!([1, 2])).len(), 2);
    }

    #[test]
    fn a_single_object_from_powershell_still_reads_as_a_list() {
        let boots: Boots = serde_json::from_value(json!({"available": true, "items": {"date": "x", "ms": 5}})).unwrap();
        assert_eq!(boots.items.len(), 1);
    }

    #[test]
    fn compares_two_scans_without_inventing_news_on_the_first() {
        assert_eq!(compare_findings(None, &[finding("a", Level::Alert)]), (vec![], 0));
        let previous = vec!["a".to_string(), "b".to_string()];
        let now = [finding("a", Level::Alert), finding("c", Level::Warn), finding("d", Level::Ok)];
        assert_eq!(compare_findings(Some(&previous), &now), (vec!["c".to_string()], 1));
    }

    #[test]
    fn memory_keeps_the_comparison_for_the_same_scan() {
        let first = remember(None, "t1", &[finding("a", Level::Alert)]);
        assert!(first.fresh.is_empty());
        let second = remember(Some(first.clone()), "t2", &[finding("b", Level::Warn)]);
        assert_eq!(second.fresh, ["b"]);
        assert_eq!(second.solved, 1);
        assert_eq!(remember(Some(second.clone()), "t2", &[]), second);
    }

    #[test]
    fn biggest_cleanups_first_and_unavailable_last() {
        let mut rows = vec![(5.0, false), (100.0, true), (0.0, false), (50.0, false)];
        by_size_then_usable(&mut rows, |r| (r.0, r.1, false));
        assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), [50.0, 5.0, 100.0, 0.0]);
    }

    #[test]
    fn journal_day_is_in_french() {
        assert!(journal_day(Utc.with_ymd_and_hms(2026, 11, 4, 12, 0, 0).unwrap().timestamp()).contains("novembre"));
        assert!(journal_day(Utc.with_ymd_and_hms(2026, 11, 4, 12, 0, 0).unwrap().timestamp()).starts_with("mercredi 4"));
    }

    #[test]
    fn reports_nothing_on_a_healthy_pc() {
        let scan = healthy();
        assert!(issues(&scan).is_empty());
        assert_eq!(headline(&findings(&scan)), "Rien à signaler sur ce PC.");
    }

    #[test]
    fn raises_space_alerts_by_remaining_ratio() {
        let mut low = healthy();
        low.volumes[0].free_bytes = 50.0;
        let first = &issues(&low)[0];
        assert_eq!((first.id, first.level, first.page), ("space", Level::Alert, Some("nettoyage")));
        low.volumes[0].free_bytes = 150.0;
        let first = &issues(&low)[0];
        assert_eq!((first.id, first.level), ("space", Level::Warn));
    }

    #[test]
    fn names_the_frequent_gpu_code_and_sorts_alerts_first() {
        let mut scan = healthy();
        scan.pending_reboot = true;
        scan.stability.gpu_hangs = 6;
        scan.stability.gpu_codes = vec![CodeCount { code: "1b8".into(), count: 6 }];
        let list = issues(&scan);
        assert_eq!(list[0].id, "gpu");
        assert!(list[0].detail.contains("écran noir"));
        assert_eq!(list[1].id, "reboot");
        assert_eq!(headline(&findings(&scan)), "2 points à traiter sur ce PC.");
    }

    #[test]
    fn no_defender_warning_when_another_antivirus_runs() {
        let mut scan = healthy();
        scan.security = Security {
            defender: Some(Defender { running: true, realtime: false, signature_age_days: 90 }),
            other_antivirus: vec!["Malwarebytes".into()],
            firewall_off: Some(vec![]),
        };
        assert!(issues(&scan).is_empty());
        scan.security = Security { defender: None, other_antivirus: vec![], firewall_off: Some(vec![]) };
        let first = &issues(&scan)[0];
        assert_eq!((first.id, first.level), ("av", Level::Alert));
    }

    #[test]
    fn invents_no_fact_when_the_disk_list_is_empty() {
        let mut scan = healthy();
        scan.disks.clear();
        assert!(findings(&scan).iter().all(|f| f.id != "disk"));
    }

    #[test]
    fn formats_bytes_in_french() {
        assert_eq!(format_bytes(1536.0 * 1024.0 * 1024.0), "1,5 Go");
        assert_eq!(format_bytes(2048.0 * 1024.0 * 1024.0), "2 Go");
        assert_eq!(format_bytes(1023.0 * 1024.0 * 1024.0), "1\u{202f}023 Mo");
        assert_eq!(format_bytes(f64::NAN), "inconnu");
    }

    #[test]
    fn computes_real_deltas_only() {
        assert_eq!(measure_delta(&json!(10), &json!(7), "count").unwrap(), "−3");
        assert_eq!(measure_delta(&json!(5e9), &json!(5e9), "bytes").unwrap(), "sans changement");
        assert!(measure_delta(&json!("a"), &json!("b"), "text").is_none());
        assert_eq!(format_measure_value(&Value::Null, "bytes"), "inconnu");
    }

    #[test]
    fn compares_boots_around_the_last_change_only_when_both_sides_exist() {
        let boots = vec![
            Boot { date: "2026-11-03T10:00:00Z".into(), ms: 20000.0 },
            Boot { date: "2026-11-01T10:00:00Z".into(), ms: 40000.0 },
        ];
        let change = [Utc.with_ymd_and_hms(2026, 11, 2, 10, 0, 0).unwrap().timestamp()];
        assert_eq!(boot_comparison(&boots, &change), Some((40000.0, 20000.0)));
        assert_eq!(boot_comparison(&boots, &[]), None);
        assert_eq!(boot_comparison(&boots[1..], &change), None);
    }

    #[test]
    fn dates_read_in_french() {
        assert_eq!(format_date(Some("2026-11-04")), "4 nov. 2026");
        assert_eq!(format_date(None), "inconnue");
        assert_eq!(format_date_no_year("2026-02-01"), "1 févr.");
    }

    #[test]
    fn durations_read_in_french() {
        assert_eq!(format_duration(90061.0), "1 j 1 h");
        assert_eq!(format_duration(3660.0), "1 h 1 min");
        assert_eq!(format_duration(120.0), "2 min");
    }

    #[test]
    fn only_the_newest_change_on_a_target_can_be_undone() {
        let undo = json!({"module": "startup", "action": "set", "payload": {"source": "s", "name": "n"}});
        let entries = vec![
            json!({"ok": true, "timestamp": 200, "undo": undo}),
            json!({"ok": true, "timestamp": 100, "undo": undo}),
            json!({"ok": false, "timestamp": 90, "undo": undo}),
        ];
        assert!(still_undoable(&entries, 0, 300));
        assert!(!still_undoable(&entries, 1, 300));
        assert!(!still_undoable(&entries, 2, 300));
        let reboot = vec![json!({"ok": true, "timestamp": 100, "undo": {"module": "updates", "action": "reboot-cancel"}})];
        assert!(still_undoable(&reboot, 0, 150));
        assert!(!still_undoable(&reboot, 0, 200));
    }
}
