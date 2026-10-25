use std::{
    ffi::{c_void, OsString},
    os::windows::ffi::OsStringExt,
    path::PathBuf,
};
use std::{
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::Path,
};
#[link(name = "kernel32")]
extern "system" {
    fn GetFileInformationByHandle(handle: *mut c_void, info: *mut c_void) -> i32;
}
pub fn write_owned(path: &Path, bytes: &[u8], append: bool) -> Result<(), String> {
    let parent = path.parent().ok_or("Dossier manquant")?;
    let _held = super::hold_data_root(parent)?;
    let mut file: File = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .share_mode(0)
        .custom_flags(0x00200000)
        .open(path)
        .map_err(|_| "Fichier Nova inaccessible")?;
    let mut info = [0u32; 13];
    if file
        .metadata()
        .map_err(|_| "Métadonnées indisponibles")?
        .file_attributes()
        & 0x400
        != 0
        || unsafe {
            GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr() as *mut c_void)
        } == 0
        || info[10] != 1
    {
        return Err("Lien ou fichier non autorisé".into());
    }
    if append {
        file.seek(SeekFrom::End(0))
            .map_err(|_| "Écriture impossible")?;
    } else {
        file.set_len(0).map_err(|_| "Écriture impossible")?;
    }
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "Écriture impossible".into())
}
#[repr(C)]
struct Guid {
    a: u32,
    b: u16,
    c: u16,
    d: [u8; 8],
}
#[link(name = "shell32")]
extern "system" {
    fn SHGetKnownFolderPath(
        id: *const Guid,
        flags: u32,
        token: *mut c_void,
        path: *mut *mut u16,
    ) -> i32;
}
#[link(name = "ole32")]
extern "system" {
    fn CoTaskMemFree(pointer: *mut c_void);
}
pub fn local_data() -> Result<PathBuf, String> {
    let id = Guid {
        a: 0xF1B32785,
        b: 0x6FBA,
        c: 0x4FCF,
        d: [0x9D, 0x55, 0x7B, 0x8E, 0x7F, 0x15, 0x70, 0x91],
    };
    let mut pointer = std::ptr::null_mut();
    if unsafe { SHGetKnownFolderPath(&id, 0, std::ptr::null_mut(), &mut pointer) } < 0
        || pointer.is_null()
    {
        return Err("Dossier utilisateur Windows indisponible".into());
    }
    let path = unsafe {
        let mut len = 0;
        while *pointer.add(len) != 0 {
            len += 1;
            if len > 32767 {
                CoTaskMemFree(pointer as *mut c_void);
                return Err("Chemin utilisateur invalide".into());
            }
        }
        let value = OsString::from_wide(std::slice::from_raw_parts(pointer, len));
        CoTaskMemFree(pointer as *mut c_void);
        PathBuf::from(value)
    };
    if !path.is_absolute() {
        return Err("Dossier utilisateur Windows invalide".into());
    }
    Ok(path)
}
