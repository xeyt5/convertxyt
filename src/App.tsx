import { useState } from "react";
import { Download, FileVideo, Music } from "lucide-react";
import "./styles/app.css";
import { DownloaderView } from "./features/downloader/DownloaderView";

type Tab = "download" | "convert" | "audio";

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("download");

  return (
    <main className="app">
      <section className="window">
        <header className="header">
          <h1>convert</h1>
          <nav className="tabs">
            <button className={activeTab === "download" ? "tab active": "tab"}
            onClick={() => setActiveTab("download")}>
              <Download size={16}/>
              Descargar
            </button>
            <button className={activeTab === "convert" ? "tab active": "tab"}
              onClick={() => setActiveTab("convert")}>
              <FileVideo size={16}/>
              Convertir
            </button>
            <button className={activeTab === "audio" ? "tab active": "tab"}
            onClick={() => setActiveTab("audio")}>
              <Music size={16}/>
              Audio
            </button>
          </nav>
        </header>

        <section className="content">
        {activeTab === "download" && <DownloaderView />}
        {activeTab === "convert" && (
          <div>
            <h2>Convertir video</h2>
            <p>convirte un archivo de video a otro formato</p>
          </div>
        )}
        {activeTab === "audio" && (
          <div>
            <h2>Descarga audio</h2>
            <p>Extrae o Descarga audio de un video</p>
          </div>
        )}
        </section>
      </section>
    </main>
  );
}

export default App;
