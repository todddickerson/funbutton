import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type Backend = "auto" | "groq" | "local";

interface Settings {
  groq_api_key: string;
  backend: Backend;
  ollama_model: string;
  ollama_url: string;
  words_today: number;
  words_today_date: string;
  hotkey_label: string;
}

interface ResultPayload {
  raw: string;
  cleaned: string;
  mode: string;
  backend: string;
  word_count: number;
}

function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [status, setStatus] = useState("idle");
  const [statusMsg, setStatusMsg] = useState<string | null>(null);
  const [last, setLast] = useState<ResultPayload | null>(null);
  const [ollamaUp, setOllamaUp] = useState<boolean | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    invoke<Settings>("get_settings").then(setSettings);
    invoke<string>("get_status").then(setStatus);
    invoke<boolean>("ollama_check").then(setOllamaUp).catch(() => setOllamaUp(false));

    const unS = listen<{ status: string; message: string | null }>("funbutton:status", (e) => {
      setStatus(e.payload.status);
      setStatusMsg(e.payload.message ?? null);
    });
    const unR = listen<ResultPayload>("funbutton:result", (e) => {
      setLast(e.payload);
      invoke<Settings>("get_settings").then(setSettings);
    });
    return () => {
      unS.then((u) => u());
      unR.then((u) => u());
    };
  }, []);

  function update<K extends keyof Settings>(k: K, v: Settings[K]) {
    if (!settings) return;
    setSettings({ ...settings, [k]: v });
  }

  async function save() {
    if (!settings) return;
    await invoke("save_settings", { settings });
    setSaved(true);
    setTimeout(() => setSaved(false), 1400);
  }

  const dot = status === "recording" ? "#ff5050" :
              status === "transcribing" || status === "cleaning" || status === "pasting" ? "#ffaa00" :
              status === "error" ? "#ff5050" : "#9a9a9a";

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

      {!settings ? (
        <div className="fb-loading">loading…</div>
      ) : (
        <div className="fb-form">
          <div className="fb-section">
            <label className="fb-label">Hotkey</label>
            <div className="fb-readonly">{settings.hotkey_label}</div>
            <div className="fb-hint">Push-to-talk. Hold to record, release to transcribe.</div>
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

          <div className="fb-stats">
            <span><b>{settings.words_today}</b> words today</span>
            {last && (
              <span>last: <b>{last.word_count}</b> words · <b>{last.mode}</b> mode · <b>{last.backend}</b></span>
            )}
          </div>

          {last && (
            <div className="fb-last">
              <div className="fb-last-label">cleaned</div>
              <div className="fb-last-cleaned">{last.cleaned}</div>
              <div className="fb-last-label">raw</div>
              <div className="fb-last-raw">{last.raw}</div>
            </div>
          )}

          <div className="fb-actions">
            <button className="fb-btn" onClick={save}>{saved ? "saved ✓" : "save"}</button>
          </div>

          <footer className="fb-footer">
            v0.1.0 · GPLv3 · <a href="https://github.com/todddickerson/funbutton" target="_blank" rel="noreferrer">github</a>
          </footer>
        </div>
      )}
    </main>
  );
}

export default App;
