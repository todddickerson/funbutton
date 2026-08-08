import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ClipboardPaste, Keyboard as KeyboardIcon, Lock, Mic, Zap } from "lucide-react";
import "./onboarding.css";

type HotkeyKind = "fn" | "right_option";
type Backend = "auto" | "groq" | "local";
type EngineStatus = "starting" | "ready" | "failed";
interface EmbeddedStatus { cleanup: EngineStatus; stt: EngineStatus }

interface Settings {
  groq_api_key: string;
  backend: Backend;
  ollama_model: string;
  ollama_url: string;
  hotkey_kind: HotkeyKind;
  history_retention_days: number;
  onboarded: boolean;
  // misc fields we don't touch from the wizard
  [k: string]: unknown;
}

type PermState = "unknown" | "granted" | "denied" | "checking";

const STEP_NAMES = ["hello", "switches", "mic", "paste", "fn key", "engines", "go"];

function App() {
  const [step, setStep] = useState(1);
  const [direction, setDirection] = useState<"forward" | "back">("forward");
  const [settings, setSettings] = useState<Settings | null>(null);

  const [micPerm, setMicPerm] = useState<PermState>("unknown");
  const [accPerm, setAccPerm] = useState<PermState>("unknown");
  const [imPerm, setImPerm] = useState<PermState>("unknown");

  const [groqKey, setGroqKey] = useState("");
  const [groqState, setGroqState] = useState<"idle" | "checking" | "ok" | "bad">("idle");
  const [groqError, setGroqError] = useState<string>("");
  const [ollamaUp, setOllamaUp] = useState<boolean | null>(null);
  const [embedded, setEmbedded] = useState<EmbeddedStatus>({ cleanup: "starting", stt: "starting" });

  const [hotkeyKind, setHotkeyKind] = useState<HotkeyKind>("fn");

  // Load existing settings (so a returning user resumes with previous state)
  useEffect(() => {
    invoke<Settings>("get_settings").then((s) => {
      setSettings(s);
      if (s.groq_api_key) {
        setGroqKey(s.groq_api_key);
        setGroqState("ok");
      }
      if (s.hotkey_kind) setHotkeyKind(s.hotkey_kind);
    }).catch(() => {
      // Never strand the user on a loading dot. Every Settings field is
      // serde-defaulted on the Rust side, so running on defaults is safe.
      setSettings({
        groq_api_key: "",
        backend: "auto",
        ollama_model: "qwen2.5:1.5b",
        ollama_url: "http://localhost:11434",
        hotkey_kind: "fn",
        history_retention_days: 30,
        onboarded: false,
      });
    });
  }, []);

  // Permission polling whenever we land on a permission step
  useEffect(() => {
    if (step < 2 || step > 5) return;
    let cancelled = false;
    const tick = async () => {
      try {
        const [m, a, i] = await Promise.all([
          invoke<boolean>("plugin:macos-permissions|check_microphone_permission").catch(() => false),
          invoke<boolean>("plugin:macos-permissions|check_accessibility_permission").catch(() => false),
          invoke<boolean>("plugin:macos-permissions|check_input_monitoring_permission").catch(() => false),
        ]);
        if (cancelled) return;
        // Functional updates — no stale closures. A switch that was granted
        // and now reads false got revoked: surface it as "denied" so the user
        // watches the circuit break instead of a wizard stuck on green.
        const drop = (prev: PermState): PermState => (prev === "granted" ? "denied" : prev);
        setMicPerm((p) => (m ? "granted" : drop(p)));
        setAccPerm((p) => (a ? "granted" : drop(p)));
        setImPerm((p) => (i ? "granted" : drop(p)));
      } catch {}
    };
    tick();
    const id = setInterval(tick, 700);
    return () => { cancelled = true; clearInterval(id); };
  }, [step]);

  // Auto-advance on grant for steps 3/4/5
  useEffect(() => {
    if (step === 3 && micPerm === "granted") {
      const t = setTimeout(() => goto(4), 600);
      return () => clearTimeout(t);
    }
    if (step === 4 && accPerm === "granted") {
      const t = setTimeout(() => goto(5), 600);
      return () => clearTimeout(t);
    }
    if (step === 5 && imPerm === "granted") {
      const t = setTimeout(() => goto(6), 600);
      return () => clearTimeout(t);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [micPerm, accPerm, imPerm, step]);

  // Backend probes on step 6 — poll every 1.5s so the bundled model's
  // "warming up → ready" transition (and a just-started `ollama serve`)
  // shows up live, same pattern as the permission steps.
  useEffect(() => {
    if (step !== 6) return;
    let cancelled = false;
    const tick = () => {
      invoke<boolean>("ollama_check")
        .then((v) => { if (!cancelled) setOllamaUp(v); })
        .catch(() => { if (!cancelled) setOllamaUp(false); });
      invoke<EmbeddedStatus>("embedded_check")
        .then((v) => { if (!cancelled) setEmbedded(v); })
        .catch(() => { if (!cancelled) setEmbedded({ cleanup: "failed", stt: "failed" }); });
    };
    tick();
    const id = setInterval(tick, 1500);
    return () => { cancelled = true; clearInterval(id); };
  }, [step]);

  // Keyboard navigation
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA")) return;
      if (e.key === "Escape") {
        // The footer promises "esc closes" — keep the promise.
        invoke("close_onboarding").catch(() => {});
      } else if (e.key === "ArrowRight" || e.key === "Enter") {
        // Step 7 is the last stop: enter keeps the footer's promise and
        // actually finishes instead of bouncing off the goto() clamp.
        if (step === 7) finish();
        else if (canAdvance()) goto(step + 1);
      } else if (e.key === "ArrowLeft") {
        if (step > 1) goto(step - 1);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  });

  function goto(n: number) {
    setDirection(n > step ? "forward" : "back");
    setStep(Math.max(1, Math.min(7, n)));
  }

  function canAdvance() {
    // Step 2 always advances: the per-permission slides (3-5) ARE the path
    // for a fresh user — they auto-request, open System Settings, and rescue.
    // Dictation needs speech-to-text: the bundled model or a Groq key.
    if (step === 6) return embedded.stt === "ready" || groqState === "ok";
    if (step === 7) return true;
    return true;
  }

  async function requestPerm(kind: "mic" | "acc" | "im") {
    const cmd = {
      mic: "plugin:macos-permissions|request_microphone_permission",
      acc: "plugin:macos-permissions|request_accessibility_permission",
      im: "plugin:macos-permissions|request_input_monitoring_permission",
    }[kind];
    try {
      await invoke(cmd);
    } catch {}
  }

  async function openSysPanel(panel: "microphone" | "accessibility" | "input_monitoring") {
    try {
      await invoke("open_system_settings_panel", { panel });
    } catch (e) { console.error(e); }
  }

  async function validateAndSaveGroq() {
    setGroqState("checking");
    setGroqError("");
    try {
      await invoke<boolean>("validate_groq_key", { key: groqKey });
      setGroqState("ok");
      await persistPartial({ groq_api_key: groqKey });
    } catch (e: unknown) {
      setGroqState("bad");
      setGroqError(typeof e === "string" ? e : "validation failed");
    }
  }

  async function persistPartial(patch: Partial<Settings>) {
    if (!settings) return;
    const merged = { ...settings, ...patch };
    setSettings(merged);
    await invoke("save_settings", { settings: merged });
  }

  async function pickHotkey(kind: HotkeyKind) {
    setHotkeyKind(kind);
    await persistPartial({ hotkey_kind: kind });
  }

  async function finish() {
    // One merged write, one save_settings call — nothing races, nothing
    // gets dropped. A known-bad key is the only thing we refuse to keep.
    const patch: Partial<Settings> = { hotkey_kind: hotkeyKind };
    if (groqKey && groqState !== "bad") patch.groq_api_key = groqKey;
    await persistPartial(patch);
    await invoke("close_onboarding").catch(() => {});
  }

  if (!settings) {
    return <div className="ob-loading">…</div>;
  }

  return (
    <div className="ob-root">
      <header className="ob-header">
        <div className="ob-brand">
          <span className="ob-dot" /> FunButton
        </div>
        <Stepper current={step} total={7} />
      </header>

      <div key={step} className={`ob-stage ${direction}`}>
        {step === 1 && <Step1 onNext={() => goto(2)} onSkip={() => goto(6)} />}
        {step === 2 && (
          <Step2
            mic={micPerm}
            acc={accPerm}
            im={imPerm}
            onWireUp={() => goto(3)}
            onAllDone={() => goto(6)}
          />
        )}
        {step === 3 && (
          <PermSlide
            title="Microphone"
            why="We can't transcribe what we can't hear. Audio never leaves this Mac."
            icon={<Mic size={24} />}
            settingsPath="Privacy & Security → Microphone"
            state={micPerm}
            onRequest={() => requestPerm("mic")}
            onOpen={() => openSysPanel("microphone")}
            onContinue={() => goto(4)}
          />
        )}
        {step === 4 && (
          <PermSlide
            title="Accessibility"
            why="This is how the cleaned text lands at your cursor. We paste, restore your clipboard, and get out."
            icon={<ClipboardPaste size={24} />}
            settingsPath="Privacy & Security → Accessibility"
            state={accPerm}
            onRequest={() => requestPerm("acc")}
            onOpen={() => openSysPanel("accessibility")}
            onContinue={() => goto(5)}
          />
        )}
        {step === 5 && (
          <PermSlide
            title="Input Monitoring"
            why="Fn isn't a normal key. macOS only tells apps about it once this switch is flipped."
            icon={<KeyboardIcon size={24} />}
            settingsPath="Privacy & Security → Input Monitoring"
            state={imPerm}
            onRequest={() => requestPerm("im")}
            onOpen={() => openSysPanel("input_monitoring")}
            onContinue={() => goto(6)}
            tertiary={
              <button className="ob-tertiary" onClick={async () => { await pickHotkey("right_option"); goto(6); }}>
                Hate this one? Use Right Option as the hotkey instead →
              </button>
            }
          />
        )}
        {step === 6 && (
          <Step6
            groqKey={groqKey}
            setGroqKey={(v) => { setGroqKey(v); setGroqState("idle"); }}
            groqState={groqState}
            groqError={groqError}
            onValidate={validateAndSaveGroq}
            ollamaUp={ollamaUp}
            embedded={embedded}
            onNext={() => goto(7)}
          />
        )}
        {step === 7 && <Step7 onFinish={finish} hotkeyKind={hotkeyKind} />}
      </div>

      <footer className="ob-footer">
        <button className="ob-back" onClick={() => goto(step - 1)} disabled={step === 1}>← back</button>
        <span className="ob-help">esc closes · ←/→ move · enter advances</span>
        <span />
      </footer>
    </div>
  );
}

function Stepper({ current, total }: { current: number; total: number }) {
  return (
    <div className="ob-stepper" role="progressbar" aria-valuemin={1} aria-valuemax={total} aria-valuenow={current}>
      <div className="ob-rail">
        {Array.from({ length: total }, (_, i) => {
          const idx = i + 1;
          const cls = idx === current ? "on" : idx < current ? "done" : "future";
          return (
            <span key={idx} className="ob-rail-piece">
              {idx > 1 && <span className={`ob-seg ${idx <= current ? "on" : ""}`} />}
              <span className={`ob-node ${cls}`}>{idx < current ? "✓" : ""}</span>
            </span>
          );
        })}
      </div>
      <span className="ob-step-tag">
        {String(current).padStart(2, "0")}/{String(total).padStart(2, "0")} · {STEP_NAMES[current - 1]}
      </span>
    </div>
  );
}

/** Ring that draws itself closed, then draws the check — the "circuit closing". */
function CheckRing({ size = 26 }: { size?: number }) {
  return (
    <svg className="ob-ring" width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
      <circle className="ob-ring-track" cx="16" cy="16" r="14" />
      <circle className="ob-ring-arc" cx="16" cy="16" r="14" />
      <path className="ob-ring-check" d="M10 16.5l4.2 4.2L22.5 12" />
    </svg>
  );
}

/** Pulsing dot + label: makes the 700ms live-poll visible so waiting feels active. */
function LiveScan({ label }: { label: string }) {
  return (
    <span className="ob-scan">
      <span className="ob-scan-dot" />
      {label}
    </span>
  );
}

function Step1({ onNext, onSkip }: { onNext: () => void; onSkip: () => void }) {
  return (
    <section className="ob-slide">
      <Keyboard pulsing />
      <h1 className="ob-h1">
        Your Mac shipped with a button that does nothing. <span className="ob-accent">Until now.</span>
      </h1>
      <p className="ob-sub">
        Hold <Keycap>fn</Keycap>. Say the thing. Let go. Clean text lands at your cursor.<br />
        No account. No API key. No cloud. The Fn button is finally the fun button.
      </p>
      <div className="ob-cta-row">
        <button className="ob-btn primary" onClick={onNext}>Wire it up →</button>
        <button className="ob-link" onClick={onSkip}>I&apos;ve met a setup wizard before. Skip →</button>
      </div>
    </section>
  );
}

function Step2({
  mic, acc, im, onWireUp, onAllDone,
}: {
  mic: PermState; acc: PermState; im: PermState;
  onWireUp: () => void; onAllDone: () => void;
}) {
  const allGranted = mic === "granted" && acc === "granted" && im === "granted";
  return (
    <section className="ob-slide compact">
      <h1 className="ob-h1 small">Three switches. That&apos;s the whole toll.</h1>
      <p className="ob-sub small">
        macOS keeps the good stuff behind Privacy &amp; Security. We&apos;ll walk each one — flip a switch over there and this screen reacts on its own.
      </p>
      <div className="ob-perm-stack">
        <PermCard title="Microphone" why="We can't transcribe what we can't hear." state={mic} icon={<Mic size={13} />} />
        <span className={`ob-wire ${mic === "granted" ? "on" : ""}`} aria-hidden="true" />
        <PermCard title="Accessibility" why="How the text lands at your cursor." state={acc} icon={<ClipboardPaste size={13} />} />
        <span className={`ob-wire ${acc === "granted" ? "on" : ""}`} aria-hidden="true" />
        <PermCard title="Input Monitoring" why="Fn is special. macOS hides it from everyone else." state={im} icon={<KeyboardIcon size={13} />} />
      </div>
      <LiveScan label="watching System Settings live" />
      <div className="ob-cta-row tight">
        <button className="ob-btn primary" onClick={allGranted ? onAllDone : onWireUp}>
          {allGranted ? "Circuit closed. Onward →" : "Wire them up one by one →"}
        </button>
      </div>
    </section>
  );
}

function PermCard({ title, why, state, icon }: { title: string; why: string; state: PermState; icon: ReactNode }) {
  return (
    <div className={`ob-perm-card ${state}`}>
      <div className="ob-perm-row">
        <span className="ob-perm-icon">
          {state === "granted" ? <CheckRing size={24} /> : icon}
        </span>
        <div className="ob-perm-meat">
          <div className="ob-perm-title">{title}</div>
          <div className="ob-perm-why">{why}</div>
        </div>
        <span className="ob-perm-state">{stateLabel(state)}</span>
      </div>
      <span className="ob-perm-sweep" aria-hidden="true" />
    </div>
  );
}

function PermSlide({
  title, why, state, icon, settingsPath, onRequest, onOpen, onContinue, tertiary,
}: {
  title: string; why: string; state: PermState;
  icon: ReactNode; settingsPath: string;
  onRequest: () => void; onOpen: () => void; onContinue: () => void;
  tertiary?: ReactNode;
}) {
  // auto-trigger the request prompt once on mount
  const requestedRef = useRef(false);
  useEffect(() => {
    if (!requestedRef.current && state !== "granted") {
      requestedRef.current = true;
      onRequest();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Escalate to a manual rescue path if the grant hasn't landed after a
  // while — or immediately if the switch got revoked out from under us.
  const [slow, setSlow] = useState(false);
  useEffect(() => {
    if (state === "granted") return;
    const t = setTimeout(() => setSlow(true), 8000);
    return () => clearTimeout(t);
  }, [state]);
  const showRescue = slow || state === "denied";

  return (
    <section className="ob-slide compact">
      <div className="ob-perm-hero">
        <span className={`ob-perm-bigicon ${state}`}>
          {state === "granted" ? <CheckRing size={34} /> : icon}
        </span>
        <div>
          <h1 className="ob-h1 small">{title}</h1>
          <p className="ob-sub small">{why}</p>
        </div>
      </div>

      {state === "granted" ? (
        <div className="ob-perm-ok">✓ wired — moving on</div>
      ) : (
        <div className="ob-perm-actions">
          <button className="ob-btn primary" onClick={onOpen}>
            Open System Settings
          </button>
          <LiveScan label="re-checking every 0.7s · flip it and we jump" />
          {showRescue && (
            <div className="ob-rescue">
              <div className="ob-rescue-title">
                {state === "denied" ? "macOS yanked that switch back off." : "macOS being stubborn?"}
              </div>
              <ol>
                <li>System Settings → {settingsPath}</li>
                <li>Find FunButton. Flip the switch on.</li>
                <li>Already on? Flip it off and on again. Classic.</li>
              </ol>
              <p>We&apos;re still watching. The moment it lands, we move.</p>
              <button className="ob-link" onClick={onContinue}>It&apos;s on, I swear — continue anyway →</button>
            </div>
          )}
          {tertiary}
        </div>
      )}
    </section>
  );
}

/** One engine in the ignition panel: node lights, energy travels, then locks green. */
function IgnitionRow({ label, engine, status }: { label: string; engine: string; status: EngineStatus }) {
  return (
    <div className={`ob-ign-row ${status}`}>
      <span className="ob-ign-node">{status === "ready" ? "✓" : ""}</span>
      <span className="ob-ign-label">
        {label}
        <em>{engine}</em>
      </span>
      <span className="ob-ign-track"><span className="ob-ign-fill" /></span>
      <span className="ob-ign-status">
        {status === "ready" ? "online" : status === "failed" ? "offline" : "warming"}
      </span>
    </div>
  );
}

function Step6({
  groqKey, setGroqKey, groqState, groqError, onValidate, ollamaUp, embedded, onNext,
}: {
  groqKey: string; setGroqKey: (v: string) => void;
  groqState: "idle" | "checking" | "ok" | "bad"; groqError: string;
  onValidate: () => void; ollamaUp: boolean | null;
  embedded: EmbeddedStatus;
  onNext: () => void;
}) {
  const ready = embedded.stt === "ready" || groqState === "ok";
  const bothReady = embedded.stt === "ready" && embedded.cleanup === "ready";
  const anyFailed = embedded.stt === "failed" || embedded.cleanup === "failed";
  const overall = bothReady ? "ready" : anyFailed ? "failed" : "starting";
  // The optional paths live behind a disclosure so the slide fits 720x520.
  // If an on-device engine dies, the fallback IS the path — pop it open.
  const [altOpen, setAltOpen] = useState(false);
  useEffect(() => { if (anyFailed) setAltOpen(true); }, [anyFailed]);
  return (
    <section className="ob-slide compact">
      <h1 className="ob-h1 small">No API key. No account. Ever.</h1>
      <p className="ob-sub small">
        Whisper hears you. Qwen tidies it up. All on this Mac, even on a plane.
      </p>
      <div className={`ob-ignition ${overall}`}>
        <div className="ob-ign-head">
          <span>on-device engines</span>
          <span className={`ob-ign-overall ${overall}`}>
            {overall === "ready" ? "all systems go" : overall === "failed" ? "needs a fallback" : "igniting…"}
          </span>
        </div>
        <IgnitionRow label="speech-to-text" engine="whisper · local" status={embedded.stt} />
        <IgnitionRow label="cleanup" engine="qwen 2.5 · local" status={embedded.cleanup} />
        {overall === "failed" && (
          <p className="ob-ign-fallback">
            {embedded.stt === "failed"
              ? "On-device speech-to-text couldn't start. Paste a Groq key below to dictate."
              : "On-device cleanup couldn't start. Dictation still works. Add a Groq key or Ollama for cleanup."}
          </p>
        )}
      </div>
      <details
        className="ob-alt"
        open={altOpen}
        onToggle={(e) => setAltOpen(e.currentTarget.open)}
      >
        <summary>optional: faster cloud (Groq) or your own Ollama — we never see a key</summary>
        <div className={`ob-alt-row ${groqState === "ok" ? "good" : ""}`}>
          <span className="ob-alt-tag"><Zap size={11} /> fast</span>
          <input
            className="ob-input"
            type="password"
            value={groqKey}
            placeholder="gsk_… — stays in your macOS Keychain"
            onChange={(e) => setGroqKey(e.target.value)}
          />
          <button className="ob-btn small" onClick={onValidate} disabled={!groqKey || groqState === "checking"}>
            {groqState === "checking" ? "checking…" : groqState === "ok" ? "✓ valid" : "validate"}
          </button>
          {groqState === "bad" ? (
            <span className="ob-err" title={groqError}>{groqError}</span>
          ) : (
            <a className="ob-alt-link" href="https://console.groq.com/keys" target="_blank" rel="noreferrer">free key →</a>
          )}
        </div>
        <div className={`ob-alt-row ${ollamaUp === true ? "good" : ""}`}>
          <span className="ob-alt-tag"><Lock size={11} /> local</span>
          {ollamaUp === true ? (
            <p className="ob-alt-ok">✓ Ollama up at localhost:11434 — just pull <code>qwen2.5:1.5b</code>.</p>
          ) : (
            <CopyBlock text="brew install ollama && ollama pull qwen2.5:1.5b" />
          )}
        </div>
      </details>
      <div className="ob-cta-row tight">
        <button className="ob-btn primary" onClick={onNext} disabled={!ready}>
          {ready ? "Try it now →" : "waiting for an engine…"}
        </button>
        <button className="ob-link" onClick={onNext}>I&apos;ll set this up later →</button>
      </div>
    </section>
  );
}

/** Payload of `funbutton:result` — must mirror PipelinePayload in lib.rs. */
interface ResultPayload {
  raw: string;
  cleaned: string;
  mode: string;
  backend: string;
  word_count: number;
}

const PIPE_STAGES = ["mic", "whisper", "qwen", "paste"] as const;

function backendLabel(b: string): string {
  if (b === "embedded") return "on-device";
  if (b === "cloud-fallback") return "cloud";
  return b;
}

function Step7({ onFinish, hotkeyKind }: { onFinish: () => void; hotkeyKind: HotkeyKind }) {
  const [waveform, setWaveform] = useState<number[]>(new Array(16).fill(0));
  const audioRef = useRef<{ ctx: AudioContext; analyser: AnalyserNode; data: Uint8Array; stream: MediaStream } | null>(null);
  const padRef = useRef<HTMLTextAreaElement | null>(null);

  // -1 idle · 0..3 = the stage currently working · 4 = loop complete
  const [stage, setStage] = useState(-1);
  const [landed, setLanded] = useState<ResultPayload | null>(null);
  const [pipeErr, setPipeErr] = useState("");

  // Mic level meter — proves the mic is hot before the user even presses.
  useEffect(() => {
    let cancelled = false;
    let raf = 0;
    (async () => {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        if (cancelled) { stream.getTracks().forEach(t => t.stop()); return; }
        const ctx = new AudioContext();
        const src = ctx.createMediaStreamSource(stream);
        const analyser = ctx.createAnalyser();
        analyser.fftSize = 64;
        src.connect(analyser);
        const data = new Uint8Array(analyser.frequencyBinCount);
        audioRef.current = { ctx, analyser, data, stream };
        const tick = () => {
          if (cancelled) return;
          analyser.getByteFrequencyData(data);
          const bars: number[] = [];
          const step = Math.floor(data.length / 16);
          for (let i = 0; i < 16; i++) {
            bars.push(data[i * step] / 255);
          }
          setWaveform(bars);
          raf = requestAnimationFrame(tick);
        };
        tick();
      } catch {
        // mic not granted; keep flat bars
      }
    })();
    return () => {
      cancelled = true;
      if (raf) cancelAnimationFrame(raf);
      audioRef.current?.stream.getTracks().forEach(t => t.stop());
      audioRef.current?.ctx.close();
    };
  }, []);

  // The REAL pipeline narrates itself: `funbutton:status` drives the stage
  // rail and `funbutton:result` is the proof — the same events the tray and
  // settings window already consume, so nothing here is simulated.
  useEffect(() => {
    let alive = true;
    const subs = [
      listen<{ status: string; message: string | null }>("funbutton:status", (e) => {
        if (!alive) return;
        const s = e.payload.status;
        if (s === "recording") {
          // New take — clear the pad so the landing is unmistakable.
          setStage(0); setLanded(null); setPipeErr("");
          if (padRef.current) padRef.current.value = "";
        } else if (s === "transcribing") setStage(1);
        else if (s === "cleaning") setStage(2);
        else if (s === "pasting") setStage(3);
        else if (s === "error") {
          setStage(-1);
          setPipeErr(e.payload.message || "pipeline hiccup — hold fn and try again");
        } else if (s === "idle" && e.payload.message === "too short") {
          setStage(-1);
          setPipeErr("too short — hold fn a beat longer");
        }
      }).catch(() => null),
      listen<ResultPayload>("funbutton:result", (e) => {
        if (!alive) return;
        setStage(4); setPipeErr(""); setLanded(e.payload);
        // The paste is a real ⌘V into the focused pad. If it was blocked or
        // landed somewhere else, echo the transcript so the user still
        // watches their words arrive.
        const cleaned = e.payload.cleaned;
        setTimeout(() => {
          const el = padRef.current;
          if (el && el.value.trim() === "") el.value = cleaned;
        }, 700);
      }).catch(() => null),
    ];
    return () => {
      alive = false;
      subs.forEach((p) => p.then((un) => { if (un) un(); }));
    };
  }, []);

  const peak = useMemo(() => waveform.reduce((m, v) => Math.max(m, v), 0), [waveform]);
  const isLive = peak > 0.04;

  const railMsg = pipeErr
    ? pipeErr
    : landed
      ? `✓ landed · ${landed.word_count} ${landed.word_count === 1 ? "word" : "words"} · ${backendLabel(landed.backend)}`
      : stage === 0 ? "listening…"
      : stage === 1 ? "whisper decoding…"
      : stage === 2 ? "qwen scrubbing…"
      : stage === 3 ? "landing at your cursor…"
      : "waiting on the button";

  return (
    <section className="ob-slide compact go">
      <h1 className="ob-h1 small">Push the button.</h1>
      <p className="ob-sub small">
        Hold <Keycap>{hotkeyKind === "fn" ? "fn" : "right option"}</Keycap>, say what you&apos;re hacking on, let go.
      </p>
      <div className={`ob-pad ${landed ? "landed" : ""} ${stage >= 0 && stage < 4 ? "busy" : ""} ${pipeErr ? "stalled" : ""}`}>
        <div className="ob-pad-top">
          <div className={`ob-meter ${isLive ? "live" : ""}`} aria-hidden="true">
            {waveform.map((v, i) => (
              <span key={i} className="ob-meter-bar" style={{ height: `${3 + v * 14}px` }} />
            ))}
          </div>
          <LiveScan label={isLive ? "mic hears you" : "mic is hot"} />
        </div>
        <textarea
          ref={padRef}
          className="ob-input ob-pad-input"
          rows={2}
          autoFocus
          spellCheck={false}
          aria-label="dictation landing pad"
          placeholder="Hold fn and let it rip — the cleaned text lands right here."
        />
        <div className="ob-pad-rail">
          {PIPE_STAGES.map((label, i) => (
            <span key={label} className={`ob-pipe ${stage === 4 || i < stage ? "done" : i === stage ? "active" : ""}`}>
              {label}
            </span>
          ))}
          <span className={`ob-pad-msg ${pipeErr ? "err" : landed ? "ok" : ""}`}>{railMsg}</span>
        </div>
      </div>
      <p className="ob-muted ob-pad-note">
        {landed
          ? "That was the whole loop. It lands like that in every app you own."
          : "This box is just the demo. Fn is caught OS-wide, so every app is fair game."}
      </p>
      <div className="ob-cta-row tight">
        <button className="ob-btn primary" onClick={onFinish}>
          {landed ? "It works. Let me loose →" : "I'm ready. Let me loose →"}
        </button>
      </div>
      {hotkeyKind === "fn" && (
        <details className="ob-aside">
          <summary>Fn opening the emoji picker instead?</summary>
          <p>macOS grabs Fn for emoji by default. One line hands it back:</p>
          <CopyBlock text="defaults write com.apple.HIToolbox AppleFnUsageType -int 0" />
          <p className="ob-muted small">Same as System Settings → Keyboard → &quot;Press fn key to&quot; → &quot;Do Nothing&quot;.</p>
        </details>
      )}
    </section>
  );
}

function Keycap({ children }: { children: ReactNode }) {
  return <span className="ob-keycap">{children}</span>;
}

function CopyBlock({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="ob-copy">
      <code>{text}</code>
      <button
        className="ob-copy-btn"
        onClick={async () => {
          try {
            await navigator.clipboard.writeText(text);
            setCopied(true);
            setTimeout(() => setCopied(false), 1500);
          } catch {}
        }}
      >{copied ? "copied ✓" : "copy"}</button>
    </div>
  );
}

function stateLabel(s: PermState): string {
  switch (s) {
    case "granted": return "wired";
    case "denied": return "blocked";
    case "checking": return "checking…";
    default: return "waiting";
  }
}

/**
 * Hero: the bottom-left corner of a Mac keyboard, zoomed in so the Fn key is
 * unmistakably the star. Pure SVG, no layout APIs anywhere near this —
 * it's an illustration, not a key reader.
 */
function Keyboard({ pulsing }: { pulsing?: boolean }) {
  return (
    <svg
      className={`ob-keyboard ${pulsing ? "pulse" : ""}`}
      viewBox="0 0 360 148"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <defs>
        <linearGradient id="obCap" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#2d2d30" />
          <stop offset="1" stopColor="#1d1d20" />
        </linearGradient>
        <linearGradient id="obFnCap" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#ff4a78" />
          <stop offset="1" stopColor="#dd2054" />
        </linearGradient>
        <radialGradient id="obFnGlow" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0" stopColor="#ff3366" stopOpacity="0.28" />
          <stop offset="1" stopColor="#ff3366" stopOpacity="0" />
        </radialGradient>
      </defs>

      {/* aluminum plate, cropped: bleeds off the top and right edges */}
      <rect x="8" y="-44" width="420" height="172" rx="18" className="ob-kb-frame" />

      {/* sliver of the home row peeking in from the top — sells the zoom */}
      <KbKey x={14} y={-30} w={76} h={40} />
      {[96, 146, 196, 246, 296, 346].map((x) => (
        <KbKey key={`sl-${x}`} x={x} y={-30} w={44} h={40} />
      ))}

      {/* z row */}
      <KbKey x={14} y={16} w={76} h={44} label="⇧" labelDy={6} />
      {["z", "x", "c", "v", "b", "n"].map((l, i) => (
        <KbKey key={`z-${l}`} x={96 + i * 50} y={16} w={44} h={44} label={l} labelDy={5} />
      ))}

      {/* ambient glow around the star before we draw it */}
      <circle cx="38" cy="90" r="58" fill="url(#obFnGlow)" pointerEvents="none" />

      {/* bottom row: fn ⌃ ⌥ ⌘ space */}
      <g className="ob-kb-fng">
        <rect x="14" y="68" width="48" height="48" rx="9" className="ob-kb-base" />
        <rect x="14" y="66" width="48" height="48" rx="9" className="ob-kb-fncap" />
        <text x="54" y="82" textAnchor="end" className="ob-kb-fnlabel">fn</text>
        {/* globe glyph, drawn (no emoji in system UI) */}
        <g className="ob-kb-globe">
          <circle cx="28.5" cy="101" r="5.5" />
          <ellipse cx="28.5" cy="101" rx="2.6" ry="5.5" />
          <line x1="23" y1="101" x2="34" y2="101" />
        </g>
      </g>
      <rect x="11" y="63" width="54" height="54" rx="11" className="ob-kb-halo" />
      <rect x="11" y="63" width="54" height="54" rx="11" className="ob-kb-halo late" />

      <KbKey x={68} y={66} w={48} h={48} label="⌃" labelDy={6} />
      <KbKey x={122} y={66} w={48} h={48} label="⌥" labelDy={6} />
      <KbKey x={176} y={66} w={62} h={48} label="⌘" labelDy={6} />
      <KbKey x={244} y={66} w={200} h={48} />
    </svg>
  );
}

function KbKey({ x, y, w, h, label, labelDy = 4 }: {
  x: number; y: number; w: number; h: number; label?: string; labelDy?: number;
}) {
  return (
    <g>
      <rect x={x} y={y + 2} width={w} height={h} rx={8} className="ob-kb-base" />
      <rect x={x} y={y} width={w} height={h} rx={8} className="ob-kb-cap" />
      {label && (
        <text x={x + w / 2} y={y + h / 2 + labelDy} textAnchor="middle" className="ob-kb-label">
          {label}
        </text>
      )}
    </g>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(<App />);
