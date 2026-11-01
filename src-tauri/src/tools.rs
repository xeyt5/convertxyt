//! Localiza yt-dlp, ffmpeg, ffprobe y Deno: los incluidos en la app o, en desarrollo, los del sistema.

use crate::job;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};

pub struct Tools {
    pub ytdlp: PathBuf,
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    /// Motor de JavaScript que yt-dlp necesita para YouTube; `None` si no va incluido.
    pub deno: Option<PathBuf>,
}

static TOOLS: OnceLock<Tools> = OnceLock::new();
// Una descarga espera a que termine `yt-dlp -U` para no ejecutar un binario a medio reemplazar.
static YTDLP_UPDATE: Mutex<()> = Mutex::new(());

fn executable(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

impl Tools {
    fn from_path() -> Self {
        Self {
            ytdlp: "yt-dlp".into(),
            ffmpeg: "ffmpeg".into(),
            ffprobe: "ffprobe".into(),
            deno: None,
        }
    }

    /// Usa los binarios de `dir`; el que falte se busca en el PATH.
    pub fn from_dir(dir: &Path) -> Self {
        let bundled = |name: &str| Some(dir.join(executable(name))).filter(|path| path.is_file());
        let or_path = |name: &str| bundled(name).unwrap_or_else(|| name.into());
        Self {
            ytdlp: or_path("yt-dlp"),
            ffmpeg: or_path("ffmpeg"),
            ffprobe: or_path("ffprobe"),
            deno: bundled("deno"),
        }
    }
}

pub fn get() -> &'static Tools {
    TOOLS.get_or_init(Tools::from_path)
}

/// Solo las versiones publicadas traen herramientas propias y pueden actualizarse solas.
pub fn is_packaged() -> bool {
    if cfg!(debug_assertions) {
        false
    } else if cfg!(target_os = "linux") {
        std::env::var_os("APPIMAGE").is_some()
    } else {
        cfg!(target_os = "windows")
    }
}

pub fn init(data_dir: Option<PathBuf>) {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let Some(exe_dir) = exe_dir.filter(|_| is_packaged()) else {
        return;
    };

    let mut tools = Tools::from_dir(&exe_dir);
    // La instalación es de solo lectura: yt-dlp se actualiza en una copia dentro de los datos de la app.
    if let Some(copy) = data_dir.and_then(|dir| writable_copy(&tools.ytdlp, &dir)) {
        tools.ytdlp = copy.clone();
        std::thread::spawn(move || update_ytdlp(&copy));
    }
    let _ = TOOLS.set(tools);
}

fn writable_copy(bundled: &Path, data_dir: &Path) -> Option<PathBuf> {
    let copy = data_dir.join("tools").join(executable("yt-dlp"));
    if !copy.is_file() {
        std::fs::create_dir_all(copy.parent()?).ok()?;
        std::fs::copy(bundled, &copy).ok()?;
    }
    Some(copy)
}

fn update_ytdlp(path: &Path) {
    let _lock = YTDLP_UPDATE.lock();
    // Sin conexión falla y se sigue usando la versión actual.
    let _ = job::command(path)
        .arg("-U")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

pub fn wait_for_ytdlp_update() {
    drop(YTDLP_UPDATE.lock());
}

#[cfg(test)]
pub fn set_for_test(dir: &Path) -> &'static Tools {
    TOOLS.get_or_init(|| Tools::from_dir(dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usa_el_binario_incluido_y_si_falta_el_del_path() {
        let dir = std::env::temp_dir().join(format!("cxyt-tools-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(executable("ffmpeg")), b"x").unwrap();
        std::fs::write(dir.join(executable("deno")), b"x").unwrap();

        let tools = Tools::from_dir(&dir);
        assert_eq!(tools.ffmpeg, dir.join(executable("ffmpeg")));
        assert_eq!(tools.deno, Some(dir.join(executable("deno"))));
        assert_eq!(tools.ytdlp, PathBuf::from("yt-dlp"));

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn la_copia_de_yt_dlp_no_pisa_una_ya_actualizada() {
        let dir = std::env::temp_dir().join(format!("cxyt-copy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bundled = dir.join("incluido");
        std::fs::write(&bundled, b"v1").unwrap();

        let copy = writable_copy(&bundled, &dir).unwrap();
        assert_eq!(std::fs::read(&copy).unwrap(), b"v1");

        std::fs::write(&copy, b"v2").unwrap();
        writable_copy(&bundled, &dir).unwrap();
        assert_eq!(std::fs::read(&copy).unwrap(), b"v2");

        std::fs::remove_dir_all(dir).unwrap();
    }
}
