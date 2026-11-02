//! Traduce las opciones de la interfaz a llamadas de yt-dlp y ffmpeg.

use crate::job::{self, JobManager, Outcome};
use crate::tools;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize)]
pub struct Progress {
    /// `None` cuando no se puede medir (la barra se muestra indeterminada).
    pub percent: Option<f64>,
    pub message: String,
}

impl Progress {
    fn new(percent: Option<f64>, message: impl Into<String>) -> Self {
        Self {
            percent,
            message: message.into(),
        }
    }
}

/// Ruta del archivo final, o `None` si el usuario canceló.
pub type JobResult = Result<Option<String>, String>;

const DOWNLOAD_PREFIX: &str = "[cxyt-dl]";
const POSTPROCESS_PREFIX: &str = "[cxyt-pp]";
const FILE_PREFIX: &str = "[cxyt-file]";

// H.264 con 4:2:0 exige dimensiones pares.
const EVEN_SIZE: &str = "scale=trunc(iw/2)*2:trunc(ih/2)*2";
const H264_AAC: &[&str] = &[
    "-vf", EVEN_SIZE, "-c:v", "libx264", "-preset", "medium", "-crf", "18", "-pix_fmt", "yuv420p",
    "-c:a", "aac", "-b:a", "192k",
];
const H264_NVENC_AAC: &[&str] = &[
    "-vf", EVEN_SIZE, "-c:v", "h264_nvenc", "-preset", "p5", "-tune", "hq", "-rc", "vbr", "-cq",
    "19", "-b:v", "0", "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "192k",
];
// Decodifica con NVDEC. Si la tarjeta no admite el códec de entrada, ffmpeg usa la CPU.
const GPU_DECODE: &[&str] = &["-hwaccel", "cuda"];
const FASTSTART: &[&str] = &["-movflags", "+faststart"];
// DaVinci Resolve gratis en Linux no lee H.264/AAC: necesita DNxHR y audio PCM.
const DAVINCI: &[&str] = &[
    "-c:v", "dnxhd", "-profile:v", "dnxhr_hq", "-pix_fmt", "yuv422p", "-c:a", "pcm_s16le",
];
const VP9_OPUS: &[&str] = &[
    "-c:v", "libvpx-vp9", "-crf", "32", "-b:v", "0", "-row-mt", "1", "-c:a", "libopus", "-b:a",
    "128k",
];

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn h264_aac(gpu: bool) -> &'static [&'static str] {
    if gpu {
        H264_NVENC_AAC
    } else {
        H264_AAC
    }
}

/// Intenta con la GPU si la hay. NVENC rechaza algunos videos (H.264 por encima de 4096 px, por
/// ejemplo), así que un fallo se repite en CPU; cancelar no es un fallo y no se repite.
fn prefer_gpu(gpu_available: bool, mut attempt: impl FnMut(bool) -> JobResult) -> JobResult {
    if gpu_available {
        attempt(true).or_else(|_| attempt(false))
    } else {
        attempt(false)
    }
}

fn validate_url(url: &str) -> Result<&str, String> {
    let url = url.trim();
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(url)
    } else {
        Err("La URL debe empezar por http:// o https://".to_string())
    }
}

fn validate_folder(folder: &str) -> Result<&Path, String> {
    let path = Path::new(folder);
    if folder.is_empty() {
        Err("Selecciona una carpeta de destino".to_string())
    } else if !path.is_dir() {
        Err(format!("La carpeta no existe: {folder}"))
    } else {
        Ok(path)
    }
}

fn validate_input(input: &str) -> Result<&Path, String> {
    let path = Path::new(input);
    if input.is_empty() {
        Err("Selecciona un archivo".to_string())
    } else if !path.is_absolute() || !path.is_file() {
        Err(format!("No se encontró el archivo: {input}"))
    } else {
        Ok(path)
    }
}

fn parse_height(quality: &str) -> Result<Option<u32>, String> {
    match quality {
        "best" => Ok(None),
        other => other
            .parse::<u32>()
            .map(Some)
            .map_err(|_| format!("Calidad no válida: {other}")),
    }
}

/// Argumentos de formato y recodificación para `download_video`.
fn video_args(quality: &str, profile: &str, gpu: bool) -> Result<Vec<String>, String> {
    let height = parse_height(quality)?;
    let selector = match height {
        Some(h) => format!("bv*[height<={h}]+ba/b[height<={h}]"),
        None => "bv*+ba/b".to_string(),
    };
    // mkv admite cualquier combinación de códecs, así nunca se pierde calidad al unir.
    let mut args = strings(&["-f", &selector, "--merge-output-format", "mkv"]);

    let recode = |args: &mut Vec<String>, container: &str, codec_args: Vec<&str>| {
        args.extend(strings(&["--recode-video", container, "--postprocessor-args"]));
        args.push(format!("VideoConvertor:{}", codec_args.join(" ")));
    };
    match profile {
        "best" => {}
        "davinci-linux" => recode(&mut args, "mov", DAVINCI.to_vec()),
        "youtube" => recode(&mut args, "mp4", [h264_aac(gpu), FASTSTART].concat()),
        "small" => {
            // Sin límite elegido, "menor peso" se queda en 720p.
            let target = height.unwrap_or(720);
            args.extend(strings(&["-S", &format!("res:{target},+size,+br")]));
        }
        other => return Err(format!("Perfil no válido: {other}")),
    }
    Ok(args)
}

fn check_audio_format(format: &str) -> Result<(), String> {
    match format {
        "mp3" | "m4a" | "opus" | "flac" | "wav" => Ok(()),
        other => Err(format!("Formato de audio no válido: {other}")),
    }
}

pub fn download_video(
    manager: &JobManager,
    url: &str,
    quality: &str,
    profile: &str,
    output_folder: &str,
    emit: &mut dyn FnMut(Progress),
) -> JobResult {
    // Solo este perfil recodifica a H.264; los demás no pasan por NVENC.
    let gpu_available = profile == "youtube" && tools::nvenc_available();
    prefer_gpu(gpu_available, |gpu| {
        let format_args = video_args(quality, profile, gpu)?;
        run_ytdlp(manager, url, output_folder, format_args, &mut *emit)
    })
}

pub fn download_audio(
    manager: &JobManager,
    url: &str,
    format: &str,
    output_folder: &str,
    emit: &mut dyn FnMut(Progress),
) -> JobResult {
    check_audio_format(format)?;
    let format_args = strings(&["-f", "ba/b", "-x", "--audio-format", format, "--audio-quality", "0"]);
    run_ytdlp(manager, url, output_folder, format_args, emit)
}

fn run_ytdlp(
    manager: &JobManager,
    url: &str,
    output_folder: &str,
    format_args: Vec<String>,
    emit: &mut dyn FnMut(Progress),
) -> JobResult {
    let url = validate_url(url)?;
    let folder = validate_folder(output_folder)?;
    // Los temporales van aparte para que cancelar no deje restos en la carpeta del usuario.
    let temp = folder.join(format!(".convertxyt-tmp-{}", unique_suffix()));

    tools::wait_for_ytdlp_update();
    let tools = tools::get();
    let mut command = job::command(&tools.ytdlp);
    command
        .env("PYTHONIOENCODING", "utf-8")
        .args(["--no-playlist", "--newline", "--no-warnings", "--progress"]);
    match &tools.deno {
        Some(deno) => command.arg("--js-runtimes").arg(format!("deno:{}", deno.display())),
        None => command.args(["--js-runtimes", "node"]),
    };
    if tools.ffmpeg.is_absolute() {
        command.arg("--ffmpeg-location").arg(&tools.ffmpeg);
    }
    command
        .args(["--print", &format!("after_move:{FILE_PREFIX}%(filepath)s")])
        .arg("--progress-template")
        .arg(format!(
            "download:{DOWNLOAD_PREFIX}%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress._speed_str)s|%(progress._eta_str)s"
        ))
        .arg("--progress-template")
        .arg(format!(
            "postprocess:{POSTPROCESS_PREFIX}%(progress.postprocessor)s|%(progress.status)s"
        ))
        .args(format_args)
        .arg("-P")
        .arg(format!("home:{}", folder.display()))
        .arg("-P")
        .arg(format!("temp:{}", temp.display()))
        .args(["-o", "%(title).150B [%(id)s].%(ext)s"])
        .args(["--", url]);

    emit(Progress::new(None, "Preparando descarga…"));
    let mut final_path = None;
    let outcome = manager.run(&mut command, |line| {
        if let Some(path) = line.strip_prefix(FILE_PREFIX) {
            final_path = Some(path.to_string());
        } else if let Some(progress) = parse_ytdlp_line(line) {
            emit(progress);
        }
    });
    let _ = std::fs::remove_dir_all(&temp);

    match outcome? {
        Outcome::Cancelled => Ok(None),
        Outcome::Finished => final_path
            .map(Some)
            .ok_or_else(|| "yt-dlp terminó sin producir ningún archivo".to_string()),
    }
}

fn parse_ytdlp_line(line: &str) -> Option<Progress> {
    if let Some(data) = line.strip_prefix(DOWNLOAD_PREFIX) {
        let fields: Vec<&str> = data.split('|').map(str::trim).collect();
        let [downloaded, total, estimate, speed, eta] = fields[..] else {
            return None;
        };
        let percent = downloaded.parse::<f64>().ok().and_then(|downloaded| {
            let total = total.parse::<f64>().or_else(|_| estimate.parse::<f64>()).ok()?;
            (total > 0.0).then(|| (downloaded / total * 100.0).min(100.0))
        });
        let mut parts = vec!["Descargando".to_string()];
        if !speed.contains("Unknown") && speed != "NA" {
            parts.push(speed.to_string());
        }
        if !eta.contains("Unknown") && eta != "NA" {
            parts.push(format!("quedan {eta}"));
        }
        return Some(Progress::new(percent, parts.join(" · ")));
    }

    let (processor, status) = line.strip_prefix(POSTPROCESS_PREFIX)?.split_once('|')?;
    if status != "started" {
        return None;
    }
    let message = match processor {
        "Merger" => "Uniendo video y audio…",
        "VideoConvertor" => "Convirtiendo el video, puede tardar…",
        "ExtractAudio" => "Extrayendo el audio…",
        "MoveFiles" => "Guardando el archivo…",
        _ => return None,
    };
    Some(Progress::new(None, message))
}

pub fn convert_video(
    manager: &JobManager,
    input: &str,
    format: &str,
    output_folder: &str,
    emit: &mut dyn FnMut(Progress),
) -> JobResult {
    prefer_gpu(tools::nvenc_available(), |gpu| {
        let (extension, codec_args, h264): (&str, Vec<&str>, bool) = match format {
            "mp4" => ("mp4", [h264_aac(gpu), FASTSTART].concat(), true),
            "mkv" => ("mkv", h264_aac(gpu).to_vec(), true),
            // NVIDIA no codifica VP9 ni DNxHR: la GPU solo ayuda a decodificar.
            "webm" => ("webm", VP9_OPUS.to_vec(), false),
            "davinci-linux" => ("mov", DAVINCI.to_vec(), false),
            other => return Err(format!("Formato no válido: {other}")),
        };
        let encoding = Encoding {
            decode_args: if gpu { GPU_DECODE } else { &[] },
            extension,
            codec_args: &codec_args,
            label: if gpu && h264 { "Convirtiendo con GPU" } else { "Convirtiendo" },
        };
        run_ffmpeg(manager, input, &encoding, output_folder, &mut *emit)
    })
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BatchSummary {
    pub total: usize,
    pub converted: usize,
    pub cancelled: bool,
    /// Último archivo convertido, para el botón "Ver archivo".
    pub path: Option<String>,
    pub failures: Vec<String>,
}

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "mov", "webm", "avi", "m4v", "mpg", "mpeg", "mts", "m2ts", "wmv", "flv",
];

/// Videos directamente dentro de `folder`, sin entrar en subcarpetas y en orden alfabético.
pub fn list_videos(folder: &str) -> Result<Vec<String>, String> {
    let folder = validate_folder(folder)?;
    let entries = std::fs::read_dir(folder)
        .map_err(|error| format!("No se pudo leer la carpeta: {error}"))?;
    let mut videos: Vec<String> = entries
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        VIDEO_EXTENSIONS.contains(&extension.to_lowercase().as_str())
                    })
        })
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    videos.sort();
    Ok(videos)
}

/// Convierte en orden. Un video que falla no detiene a los demás.
/// Devuelve `None` solo si se canceló antes de terminar ninguno.
pub fn convert_videos(
    manager: &JobManager,
    inputs: &[String],
    format: &str,
    output_folder: &str,
    emit: &mut dyn FnMut(Progress),
) -> Result<Option<BatchSummary>, String> {
    if inputs.is_empty() {
        return Err("Añade al menos un video".to_string());
    }
    let total = inputs.len();
    let mut summary = BatchSummary {
        total,
        converted: 0,
        cancelled: false,
        path: None,
        failures: Vec::new(),
    };

    for (index, input) in inputs.iter().enumerate() {
        if manager.is_cancelled() {
            summary.cancelled = true;
            break;
        }
        let name = Path::new(input)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| input.clone());
        let mut report = |progress: Progress| {
            let current = progress.percent.unwrap_or(0.0) / 100.0;
            emit(Progress::new(
                Some((index as f64 + current) / total as f64 * 100.0),
                format!("Video {} de {total} · {name} · {}", index + 1, progress.message),
            ));
        };
        match convert_video(manager, input, format, output_folder, &mut report) {
            Ok(Some(path)) => {
                summary.converted += 1;
                summary.path = Some(path);
            }
            Ok(None) => {
                summary.cancelled = true;
                break;
            }
            Err(error) => summary.failures.push(format!("{name}: {error}")),
        }
    }

    if summary.converted > 0 {
        Ok(Some(summary))
    } else if summary.cancelled {
        Ok(None)
    } else {
        Err(summary.failures.join("\n"))
    }
}

pub fn extract_audio(
    manager: &JobManager,
    input: &str,
    format: &str,
    output_folder: &str,
    emit: &mut dyn FnMut(Progress),
) -> JobResult {
    check_audio_format(format)?;
    let codec: &[&str] = match format {
        "mp3" => &["-c:a", "libmp3lame", "-q:a", "0"],
        "m4a" => &["-c:a", "aac", "-b:a", "256k"],
        "opus" => &["-c:a", "libopus", "-b:a", "160k"],
        "flac" => &["-c:a", "flac"],
        _ => &["-c:a", "pcm_s16le"],
    };
    let args = [&["-vn"], codec].concat();
    let encoding = Encoding {
        decode_args: &[],
        extension: format,
        codec_args: &args,
        label: "Extrayendo audio",
    };
    run_ffmpeg(manager, input, &encoding, output_folder, emit)
}

struct Encoding<'a> {
    decode_args: &'a [&'a str],
    extension: &'a str,
    codec_args: &'a [&'a str],
    label: &'a str,
}

fn run_ffmpeg(
    manager: &JobManager,
    input: &str,
    encoding: &Encoding,
    output_folder: &str,
    emit: &mut dyn FnMut(Progress),
) -> JobResult {
    let input_path = validate_input(input)?;
    let folder = validate_folder(output_folder)?;
    let output = unique_output(input_path, folder, encoding.extension);
    let duration_us = probe_duration_us(input_path);
    let label = encoding.label;

    let mut command = job::command(&tools::get().ffmpeg);
    command
        .args(["-hide_banner", "-nostdin", "-n", "-loglevel", "error"])
        .args(["-nostats", "-progress", "pipe:1"])
        .args(encoding.decode_args)
        .arg("-i")
        .arg(input_path)
        .args(encoding.codec_args)
        .arg(&output);

    emit(Progress::new(duration_us.map(|_| 0.0), format!("{label}…")));
    let outcome = manager.run(&mut command, |line| {
        if let Some(percent) = parse_ffmpeg_line(line, duration_us) {
            emit(Progress::new(Some(percent), format!("{label}…")));
        }
    });

    match outcome {
        Ok(Outcome::Finished) => Ok(Some(output.to_string_lossy().into_owned())),
        Ok(Outcome::Cancelled) => {
            let _ = std::fs::remove_file(&output);
            Ok(None)
        }
        Err(error) => {
            let _ = std::fs::remove_file(&output);
            Err(error)
        }
    }
}

fn parse_ffmpeg_line(line: &str, duration_us: Option<f64>) -> Option<f64> {
    let duration = duration_us?;
    let elapsed: f64 = line.strip_prefix("out_time_us=")?.trim().parse().ok()?;
    Some((elapsed / duration * 100.0).clamp(0.0, 100.0))
}

fn probe_duration_us(input: &Path) -> Option<f64> {
    let output = job::command(&tools::get().ffprobe)
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"])
        .arg(input)
        .output()
        .ok()?;
    let seconds: f64 = String::from_utf8_lossy(&output.stdout).trim().parse().ok()?;
    (seconds > 0.0).then_some(seconds * 1_000_000.0)
}

/// `nombre.ext`, o `nombre (1).ext` si ya existe. Nunca pisa un archivo, tampoco el original.
fn unique_output(input: &Path, folder: &Path, extension: &str) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "salida".to_string());
    let mut candidate = folder.join(format!("{stem}.{extension}"));
    let mut counter = 1;
    while candidate.exists() {
        candidate = folder.join(format!("{stem} ({counter}).{extension}"));
        counter += 1;
    }
    candidate
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cxyt-{name}-{}", unique_suffix()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Crea un video corto con imagen y sonido, de tamaño impar.
    fn sample_video(dir: &Path) -> PathBuf {
        let path = dir.join("muestra.mkv");
        let status = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
            .arg("testsrc=duration=2:size=321x241:rate=15")
            .args(["-f", "lavfi", "-i", "sine=duration=2"])
            .args(["-c:v", "ffv1", "-c:a", "flac"])
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
        path
    }

    fn codecs(path: &str) -> String {
        let output = Command::new("ffprobe")
            .args(["-v", "error", "-show_entries", "stream=codec_name", "-of", "csv=p=0"])
            .arg(path)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).split_whitespace().collect::<Vec<_>>().join(",")
    }

    #[test]
    fn construye_el_selector_de_calidad() {
        assert_eq!(video_args("1080", "best", false).unwrap()[1], "bv*[height<=1080]+ba/b[height<=1080]");
        assert_eq!(video_args("best", "best", false).unwrap()[1], "bv*+ba/b");
    }

    #[test]
    fn menor_peso_limita_a_720p_por_defecto() {
        assert!(video_args("best", "small", false).unwrap().contains(&"res:720,+size,+br".to_string()));
        assert!(video_args("1080", "small", false).unwrap().contains(&"res:1080,+size,+br".to_string()));
    }

    #[test]
    fn el_perfil_youtube_elige_el_codificador_segun_la_gpu() {
        let recode = |gpu| video_args("best", "youtube", gpu).unwrap().join(" ");
        assert!(recode(true).contains("-c:v h264_nvenc"));
        assert!(recode(false).contains("-c:v libx264"));
    }

    #[test]
    fn un_fallo_en_gpu_se_repite_en_cpu() {
        let mut attempts = Vec::new();
        let result = prefer_gpu(true, |gpu| {
            attempts.push(gpu);
            if gpu { Err("sin NVENC".to_string()) } else { Ok(Some("salida".to_string())) }
        });
        assert_eq!(result, Ok(Some("salida".to_string())));
        assert_eq!(attempts, [true, false]);
    }

    #[test]
    fn cancelar_en_gpu_no_se_repite_y_sin_gpu_va_directo_a_cpu() {
        let mut attempts = Vec::new();
        assert_eq!(prefer_gpu(true, |gpu| { attempts.push(gpu); Ok(None) }), Ok(None));
        assert_eq!(attempts, [true]);

        attempts.clear();
        assert_eq!(prefer_gpu(false, |gpu| { attempts.push(gpu); Ok(None) }), Ok(None));
        assert_eq!(attempts, [false]);
    }

    #[test]
    fn avisa_si_convierte_con_gpu() {
        let dir = test_dir("gpu-label");
        let input = sample_video(&dir);
        let manager = JobManager::default();
        let mut message = String::new();
        convert_video(&manager, input.to_str().unwrap(), "mp4", dir.to_str().unwrap(), &mut |p| {
            message = p.message
        })
        .unwrap()
        .unwrap();
        assert_eq!(message.contains("con GPU"), tools::nvenc_available(), "{message}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// NVENC no admite H.264 tan ancho: con GPU debe terminar igual, en la CPU.
    #[test]
    fn un_video_que_la_gpu_rechaza_se_convierte_en_cpu() {
        let dir = test_dir("gpu-fallback");
        let input = dir.join("ancho.mkv");
        let status = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
            .arg("color=black:s=4200x64:d=0.2:r=5")
            .args(["-c:v", "ffv1"])
            .arg(&input)
            .status()
            .unwrap();
        assert!(status.success());
        let manager = JobManager::default();
        let path = convert_video(&manager, input.to_str().unwrap(), "mp4", dir.to_str().unwrap(), &mut |_| {})
            .unwrap()
            .unwrap();
        assert_eq!(codecs(&path), "h264");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rechaza_opciones_invalidas() {
        assert!(video_args("abc", "best", false).is_err());
        assert!(video_args("best", "otro", false).is_err());
        assert!(validate_url("--exec=rm").is_err());
        assert!(validate_url("ftp://x").is_err());
        assert!(validate_folder("").is_err());
        assert!(validate_input("-i").is_err());
        assert!(check_audio_format("exe").is_err());
    }

    #[test]
    fn interpreta_el_progreso_de_descarga() {
        let progress = parse_ytdlp_line("[cxyt-dl]50|200|NA|  1.00MiB/s|00:10").unwrap();
        assert_eq!(progress.percent, Some(25.0));
        assert_eq!(progress.message, "Descargando · 1.00MiB/s · quedan 00:10");

        let estimated = parse_ytdlp_line("[cxyt-dl]50|NA|100.0|Unknown B/s|Unknown").unwrap();
        assert_eq!(estimated.percent, Some(50.0));
        assert_eq!(estimated.message, "Descargando");

        assert_eq!(parse_ytdlp_line("[cxyt-dl]50|NA|NA|NA|NA").unwrap().percent, None);
    }

    #[test]
    fn interpreta_las_etapas_de_posproceso() {
        assert!(parse_ytdlp_line("[cxyt-pp]Merger|started").is_some());
        assert!(parse_ytdlp_line("[cxyt-pp]Merger|finished").is_none());
        assert!(parse_ytdlp_line("[cxyt-pp]Desconocido|started").is_none());
        assert!(parse_ytdlp_line("otra cosa").is_none());
    }

    #[test]
    fn no_pisa_archivos_existentes() {
        let dir = test_dir("unique");
        let input = dir.join("video.mp4");
        std::fs::write(&input, b"x").unwrap();
        assert_eq!(unique_output(&input, &dir, "mp4"), dir.join("video (1).mp4"));
        assert_eq!(unique_output(&input, &dir, "mkv"), dir.join("video.mkv"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn convierte_a_cada_formato() {
        let dir = test_dir("convert");
        let input = sample_video(&dir);
        let input = input.to_str().unwrap();
        for (format, expected) in [
            ("mp4", "h264,aac"),
            ("mkv", "h264,aac"),
            ("webm", "vp9,opus"),
            ("davinci-linux", "dnxhd,pcm_s16le"),
        ] {
            let manager = JobManager::default();
            let mut last = None;
            let path = convert_video(&manager, input, format, dir.to_str().unwrap(), &mut |p| {
                last = p.percent
            })
            .unwrap_or_else(|error| panic!("{format}: {error}"))
            .unwrap();
            assert_eq!(codecs(&path), expected, "{format}");
            assert!(last.is_some(), "{format} debe informar progreso");
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn extrae_audio_en_cada_formato() {
        let dir = test_dir("audio");
        let input = sample_video(&dir);
        for (format, expected) in [
            ("mp3", "mp3"),
            ("m4a", "aac"),
            ("opus", "opus"),
            ("flac", "flac"),
            ("wav", "pcm_s16le"),
        ] {
            let manager = JobManager::default();
            let path = extract_audio(
                &manager,
                input.to_str().unwrap(),
                format,
                dir.to_str().unwrap(),
                &mut |_| {},
            )
            .unwrap_or_else(|error| panic!("{format}: {error}"))
            .unwrap();
            assert_eq!(codecs(&path), expected, "{format}");
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn un_archivo_invalido_da_error_y_no_deja_salida() {
        let dir = test_dir("invalid");
        let input = dir.join("roto.mp4");
        std::fs::write(&input, b"esto no es un video").unwrap();
        let manager = JobManager::default();
        let result = convert_video(&manager, input.to_str().unwrap(), "mkv", dir.to_str().unwrap(), &mut |_| {});
        assert!(result.is_err());
        assert!(!dir.join("roto.mkv").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn lista_solo_los_videos_de_la_carpeta() {
        let dir = test_dir("list");
        for name in ["b.MP4", "a.mkv", "notas.txt", "codigo.ts", "sin-extension"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        std::fs::create_dir(dir.join("sub.mp4")).unwrap();
        std::fs::write(dir.join("sub.mp4/dentro.mp4"), b"x").unwrap();
        let videos = list_videos(dir.to_str().unwrap()).unwrap();
        let expected = [dir.join("a.mkv"), dir.join("b.MP4")];
        assert_eq!(videos, expected.map(|path| path.to_string_lossy().into_owned()));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn un_video_roto_no_detiene_el_lote() {
        let dir = test_dir("batch");
        let good = sample_video(&dir).to_string_lossy().into_owned();
        let broken = dir.join("roto.mp4");
        std::fs::write(&broken, b"no es un video").unwrap();
        let inputs = [broken.to_string_lossy().into_owned(), good];
        let manager = JobManager::default();
        let mut percents = Vec::new();
        let summary = convert_videos(&manager, &inputs, "mkv", dir.to_str().unwrap(), &mut |p| {
            percents.push(p.percent.unwrap())
        })
        .unwrap()
        .unwrap();
        assert_eq!((summary.total, summary.converted, summary.cancelled), (2, 1, false));
        assert_eq!(summary.failures.len(), 1);
        assert!(summary.failures[0].starts_with("roto.mp4: "), "{:?}", summary.failures);
        assert!(summary.path.unwrap().ends_with("muestra (1).mkv"));
        assert!(percents.iter().all(|percent| (0.0..=100.0).contains(percent)));
        assert!(percents.windows(2).all(|pair| pair[0] <= pair[1]), "{percents:?}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn un_lote_sin_exitos_es_un_error() {
        let dir = test_dir("batch-fail");
        let broken = dir.join("roto.mp4");
        std::fs::write(&broken, b"no es un video").unwrap();
        let inputs = [broken.to_string_lossy().into_owned()];
        let manager = JobManager::default();
        let error = convert_videos(&manager, &inputs, "mkv", dir.to_str().unwrap(), &mut |_| {})
            .unwrap_err();
        assert!(error.starts_with("roto.mp4: "), "{error}");
        assert!(convert_videos(&manager, &[], "mkv", dir.to_str().unwrap(), &mut |_| {}).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cancelar_detiene_el_lote_y_conserva_lo_terminado() {
        let dir = test_dir("batch-cancel");
        let input = sample_video(&dir).to_string_lossy().into_owned();
        let inputs = [input.clone(), input];
        let manager = JobManager::default();
        let _guard = manager.begin().unwrap();
        let canceller = manager.clone();
        let summary = convert_videos(&manager, &inputs, "mkv", dir.to_str().unwrap(), &mut |p| {
            if p.message.starts_with("Video 2") {
                canceller.cancel();
            }
        })
        .unwrap()
        .unwrap();
        assert!(summary.cancelled);
        assert_eq!(summary.converted, 1);
        assert!(dir.join("muestra (1).mkv").exists());
        assert!(!dir.join("muestra (2).mkv").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    const SHORT_VIDEO: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";

    /// Solo la carpeta y el archivo final deben quedar: ningún temporal.
    fn only_file(dir: &Path) -> String {
        let names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 1, "{names:?}");
        names[0].clone()
    }

    #[test]
    #[ignore = "necesita red"]
    fn descarga_real_con_cada_perfil() {
        for (profile, extension, expected) in [
            ("best", ".mkv", None),
            ("davinci-linux", ".mov", Some("dnxhd,pcm_s16le")),
            ("youtube", ".mp4", Some("h264,aac")),
            ("small", ".mkv", None),
        ] {
            let dir = test_dir("download");
            let manager = JobManager::default();
            let mut updates = 0;
            let path = download_video(&manager, SHORT_VIDEO, "240", profile, dir.to_str().unwrap(), &mut |_| {
                updates += 1
            })
            .unwrap_or_else(|error| panic!("{profile}: {error}"))
            .unwrap();
            assert!(path.ends_with(extension), "{profile}: {path}");
            assert!(Path::new(&path).is_file(), "{profile}: {path}");
            assert!(updates > 1, "{profile}");
            only_file(&dir);
            if let Some(expected) = expected {
                assert_eq!(codecs(&path), expected, "{profile}");
            }
            std::fs::remove_dir_all(dir).unwrap();
        }
    }

    /// Con `CXYT_TOOLS_DIR` apuntando a los binarios incluidos y sin nada en el PATH:
    /// `env -i CXYT_TOOLS_DIR=... target/debug/deps/convertxyt_lib-* --ignored herramientas_incluidas`
    #[test]
    #[ignore = "necesita red y CXYT_TOOLS_DIR"]
    fn herramientas_incluidas_funcionan_solas() {
        let tools_dir = std::env::var("CXYT_TOOLS_DIR").unwrap();
        assert!(tools::set_for_test(Path::new(&tools_dir)).deno.is_some());
        let dir = test_dir("bundled");
        let manager = JobManager::default();
        let video = download_video(&manager, SHORT_VIDEO, "best", "best", dir.to_str().unwrap(), &mut |_| {})
            .unwrap()
            .unwrap();
        let converted = convert_video(&manager, &video, "mp4", dir.to_str().unwrap(), &mut |_| {})
            .unwrap()
            .unwrap();
        assert!(Path::new(&converted).is_file());
        let audio = extract_audio(&manager, &video, "mp3", dir.to_str().unwrap(), &mut |_| {})
            .unwrap()
            .unwrap();
        assert!(Path::new(&audio).is_file());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[ignore = "necesita red"]
    fn descarga_real_de_audio() {
        let dir = test_dir("download-audio");
        let manager = JobManager::default();
        let path = download_audio(&manager, SHORT_VIDEO, "mp3", dir.to_str().unwrap(), &mut |_| {})
            .unwrap()
            .unwrap();
        assert_eq!(codecs(&path), "mp3");
        only_file(&dir);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[ignore = "necesita red"]
    fn cancelar_una_descarga_no_deja_restos() {
        let dir = test_dir("download-cancel");
        let manager = JobManager::default();
        let _guard = manager.begin().unwrap();
        let canceller = manager.clone();
        let mut cancelled = false;
        let mut seen = Vec::new();
        let result = download_video(&manager, SHORT_VIDEO, "best", "davinci-linux", dir.to_str().unwrap(), &mut |p| {
            if !p.message.starts_with("Descargando") {
                seen.push(p.message.clone());
            }
            // Cancela en plena recodificación: ahí sí hay un ffmpeg hijo que matar.
            if !cancelled && p.message.starts_with("Convirtiendo") {
                cancelled = true;
                canceller.cancel();
            }
        });
        assert!(cancelled, "resultado: {result:?}, mensajes: {seen:?}");
        assert_eq!(result, Ok(None));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
