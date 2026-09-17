import { useState } from "react";
import { UrlInput } from "./components/UrlInput";
import { QuealitySelector } from "./components/QualitySelector";
import { ProfileSelector } from "./components/ProfileSelector";
import { OutputFolder } from "./components/OutputFolder";
import { Download } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

export function DownloaderView() {
    const [url, setUrl] = useState ("");
    const [quality, setQuality] = useState("best");
    const [profile, setProfile] = useState("best");
    const [outputFolder, setOutputFolder] = useState(() => localStorage.getItem("download-folder") ?? "",);
    const headleDownload = async () =>{ const result = await invoke<string>("download_video",{ url, quality, profile, outputFolder}); console.log(result)};

    return(
        <section className="downloader">
            <UrlInput value={url} onChange={setUrl}/>

            <QuealitySelector value={quality} onChange={setQuality}/>

            <ProfileSelector value={profile} onChange={setProfile}/>

            <OutputFolder value={outputFolder} onChange={setOutputFolder}/>

            <button type="button" className="download-button" onClick={headleDownload}>
                <Download size={18}/>
                Descargar
            </button>
        </section>
    );
    
}