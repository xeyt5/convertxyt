import { useState } from "react";
import { FileInput } from "../../components/FileInput";
import { SelectField } from "../../components/SelectField";
import { OutputFolder } from "../downloader/components/OutputFolder";
import { UrlInput } from "../downloader/components/UrlInput";
import { useOutputFolder } from "../job/useOutputFolder";
import { extensionOf, useFileDrop } from "../job/useFileDrop";
import type { Job } from "../job/useJob";

type Source = "url" | "file";

const sources = [
  { value: "url", label: "URL" },
  { value: "file", label: "Archivo de mi equipo" },
];

const MEDIA_EXTENSIONS = [
  "mp4", "mkv", "mov", "webm", "avi", "m4v", "mpg", "mpeg", "ts", "mts", "m2ts", "wmv", "flv",
  "mp3", "m4a", "aac", "opus", "ogg", "flac", "wav",
];

const formats = [
  { value: "mp3", label: "MP3" },
  { value: "m4a", label: "M4A (AAC)" },
  { value: "opus", label: "Opus" },
  { value: "flac", label: "FLAC (sin pérdida)" },
  { value: "wav", label: "WAV (sin pérdida)" },
];

export function AudioView({ job }: { job: Job }) {
  const [source, setSource] = useState<Source>("url");
  const [url, setUrl] = useState("");
  const [input, setInput] = useState("");
  const [format, setFormat] = useState("mp3");
  const [outputFolder, setOutputFolder] = useOutputFolder("audio");
  const [notice, setNotice] = useState("");

  // Soltar un archivo cambia el origen a "Archivo de mi equipo".
  const dragging = useFileDrop((paths) => {
    const file = paths.find((path) => MEDIA_EXTENSIONS.includes(extensionOf(path)));
    if (!file) {
      setNotice("Ese archivo no es de video ni de audio.");
      return;
    }
    setNotice("");
    setSource("file");
    setInput(file);
  }, job.status !== "running");

  const hasSource = source === "url" ? url.trim() !== "" : input !== "";
  const ready = hasSource && outputFolder !== "" && job.status !== "running";

  const handleStart = () =>
    source === "url"
      ? job.start("download_audio", { url, format, outputFolder })
      : job.start("extract_audio", { input, format, outputFolder });

  return (
    <section className="view">
      <div className={dragging ? "group drop-target dragging" : "group drop-target"}>
        <SelectField
          id="audio-source"
          label="Origen"
          value={source}
          options={sources}
          onChange={(value) => setSource(value as Source)}
        />
        {source === "url" ? (
          <UrlInput value={url} onChange={setUrl} />
        ) : (
          <FileInput
            label="Archivo"
            value={input}
            placeholder="Arrastra un archivo aquí"
            onChange={setInput}
          />
        )}
        <SelectField
          id="audio-format"
          label="Formato"
          value={format}
          options={formats}
          onChange={setFormat}
        />
        <OutputFolder value={outputFolder} onChange={setOutputFolder} />
      </div>

      {notice && <p className="notice">{notice}</p>}

      <button type="button" className="primary" disabled={!ready} onClick={handleStart}>
        {source === "url" ? "Descargar audio" : "Extraer audio"}
      </button>
    </section>
  );
}
