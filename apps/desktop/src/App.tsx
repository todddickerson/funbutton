import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AlertTriangle, ChevronRight } from "lucide-react";
import "./App.css";
import { HotkeyPicker } from "./HotkeyPicker";
import { hotkeyHold, hotkeyName, hotkeyShort, type HotkeyKind } from "./hotkeys";
import { ModelManager, type EngineStatus } from "./models";

type Backend = "auto" | "groq" | "local" | "embedded";
type SttBackend = "local" | "groq";
type ModeOverride = "auto" | "code" | "email" | "slack" | "raw";

type PremiumModel = "fast" | "premium-haiku" | "premium-sonnet" | "premium-opus" | "premium-gpt41";

interface Settings {
  groq_api_key: string;
  backend: Backend;
  stt_backend: SttBackend;
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
  stt_model_id: string;
  cleanup_model_id: string;
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

// Mirrors src-tauri/src/app_detect.rs + cleanup.rs routing. Presentation only —
// the actual decision happens in Rust on every dictation.
const MODE_MAP: { mode: string; apps: string[]; more?: string; what: string }[] = [
  {
    mode: "code",
    apps: ["Cursor", "VS Code", "JetBrains", "Xcode", "Vim", "Zed", "Windsurf"],
    more: "+ AI IDEs, git clients, Sublime, Emacs",
    what: "identifiers stay identifiers. no prose padding.",
  },
  {
    mode: "term",
    apps: ["Terminal", "iTerm2", "Warp", "Ghostty", "kitty"],
    more: "+ Alacritty, WezTerm, Tabby, Hyper",
    what: "literal commands, exactly as spoken.",
  },
  {
    mode: "email",
    apps: ["Mail"],
    what: "greeting, clean paragraphs, sign-off.",
  },
  {
    mode: "slack",
    apps: ["Slack", "Discord", "Messages"],
    what: "casual and short. no corporate polish.",
  },
];

function App() {
  const [tab, setTab] = useState<Tab>("settings");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [status, setStatus] = useState("idle");
  const [statusMsg, setStatusMsg] = useState<string | null>(null);
  const [last, setLast] = useState<ResultPayload | null>(null);
  const [ollamaUp, setOllamaUp] = useState<boolean | null>(null);
  const [cleanupStatus, setCleanupStatus] = useState<EngineStatus>("starting");
  const [sttStatus, setSttStatus] = useState<EngineStatus>("starting");
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
  // Permissions snapshot (refreshed on mount, on window focus, and after Grant).
  const [perms, setPerms] = useState<{ microphone: boolean; accessibility: boolean; input_monitoring: boolean } | null>(null);
  // Welcome card is first-run only: dismissed once, stays gone (window-local flag,
  // independent of the onboarding wizard's settings.onboarded).
  const [welcomeDismissed, setWelcomeDismissed] = useState<boolean>(
    () => localStorage.getItem("fb_welcome_dismissed") === "1"
  );
  // All-granted permissions collapse to one row; click re-expands.
  const [permsOpen, setPermsOpen] = useState(false);

  function dismissWelcome() {
    localStorage.setItem("fb_welcome_dismissed", "1");
    setWelcomeDismissed(true);
  }

  async function refreshPerms() {
    try {
      const [mic, acc, im] = await Promise.all([
        invoke<boolean>("plugin:macos-permissions|check_microphone_permission"),
        invoke<boolean>("plugin:macos-permissions|check_accessibility_permission"),
        invoke<boolean>("plugin:macos-permissions|check_input_monitoring_permission"),
      ]);
      setPerms({ microphone: mic, accessibility: acc, input_monitoring: im });
    } catch (e) {
      console.warn("perms check failed", e);
    }
  }

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

  function recheckEngines() {
    setOllamaUp(null);
    invoke<boolean>("ollama_check").then(setOllamaUp).catch(() => setOllamaUp(false));
    // Snapshot both embedded engines — ready/failed events may have fired
    // before this window mounted.
    invoke<{ cleanup: EngineStatus; stt: EngineStatus }>("embedded_check")
      .then((s) => {
        setSttStatus(s.stt);
        setCleanupStatus(s.cleanup);
      })
      .catch(() => {});
  }

  useEffect(() => {
    invoke<Settings>("get_settings").then(setSettings);
    invoke<string>("get_status").then(setStatus);
    recheckEngines();
    refreshPerms();
    refreshHistory();

    // Re-check perms whenever the window regains focus (after a System
    // Settings round-trip the user will tab back to FunButton).
    const onFocus = () => refreshPerms();
    window.addEventListener("focus", onFocus);

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
    const unEmbReady = listen("funbutton:embedded-ready", () => {
      setCleanupStatus("ready");
      pushToast("ok", "On-device cleanup model ready — no API key needed.");
    });
    const unEmbFail = listen<string>("funbutton:embedded-failed", (e) => {
      setCleanupStatus("failed");
      console.warn("embedded LLM failed:", e.payload);
    });
    const unSttReady = listen("funbutton:stt-ready", () => setSttStatus("ready"));
    const unSttFail = listen<string>("funbutton:stt-failed", (e) => {
      setSttStatus("failed");
      console.warn("embedded STT failed:", e.payload);
    });
    const unL = listen("funbutton:license-activated", () => {
      invoke<Settings>("get_settings").then((s) => {
        setSettings(s);
        if (s.license_jwt) {
          setTab("license");
          // immediately verify against the worker
          invoke<LicenseInfo>("validate_license", { jwt: s.license_jwt })
            .then((info) => {
              setLicenseInfo(info);
              setCapDraftCents(info.cap_cents);
              pushToast("ok", `License activated · ${info.tier.replace("_", " ")}`);
            })
            .catch((e) => pushToast("warn", `Activation verify failed: ${e}`));
        }
      });
    });
    return () => {
      unS.then((u) => u());
      unR.then((u) => u());
      unF.then((u) => u());
      unH.then((u) => u());
      unL.then((u) => u());
      unEmbReady.then((u) => u());
      unEmbFail.then((u) => u());
      unSttReady.then((u) => u());
      unSttFail.then((u) => u());
      window.removeEventListener("focus", onFocus);
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

  function flashSaved() {
    setSaved(true);
    setTimeout(() => setSaved(false), 1400);
  }

  async function persist(next: Settings, purge = false) {
    setSettings(next);
    await invoke("save_settings", { settings: next });
    if (purge) await invoke("history_purge_now").catch(() => {});
    flashSaved();
  }

  // Pills and chips save instantly — no separate save step for discrete choices.
  function setAndSave<K extends keyof Settings>(k: K, v: Settings[K], purge = false) {
    if (!settings) return;
    void persist({ ...settings, [k]: v }, purge);
  }

  // Text fields save on blur.
  async function saveDrafts() {
    if (!settings) return;
    await persist(settings);
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

  const dot = status === "recording" ? "var(--bad)" :
              status === "transcribing" || status === "cleaning" || status === "pasting" ? "var(--warn)" :
              status === "error" ? "var(--bad)" : "var(--dim)";

  const modesInHistory = useMemo(() => {
    const set = new Set(history.map(h => h.mode_used));
    return Array.from(set).sort();
  }, [history]);

  const historyGroups = useMemo(() => {
    const out: { label: string; items: HistoryEntry[] }[] = [];
    for (const h of history) {
      const label = dayLabel(h.ts);
      if (!out.length || out[out.length - 1].label !== label) {
        out.push({ label, items: [] });
      }
      out[out.length - 1].items.push(h);
    }
    return out;
  }, [history]);

  const lastRouted = history.length > 0 ? history[0] : null;

  const hasGroqKey = (settings?.groq_api_key.trim() ?? "") !== "";

  // Mirrors pipeline.rs auto order: embedded → ollama → groq.
  function autoRouteLabel(): string {
    if (!settings) return "";
    if (settings.backend === "embedded") return "pinned → on-device model";
    if (settings.backend === "local") return "pinned → ollama";
    if (settings.backend === "groq") return "pinned → groq cloud";
    if (cleanupStatus === "ready") return "auto → on-device";
    if (ollamaUp) return "auto → ollama";
    if (hasGroqKey) return "auto → groq cloud";
    if (cleanupStatus === "downloading") return "auto → downloading model…";
    if (cleanupStatus === "missing") return "auto → no engine (get a model below)";
    return cleanupStatus === "failed" ? "auto → no engine up" : "auto → warming up";
  }

  return (
    <main className="fb-root">
      <header className="fb-header">
        <div className="fb-brand">
          <span className="fb-logo">●</span>
          <span className="fb-name">FunButton</span>
          <span className="fb-tag">talk fast. stay local. pay less.</span>
        </div>
        <div className="fb-status">
          {settings && settings.words_today > 0 && (
            <>
              <span className="fb-header-words" title="words dictated today">
                <b>{settings.words_today.toLocaleString()}</b> words today
              </span>
              <span className="fb-header-sep" aria-hidden />
            </>
          )}
          <span className="fb-dot" style={{ background: dot }} />
          <span className="fb-status-label">{status}{statusMsg ? ` — ${statusMsg}` : ""}</span>
          {saved && <span className="fb-savetick">saved ✓</span>}
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
        <div className="fb-scroll">
        <div className="fb-form">
          {!welcomeDismissed && (
            <div className="fb-welcome">
              <button
                className="fb-welcome-x"
                aria-label="dismiss intro"
                title="dismiss — replay any time via Replay onboarding below"
                onClick={dismissWelcome}
              >×</button>
              <div className="fb-welcome-title">meet the Fun Button.</div>
              <div className="fb-welcome-body">
                <strong>FunButton = Fn Button.</strong> On MacBooks it&apos;s the
                bottom-left key nobody used — we gave it a job. Different keyboard?
                Pick any key below.<br/>
                <strong>Zero setup.</strong> Speech-to-text and cleanup run on on-device
                models — a small first-run download (~1.1GB), then no account, no key, works
                on a plane.<br/>
                Hold <kbd>{hotkeyHold(settings.hotkey_kind)}</kbd> in any text field, talk, release. Grant the three
                permissions below and you're dictating.
              </div>
            </div>
          )}

          <div className="fb-section">
            <div className="fb-label-row">
              <label className="fb-label">The Button</label>
              <span className="fb-label-aux">armed: {hotkeyShort(settings.hotkey_kind)}</span>
            </div>
            <HotkeyPicker
              selected={settings.hotkey_kind}
              onPick={(k) => {
                // Hot-swap: persist immediately so the Rust side flips the
                // armed-hotkey atomic on save_settings. No app restart needed.
                setAndSave("hotkey_kind", k);
                pushToast("ok", `Hotkey armed: ${hotkeyName(k)}`);
              }}
              variant="settings"
            />

            {perms && perms.microphone && perms.accessibility && perms.input_monitoring && (
              <button
                className={`fb-perms-compact ${permsOpen ? "open" : ""}`}
                aria-expanded={permsOpen}
                onClick={() => setPermsOpen(o => !o)}
              >
                <span className="fb-perms-compact-dot" aria-hidden>●</span>
                <span className="fb-perms-compact-name">permissions</span>
                <span className="fb-perms-compact-hint">mic · accessibility · input monitoring</span>
                <span className="fb-perms-compact-state">3/3 granted</span>
                <ChevronRight size={12} className="fb-perms-chev" aria-hidden />
              </button>
            )}
            {perms && (permsOpen || !(perms.microphone && perms.accessibility && perms.input_monitoring)) && (
              <div className="fb-perms">
                <PermRow
                  name="Microphone"
                  granted={perms.microphone}
                  why="record what you say"
                  onGrant={async () => {
                    await invoke("plugin:macos-permissions|request_microphone_permission").catch(() => {});
                    setTimeout(refreshPerms, 600);
                  }}
                />
                <PermRow
                  name="Accessibility"
                  granted={perms.accessibility}
                  why="paste the cleaned text at your cursor (⌘V)"
                  onGrant={async () => {
                    await invoke("plugin:macos-permissions|request_accessibility_permission").catch(() => {});
                    setTimeout(refreshPerms, 600);
                  }}
                />
                <PermRow
                  name="Input Monitoring"
                  granted={perms.input_monitoring}
                  why="required to see your button — every key works this way"
                  required={true}
                  onGrant={async () => {
                    await invoke("plugin:macos-permissions|request_input_monitoring_permission").catch(() => {});
                    setTimeout(refreshPerms, 600);
                  }}
                />
              </div>
            )}
            {perms && !perms.input_monitoring && (
              <div className="fb-callout">
                <AlertTriangle size={13} aria-hidden />
                <span>{hotkeyName(settings.hotkey_kind)} will NOT fire without Input Monitoring. Grant it above — picked up within seconds, no relaunch.</span>
              </div>
            )}

            <div className="fb-testrow">
              <button
                className="fb-btn-small"
                onClick={async () => {
                  pushToast("info", "Simulating Down→Up (1.5s)…");
                  try {
                    await invoke("simulate_hotkey", { durationMs: 1500 });
                    pushToast("ok", "Hotkey simulated — pipeline should record, transcribe, and paste at your cursor");
                  } catch (e) {
                    pushToast("warn", `Simulate failed: ${e}`);
                  }
                }}
              >Test the button</button>
              <span className="fb-hint">
                bypasses the key listener. works here but not when holding {hotkeyName(settings.hotkey_kind)}? macOS is blocking the listener — grant the permission above.
              </span>
            </div>
            <div className="fb-hint">
              <kbd>⌘⇧V</kbd> re-pastes last · <kbd>⌘⇧H</kbd> opens history · hotkey switches apply instantly
            </div>
          </div>

          <div className="fb-section">
            <div className="fb-label-row">
              <label className="fb-label">Engines</label>
              <span className="fb-label-aux">{autoRouteLabel()}</span>
              <button className="fb-linkbtn" onClick={recheckEngines}>recheck</button>
            </div>
            <div className="fb-engines">
              <EngineRow
                state={engineRowState(sttStatus)}
                name="whisper"
                detail="on-device speech-to-text"
                label={engineRowLabel(sttStatus)}
              />
              <EngineRow
                state={engineRowState(cleanupStatus)}
                name="cleanup"
                detail="on-device model · manage below"
                label={engineRowLabel(cleanupStatus)}
              />
              <EngineRow
                state={ollamaUp === true ? "ready" : ollamaUp === null ? "busy" : "off"}
                name="ollama"
                detail={`${settings.ollama_model} @ ${settings.ollama_url.replace(/^https?:\/\//, "")}`}
                label={ollamaUp === true ? "detected" : ollamaUp === null ? "checking" : "not running"}
              />
              <EngineRow
                state={hasGroqKey ? "ready" : "off"}
                name="groq cloud"
                detail="whisper turbo + llama 3.3 · BYOK"
                label={hasGroqKey ? "key set" : "no key"}
              />
            </div>

            <div className="fb-selrow">
              <span className="fb-selname">speech-to-text</span>
              <div className="fb-radios fb-radios-grow">
                {(["local", "groq"] as const).map(b => (
                  <button
                    key={b}
                    className={`fb-pill ${settings.stt_backend === b ? "on" : ""}`}
                    onClick={() => setAndSave("stt_backend", b)}
                  >{b === "local" ? "on-device" : "groq cloud"}</button>
                ))}
              </div>
            </div>
            <div className="fb-selrow">
              <span className="fb-selname">cleanup</span>
              <div className="fb-radios fb-radios-grow">
                {(["auto","embedded","groq","local"] as const).map(b => (
                  <button
                    key={b}
                    className={`fb-pill ${settings.backend === b ? "on" : ""}`}
                    onClick={() => setAndSave("backend", b)}
                  >{b === "embedded" ? "bundled" : b === "local" ? "ollama" : b}</button>
                ))}
              </div>
            </div>
            <div className="fb-hint">
              <strong>auto</strong> tries bundled → ollama → groq and takes the first one alive.
              whichever you pin, the rest stay as silent fallbacks. audio never leaves this Mac
              unless you pick groq.
            </div>

            <div className="fb-fieldrow">
              <span className="fb-selname">groq key</span>
              <input
                className="fb-input"
                type="password"
                value={settings.groq_api_key}
                onChange={(e) => update("groq_api_key", e.target.value)}
                onBlur={saveDrafts}
                placeholder="gsk_… (optional — everything works without it)"
              />
            </div>
            <div className="fb-fieldrow">
              <span className="fb-selname">ollama model</span>
              <input
                className="fb-input"
                value={settings.ollama_model}
                onChange={(e) => update("ollama_model", e.target.value)}
                onBlur={saveDrafts}
              />
            </div>
            <div className="fb-hint">
              free key at <a href="https://console.groq.com/keys" target="_blank" rel="noreferrer">console.groq.com/keys</a> —
              stored in your macOS Keychain, never plaintext on disk.
              ollama: run <code>ollama pull qwen2.5:1.5b</code> once.
            </div>
          </div>

          <div className="fb-section">
            <div className="fb-label-row">
              <label className="fb-label">Models</label>
              <span className="fb-label-aux">downloaded, not bundled · pick your own</span>
            </div>
            <ModelManager pushToast={pushToast} />
          </div>

          <div className="fb-section">
            <div className="fb-label-row">
              <label className="fb-label">Modes</label>
              <span className="fb-label-aux">
                {settings.mode_override === "auto"
                  ? lastRouted && lastRouted.frontmost_app
                    ? `last: ${lastRouted.frontmost_app} → ${lastRouted.mode_used}`
                    : "routed per app, per dictation"
                  : `override on: everything → ${settings.mode_override}`}
              </span>
            </div>
            <div className="fb-radios">
              {(["auto","code","email","slack","raw"] as const).map(m => (
                <button
                  key={m}
                  className={`fb-pill ${settings.mode_override === m ? "on" : ""}`}
                  onClick={() => setAndSave("mode_override", m)}
                >{m}</button>
              ))}
            </div>
            <div className={`fb-modemap ${settings.mode_override !== "auto" ? "bypassed" : ""}`}>
              {MODE_MAP.map(row => (
                <div className="fb-modemap-row" key={row.mode}>
                  <span className="fb-mode-tag">{row.mode}</span>
                  <div className="fb-modemap-apps">
                    {row.apps.map(a => <span className="fb-appchip" key={a}>{a}</span>)}
                    {row.more && <span className="fb-appchip more">{row.more}</span>}
                    <span className="fb-modemap-what">{row.what}</span>
                  </div>
                </div>
              ))}
              <div className="fb-modemap-row">
                <span className="fb-mode-tag dim">auto</span>
                <div className="fb-modemap-apps">
                  <span className="fb-modemap-what">everything else — cleanup reads intent from what you said.</span>
                </div>
              </div>
            </div>
            {settings.mode_override !== "auto" && (
              <div className="fb-hint">
                per-app routing is paused while an override is set. flip back to <strong>auto</strong> to re-enable it.
              </div>
            )}
          </div>

          <div className="fb-section">
            <div className="fb-label-row">
              <label className="fb-label">Dictionary</label>
              <span className="fb-label-aux">{settings.dictionary.length === 0 ? "empty" : `${settings.dictionary.length} term${settings.dictionary.length === 1 ? "" : "s"}`}</span>
            </div>
            <DictionaryEditor
              words={settings.dictionary}
              onChange={(w) => setAndSave("dictionary", w)}
            />
            <div className="fb-hint">
              brand names, jargon, project names. cleanup keeps these spellings even when
              whisper hears them slightly off. enter or comma to add.
            </div>
          </div>

          <div className="fb-section">
            <div className="fb-label-row">
              <label className="fb-label">History retention</label>
              <span className="fb-label-aux">local-only · ~/.funbutton/history.db</span>
            </div>
            <div className="fb-radios">
              {RETENTION_OPTIONS.map(opt => (
                <button
                  key={opt.days}
                  className={`fb-pill ${settings.history_retention_days === opt.days ? "on" : ""}`}
                  onClick={() => setAndSave("history_retention_days", opt.days, true)}
                >{opt.label}</button>
              ))}
            </div>
            <div className="fb-hint">
              never sent to any cloud. old entries auto-delete on launch and on change.
            </div>
          </div>

          {last && (
            <div className="fb-stats">
              <span>last: <b>{last.word_count}</b> words · <b>{last.mode}</b> · <b>{last.backend}</b></span>
            </div>
          )}

          <div className="fb-section">
            <button
              className="fb-btn-small"
              onClick={() => invoke("open_onboarding")}
              style={{ alignSelf: "flex-start" }}
            >Replay onboarding ↻</button>
            <div className="fb-hint">the keyboard walkthrough, the three permissions, and the cleanup setup — again.</div>
          </div>

          <footer className="fb-footer">
            v0.1.7 · GPLv3 · <a href="https://github.com/todddickerson/funbutton" target="_blank" rel="noreferrer">github</a>
          </footer>
        </div>
        </div>
      ) : tab === "license" ? (
        <div className="fb-scroll">
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
        </div>
      ) : (
        // history tab
        <div className="fb-scroll">
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
            <div className="fb-history-empty">
              nothing yet. hold {hotkeyName(settings.hotkey_kind)} and say something.
            </div>
          ) : (
            <div className="fb-history-list">
              {historyGroups.map(g => (
                <div className="fb-history-group" key={g.label}>
                  <div className="fb-history-day">{g.label}</div>
                  {g.items.map(h => (
                    <div key={h.id} className={`fb-history-row ${h.paste_succeeded ? "" : "fb-history-failed"}`}>
                      <div className="fb-history-meta">
                        <span className="fb-history-ts">{fmtTs(h.ts)}</span>
                        {h.frontmost_app && <span className="fb-history-app">{h.frontmost_app}</span>}
                        <span className="fb-history-mode">{h.mode_used}</span>
                        {h.audio_duration_ms != null && <span className="fb-history-dur">{(h.audio_duration_ms / 1000).toFixed(1)}s</span>}
                        {!h.paste_succeeded && <span className="fb-history-flag">paste failed</span>}
                        <span className="fb-history-spacer" />
                        <button className="fb-btn-small" onClick={() => copyEntry(h.id)}>
                          {copiedId === h.id ? "copied ✓" : "copy"}
                        </button>
                      </div>
                      <div className="fb-history-cleaned">{h.cleaned_text}</div>
                      {h.raw_transcript !== h.cleaned_text && (
                        <details className="fb-history-raw-wrap">
                          <summary>as heard</summary>
                          <div className="fb-history-raw">{h.raw_transcript}</div>
                        </details>
                      )}
                    </div>
                  ))}
                </div>
              ))}
            </div>
          )}

          <footer className="fb-footer">
            local-only · {history.length} entries · auto-delete: {settings.history_retention_days === 0 ? "never" : `${settings.history_retention_days} days`}
          </footer>
        </div>
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

// -------------------- Engine status row --------------------

type EngineRowState = "ready" | "busy" | "failed" | "off";

function engineRowState(s: EngineStatus): EngineRowState {
  switch (s) {
    case "ready": return "ready";
    case "failed": return "failed";
    case "downloading":
    case "starting": return "busy";
    case "missing": return "off";
  }
}

function engineRowLabel(s: EngineStatus): string {
  switch (s) {
    case "ready": return "ready";
    case "failed": return "failed";
    case "downloading": return "downloading";
    case "starting": return "loading";
    case "missing": return "not downloaded";
  }
}

function EngineRow(props: { state: EngineRowState; name: string; detail: string; label: string }) {
  const glyph = props.state === "ready" ? "●" : props.state === "busy" ? "◐" : props.state === "failed" ? "✕" : "○";
  return (
    <div className={`fb-eng eng-${props.state}`}>
      <span className="fb-eng-dot" aria-hidden>{glyph}</span>
      <span className="fb-eng-name">{props.name}</span>
      <span className="fb-eng-detail">{props.detail}</span>
      <span className="fb-eng-state">{props.label}</span>
    </div>
  );
}

// -------------------- Dictionary chip editor --------------------
// Settings.dictionary stays string[] — this is presentation only.

function DictionaryEditor(props: { words: string[]; onChange: (w: string[]) => void }) {
  const { words, onChange } = props;
  const [draft, setDraft] = useState("");

  function commit() {
    const parts = draft.split(/[,\n]/).map(s => s.trim()).filter(Boolean);
    if (parts.length === 0) return;
    const next = [...words];
    for (const p of parts) {
      if (!next.includes(p)) next.push(p);
    }
    setDraft("");
    if (next.length !== words.length) onChange(next);
  }

  return (
    <div className="fb-dict">
      {words.map(w => (
        <span className="fb-chip" key={w}>
          {w}
          <button
            className="fb-chip-x"
            aria-label={`remove ${w}`}
            onClick={() => onChange(words.filter(x => x !== w))}
          >×</button>
        </span>
      ))}
      <input
        className="fb-chip-input"
        value={draft}
        placeholder={words.length === 0 ? "ClickFunnels, kubectl, Qwen…" : "add term…"}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === ",") {
            e.preventDefault();
            commit();
          } else if (e.key === "Backspace" && draft === "" && words.length > 0) {
            onChange(words.slice(0, -1));
          }
        }}
        onBlur={commit}
      />
    </div>
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
          <label className="fb-label">No license. Everything still works.</label>
          <div className="fb-hint">
            the free tier is the whole app — bundled models, Groq BYOK, Ollama, forever.
            a license adds Claude Haiku / Sonnet / Opus / GPT-4.1 cleanup, 50K premium
            words/mo on Pro, and metered overage with a cap you set.
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
              cleanup falls back to fast tier automatically if your monthly cap is hit.
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
              <strong>$0 = hard stop.</strong> when spend hits the cap, cleanup silently
              falls back to the free fast tier and you get a toast. raise, lower, or kill
              it any time. off by default.
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
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function dayLabel(ts: number): string {
  const d = new Date(ts * 1000);
  const now = new Date();
  const today = now.toDateString();
  const yesterday = new Date(now.getTime() - 86400000).toDateString();
  if (d.toDateString() === today) return "today";
  if (d.toDateString() === yesterday) return "yesterday";
  return d.toLocaleDateString([], { month: "short", day: "numeric" });
}

function PermRow(props: {
  name: string;
  granted: boolean;
  why: string;
  required?: boolean;
  onGrant: () => void;
}) {
  const { name, granted, why, required, onGrant } = props;
  return (
    <div className={`fb-perm-row ${granted ? "ok" : required ? "bad" : "warn"}`}>
      <span className="fb-perm-dot" aria-hidden>{granted ? "●" : "○"}</span>
      <span className="fb-perm-name">{name}</span>
      <span className="fb-perm-why">{why}</span>
      {granted ? (
        <span className="fb-perm-state">granted</span>
      ) : (
        <button className="fb-btn-small fb-perm-grant" onClick={onGrant}>
          Grant
        </button>
      )}
    </div>
  );
}

export default App;
