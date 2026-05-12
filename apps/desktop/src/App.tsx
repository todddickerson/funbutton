import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type Backend = "auto" | "groq" | "local";
type ModeOverride = "auto" | "code" | "email" | "slack" | "raw";
type HotkeyKind = "fn" | "right_option";

type PremiumModel = "fast" | "premium-haiku" | "premium-sonnet" | "premium-opus" | "premium-gpt41";

interface Settings {
  groq_api_key: string;
  backend: Backend;
  ollama_model: string;
  ollama_url: string;
  words_today: number;
  words_today_date: string;
  hotkey_label: string;
  hotkey_kind: HotkeyKind;
  mode_override: ModeOverride;
  dictionary: string[];
  history_retention_days: number;
  onboarded: boolean;
  license_jwt: string;
  cloud_api_base: string;
  premium_model: PremiumModel;
}

interface LicenseInfo {
  valid: boolean;
  tier: string;
  expires_at: number;
  included_premium_words: number;
  words_used_this_month: number;
  cap_cents: number;
}

interface ResultPayload {
  raw: string;
  cleaned: string;
  mode: string;
  backend: string;
  word_count: number;
}

interface HistoryEntry {
  id: number;
  ts: number;
  raw_transcript: string;
  cleaned_text: string;
  mode_used: string;
  frontmost_app: string | null;
  paste_succeeded: boolean;
  audio_duration_ms: number | null;
  model_used: string;
}

type Tab = "settings" | "history" | "license";

interface Toast {
  id: number;
  kind: "info" | "warn" | "ok";
  text: string;
}

const RETENTION_OPTIONS: { label: string; days: number }[] = [
  { label: "7 days", days: 7 },
  { label: "30 days", days: 30 },
  { label: "90 days", days: 90 },
  { label: "never", days: 0 },
];

function App() {
  const [tab, setTab] = useState<Tab>("settings");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [status, setStatus] = useState("idle");
  const [statusMsg, setStatusMsg] = useState<string | null>(null);
  const [last, setLast] = useState<ResultPayload | null>(null);
  const [ollamaUp, setOllamaUp] = useState<boolean | null>(null);
  const [saved, setSaved] = useState(false);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [historyQuery, setHistoryQuery] = useState("");
  const [historyMode, setHistoryMode] = useState<string>("all");
  const [lastFailed, setLastFailed] = useState<HistoryEntry | null>(null);
  const [copiedId, setCopiedId] = useState<number | null>(null);
  // License state
  const [licenseInfo, setLicenseInfo] = useState<LicenseInfo | null>(null);
  const [licenseDraftJwt, setLicenseDraftJwt] = useState<string>("");
  const [licenseValidating, setLicenseValidating] = useState(false);
  const [licenseError, setLicenseError] = useState<string | null>(null);
  const [capDraftCents, setCapDraftCents] = useState<number>(2000);
  const [showCapDisclosure, setShowCapDisclosure] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);

  function pushToast(kind: Toast["kind"], text: string) {
    const id = Date.now() + Math.random();
    setToasts((t) => [...t, { id, kind, text }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 5000);
  }

  async function refreshHistory() {
    try {
      const list = await invoke<HistoryEntry[]>("history_list", {
        limit: 200,
        search: historyQuery || null,
        mode: historyMode !== "all" ? historyMode : null,
      });
      setHistory(list);
    } catch (e) {
      console.error("history_list failed", e);
    }
    try {
      const f = await invoke<HistoryEntry | null>("history_last_failed");
      setLastFailed(f);
    } catch {
      setLastFailed(null);
    }
  }

  useEffect(() => {
    invoke<Settings>("get_settings").then(setSettings);
    invoke<string>("get_status").then(setStatus);
    invoke<boolean>("ollama_check").then(setOllamaUp).catch(() => setOllamaUp(false));
    refreshHistory();

    const unS = listen<{ status: string; message: string | null }>("funbutton:status", (e) => {
      setStatus(e.payload.status);
      setStatusMsg(e.payload.message ?? null);
    });
    const unR = listen<ResultPayload>("funbutton:result", (e) => {
      setLast(e.payload);
      invoke<Settings>("get_settings").then(setSettings);
      refreshHistory();
      if (e.payload.backend === "cloud-fallback") {
        pushToast(
          "warn",
          "Monthly cap hit. Switched to fast tier. Adjust in Settings → License."
        );
      }
    });
    const unF = listen("funbutton:paste-failed", () => {
      refreshHistory();
    });
    const unH = listen("funbutton:open-history", () => {
      setTab("history");
      refreshHistory();
    });
    return () => {
      unS.then((u) => u());
      unR.then((u) => u());
      unF.then((u) => u());
      unH.then((u) => u());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // refresh history when filter changes
  useEffect(() => {
    if (tab === "history") refreshHistory();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [historyQuery, historyMode, tab]);

  function update<K extends keyof Settings>(k: K, v: Settings[K]) {
    if (!settings) return;
    setSettings({ ...settings, [k]: v });
  }

  async function save() {
    if (!settings) return;
    await invoke("save_settings", { settings });
    await invoke("history_purge_now").catch(() => {});
    setSaved(true);
    setTimeout(() => setSaved(false), 1400);
  }

  // -------- License --------
  async function validateLicense(jwt: string) {
    setLicenseValidating(true);
    setLicenseError(null);
    try {
      const info = await invoke<LicenseInfo>("validate_license", { jwt });
      setLicenseInfo(info);
      setCapDraftCents(info.cap_cents);
      // Persist to settings so the pipeline picks it up.
      if (settings) {
        const next = { ...settings, license_jwt: jwt };
        setSettings(next);
        await invoke("save_settings", { settings: next });
      }
      pushToast("ok", `License active · ${info.tier.replace("_", " ")}`);
    } catch (e) {
      setLicenseError(String(e));
      setLicenseInfo(null);
    } finally {
      setLicenseValidating(false);
    }
  }

  async function refreshLicense() {
    if (!settings?.license_jwt) {
      setLicenseInfo(null);
      return;
    }
    try {
      const info = await invoke<LicenseInfo>("validate_license", {
        jwt: settings.license_jwt,
      });
      setLicenseInfo(info);
      setCapDraftCents(info.cap_cents);
    } catch (e) {
      console.warn("license refresh failed", e);
    }
  }

  async function commitCap(cents: number) {
    try {
      await invoke("set_cap_cents", { capCents: cents });
      pushToast("ok", cents === 0 ? "Auto top-up disabled" : `Cap set to $${(cents / 100).toFixed(0)}/mo`);
      refreshLicense();
    } catch (e) {
      pushToast("warn", `Cap update failed: ${e}`);
    }
  }

  async function openPortal() {
    if (!settings?.license_jwt) return;
    try {
      const base = settings.cloud_api_base.replace(/\/+$/, "");
      const res = await fetch(`${base}/v1/portal/portal`, {
        method: "POST",
        headers: { Authorization: `Bearer ${settings.license_jwt}` },
      });
      if (!res.ok) {
        pushToast("warn", `Portal unavailable (${res.status})`);
        return;
      }
      const json = (await res.json()) as { url?: string };
      if (json.url) {
        await invoke("plugin:opener|open_url", { url: json.url }).catch(() => {
          // fallback: best-effort window.open
          window.open(json.url, "_blank");
        });
      }
    } catch (e) {
      pushToast("warn", `Portal failed: ${e}`);
    }
  }

  // refresh license info when the tab opens or the JWT changes
  useEffect(() => {
    if (tab === "license") refreshLicense();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab, settings?.license_jwt]);

  async function copyEntry(id: number) {
    try {
      await invoke("history_copy", { id });
      setCopiedId(id);
      setTimeout(() => setCopiedId(null), 1200);
    } catch (e) {
      console.error("history_copy failed", e);
    }
  }

  const dot = status === "recording" ? "#ff5050" :
              status === "transcribing" || status === "cleaning" || status === "pasting" ? "#ffaa00" :
              status === "error" ? "#ff5050" : "#9a9a9a";

  const modesInHistory = useMemo(() => {
    const set = new Set(history.map(h => h.mode_used));
    return Array.from(set).sort();
  }, [history]);

  return (
    <main className="fb-root">
      <header className="fb-header">
        <div className="fb-brand">
          <span className="fb-logo">●</span>
          <span className="fb-name">FunButton</span>
          <span className="fb-tag">talk fast. stay local. pay less.</span>
        </div>
        <div className="fb-status">
          <span className="fb-dot" style={{ background: dot }} />
          <span className="fb-status-label">{status}{statusMsg ? ` — ${statusMsg}` : ""}</span>
        </div>
      </header>

      <div className="fb-tabs">
        <button className={`fb-tab ${tab === "settings" ? "on" : ""}`} onClick={() => setTab("settings")}>settings</button>
        <button className={`fb-tab ${tab === "history" ? "on" : ""}`} onClick={() => { setTab("history"); refreshHistory(); }}>
          history
          {lastFailed && <span className="fb-tab-pill">!</span>}
        </button>
        <button className={`fb-tab ${tab === "license" ? "on" : ""}`} onClick={() => setTab("license")}>
          license
          {licenseInfo && <span className="fb-tab-pill ok">●</span>}
        </button>
      </div>

      {!settings ? (
        <div className="fb-loading">loading…</div>
      ) : tab === "settings" ? (
        <div className="fb-form">
          {settings.groq_api_key.trim() === "" && ollamaUp === false && (
            <div className="fb-welcome">
              <div className="fb-welcome-title">welcome — meet the Fun Button.</div>
              <div className="fb-welcome-body">
                <strong>FunButton = Fn Button.</strong> The key at the bottom-left corner of your Mac keyboard.
                You probably never used it. We just gave it a job.<br/><br/>
                <strong>Step 1.</strong> Pick a cleanup backend:<br/>
                &nbsp;&nbsp;a) paste a Groq API key below (free at console.groq.com/keys), <em>or</em><br/>
                &nbsp;&nbsp;b) install Ollama and run <code>ollama pull qwen2.5:1.5b</code> — fully local, no API key.<br/><br/>
                <strong>Step 2.</strong> Close this window. Hold <kbd>fn</kbd> in any text field, talk, release.<br/>
                <strong>Step 3.</strong> macOS will ask for <strong>Microphone</strong>, <strong>Accessibility</strong>, and <strong>Input Monitoring</strong>. Grant all three. The Input Monitoring one is what lets us see Fn — macOS doesn't expose it as a normal modifier.
              </div>
            </div>
          )}
          <div className="fb-section">
            <label className="fb-label">The Fun Button</label>
            <div className="fb-keyboard-glyph">
              <div className={`fb-key fn ${settings.hotkey_kind === "fn" ? "on" : ""}`} title="The Fn key — bottom-left of every Mac keyboard">
                <span className="fb-key-label">fn</span>
                <span className="fb-key-fun">FUN</span>
              </div>
              <div className="fb-key-arrow">↑</div>
              <div className="fb-keyboard-caption">
                that key, bottom-left of your keyboard.<br/>
                <span className="fb-muted">nobody used it. we just gave it a job.</span>
              </div>
            </div>
            <div className="fb-radios" style={{marginTop:8}}>
              {(["fn","right_option"] as const).map(k => (
                <button
                  key={k}
                  className={`fb-pill ${settings.hotkey_kind === k ? "on" : ""}`}
                  onClick={() => update("hotkey_kind", k)}
                >{k === "fn" ? "Fn (default)" : "Right Option"}</button>
              ))}
            </div>
            <div className="fb-hint">
              Default is the <strong>Fun Button</strong> (Fn — bottom-left). Needs Input Monitoring permission on first run.
              {" "}Switch to <strong>Right Option</strong> if you've already mapped Fn elsewhere (Karabiner, Hyperkey, etc).
              <br/>
              <kbd>⌘⇧V</kbd> re-pastes last cleaned · <kbd>⌘⇧H</kbd> opens history · changes apply on next launch.
            </div>
          </div>

          <div className="fb-section">
            <label className="fb-label">Mode</label>
            <div className="fb-radios">
              {(["auto","code","email","slack","raw"] as const).map(m => (
                <button
                  key={m}
                  className={`fb-pill ${settings.mode_override === m ? "on" : ""}`}
                  onClick={() => update("mode_override", m)}
                >{m}</button>
              ))}
            </div>
            <div className="fb-hint">
              <strong>auto</strong> picks based on the frontmost app (Cursor / VS Code / Mail / Slack → matching prompt). Override to force a mode.
            </div>
          </div>

          <div className="fb-section">
            <label className="fb-label">Cleanup backend</label>
            <div className="fb-radios">
              {(["auto","groq","local"] as const).map(b => (
                <button
                  key={b}
                  className={`fb-pill ${settings.backend === b ? "on" : ""}`}
                  onClick={() => update("backend", b)}
                >{b}</button>
              ))}
            </div>
            <div className="fb-hint">
              <strong>auto</strong> uses local Ollama if running, falls back to Groq.{" "}
              <strong>groq</strong> always uses cloud.{" "}
              <strong>local</strong> requires Ollama at <code>{settings.ollama_url}</code>.
              {ollamaUp === true && <span className="fb-up"> · ollama detected ✓</span>}
              {ollamaUp === false && <span className="fb-down"> · ollama not running</span>}
            </div>
          </div>

          <div className="fb-section">
            <label className="fb-label">Groq API key</label>
            <input
              className="fb-input"
              type="password"
              value={settings.groq_api_key}
              onChange={(e) => update("groq_api_key", e.target.value)}
              placeholder="gsk_…"
            />
            <div className="fb-hint">
              Bring your own key — get one free at{" "}
              <a href="https://console.groq.com/keys" target="_blank" rel="noreferrer">console.groq.com/keys</a>.
            </div>
          </div>

          <div className="fb-section">
            <label className="fb-label">Local model (Ollama)</label>
            <input
              className="fb-input"
              value={settings.ollama_model}
              onChange={(e) => update("ollama_model", e.target.value)}
            />
            <div className="fb-hint">
              Recommended: <code>qwen2.5:1.5b</code>. Run <code>ollama pull qwen2.5:1.5b</code> once.
            </div>
          </div>

          <div className="fb-section">
            <label className="fb-label">Dictionary</label>
            <textarea
              className="fb-input fb-textarea"
              rows={3}
              value={settings.dictionary.join("\n")}
              onChange={(e) => update("dictionary", e.target.value.split("\n").map(s => s.trim()).filter(Boolean))}
              placeholder="One name or term per line. e.g.&#10;ClickFunnels&#10;Spontent&#10;Russell"
            />
            <div className="fb-hint">
              Brand names, jargon, project names. Cleanup preserves these spellings even when Whisper hears them slightly off.
            </div>
          </div>

          <div className="fb-section">
            <label className="fb-label">History retention</label>
            <div className="fb-radios">
              {RETENTION_OPTIONS.map(opt => (
                <button
                  key={opt.days}
                  className={`fb-pill ${settings.history_retention_days === opt.days ? "on" : ""}`}
                  onClick={() => update("history_retention_days", opt.days)}
                >{opt.label}</button>
              ))}
            </div>
            <div className="fb-hint">
              History is local-only — never sent to any cloud. Old entries auto-delete on save / launch. Stored at <code>~/.funbutton/history.db</code>.
            </div>
          </div>

          <div className="fb-stats">
            <span><b>{settings.words_today}</b> words today</span>
            {last && (
              <span>last: <b>{last.word_count}</b> words · <b>{last.mode}</b> mode · <b>{last.backend}</b></span>
            )}
          </div>

          <div className="fb-actions">
            <button className="fb-btn" onClick={save}>{saved ? "saved ✓" : "save"}</button>
          </div>

          <div className="fb-section">
            <label className="fb-label">Help</label>
            <button
              className="fb-btn-small"
              onClick={() => invoke("open_onboarding")}
              style={{ alignSelf: "flex-start" }}
            >Replay onboarding ↻</button>
            <div className="fb-hint">Walks through the Fn key intro, the three permissions, and the cleanup setup again.</div>
          </div>

          <footer className="fb-footer">
            v0.1.0 · GPLv3 · <a href="https://github.com/todddickerson/funbutton" target="_blank" rel="noreferrer">github</a>
          </footer>
        </div>
      ) : tab === "license" ? (
        <LicensePanel
          settings={settings}
          info={licenseInfo}
          validating={licenseValidating}
          error={licenseError}
          draftJwt={licenseDraftJwt}
          setDraftJwt={setLicenseDraftJwt}
          onValidate={() => validateLicense(licenseDraftJwt.trim())}
          onClear={async () => {
            if (!settings) return;
            const next: Settings = { ...settings, license_jwt: "" };
            setSettings(next);
            await invoke("save_settings", { settings: next });
            setLicenseInfo(null);
            setLicenseDraftJwt("");
            pushToast("info", "License cleared. Back to BYOK mode.");
          }}
          capDraftCents={capDraftCents}
          setCapDraftCents={(c) => {
            // If moving from $0 to >$0, gate behind disclosure.
            if ((licenseInfo?.cap_cents ?? 0) === 0 && c > 0) {
              setCapDraftCents(c);
              setShowCapDisclosure(true);
            } else {
              setCapDraftCents(c);
            }
          }}
          onCommitCap={() => commitCap(capDraftCents)}
          onChangePremiumModel={async (m) => {
            if (!settings) return;
            const next: Settings = { ...settings, premium_model: m };
            setSettings(next);
            await invoke("save_settings", { settings: next });
          }}
          onOpenPortal={openPortal}
        />
      ) : (
        // history tab
        <div className="fb-form">
          {lastFailed && (
            <div className="fb-banner-fail">
              <div className="fb-banner-title">Last paste did not land</div>
              <div className="fb-banner-body">
                <span className="fb-banner-text">&ldquo;{truncate(lastFailed.cleaned_text, 120)}&rdquo;</span>
                <button className="fb-btn-small" onClick={() => copyEntry(lastFailed.id)}>
                  {copiedId === lastFailed.id ? "copied ✓" : "copy to clipboard"}
                </button>
              </div>
            </div>
          )}

          <div className="fb-history-controls">
            <input
              className="fb-input fb-history-search"
              placeholder="search transcripts…"
              value={historyQuery}
              onChange={(e) => setHistoryQuery(e.target.value)}
            />
            <select
              className="fb-input fb-history-filter"
              value={historyMode}
              onChange={(e) => setHistoryMode(e.target.value)}
            >
              <option value="all">all modes</option>
              {modesInHistory.map(m => <option key={m} value={m}>{m}</option>)}
            </select>
          </div>

          {history.length === 0 ? (
            <div className="fb-history-empty">No history yet. Hold Right Option and dictate something.</div>
          ) : (
            <div className="fb-history-list">
              {history.map(h => (
                <div key={h.id} className={`fb-history-row ${h.paste_succeeded ? "" : "fb-history-failed"}`}>
                  <div className="fb-history-meta">
                    <span className="fb-history-ts">{fmtTs(h.ts)}</span>
                    <span className="fb-history-mode">{h.mode_used}</span>
                    {h.frontmost_app && <span className="fb-history-app">{h.frontmost_app}</span>}
                    {h.audio_duration_ms != null && <span className="fb-history-dur">{(h.audio_duration_ms / 1000).toFixed(1)}s</span>}
                    {!h.paste_succeeded && <span className="fb-history-flag">paste failed</span>}
                  </div>
                  <div className="fb-history-cleaned">{h.cleaned_text}</div>
                  {h.raw_transcript !== h.cleaned_text && (
                    <details className="fb-history-raw-wrap">
                      <summary>raw</summary>
                      <div className="fb-history-raw">{h.raw_transcript}</div>
                    </details>
                  )}
                  <div className="fb-history-actions">
                    <button className="fb-btn-small" onClick={() => copyEntry(h.id)}>
                      {copiedId === h.id ? "copied ✓" : "copy"}
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}

          <footer className="fb-footer">
            local-only · {history.length} entries · auto-delete after {settings.history_retention_days === 0 ? "never" : `${settings.history_retention_days} days`}
          </footer>
        </div>
      )}

      {showCapDisclosure && (
        <CapDisclosureModal
          capCents={capDraftCents}
          onEnable={async () => {
            setShowCapDisclosure(false);
            await commitCap(capDraftCents);
          }}
          onCancel={() => {
            setShowCapDisclosure(false);
            setCapDraftCents(0);
          }}
        />
      )}

      <div className="fb-toast-rail">
        {toasts.map((t) => (
          <div key={t.id} className={`fb-toast fb-toast-${t.kind}`}>{t.text}</div>
        ))}
      </div>
    </main>
  );
}

// -------------------- License panel + Cap disclosure --------------------

interface LicensePanelProps {
  settings: Settings;
  info: LicenseInfo | null;
  validating: boolean;
  error: string | null;
  draftJwt: string;
  setDraftJwt: (v: string) => void;
  onValidate: () => void;
  onClear: () => void;
  capDraftCents: number;
  setCapDraftCents: (v: number) => void;
  onCommitCap: () => void;
  onChangePremiumModel: (m: PremiumModel) => void;
  onOpenPortal: () => void;
}

const PREMIUM_MODELS: { value: PremiumModel; label: string; rate: string }[] = [
  { value: "fast", label: "Fast (free)", rate: "Groq Llama 3.3 · included" },
  { value: "premium-haiku", label: "Haiku 4.5", rate: "$0.40 / 10K words · best $/quality" },
  { value: "premium-sonnet", label: "Sonnet 4.7", rate: "$0.60 / 10K words · long-form" },
  { value: "premium-opus", label: "Opus 4.7", rate: "$0.99 / 10K words · reasoning" },
  { value: "premium-gpt41", label: "GPT-4.1", rate: "$0.50 / 10K words · alt provider" },
];

function LicensePanel(p: LicensePanelProps) {
  const { settings, info } = p;
  const hasLicense = !!settings.license_jwt && !!info?.valid;
  const includedRemaining = info ? Math.max(0, info.included_premium_words - info.words_used_this_month) : 0;

  return (
    <div className="fb-form">
      {!hasLicense && (
        <div className="fb-section">
          <label className="fb-label">No license — running in BYOK mode</label>
          <div className="fb-hint">
            FunButton stays 100% functional on the free tier (Groq BYOK or local Ollama).
            Upgrade unlocks Claude Haiku / Sonnet / Opus / GPT-4.1 cleanup, 50K
            premium words/mo included on Pro, and metered overage with a user-set cap.
          </div>
          <a
            className="fb-btn"
            href="https://funbutton.ai/#pricing"
            target="_blank"
            rel="noreferrer"
            style={{ alignSelf: "flex-start", marginTop: 12, textDecoration: "none" }}
          >See pricing →</a>
        </div>
      )}

      <div className="fb-section">
        <label className="fb-label">{hasLicense ? "License" : "Activate license"}</label>
        {hasLicense ? (
          <div className="fb-license-summary">
            <div className="fb-license-row">
              <span className="fb-license-key">Tier</span>
              <span className="fb-license-val">{info!.tier.replace(/_/g, " ")}</span>
            </div>
            <div className="fb-license-row">
              <span className="fb-license-key">JWT expires</span>
              <span className="fb-license-val">{new Date(info!.expires_at).toLocaleDateString()}</span>
            </div>
            <div className="fb-license-row">
              <span className="fb-license-key">Included premium words</span>
              <span className="fb-license-val">
                {info!.included_premium_words === 0
                  ? "0 (pay-as-you-go)"
                  : `${includedRemaining.toLocaleString()} / ${info!.included_premium_words.toLocaleString()} remaining`}
              </span>
            </div>
            <div className="fb-license-row">
              <span className="fb-license-key">Active cap</span>
              <span className="fb-license-val">
                {info!.cap_cents === 0 ? "$0 (hard stop / fast tier only)" : `$${(info!.cap_cents / 100).toFixed(0)}/mo`}
              </span>
            </div>
            <div className="fb-license-actions">
              <button className="fb-btn-small" onClick={p.onOpenPortal}>
                Manage subscription ↗
              </button>
              <button className="fb-btn-small fb-btn-danger" onClick={p.onClear}>
                Sign out (BYOK)
              </button>
            </div>
          </div>
        ) : (
          <>
            <textarea
              className="fb-input fb-textarea"
              rows={3}
              placeholder="Paste your license JWT (received via email after purchase)"
              value={p.draftJwt}
              onChange={(e) => p.setDraftJwt(e.target.value)}
            />
            <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
              <button
                className="fb-btn"
                onClick={p.onValidate}
                disabled={p.validating || !p.draftJwt.trim()}
              >
                {p.validating ? "validating…" : "activate license"}
              </button>
            </div>
            {p.error && <div className="fb-hint fb-down">✗ {p.error}</div>}
          </>
        )}
      </div>

      {hasLicense && (
        <>
          <div className="fb-section">
            <label className="fb-label">Premium model preference</label>
            <div className="fb-radios">
              {PREMIUM_MODELS.map((m) => (
                <button
                  key={m.value}
                  className={`fb-pill ${settings.premium_model === m.value ? "on" : ""}`}
                  onClick={() => p.onChangePremiumModel(m.value)}
                >{m.label}</button>
              ))}
            </div>
            <div className="fb-hint">
              {PREMIUM_MODELS.find(m => m.value === settings.premium_model)?.rate}
              <br />
              Cleanup falls back to fast tier automatically if your monthly cap is hit.
            </div>
          </div>

          <div className="fb-section">
            <label className="fb-label">Monthly cap (auto top-up)</label>
            <div className="fb-cap-slider-wrap">
              <input
                type="range"
                min={0}
                max={10000}
                step={500}
                value={p.capDraftCents}
                onChange={(e) => p.setCapDraftCents(parseInt(e.target.value, 10))}
                className="fb-cap-slider"
              />
              <div className="fb-cap-value">
                {p.capDraftCents === 0 ? "$0 — OFF" : `$${(p.capDraftCents / 100).toFixed(0)}/mo`}
              </div>
            </div>
            <div className="fb-hint">
              <strong>$0 = hard stop.</strong> When your spend hits the cap (or any time at $0),
              cleanup silently falls back to free Groq fast tier with a toast.
              You can raise / lower / disable this any time. Opt-in OFF by default.
            </div>
            {p.capDraftCents !== info?.cap_cents && (
              <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
                <button className="fb-btn-small" onClick={p.onCommitCap}>
                  Save cap
                </button>
                <button
                  className="fb-btn-small"
                  onClick={() => p.setCapDraftCents(info?.cap_cents ?? 0)}
                >cancel</button>
              </div>
            )}
          </div>
        </>
      )}

      <footer className="fb-footer">
        api: <code>{settings.cloud_api_base}</code>
      </footer>
    </div>
  );
}

function CapDisclosureModal({
  capCents,
  onEnable,
  onCancel,
}: {
  capCents: number;
  onEnable: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="fb-modal-overlay" onClick={onCancel}>
      <div className="fb-modal" onClick={(e) => e.stopPropagation()}>
        <div className="fb-modal-title">Enable auto top-up?</div>
        <div className="fb-modal-body">
          By enabling auto top-up, you authorize FunButton to charge your saved
          card up to <strong>${(capCents / 100).toFixed(0)} per month</strong> for
          premium model usage above any included quota.
          <br /><br />
          You can change this amount or disable it any time in Settings → License.
          We'll email you an itemized receipt every month, and you can cancel
          your subscription with one click.
        </div>
        <div className="fb-modal-actions">
          <button className="fb-btn-small" onClick={onCancel}>Cancel</button>
          <button className="fb-btn" onClick={onEnable}>
            Enable ${(capCents / 100).toFixed(0)}/mo cap
          </button>
        </div>
      </div>
    </div>
  );
}

function truncate(s: string, n: number): string {
  return s.length > n ? s.slice(0, n - 1) + "…" : s;
}

function fmtTs(ts: number): string {
  const d = new Date(ts * 1000);
  const today = new Date();
  const sameDay = d.toDateString() === today.toDateString();
  const time = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  if (sameDay) return time;
  return `${d.toLocaleDateString([], { month: "short", day: "numeric" })} ${time}`;
}

export default App;
