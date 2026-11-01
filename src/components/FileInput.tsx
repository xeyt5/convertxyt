import { open } from "@tauri-apps/plugin-dialog";

interface FileInputProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
}

const MEDIA_EXTENSIONS = [
  "mp4", "mkv", "mov", "webm", "avi", "m4v", "mpg", "mpeg", "ts", "wmv", "flv",
  "mp3", "m4a", "opus", "ogg", "flac", "wav",
];

export function FileInput({ label, value, onChange }: FileInputProps) {
  const selectFile = async () => {
    const selected = await open({
      multiple: false,
      title: "Selecciona un archivo",
      filters: [
        { name: "Video y audio", extensions: MEDIA_EXTENSIONS },
        { name: "Todos los archivos", extensions: ["*"] },
      ],
    });
    if (typeof selected === "string") onChange(selected);
  };

  return (
    <div className="row">
      <span className="row-label">{label}</span>
      <span className="row-path" title={value}>
        <bdi>{value || "Ningún archivo"}</bdi>
      </span>
      <button type="button" className="button" onClick={selectFile}>
        Examinar
      </button>
    </div>
  );
}
