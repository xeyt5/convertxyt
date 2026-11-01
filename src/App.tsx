import { useState } from "react";
import { Download, FileVideo, Music } from "lucide-react";
import "./styles/app.css";
import { DownloaderView } from "./features/downloader/DownloaderView";
import { ConvertView } from "./features/convert/ConvertView";
import { AudioView } from "./features/audio/AudioView";
import { JobPanel } from "./features/job/JobPanel";
import { useJob } from "./features/job/useJob";
import { UpdateBanner } from "./features/update/UpdateBanner";

type Tab = "download" | "convert" | "audio";

const tabs = [
  { id: "download", label: "Descargar", icon: Download },
  { id: "convert", label: "Convertir", icon: FileVideo },
  { id: "audio", label: "Audio", icon: Music },
] as const;

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("download");
  // Vive aquí para que el progreso siga visible al cambiar de pestaña.
  const job = useJob();

  return (
    <main className="app">
      <nav className="tabs">
        {tabs.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            className={activeTab === id ? "tab active" : "tab"}
            aria-current={activeTab === id ? "page" : undefined}
            onClick={() => setActiveTab(id)}
          >
            <Icon size={16} />
            {label}
          </button>
        ))}
      </nav>

      <UpdateBanner busy={job.status === "running"} />

      <section className="content">
        {activeTab === "download" && <DownloaderView job={job} />}
        {activeTab === "convert" && <ConvertView job={job} />}
        {activeTab === "audio" && <AudioView job={job} />}
        <JobPanel job={job} />
      </section>
    </main>
  );
}

export default App;
