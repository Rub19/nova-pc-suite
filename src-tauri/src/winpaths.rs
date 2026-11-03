use std::ffi::c_void;
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
