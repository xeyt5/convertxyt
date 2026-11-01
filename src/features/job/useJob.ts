import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type JobStatus = "idle" | "running" | "done" | "cancelled" | "error";

export interface JobState {
  status: JobStatus;
  percent: number | null;
  message: string;
  path: string | null;
  failures: string[];
}

export interface Job extends JobState {
  start: (command: string, args: Record<string, unknown>) => Promise<void>;
  cancel: () => void;
}

interface ProgressEvent {
  percent: number | null;
  message: string;
}

interface BatchSummary {
  total: number;
  converted: number;
  cancelled: boolean;
  path: string | null;
  failures: string[];
}

const IDLE: JobState = { status: "idle", percent: null, message: "", path: null, failures: [] };

/** El backend devuelve la ruta del archivo, un resumen de lote, o null si se canceló. */
function finishedState(result: string | BatchSummary | null): JobState {
  if (result === null) {
    return { ...IDLE, status: "cancelled", message: "Cancelado" };
  }
  if (typeof result === "string") {
    return { ...IDLE, status: "done", percent: 100, message: "Listo", path: result };
  }
  const { total, converted, cancelled, path, failures } = result;
  if (cancelled) {
    return {
      ...IDLE,
      status: "cancelled",
      message: `Cancelado: ${converted} de ${total} ya convertidos`,
      path,
      failures,
    };
  }
  const message =
    failures.length > 0
      ? `${converted} de ${total} convertidos`
      : total > 1
        ? `Listo: ${total} videos convertidos`
        : "Listo";
  return { ...IDLE, status: "done", percent: 100, message, path, failures };
}

export function useJob(): Job {
  const [state, setState] = useState<JobState>(IDLE);

  useEffect(() => {
    const unlisten = listen<ProgressEvent>("job-progress", ({ payload }) => {
      setState((current) =>
        current.status === "running" ? { ...current, ...payload } : current,
      );
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  const start = useCallback(
    async (command: string, args: Record<string, unknown>) => {
      setState({ ...IDLE, status: "running", message: "Iniciando…" });
      try {
        setState(finishedState(await invoke<string | BatchSummary | null>(command, args)));
      } catch (error) {
        setState({ ...IDLE, status: "error", message: String(error) });
      }
    },
    [],
  );

  const cancel = useCallback(() => {
    setState((current) =>
      current.status === "running" ? { ...current, message: "Cancelando…" } : current,
    );
    void invoke("cancel_job");
  }, []);

  return { ...state, start, cancel };
}
