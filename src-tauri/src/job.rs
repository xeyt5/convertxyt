//! Ejecuta un proceso externo a la vez, con lectura de su salida y cancelación.

use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};

/// Un `Command` que en Windows no abre una ventana de consola por cada proceso.
pub fn command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[derive(Default)]
struct Slot {
    busy: bool,
    cancelled: bool,
    pid: Option<u32>,
}

#[derive(Clone, Default)]
pub struct JobManager(Arc<Mutex<Slot>>);

/// Reserva el único trabajo permitido; lo libera al salir de ámbito.
pub struct JobGuard(JobManager);

impl Drop for JobGuard {
    fn drop(&mut self) {
        *self.0.lock() = Slot::default();
    }
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Finished,
    Cancelled,
}

impl JobManager {
    fn lock(&self) -> std::sync::MutexGuard<'_, Slot> {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn begin(&self) -> Result<JobGuard, String> {
        let mut slot = self.lock();
        if slot.busy {
            return Err("Ya hay un trabajo en curso".to_string());
        }
        slot.busy = true;
        Ok(JobGuard(self.clone()))
    }

    pub fn cancel(&self) {
        let mut slot = self.lock();
        if slot.busy {
            slot.cancelled = true;
            if let Some(pid) = slot.pid {
                kill_tree(pid);
            }
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.lock().cancelled
    }

    /// Lanza `command`, entrega cada línea de stdout a `on_line` y espera a que termine.
    /// Si falla, el error es la última línea relevante de stderr.
    pub fn run(
        &self,
        command: &mut Command,
        mut on_line: impl FnMut(&str),
    ) -> Result<Outcome, String> {
        let program = command.get_program().to_string_lossy().into_owned();
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Grupo propio: al cancelar yt-dlp también debe morir el ffmpeg que haya lanzado.
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(command, 0);

        let mut child = command.spawn().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                format!("No se encontró {program}. Instálalo y vuelve a intentarlo.")
            } else {
                format!("No se pudo iniciar {program}: {error}")
            }
        })?;

        {
            let mut slot = self.lock();
            slot.pid = Some(child.id());
            if slot.cancelled {
                kill_tree(child.id());
            }
        }

        // yt-dlp escribe parte de su progreso en stderr, así que se leen las dos salidas.
        let (sender, receiver) = mpsc::channel();
        let stdout = child.stdout.take().expect("stdout está canalizado");
        let stderr = child.stderr.take().expect("stderr está canalizado");
        forward_lines(stdout, false, sender.clone());
        forward_lines(stderr, true, sender);

        let mut stderr_text = String::new();
        for (is_stderr, line) in receiver {
            on_line(&line);
            if is_stderr {
                stderr_text.push_str(&line);
                stderr_text.push('\n');
            }
        }

        let status = child.wait();
        self.lock().pid = None;

        if self.is_cancelled() {
            return Ok(Outcome::Cancelled);
        }
        match status {
            Ok(status) if status.success() => Ok(Outcome::Finished),
            Ok(status) => Err(error_message(&stderr_text)
                .unwrap_or_else(|| format!("{program} terminó con error ({status})"))),
            Err(error) => Err(format!("Error esperando a {program}: {error}")),
        }
    }
}

fn forward_lines(
    pipe: impl Read + Send + 'static,
    is_stderr: bool,
    sender: mpsc::Sender<(bool, String)>,
) {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(pipe);
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            match reader.read_until(b'\n', &mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&buffer).trim_end().to_string();
                    if sender.send((is_stderr, line)).is_err() {
                        break;
                    }
                }
            }
        }
    });
}

/// Prefiere la última línea `ERROR:` (yt-dlp) y, si no hay, la última no vacía (ffmpeg).
fn error_message(stderr: &str) -> Option<String> {
    let mut lines = stderr.lines().map(str::trim).filter(|line| !line.is_empty());
    let last_error = lines.clone().rev().find(|line| line.starts_with("ERROR:"));
    last_error
        .or_else(|| lines.next_back())
        .map(|line| line.trim_start_matches("ERROR:").trim().to_string())
}

#[cfg(unix)]
fn kill_tree(pid: u32) {
    let _ = Command::new("kill")
        .args(["-TERM", "--", &format!("-{pid}")])
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(unix))]
fn kill_tree(pid: u32) {
    let _ = command("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn solo_permite_un_trabajo_a_la_vez() {
        let manager = JobManager::default();
        let guard = manager.begin().unwrap();
        assert!(manager.begin().is_err());
        drop(guard);
        assert!(manager.begin().is_ok());
    }

    #[test]
    fn entrega_lineas_de_stdout_y_stderr() {
        let manager = JobManager::default();
        let _guard = manager.begin().unwrap();
        let mut lines = Vec::new();
        let outcome = manager
            .run(
                Command::new("sh").args(["-c", "echo uno; echo dos >&2"]),
                |line| lines.push(line.to_string()),
            )
            .unwrap();
        assert_eq!(outcome, Outcome::Finished);
        lines.sort();
        assert_eq!(lines, ["dos", "uno"]);
    }

    #[test]
    fn devuelve_la_ultima_linea_de_error() {
        let manager = JobManager::default();
        let _guard = manager.begin().unwrap();
        let error = manager
            .run(
                Command::new("sh").args(["-c", "echo 'ERROR: malo' >&2; echo adios >&2; exit 3"]),
                |_| {},
            )
            .unwrap_err();
        assert_eq!(error, "malo");
    }

    #[test]
    fn avisa_si_falta_el_programa() {
        let manager = JobManager::default();
        let _guard = manager.begin().unwrap();
        let error = manager
            .run(&mut Command::new("programa-que-no-existe"), |_| {})
            .unwrap_err();
        assert!(error.contains("No se encontró"), "{error}");
    }

    #[test]
    fn cancelar_mata_tambien_a_los_procesos_hijos() {
        let manager = JobManager::default();
        let _guard = manager.begin().unwrap();
        let canceller = manager.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            canceller.cancel();
        });
        let started = Instant::now();
        // `sleep` es un hijo del shell y comparte sus tuberías: si sobreviviera, run() no volvería.
        let outcome = manager
            .run(Command::new("sh").args(["-c", "sleep 30; echo fin"]), |_| {})
            .unwrap();
        assert_eq!(outcome, Outcome::Cancelled);
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
