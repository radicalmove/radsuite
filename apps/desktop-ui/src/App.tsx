import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type EngineStatus = {
  id: string;
  label: string;
  available: boolean;
  detail: string;
};

type AppStatus = {
  app_name: string;
  database_ready: boolean;
  sync_configured: boolean;
  engines: EngineStatus[];
};

const fallbackStatus: AppStatus = {
  app_name: "RADsuite",
  database_ready: false,
  sync_configured: false,
  engines: [],
};

export default function App() {
  const [status, setStatus] = useState<AppStatus>(fallbackStatus);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<AppStatus>("get_app_status")
      .then((nextStatus) => {
        setStatus(nextStatus);
        setError(null);
      })
      .catch((reason: unknown) => {
        setError(reason instanceof Error ? reason.message : String(reason));
      });
  }, []);

  return (
    <main className="app-shell">
      <header className="topbar">
        <div>
          <p className="eyebrow">Internal alpha</p>
          <h1>{status.app_name}</h1>
        </div>
        <div className="status-strip" aria-label="Application status">
          <StatusPill label="Local DB" active={status.database_ready} />
          <StatusPill label="Sync" active={status.sync_configured} />
        </div>
      </header>

      {error ? <div className="notice">Command bridge unavailable: {error}</div> : null}

      <section className="workspace-grid">
        <section className="panel project-panel" aria-labelledby="projects-heading">
          <div className="panel-heading">
            <p className="eyebrow">Workspace</p>
            <h2 id="projects-heading">Projects</h2>
          </div>
          <div className="empty-state">
            <strong>No local projects yet</strong>
            <span>Project sync and creation controls land after the foundation shell.</span>
          </div>
        </section>

        <section className="panel" aria-labelledby="engines-heading">
          <div className="panel-heading">
            <p className="eyebrow">Native runtimes</p>
            <h2 id="engines-heading">Engines</h2>
          </div>
          <div className="engine-list">
            {status.engines.map((engine) => (
              <article key={engine.id} className="engine-row">
                <div>
                  <strong>{engine.label}</strong>
                  <span>{engine.detail}</span>
                </div>
                <StatusPill label={engine.available ? "Ready" : "Missing"} active={engine.available} />
              </article>
            ))}
          </div>
        </section>
      </section>
    </main>
  );
}

function StatusPill({ label, active }: { label: string; active: boolean }) {
  return <span className={active ? "pill pill-active" : "pill"}>{label}</span>;
}
