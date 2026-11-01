#!/usr/bin/env node
// Descarga y verifica las herramientas que van dentro de la app publicada.
// Solo hace falta para generar instaladores: en desarrollo se usan las del sistema.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const DENO_VERSION = "2.9.8";

const PLATFORMS = {
  linux: {
    triple: "x86_64-unknown-linux-gnu",
    extension: "",
    ytdlp: "yt-dlp_linux",
    ffmpeg: "ffmpeg-master-latest-linux64-gpl.tar.xz",
  },
  win32: {
    triple: "x86_64-pc-windows-msvc",
    extension: ".exe",
    ytdlp: "yt-dlp.exe",
    ffmpeg: "ffmpeg-master-latest-win64-gpl.zip",
  },
};

const platform = PLATFORMS[process.platform];
if (!platform) throw new Error(`Sistema no soportado: ${process.platform}`);

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const binaries = join(root, "src-tauri", "binaries");
const temp = join(binaries, ".tmp");

const target = (name) => join(binaries, `${name}-${platform.triple}${platform.extension}`);

async function download(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
  return Buffer.from(await response.arrayBuffer());
}

/** Busca el hash de `name` en un archivo de sumas con líneas `hash  nombre`. */
function findHash(sums, name) {
  for (const line of sums.split("\n")) {
    const [hash, file] = line.trim().split(/\s+/);
    if (file?.replace(/^\*/, "") === name) return hash;
  }
  throw new Error(`No hay suma de verificación para ${name}`);
}

async function downloadVerified(url, name, expectedHash) {
  console.log(`Descargando ${name}…`);
  const data = await download(url);
  const actual = createHash("sha256").update(data).digest("hex");
  if (actual !== expectedHash.toLowerCase()) {
    throw new Error(`Suma de verificación incorrecta en ${name}`);
  }
  const path = join(temp, name);
  writeFileSync(path, data);
  return path;
}

function extract(archive, destination) {
  mkdirSync(destination, { recursive: true });
  if (process.platform === "win32") {
    // El tar de Windows abre .zip; el de Git, que puede estar antes en el PATH, no.
    const tar = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
    execFileSync(tar, ["-xf", archive, "-C", destination], { stdio: "inherit" });
  } else if (archive.endsWith(".zip")) {
    execFileSync("unzip", ["-qo", archive, "-d", destination], { stdio: "inherit" });
  } else {
    execFileSync("tar", ["-xJf", archive, "-C", destination], { stdio: "inherit" });
  }
}

function install(source, name) {
  copyFileSync(source, target(name));
  chmodSync(target(name), 0o755);
}

async function fetchYtDlp() {
  const base = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
  const sums = (await download(`${base}/SHA2-256SUMS`)).toString();
  const path = await downloadVerified(
    `${base}/${platform.ytdlp}`,
    platform.ytdlp,
    findHash(sums, platform.ytdlp),
  );
  install(path, "yt-dlp");
}

async function fetchFfmpeg() {
  const base = "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest";
  const sums = (await download(`${base}/checksums.sha256`)).toString();
  const archive = await downloadVerified(
    `${base}/${platform.ffmpeg}`,
    platform.ffmpeg,
    findHash(sums, platform.ffmpeg),
  );
  const destination = join(temp, "ffmpeg");
  extract(archive, destination);
  const folder = platform.ffmpeg.replace(/\.(tar\.xz|zip)$/, "");
  for (const name of ["ffmpeg", "ffprobe"]) {
    install(join(destination, folder, "bin", `${name}${platform.extension}`), name);
  }
}

async function fetchDeno() {
  const name = `deno-${platform.triple}.zip`;
  const base = `https://github.com/denoland/deno/releases/download/v${DENO_VERSION}`;
  const sums = (await download(`${base}/${name}.sha256sum`)).toString();
  const archive = await downloadVerified(`${base}/${name}`, name, findHash(sums, name));
  const destination = join(temp, "deno");
  extract(archive, destination);
  install(join(destination, `deno${platform.extension}`), "deno");
}

const steps = [
  [["yt-dlp"], fetchYtDlp],
  [["ffmpeg", "ffprobe"], fetchFfmpeg],
  [["deno"], fetchDeno],
];

mkdirSync(temp, { recursive: true });
try {
  for (const [names, fetchTool] of steps) {
    if (names.every((name) => existsSync(target(name)))) {
      console.log(`Ya existe: ${names.join(", ")}`);
    } else {
      await fetchTool();
    }
  }
} finally {
  rmSync(temp, { recursive: true, force: true });
}

const size = steps
  .flatMap(([names]) => names)
  .reduce((total, name) => total + readFileSync(target(name)).length, 0);
console.log(`Herramientas listas: ${Math.round(size / 1024 / 1024)} MB en ${binaries}`);
