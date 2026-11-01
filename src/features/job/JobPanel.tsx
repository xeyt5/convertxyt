import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { Job } from "./useJob";

export function JobPanel({ job }: { job: Job }) {
  if (job.status === "idle") return null;

  const running = job.status === "running";
  const indeterminate = running && job.percent === null;

  return (
    <section className={`job-panel ${job.status}`} aria-live="polite">
      {(running || job.status === "done") && (
        <div
          className={indeterminate ? "progress indeterminate" : "progress"}
          role="progressbar"
          aria-valuenow={job.percent === null ? undefined : Math.round(job.percent)}
          aria-valuemin={0}
          aria-valuemax={100}
        >
          <div style={indeterminate ? undefined : { width: `${job.percent ?? 0}%` }} />
        </div>
      )}

      <div className="job-row">
        <p className="job-message">
          {job.percent !== null && running && `${Math.round(job.percent)}% · `}
          {job.status === "error" ? `Error: ${job.message}` : job.message}
        </p>

        {running && (
          <button type="button" className="button" onClick={job.cancel}>
            Cancelar
          </button>
        )}
        {job.path && (
          <button
            type="button"
            className="button"
            onClick={() => revealItemInDir(job.path as string)}
          >
            Ver archivo
          </button>
        )}
      </div>

      {job.path && <p className="job-path">{job.path}</p>}

      {job.failures.length > 0 && (
        <ul className="job-failures">
          {job.failures.map((failure) => (
            <li key={failure}>{failure}</li>
          ))}
        </ul>
      )}
    </section>
  );
}
