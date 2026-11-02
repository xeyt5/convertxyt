import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { X } from "lucide-react";
import { SelectField } from "../../components/SelectField";
import { OutputFolder } from "../downloader/components/OutputFolder";
import { useOutputFolder } from "../job/useOutputFolder";
import type { Job } from "../job/useJob";

const formats = [
  { value: "mp4", label: "MP4 (H.264 + AAC)" },
  { value: "mkv", label: "MKV (H.264 + AAC)" },
  { value: "webm", label: "WebM (VP9 + Opus)" },
  { value: "davinci-linux", label: "DaVinci Resolve Linux" },
];

const VIDEO_EXTENSIONS = [
  "mp4", "mkv", "mov", "webm", "avi", "m4v", "mpg", "mpeg", "ts", "mts", "m2ts", "wmv", "flv",
];

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

export function ConvertView({ job }: { job: Job }) {
  const [inputs, setInputs] = useState<string[]>([]);
  const [notice, setNotice] = useState("");
  const [format, setFormat] = useState("mp4");
  const [outputFolder, setOutputFolder] = useOutputFolder("convert");

  const addInputs = (paths: string[]) =>
    setInputs((current) => [...new Set([...current, ...paths])]);

  const addFiles = async () => {
    setNotice("");
    const selected = await open({
      multiple: true,
      title: "Selecciona uno o varios videos",
      filters: [
        { name: "Videos", extensions: VIDEO_EXTENSIONS },
        { name: "Todos los archivos", extensions: ["*"] },
      ],
    });
    if (selected) addInputs(selected);
  };

  const addFolder = async () => {
    setNotice("");
    const folder = await open({
      directory: true,
      multiple: false,
      title: "Selecciona una carpeta con videos",
    });
    if (typeof folder !== "string") return;
    try {
      const videos = await invoke<string[]>("list_videos", { folder });
      if (videos.length === 0) setNotice("Esa carpeta no tiene videos.");
      addInputs(videos);
    } catch (error) {
      setNotice(String(error));
    }
  };

  const running = job.status === "running";
  const ready = inputs.length > 0 && outputFolder !== "" && !running;

  return (
    <section className="view">
      <div className="group">
        <div className="row">
          <span className="row-label">Videos</span>
          <span className="row-path">
            <bdi>{inputs.length === 0 ? "Ninguno" : `${inputs.length} en la lista`}</bdi>
          </span>
          <button type="button" className="button" disabled={running} onClick={addFiles}>
            Archivos
          </button>
          <button type="button" className="button" disabled={running} onClick={addFolder}>
            Carpeta
          </button>
        </div>

        {inputs.length > 0 && (
          <ul className="file-list">
            {inputs.map((path) => (
              <li key={path} title={path}>
                <span>{fileName(path)}</span>
                <button
                  type="button"
                  aria-label={`Quitar ${fileName(path)}`}
                  disabled={running}
                  onClick={() => setInputs((current) => current.filter((item) => item !== path))}
                >
                  <X size={14} />
                </button>
              </li>
            ))}
          </ul>
        )}

        <SelectField
          id="convert-format"
          label="Formato"
          value={format}
          options={formats}
          onChange={setFormat}
        />
        <OutputFolder value={outputFolder} onChange={setOutputFolder} />
      </div>

      {notice && <p className="notice">{notice}</p>}

      <button
        type="button"
        className="primary"
        disabled={!ready}
        onClick={() => job.start("convert_videos", { inputs, format, outputFolder })}
      >
        {inputs.length > 1 ? `Convertir ${inputs.length} videos` : "Convertir"}
      </button>
    </section>
  );
}
