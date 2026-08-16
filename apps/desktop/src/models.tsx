// Shared model-manager types, formatters, and the Settings "Models" panel.
//
// Models are no longer bundled inside the .app — they live in Application
// Support and download on first run, verified against a pinned SHA-256
// manifest. This module is the UI half of src-tauri/src/models.rs.
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Check, Download, Trash2, X } from "lucide-react";

export type ModelRole = "stt" | "cleanup";
export type EngineStatus = "missing" | "downloading" | "starting" | "ready" | "failed";

export interface ModelStatus {
  id: string;
  role: ModelRole;
  name: string;
  filename: string;
  size_bytes: number;
  blurb: string;
  license: string;
  url: string;
  installed: boolean;
  downloading: boolean;
  active: boolean;
  is_default: boolean;
  recommended: boolean;
}

export interface ModelsView {
  models: ModelStatus[];
  models_dir: string;
  disk_bytes: number;
}

// Mirrors ProgressEvent in models.rs.
export interface ModelProgress {
  id: string;
  role: string;
  status: "downloading" | "verifying" | "done" | "error" | "cancelled";
  downloaded: number;
  total: number;
  speed_bps: number;
  eta_secs: number;
  error: string | null;
}

export function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v >= 100 ? Math.round(v) : v.toFixed(1)} ${units[i]}`;
}

export function fmtSpeed(bps: number): string {
  if (bps <= 0) return "—";
  return `${fmtBytes(bps)}/s`;
}

export function fmtEta(secs: number): string {
  if (secs <= 0) return "—";
  if (secs < 60) return `${Math.round(secs)}s`;
  const m = Math.floor(secs / 60);
  const s = Math.round(secs % 60);
  if (m < 60) return `${m}m ${s}s`;
  const h = Math.floor(m / 60);
  return `${h}h ${m % 60}m`;
}

/** Subscribe to `funbutton:model-progress`, keyed by model id. */
export function useModelProgress(): Record<string, ModelProgress> {
  const [prog, setProg] = useState<Record<string, ModelProgress>>({});
  useEffect(() => {
    const un = listen<ModelProgress>("funbutton:model-progress", (e) => {
      setProg((p) => ({ ...p, [e.payload.id]: e.payload }));
    });
    return () => {
      un.then((u) => u());
    };
  }, []);
  return prog;
}

const ROLE_LABEL: Record<ModelRole, string> = {
  stt: "speech-to-text",
  cleanup: "cleanup",
};

/**
 * Settings → Models. Lists every manifest model with install / download /
 * active state, size, and a one-line "what it's good for". Download, delete
 * (free disk), and pick the active model per role. Shows total disk used.
 */
export function ModelManager({ pushToast }: { pushToast: (kind: "info" | "warn" | "ok", text: string) => void }) {
  const [view, setView] = useState<ModelsView | null>(null);
  const prog = useModelProgress();
  const busyRef = useRef(false);

  async function refresh() {
    try {
      setView(await invoke<ModelsView>("models_list"));
    } catch (e) {
      console.error("models_list failed", e);
    }
  }

  useEffect(() => {
    refresh();
    // `models-changed` fires after any download/delete/set-active/migration.
    const un = listen("funbutton:models-changed", () => refresh());
    // Poll while anything is downloading so install/active flip live even if an
    // event is missed.
    const id = setInterval(refresh, 1500);
    return () => {
      un.then((u) => u());
      clearInterval(id);
    };
  }, []);

  async function download(m: ModelStatus) {
    try {
      await invoke("models_download", { id: m.id });
      pushToast("info", `Downloading ${m.name} (${fmtBytes(m.size_bytes)})…`);
    } catch (e) {
      pushToast("warn", `Download failed: ${e}`);
    }
    refresh();
  }

  async function cancel(m: ModelStatus) {
    await invoke("models_cancel", { id: m.id }).catch(() => {});
    refresh();
  }

  async function remove(m: ModelStatus) {
    try {
      const freed = await invoke<number>("models_delete", { id: m.id });
      pushToast("ok", `Deleted ${m.name} — freed ${fmtBytes(freed)}`);
    } catch (e) {
      pushToast("warn", `Delete failed: ${e}`);
    }
    refresh();
  }

  async function setActive(m: ModelStatus) {
    if (busyRef.current) return;
    busyRef.current = true;
    try {
      await invoke("models_set_active", { role: m.role, id: m.id });
      pushToast("ok", `${ROLE_LABEL[m.role]} → ${m.name}`);
    } catch (e) {
      pushToast("warn", `Switch failed: ${e}`);
    } finally {
      busyRef.current = false;
    }
    refresh();
  }

  if (!view) return <div className="fb-hint">loading models…</div>;

  const roles: ModelRole[] = ["stt", "cleanup"];

  return (
    <div className="fb-models">
      <div className="fb-models-diskrow">
        <span className="fb-models-disk">{fmtBytes(view.disk_bytes)} on disk</span>
        <span className="fb-models-dir" title={view.models_dir}>
          {view.models_dir.replace(/^.*Application Support\//, "…/")}
        </span>
      </div>
      {roles.map((role) => (
        <div className="fb-models-group" key={role}>
          <div className="fb-models-grouphead">{ROLE_LABEL[role]}</div>
          {view.models
            .filter((m) => m.role === role)
            .map((m) => (
              <ModelCard
                key={m.id}
                m={m}
                prog={prog[m.id]}
                onDownload={() => download(m)}
                onCancel={() => cancel(m)}
                onDelete={() => remove(m)}
                onUse={() => setActive(m)}
              />
            ))}
        </div>
      ))}
      <div className="fb-hint">
        models download once into Application Support (never inside the app, so signing stays intact),
        verified by SHA-256. delete any you don&apos;t use to reclaim disk.
      </div>
    </div>
  );
}

function ModelCard({
  m,
  prog,
  onDownload,
  onCancel,
  onDelete,
  onUse,
}: {
  m: ModelStatus;
  prog: ModelProgress | undefined;
  onDownload: () => void;
  onCancel: () => void;
  onDelete: () => void;
  onUse: () => void;
}) {
  const downloading = m.downloading || prog?.status === "downloading" || prog?.status === "verifying";
  const pct = prog && prog.total > 0 ? Math.min(100, Math.round((prog.downloaded / prog.total) * 100)) : 0;
  const errored = prog?.status === "error";

  return (
    <div className={`fb-model ${m.active ? "active" : ""} ${downloading ? "dl" : ""}`}>
      <div className="fb-model-top">
        <span className="fb-model-name">
          {m.name}
          {m.recommended && <span className="fb-model-badge rec">recommended</span>}
          {m.active && <span className="fb-model-badge on">active</span>}
        </span>
        <span className="fb-model-size">{fmtBytes(m.size_bytes)}</span>
      </div>
      <div className="fb-model-blurb">{m.blurb}</div>
      <div className="fb-model-foot">
        <span className="fb-model-license">{m.license}</span>
        <span className="fb-model-spacer" />
        {downloading ? (
          <>
            <span className="fb-model-prog">
              {prog?.status === "verifying"
                ? "verifying SHA-256…"
                : `${pct}% · ${fmtSpeed(prog?.speed_bps ?? 0)} · ${fmtEta(prog?.eta_secs ?? 0)} left`}
            </span>
            <button className="fb-btn-small" onClick={onCancel}>
              <X size={12} /> cancel
            </button>
          </>
        ) : errored ? (
          <>
            <span className="fb-model-err" title={prog?.error ?? ""}>
              {prog?.error?.includes("sha256") ? "corrupt download" : "download failed"}
            </span>
            <button className="fb-btn-small" onClick={onDownload}>
              <Download size={12} /> retry
            </button>
          </>
        ) : m.installed ? (
          <>
            {!m.active && (
              <button className="fb-btn-small" onClick={onUse}>
                <Check size={12} /> use
              </button>
            )}
            <button className="fb-btn-small fb-btn-danger" onClick={onDelete} title="free disk">
              <Trash2 size={12} /> delete
            </button>
          </>
        ) : (
          <button className="fb-btn-small" onClick={onDownload}>
            <Download size={12} /> download
          </button>
        )}
      </div>
      {downloading && prog?.status !== "verifying" && (
        <div className="fb-model-bar">
          <div className="fb-model-bar-fill" style={{ width: `${pct}%` }} />
        </div>
      )}
    </div>
  );
}
