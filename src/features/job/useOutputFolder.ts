import { useState } from "react";

const STORAGE_KEY = "download-folder";

function readStored(): string {
  try {
    return localStorage.getItem(STORAGE_KEY) ?? "";
  } catch {
    return "";
  }
}

/** Carpeta de destino recordada entre sesiones y compartida por todas las pestañas. */
export function useOutputFolder() {
  return useState(readStored);
}
