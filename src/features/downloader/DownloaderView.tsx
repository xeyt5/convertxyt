import { useState } from "react";
import { UrlInput } from "./components/UrlInput";
import { QualitySelector } from "./components/QualitySelector";
import { ProfileSelector } from "./components/ProfileSelector";
import { OutputFolder } from "./components/OutputFolder";
import { useOutputFolder } from "../job/useOutputFolder";
import type { Job } from "../job/useJob";

export function DownloaderView({ job }: { job: Job }) {
    const [url, setUrl] = useState("");
    const [quality, setQuality] = useState("best");
    const [profile, setProfile] = useState("best");
    const [outputFolder, setOutputFolder] = useOutputFolder("download");

    const ready = url.trim() !== "" && outputFolder !== "" && job.status !== "running";

    const handleDownload = () =>
        job.start("download_video", { url, quality, profile, outputFolder });

    return (
        <section className="view">
            <div className="group">
                <UrlInput value={url} onChange={setUrl} />
                <QualitySelector value={quality} onChange={setQuality} />
                <ProfileSelector value={profile} onChange={setProfile} />
                <OutputFolder value={outputFolder} onChange={setOutputFolder} />
            </div>

            <button type="button" className="primary" disabled={!ready} onClick={handleDownload}>
                Descargar
            </button>
        </section>
    );
}
