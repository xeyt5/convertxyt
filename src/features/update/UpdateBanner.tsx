import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import type { Update } from "@tauri-apps/plugin-updater";

export function UpdateBanner({ busy }: { busy: boolean }) {
  const [update, setUpdate] = useState<Update | null>(null);
  const [installing, setInstalling] = useState(false);
  const [percent, setPercent] = useState<number | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;
    (async () => {
      if (!(await invoke<boolean>("can_self_update"))) return;
      const available = await check();
      if (active) setUpdate(available);
    })().catch((reason) => {
      // Sin conexión no hay nada que avisar: la app sigue funcionando igual.
      console.warn("No se pudo buscar actualizaciones:", reason);
    });
    return () => {
      active = false;
    };
  }, []);

  if (!update) return null;

  const install = async () => {
    setInstalling(true);
    setError("");
    let total = 0;
    let downloaded = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress" && total > 0) {
          downloaded += event.data.chunkLength;
          setPercent(Math.round((downloaded / total) * 100));
        }
      });
      await relaunch();
    } catch (reason) {
      setError(String(reason));
      setInstalling(false);
    }
  };

  return (
    <aside className="update-banner" aria-live="polite">
      <p>
        {error
          ? `No se pudo actualizar: ${error}`
          : installing
            ? `Actualizando${percent === null ? "…" : ` · ${percent}%`}`
            : `Hay una versión nueva: ${update.version}`}
      </p>
      {!installing && (
        <button
          type="button"
          className="button"
          disabled={busy}
          title={busy ? "Espera a que termine el trabajo en curso" : undefined}
          onClick={install}
        >
          {error ? "Reintentar" : "Actualizar"}
        </button>
      )}
    </aside>
  );
}
