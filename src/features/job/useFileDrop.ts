import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

/**
 * Recibe los archivos que se arrastran a la ventana desde el explorador.
 * Devuelve `true` mientras hay archivos encima, para resaltar la zona.
 */
export function useFileDrop(onDrop: (paths: string[]) => void, enabled = true) {
  const [dragging, setDragging] = useState(false);
  const onDropRef = useRef(onDrop);
  onDropRef.current = onDrop;

  useEffect(() => {
    if (!enabled) {
      setDragging(false);
      return;
    }

    let unlisten: (() => void) | undefined;
    let disposed = false;

    getCurrentWebview()
      .onDragDropEvent(({ payload }) => {
        if (payload.type === "enter") setDragging(true);
        else if (payload.type === "leave") setDragging(false);
        else if (payload.type === "drop") {
          setDragging(false);
          if (payload.paths.length > 0) onDropRef.current(payload.paths);
        }
      })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [enabled]);

  return dragging;
}

export function extensionOf(path: string) {
  const name = path.split(/[\\/]/).pop() ?? "";
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
}
