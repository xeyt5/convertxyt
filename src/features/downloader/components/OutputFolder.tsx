import { open } from "@tauri-apps/plugin-dialog";

interface OutputFolderProps {
    value: string;
    onChange: (value: string) => void;
}

export function OutputFolder({ value, onChange }: OutputFolderProps) {
    const selectFolder = async () => {
        const selected = await open({
            directory: true,
            multiple: false,
            title: "Selecciona la carpeta de destino",
        });
        if (typeof selected === "string") {
            onChange(selected);
            localStorage.setItem("download-folder", selected);
        }
    };

    return (
        <div className="row">
            <span className="row-label">Destino</span>
            <span className="row-path" title={value}>
                <bdi>{value || "Sin elegir"}</bdi>
            </span>
            <button type="button" className="button" onClick={selectFolder}>
                Examinar
            </button>
        </div>
    );
}
