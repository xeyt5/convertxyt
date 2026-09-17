import { open } from "@tauri-apps/plugin-dialog";

interface OutputFolderProps{
    value: string;
    onChange: (value: string) => void;
}

export function OutputFolder({
    value,
    onChange,
}: OutputFolderProps){
    const selectFolder = async () => {
        const selected = await open({
            directory: true,
            multiple: false,
            title: "Selecciona la carpeta de destino"
        });
        if (typeof selected === "string"){
            onChange(selected);
            localStorage.setItem("download-folder", selected);
        }
    };
    return(
        <div className="form-group">
            <label htmlFor="output-folder">Carpeta de destino</label>
            <div className="folder-selector">
                <input id="output-selector" type="text" value={value} placeholder="Seleccione alguna carpeta" readOnly />
                <button type="button" onClick={selectFolder}> examinar</button>
            </div>
        </div>
    );
}