//! Restricted, recoverable cleanup. Never traverses user directories recursively.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    ffi::c_void,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
    time::SystemTime,
};

#[repr(C)]
struct Blob {
    size: u32,
    bytes: *mut u8,
}
#[repr(C)]
#[derive(Default)]
struct FileInfo {
    attrs: u32,
    created: [u32; 2],
    accessed: [u32; 2],
    written: [u32; 2],
    volume: u32,
    size_high: u32,
    size_low: u32,
    links: u32,
    index_high: u32,
    index_low: u32,
}
#[link(name = "crypt32")]
extern "system" {
    fn CryptProtectData(
        input: *const Blob,
        description: *const u16,
        entropy: *const Blob,
        reserved: *mut c_void,
        prompt: *mut c_void,
        flags: u32,
        output: *mut Blob,
    ) -> i32;
    fn CryptUnprotectData(
        input: *const Blob,
        description: *mut *mut u16,
        entropy: *const Blob,
        reserved: *mut c_void,
        prompt: *mut c_void,
        flags: u32,
        output: *mut Blob,
    ) -> i32;
}
#[link(name = "kernel32")]
extern "system" {
    fn LocalFree(pointer: *mut c_void) -> *mut c_void;
    fn GetFileInformationByHandle(handle: *mut c_void, info: *mut c_void) -> i32;
    fn SetFileInformationByHandle(
        handle: *mut c_void,
        class: i32,
        info: *const c_void,
        size: u32,
    ) -> i32;
}
#[link(name = "bcrypt")]
extern "system" {
    fn BCryptGenRandom(provider: *mut c_void, buffer: *mut u8, len: u32, flags: u32) -> i32;
}

#[derive(Clone, Serialize, Deserialize)]
struct Item {
    id: String,
    name: String,
    bytes: u64,
    modified: u64,
    volume: u32,
    index: u64,
    age_days: u64,
}
#[derive(Serialize, Deserialize)]
struct Manifest {
    version: u32,
    root: PathBuf,
    items: Vec<Item>,
}
pub(crate) fn uid() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    if unsafe { BCryptGenRandom(std::ptr::null_mut(), bytes.as_mut_ptr(), 16, 2) } != 0 {
        return Err("Identifiant sécurisé indisponible".into());
    }
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}
fn leaf(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && !name.contains(['/', '\\', ':', '\0'])
        && name != "."
        && name != ".."
        && !name.ends_with(['.', ' '])
}
fn crypt(bytes: &[u8], protect: bool) -> Result<Vec<u8>, String> {
    if bytes.len() > 1_000_000 {
        return Err("Manifeste trop volumineux".into());
    }
    let input = Blob {
        size: bytes.len() as u32,
        bytes: bytes.as_ptr() as *mut u8,
    };
    let mut output = Blob {
        size: 0,
        bytes: std::ptr::null_mut(),
    };
    let ok = unsafe {
        if protect {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut output,
            )
        }
    };
    if ok == 0 {
        return Err("Manifeste de nettoyage invalide ou appartenant à un autre utilisateur".into());
    }
    let result = unsafe { std::slice::from_raw_parts(output.bytes, output.size as usize).to_vec() };
    unsafe { LocalFree(output.bytes as *mut c_void) };
    Ok(result)
}
fn temp_root() -> Result<PathBuf, String> {
    let root = super::local_data_root()?.join("Temp");
    let _guards = super::hold_data_root(&root)?;
    Ok(root)
}
fn storage(data: &Path) -> Result<PathBuf, String> {
    let path = data.join("cleanup-quarantine");
    let _held = super::hold_data_root(&path)?;
    Ok(path)
}
fn open_checked(path: &Path, deleting: bool) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options
        .access_mode(0x80000000 | if deleting { 0x00010000 } else { 0 })
        .share_mode(1)
        .custom_flags(0x00200000);
    let file = options
        .open(path)
        .map_err(|_| "Fichier occupé ou inaccessible")?;
    let metadata = file.metadata().map_err(|_| "Métadonnées indisponibles")?;
    let mut info = FileInfo::default();
    if !metadata.is_file()
        || metadata.file_attributes() & 0x400 != 0
        || unsafe {
            GetFileInformationByHandle(file.as_raw_handle(), &mut info as *mut _ as *mut c_void)
        } == 0
        || info.links != 1
    {
        return Err("Lien, dossier ou fichier non pris en charge".into());
    }
    Ok(file)
}
fn fingerprint(file: &File) -> Result<(u64, u64, u32, u64), String> {
    let m = file.metadata().map_err(|_| "Métadonnées indisponibles")?;
    let mut info = FileInfo::default();
    if unsafe {
        GetFileInformationByHandle(file.as_raw_handle(), &mut info as *mut _ as *mut c_void)
    } == 0
    {
        return Err("Identité du fichier indisponible".into());
    }
    Ok((
        m.len(),
        m.last_write_time(),
        info.volume,
        ((info.index_high as u64) << 32) | info.index_low as u64,
    ))
}
fn same(file: &File, item: &Item) -> bool {
    fingerprint(file)
        .map(|x| x == (item.bytes, item.modified, item.volume, item.index))
        .unwrap_or(false)
}
fn rename_handle(file: &File, destination: &Path) -> Result<(), String> {
    let wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // FILE_RENAME_INFO x64: flags at 0, root at 8, byte length at 16, filename at 20.
    let size = 20 + wide.len() * 2;
    let mut aligned = vec![0u64; size.div_ceil(8)];
    unsafe {
        let ptr = aligned.as_mut_ptr() as *mut u8;
        std::ptr::write_unaligned(ptr.add(16) as *mut u32, ((wide.len() - 1) * 2) as u32);
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr.add(20) as *mut u16, wide.len());
        if SetFileInformationByHandle(file.as_raw_handle(), 3, ptr as *const c_void, size as u32)
            == 0
        {
            return Err(
                "Déplacement refusé : destination existante, fichier occupé ou autre volume".into(),
            );
        }
    }
    Ok(())
}
fn save_new(path: &Path, m: &Manifest) -> Result<(), String> {
    let bytes = crypt(
        &serde_json::to_vec(m).map_err(|_| "Manifeste invalide")?,
        true,
    )?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .share_mode(0)
        .open(path)
        .map_err(|_| "Création du manifeste impossible")?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "Sauvegarde du manifeste impossible".into())
}
fn load(path: &Path, root: &Path) -> Result<Manifest, String> {
    let mut file = open_checked(path, false)?;
    if file.metadata().map_err(|_| "Manifeste inaccessible")?.len() > 1_000_000 {
        return Err("Manifeste trop volumineux".into());
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| "Manifeste illisible")?;
    let m: Manifest =
        serde_json::from_slice(&crypt(&bytes, false)?).map_err(|_| "Manifeste invalide")?;
    let mut ids = HashSet::new();
    if m.version != 1
        || m.root != root
        || m.items.len() > 1000
        || m.items
            .iter()
            .any(|i| !valid_id(&i.id) || !leaf(&i.name) || !ids.insert(i.id.clone()))
    {
        return Err("Manifeste invalide".into());
    }
    Ok(m)
}
pub fn preview(data: &Path) -> Result<Value, String> {
    preview_at(data, &temp_root()?, 7 * 86400)
}
fn preview_at(data: &Path, root: &Path, min_age: u64) -> Result<Value, String> {
    let _held = super::hold_data_root(root)?;
    let store = storage(data)?;
    let _store = super::hold_data_root(&store)?;
    let mut items = Vec::new();
    let mut examined = 0;
    for entry in fs::read_dir(root).map_err(|_| "Temporaires indisponibles")? {
        examined += 1;
        if examined > 10_000 || items.len() >= 1000 {
            break;
        }
        let Ok(entry) = entry else { continue };
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !leaf(&name) {
            continue;
        }
        let Ok(file) = open_checked(&entry.path(), false) else {
            continue;
        };
        let m = file.metadata().map_err(|_| "Métadonnées indisponibles")?;
        let age = SystemTime::now()
            .duration_since(m.modified().unwrap_or(SystemTime::now()))
            .unwrap_or_default()
            .as_secs();
        if age < min_age {
            continue;
        }
        let (bytes, modified, volume, index) = fingerprint(&file)?;
        items.push(Item {
            id: uid()?,
            name,
            bytes,
            modified,
            volume,
            index,
            age_days: age / 86400,
        });
    }
    let id = uid()?;
    let manifest = Manifest {
        version: 1,
        root: root.to_path_buf(),
        items,
    };
    save_new(&store.join(format!("preview-{id}.dat")), &manifest)?;
    // Fixed pointer contains only a validated generated filename. Old previews remain harmless encrypted manifests.
    let pointer = store.join("current-preview");
    let mut handle = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .truncate(false)
        .share_mode(0)
        .custom_flags(0x00200000)
        .open(&pointer)
        .map_err(|_| "Référence d’analyse inaccessible")?;
    let mut info = FileInfo::default();
    if handle
        .metadata()
        .map_err(|_| "Référence invalide")?
        .file_attributes()
        & 0x400
        != 0
        || unsafe {
            GetFileInformationByHandle(handle.as_raw_handle(), &mut info as *mut _ as *mut c_void)
        } == 0
        || info.links != 1
    {
        return Err("Référence redirigée : opération refusée".into());
    }
    handle
        .set_len(0)
        .and_then(|_| handle.write_all(id.as_bytes()))
        .and_then(|_| handle.sync_all())
        .map_err(|_| "Référence d’analyse impossible")?;
    Ok(
        json!({"ok":true,"data":{"items":manifest.items.iter().map(|i|json!({"id":i.id,"name":i.name,"bytes":i.bytes,"ageDays":i.age_days})).collect::<Vec<_>>(),"totalBytes":manifest.items.iter().map(|i|i.bytes).sum::<u64>(),"scope":"Fichiers uniquement à la racine des temporaires utilisateur, anciens de 7 jours minimum","limit":1000,"freedBytes":0}}),
    )
}
pub fn quarantine(data: &Path, ids: Vec<String>) -> Result<Value, String> {
    quarantine_at(data, &temp_root()?, ids)
}
pub fn delete_selected(data: &Path, ids: Vec<String>) -> Result<Value, String> {
    delete_selected_at(data, &temp_root()?, ids)
}
fn delete_selected_at(data: &Path, root: &Path, ids: Vec<String>) -> Result<Value, String> {
    let moved = quarantine_at(data, root, ids)?;
    let batch = moved["data"]["batchId"]
        .as_str()
        .ok_or("Lot indisponible")?;
    // A persisted recovery manifest exists before any deletion is requested.
    let purged = finish(data, root, batch, true).unwrap_or_else(
        |error| json!({"ok":false,"data":{"freedBytes":0,"results":[]},"error":error}),
    );
    let mut results = Vec::new();
    for item in moved["data"]["results"]
        .as_array()
        .ok_or("Résultats indisponibles")?
    {
        if item["ok"] != true {
            results.push(item.clone());
            continue;
        }
        let deleted = purged["data"]["results"]
            .as_array()
            .and_then(|items| items.iter().find(|row| row["name"] == item["name"]));
        results.push(deleted.cloned().unwrap_or_else(|| json!({"name":item["name"],"ok":false,"error":"Fichier conservé en quarantaine ; consultez ce lot pour le restaurer"})));
    }
    let ok = results.iter().all(|row| row["ok"] == true);
    let deleted_count = results.iter().filter(|row| row["ok"] == true).count();
    Ok(
        json!({"ok":ok,"data":{"batchId":batch,"freedBytes":purged["data"]["freedBytes"],"logicalBytes":true,"deletedCount":deleted_count,"preservedCount":results.len()-deleted_count,"results":results},"error":if ok {Value::Null} else {json!("Certains temporaires ont été conservés ; consultez les résultats et les lots récupérables")}}),
    )
}
pub fn automatic(data: &Path) -> Result<Value, String> {
    automatic_at(data, &temp_root()?, 7 * 86400)
}
fn automatic_at(data: &Path, root: &Path, min_age: u64) -> Result<Value, String> {
    let preview = preview_at(data, root, min_age)?;
    let ids: Vec<String> = preview["data"]["items"]
        .as_array()
        .ok_or("Analyse indisponible")?
        .iter()
        .filter_map(|row| row["id"].as_str().map(str::to_owned))
        .collect();
    if ids.is_empty() {
        return Ok(
            json!({"ok":true,"data":{"deletedCount":0,"preservedCount":0,"freedBytes":0,"logicalBytes":true,"results":[]}}),
        );
    }
    delete_selected_at(data, root, ids)
}
fn quarantine_at(data: &Path, root: &Path, ids: Vec<String>) -> Result<Value, String> {
    let _root = super::hold_data_root(root)?;
    let store = storage(data)?;
    let _store = super::hold_data_root(&store)?;
    if ids.is_empty() || ids.len() > 1000 || ids.iter().any(|s| !valid_id(s)) {
        return Err("Sélection invalide".into());
    }
    let mut pointer_file = open_checked(&store.join("current-preview"), false)?;
    if pointer_file
        .metadata()
        .map_err(|_| "Référence invalide")?
        .len()
        != 32
    {
        return Err("Référence invalide".into());
    }
    let mut pointer = String::new();
    pointer_file
        .read_to_string(&mut pointer)
        .map_err(|_| "Relancez l’analyse")?;
    if !valid_id(&pointer) {
        return Err("Référence d’analyse invalide".into());
    }
    let manifest = load(&store.join(format!("preview-{pointer}.dat")), root)?;
    let approved: HashSet<_> = ids.into_iter().collect();
    if approved
        .iter()
        .any(|id| !manifest.items.iter().any(|i| &i.id == id))
    {
        return Err("Sélection inconnue".into());
    }
    let selected: Vec<_> = manifest
        .items
        .into_iter()
        .filter(|i| approved.contains(&i.id))
        .collect();
    let batch = uid()?;
    let dir = store.join(&batch);
    let _batch = super::hold_data_root(&dir)?;
    save_new(
        &dir.join("manifest.dat"),
        &Manifest {
            version: 1,
            root: root.to_owned(),
            items: selected.clone(),
        },
    )?;
    let mut results = Vec::new();
    let mut moved = 0;
    for item in &selected {
        let result = open_checked(&root.join(&item.name), true).and_then(|f| {
            if !same(&f, item) {
                return Err("Le fichier a changé depuis l’analyse".into());
            }
            rename_handle(&f, &dir.join(format!("{}.bin", item.id)))
        });
        if result.is_ok() {
            moved += item.bytes;
        }
        results.push(json!({"name":item.name,"ok":result.is_ok(),"error":result.err()}));
    }
    Ok(
        json!({"ok":results.iter().all(|r|r["ok"]==true),"data":{"batchId":batch,"movedBytes":moved,"freedBytes":0,"results":results},"error":if results.iter().all(|r|r["ok"]==true){Value::Null}else{json!("Certains fichiers occupés ou modifiés ont été conservés")}}),
    )
}
pub fn list_batches(data: &Path) -> Result<Value, String> {
    let store = storage(data)?;
    let _held = super::hold_data_root(&store)?;
    let root = temp_root()?;
    let mut batches = Vec::new();
    for entry in fs::read_dir(&store)
        .map_err(|_| "Lots indisponibles")?
        .take(1000)
    {
        let Ok(entry) = entry else { continue };
        let Some(id) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !valid_id(&id) {
            continue;
        }
        let Ok(_batch) = super::hold_data_root(&entry.path()) else {
            continue;
        };
        let Ok(m) = load(&entry.path().join("manifest.dat"), &root) else {
            continue;
        };
        let active: Vec<_> = m
            .items
            .iter()
            .filter(|i| {
                open_checked(&entry.path().join(format!("{}.bin", i.id)), false)
                    .map(|f| same(&f, i))
                    .unwrap_or(false)
            })
            .collect();
        if !active.is_empty() {
            batches.push(json!({"id":id,"totalBytes":active.iter().map(|i|i.bytes).sum::<u64>(),"count":active.len(),"freedBytes":0}));
        }
    }
    Ok(json!({"ok":true,"data":{"batches":batches}}))
}
pub fn restore(data: &Path, batch: String) -> Result<Value, String> {
    finish(data, &temp_root()?, &batch, false)
}
pub fn purge(data: &Path, batch: String) -> Result<Value, String> {
    finish(data, &temp_root()?, &batch, true)
}
fn finish(data: &Path, root: &Path, batch: &str, purge: bool) -> Result<Value, String> {
    if !valid_id(batch) {
        return Err("Lot invalide".into());
    }
    let store = storage(data)?;
    let dir = store.join(batch);
    if !dir.is_dir() {
        return Err("Lot introuvable".into());
    }
    let _data = super::hold_data_root(&dir)?;
    let _root = super::hold_data_root(root)?;
    let m = load(&dir.join("manifest.dat"), root)?;
    let mut results = Vec::new();
    let mut freed = 0;
    for item in m.items {
        let path = dir.join(format!("{}.bin", item.id));
        if !path.exists() {
            continue;
        }
        let result = open_checked(&path, true).and_then(|f| {
            if !same(&f, &item) {
                return Err("Identité du fichier modifiée : opération refusée".into());
            }
            if purge {
                let delete: u8 = 1;
                if unsafe {
                    SetFileInformationByHandle(
                        f.as_raw_handle(),
                        4,
                        &delete as *const _ as *const c_void,
                        1,
                    )
                } == 0
                {
                    return Err("Suppression refusée".into());
                }
                Ok(())
            } else {
                rename_handle(&f, &root.join(&item.name))
            }
        });
        if purge && result.is_ok() {
            freed += item.bytes;
        }
        results.push(json!({"name":item.name,"ok":result.is_ok(),"error":result.err()}));
    }
    let ok = results.iter().all(|v| v["ok"] == true);
    Ok(
        json!({"ok":ok,"data":{"batchId":batch,"freedBytes":freed,"logicalBytes":true,"results":results},"error":if ok{Value::Null}else{json!("Certains fichiers ont été conservés ; consultez les résultats")}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (PathBuf, PathBuf, PathBuf) {
        let base = std::env::current_dir()
            .unwrap()
            .join("target/cleanup-tests")
            .join(uid().unwrap());
        let data = base.join("data");
        let temp = base.join("temp");
        fs::create_dir_all(&data).unwrap();
        fs::create_dir(&temp).unwrap();
        (base, data, temp)
    }
    #[test]
    fn direct_cleanup_only_deletes_selected_owned_unchanged_files() {
        let (_base, data, temp) = fixture();
        fs::write(temp.join("selected.tmp"), b"selected").unwrap();
        fs::write(temp.join("keep.tmp"), b"keep").unwrap();
        let preview = preview_at(&data, &temp, 0).unwrap();
        let id = preview["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["name"] == "selected.tmp")
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let result = delete_selected_at(&data, &temp, vec![id]).unwrap();
        assert_eq!(result["ok"], true);
        assert_eq!(result["data"]["freedBytes"], 8);
        assert!(!temp.join("selected.tmp").exists());
        assert_eq!(fs::read(temp.join("keep.tmp")).unwrap(), b"keep");
    }
    #[test]
    fn automatic_cleanup_skips_recent_files_and_handles_empty_sweeps() {
        let (_base, data, temp) = fixture();
        fs::write(temp.join("recent.tmp"), b"recent").unwrap();
        let skipped = automatic_at(&data, &temp, 7 * 86400).unwrap();
        assert_eq!(skipped["data"]["deletedCount"], 0);
        assert!(temp.join("recent.tmp").exists());
        let removed = automatic_at(&data, &temp, 0).unwrap();
        assert_eq!(removed["data"]["deletedCount"], 1);
        assert_eq!(removed["data"]["freedBytes"], 6);
        assert!(!temp.join("recent.tmp").exists());
        assert_eq!(
            automatic_at(&data, &temp, 0).unwrap()["data"]["deletedCount"],
            0
        );
    }
    #[test]
    fn direct_cleanup_preserves_changed_preview_files() {
        let (_base, data, temp) = fixture();
        fs::write(temp.join("changed.tmp"), b"old").unwrap();
        let preview = preview_at(&data, &temp, 0).unwrap();
        let id = preview["data"]["items"][0]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        fs::write(temp.join("changed.tmp"), b"new contents").unwrap();
        let result = delete_selected_at(&data, &temp, vec![id]).unwrap();
        assert_eq!(result["ok"], false);
        assert_eq!(result["data"]["preservedCount"], 1);
        assert_eq!(result["data"]["freedBytes"], 0);
        assert_eq!(fs::read(temp.join("changed.tmp")).unwrap(), b"new contents");
    }
    #[test]
    fn rejects_paths() {
        for s in ["../file", "x:y", "..", "a\\b", "x/../b", "a."] {
            assert!(!leaf(s));
        }
        assert!(!valid_id("../"));
    }
    #[test]
    fn protect_rejects_tamper() {
        let mut b = crypt(b"fixture", true).unwrap();
        let n = b.len() - 1;
        b[n] ^= 1;
        assert!(crypt(&b, false).is_err());
    }
    #[test]
    fn quarantine_restore_purge_are_owned_and_no_overwrite() {
        let (_base, data, temp) = fixture();
        fs::write(temp.join("sample.tmp"), b"test fixture").unwrap();
        let p = preview_at(&data, &temp, 0).unwrap();
        let id = p["data"]["items"][0]["id"].as_str().unwrap().to_owned();
        let q = quarantine_at(&data, &temp, vec![id]).unwrap();
        assert_eq!(q["ok"], true);
        assert_eq!(q["data"]["freedBytes"], 0);
        assert!(!temp.join("sample.tmp").exists());
        let batch = q["data"]["batchId"].as_str().unwrap();
        fs::write(temp.join("sample.tmp"), b"keep existing").unwrap();
        let conflict = finish(&data, &temp, batch, false).unwrap();
        assert_eq!(conflict["ok"], false);
        assert_eq!(fs::read(temp.join("sample.tmp")).unwrap(), b"keep existing");
        // Purge only the fixture quarantine; original conflict remains intact.
        let removed = finish(&data, &temp, batch, true).unwrap();
        assert_eq!(removed["data"]["freedBytes"], 12);
        assert_eq!(fs::read(temp.join("sample.tmp")).unwrap(), b"keep existing");
    }
    #[test]
    fn changed_preview_file_is_preserved() {
        let (_, data, temp) = fixture();
        fs::write(temp.join("a.tmp"), b"first").unwrap();
        let p = preview_at(&data, &temp, 0).unwrap();
        let id = p["data"]["items"][0]["id"].as_str().unwrap().to_owned();
        fs::write(temp.join("a.tmp"), b"changed longer").unwrap();
        let q = quarantine_at(&data, &temp, vec![id]).unwrap();
        assert_eq!(q["ok"], false);
        assert!(temp.join("a.tmp").is_file());
    }
    #[test]
    fn restore_preserves_bytes() {
        let (_, data, temp) = fixture();
        fs::write(temp.join("sample.tmp"), b"restore me").unwrap();
        let p = preview_at(&data, &temp, 0).unwrap();
        let id = p["data"]["items"][0]["id"].as_str().unwrap().to_owned();
        let q = quarantine_at(&data, &temp, vec![id]).unwrap();
        let r = finish(&data, &temp, q["data"]["batchId"].as_str().unwrap(), false).unwrap();
        assert_eq!(r["ok"], true);
        assert_eq!(fs::read(temp.join("sample.tmp")).unwrap(), b"restore me");
    }
    #[test]
    fn excludes_hardlinks() {
        let (_, data, temp) = fixture();
        fs::write(temp.join("a.tmp"), b"same").unwrap();
        fs::hard_link(temp.join("a.tmp"), temp.join("b.tmp")).unwrap();
        let p = preview_at(&data, &temp, 0).unwrap();
        assert_eq!(p["data"]["items"].as_array().unwrap().len(), 0);
    }
}
