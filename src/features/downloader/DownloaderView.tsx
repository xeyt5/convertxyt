import { useState } from "react";
import { UrlInput } from "./components/UrlInput";

export function DownloaderView() {
    const [url, setUrl] = useState ("");

    return(
        <section className="downloader">
            <UrlInput value={url} onChange={setUrl}/>

            
        </section>
    );
    
}