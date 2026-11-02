import { useState } from "react";

export type FolderSection = "download" | "convert" | "audio";

function readStored(key: string): string {
  try {
    return localStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

/** Carpeta de destino de una pestaña, recordada entre sesiones e independiente de las demás. */
export function useOutputFolder(section: FolderSection) {
  const key = `${section}-folder`;
  const [folder, setFolder] = useState(() => readStored(key));

  const chooseFolder = (value: string) => {
    setFolder(value);
    try {
      localStorage.setItem(key, value);
    } catch {
      // Sin almacenamiento la carpeta sigue valiendo hasta cambiar de pestaña.
    }
  };

  return [folder, chooseFolder] as const;
}
