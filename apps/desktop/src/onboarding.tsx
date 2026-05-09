import { useEffect, useMemo, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./onboarding.css";

type HotkeyKind = "fn" | "right_option";
type Backend = "auto" | "groq" | "local";

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
    }).catch(() => {});
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
        setMicPerm(m ? "granted" : prevDeny(micPerm));
        setAccPerm(a ? "granted" : prevDeny(accPerm));
        setImPerm(i ? "granted" : prevDeny(imPerm));
      } catch {}
    };
    tick();
    const id = setInterval(tick, 700);
    return () => { cancelled = true; clearInterval(id); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
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

  // Ollama probe on entering step 6
  useEffect(() => {
    if (step !== 6) return;
    invoke<boolean>("ollama_check").then(setOllamaUp).catch(() => setOllamaUp(false));
  }, [step]);

  // Keyboard navigation
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowRight" || e.key === "Enter") {
        if (canAdvance()) goto(step + 1);
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
    if (step === 2) return micPerm === "granted" && accPerm === "granted" && imPerm === "granted";
    if (step === 6) return groqState === "ok" || ollamaUp === true;
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
    if (groqKey && groqState !== "bad") {
      await persistPartial({ groq_api_key: groqKey });
    }
    await persistPartial({ hotkey_kind: hotkeyKind });
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

      <div className={`ob-stage ${direction}`}>
        {step === 1 && <Step1 onNext={() => goto(2)} onSkip={() => goto(6)} />}
        {step === 2 && (
          <Step2
            mic={micPerm}
            acc={accPerm}
            im={imPerm}
            onNext={() => goto(3)}
            onSkipImWithRightOption={async () => { await pickHotkey("right_option"); goto(6); }}
          />
        )}
        {step === 3 && (
          <PermSlide
            title="Microphone"
            why="So we can hear you when you hold Fn."
            state={micPerm}
            onRequest={() => requestPerm("mic")}
            onOpen={() => openSysPanel("microphone")}
            onContinue={() => goto(4)}
          />
        )}
        {step === 4 && (
          <PermSlide
            title="Accessibility"
            why="So we can paste the cleaned text into your active app."
            state={accPerm}
            onRequest={() => requestPerm("acc")}
            onOpen={() => openSysPanel("accessibility")}
            onContinue={() => goto(5)}
          />
        )}
        {step === 5 && (
          <PermSlide
            title="Input Monitoring"
            why="So we can detect Fn — macOS treats it specially."
            state={imPerm}
            onRequest={() => requestPerm("im")}
            onOpen={() => openSysPanel("input_monitoring")}
            onContinue={() => goto(6)}
            tertiary={
              <button className="ob-tertiary" onClick={async () => { await pickHotkey("right_option"); goto(6); }}>
                Don&apos;t want this? Use Right Option as the hotkey →
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
            onNext={() => goto(7)}
          />
        )}
        {step === 7 && <Step7 onFinish={finish} hotkeyKind={hotkeyKind} />}
      </div>

      <footer className="ob-footer">
        <button className="ob-back" onClick={() => goto(step - 1)} disabled={step === 1}>← back</button>
        <span className="ob-help">esc closes · ←/→ to move · enter advances</span>
        <span />
      </footer>
    </div>
  );
}

function prevDeny(prev: PermState): PermState {
  return prev === "granted" ? "granted" : prev === "checking" ? "checking" : "unknown";
}

function Stepper({ current, total }: { current: number; total: number }) {
  return (
    <div className="ob-stepper">
      {Array.from({ length: total }, (_, i) => {
        const idx = i + 1;
        const cls = idx === current ? "on" : idx < current ? "done" : "future";
        return <span key={idx} className={`ob-step ${cls}`}>{idx < current ? "✓" : ""}</span>;
      })}
    </div>
  );
}

function Step1({ onNext, onSkip }: { onNext: () => void; onSkip: () => void }) {
  return (
    <section className="ob-slide">
      <Keyboard pulsing />
      <h1 className="ob-h1">
        The key at the bottom-left of your keyboard <span className="ob-accent">finally has a job.</span>
      </h1>
      <p className="ob-sub">
        Hold it. Talk. Release.<br />
        We turn rambling speech into clean text. That&apos;s it.
      </p>
      <div className="ob-cta-row">
        <button className="ob-btn primary" onClick={onNext}>Show me how →</button>
        <button className="ob-link" onClick={onSkip}>I know what I&apos;m doing, skip →</button>
      </div>
    </section>
  );
}

function Step2({
  mic, acc, im, onNext, onSkipImWithRightOption,
}: {
  mic: PermState; acc: PermState; im: PermState;
  onNext: () => void; onSkipImWithRightOption: () => void;
}) {
  const allGranted = mic === "granted" && acc === "granted" && im === "granted";
  return (
    <section className="ob-slide compact">
      <h1 className="ob-h1 small">Three quick clicks.</h1>
      <p className="ob-sub small">macOS asks for one permission at a time. We&apos;ll explain each.</p>
      <div className="ob-perm-stack">
        <PermCard title="Microphone" why="To hear you when you hold Fn." state={mic} />
        <PermCard title="Accessibility" why="To paste the cleaned text into your active app." state={acc} />
        <PermCard title="Input Monitoring" why="To detect Fn — macOS treats it specially." state={im} />
      </div>
      <div className="ob-cta-row">
        <button className="ob-btn primary" onClick={onNext} disabled={!allGranted}>
          {allGranted ? "Next →" : "grant the three permissions to continue"}
        </button>
        {!allGranted && (
          <button className="ob-link" onClick={onSkipImWithRightOption}>
            skip — Right Option works without Input Monitoring →
          </button>
        )}
      </div>
    </section>
  );
}

function PermCard({ title, why, state }: { title: string; why: string; state: PermState }) {
  return (
    <div className={`ob-perm-card ${state}`}>
      <div className="ob-perm-row">
        <span className="ob-perm-icon">{stateIcon(state)}</span>
        <div className="ob-perm-meat">
          <div className="ob-perm-title">{title}</div>
          <div className="ob-perm-why">{why}</div>
        </div>
        <span className="ob-perm-state">{stateLabel(state)}</span>
      </div>
    </div>
  );
}

function PermSlide({
  title, why, state, onRequest, onOpen, onContinue, tertiary,
}: {
  title: string; why: string; state: PermState;
  onRequest: () => void; onOpen: () => void; onContinue: () => void;
  tertiary?: React.ReactNode;
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

  return (
    <section className="ob-slide compact">
      <div className="ob-perm-hero">
        <span className={`ob-perm-bigicon ${state}`}>{stateIcon(state)}</span>
        <div>
          <h1 className="ob-h1 small">{title}</h1>
          <p className="ob-sub small">{why}</p>
        </div>
      </div>

      {state === "granted" ? (
        <div className="ob-perm-ok">
          ✓ granted — moving on…
        </div>
      ) : (
        <div className="ob-perm-actions">
          <button className="ob-btn primary" onClick={onOpen}>
            Open System Settings
          </button>
          <button className="ob-btn ghost" onClick={onContinue}>
            Granted? Re-check now ↻
          </button>
          {tertiary}
        </div>
      )}
    </section>
  );
}

function Step6({
  groqKey, setGroqKey, groqState, groqError, onValidate, ollamaUp, onNext,
}: {
  groqKey: string; setGroqKey: (v: string) => void;
  groqState: "idle" | "checking" | "ok" | "bad"; groqError: string;
  onValidate: () => void; ollamaUp: boolean | null;
  onNext: () => void;
}) {
  const ready = groqState === "ok" || ollamaUp === true;
  return (
    <section className="ob-slide">
      <h1 className="ob-h1 small">How should we clean your speech?</h1>
      <p className="ob-sub small">Pick one. (Both is fine.)</p>
      <div className="ob-tiles">
        <div className={`ob-tile ${groqState === "ok" ? "good" : ""}`}>
          <div className="ob-tile-tag">⚡ FAST</div>
          <div className="ob-tile-title">Bring your Groq key</div>
          <div className="ob-tile-body">
            <p>
              ~300 ms cleanup. We never see the key — it lives in <code>~/.funbutton/settings.json</code>.
              {" "}<a href="https://console.groq.com/keys" target="_blank" rel="noreferrer">Grab one free →</a>
            </p>
            <input
              className="ob-input"
              type="password"
              value={groqKey}
              placeholder="gsk_…"
              onChange={(e) => setGroqKey(e.target.value)}
            />
            <div className="ob-tile-actions">
              <button className="ob-btn small" onClick={onValidate} disabled={!groqKey || groqState === "checking"}>
                {groqState === "checking" ? "checking…" : groqState === "ok" ? "✓ valid" : "validate"}
              </button>
              {groqState === "bad" && <span className="ob-err">{groqError}</span>}
            </div>
          </div>
        </div>
        <div className={`ob-tile ${ollamaUp === true ? "good" : ""}`}>
          <div className="ob-tile-tag">🔒 PRIVATE</div>
          <div className="ob-tile-title">Use Ollama (local)</div>
          <div className="ob-tile-body">
            {ollamaUp === true ? (
              <>
                <p className="ob-tile-ok">✓ Ollama running at localhost:11434.</p>
                <p className="ob-muted">Make sure <code>qwen2.5:1.5b</code> is pulled.</p>
              </>
            ) : (
              <>
                <p>No Ollama detected. Run this in your terminal:</p>
                <CopyBlock text="brew install ollama && ollama pull qwen2.5:1.5b" />
                <p className="ob-muted small">Then keep <code>ollama serve</code> running.</p>
              </>
            )}
          </div>
        </div>
      </div>
      <div className="ob-cta-row">
        <button className="ob-btn primary" onClick={onNext} disabled={!ready}>
          {ready ? "Try it now →" : "Configure one above to continue"}
        </button>
        <button className="ob-link" onClick={onNext}>I&apos;ll set this up later →</button>
      </div>
    </section>
  );
}

function Step7({ onFinish, hotkeyKind }: { onFinish: () => void; hotkeyKind: HotkeyKind }) {
  const [waveform, setWaveform] = useState<number[]>(new Array(28).fill(0));
  const audioRef = useRef<{ ctx: AudioContext; analyser: AnalyserNode; data: Uint8Array; stream: MediaStream } | null>(null);

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
          const step = Math.floor(data.length / 28);
          for (let i = 0; i < 28; i++) {
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

  const peak = useMemo(() => waveform.reduce((m, v) => Math.max(m, v), 0), [waveform]);
  const isLive = peak > 0.04;

  return (
    <section className="ob-slide">
      <h1 className="ob-h1">Try it now.</h1>
      <p className="ob-sub">
        Hold <Keycap>{hotkeyKind === "fn" ? "fn" : "right option"}</Keycap> and tell me what you&apos;re working on this weekend.
      </p>
      <div className={`ob-wave ${isLive ? "live" : ""}`}>
        {waveform.map((v, i) => (
          <span key={i} className="ob-wave-bar" style={{ height: `${4 + v * 56}px` }} />
        ))}
      </div>
      <p className="ob-muted small">
        Bars react to your mic — proves the permission is wired. Hotkey detection happens at the OS level (CGEventTap), so the actual dictation flow runs even though this window doesn&apos;t intercept Fn.
      </p>
      <div className="ob-cta-row">
        <button className="ob-btn primary" onClick={onFinish}>I&apos;m ready →</button>
      </div>
    </section>
  );
}

function Keycap({ children }: { children: React.ReactNode }) {
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

function stateIcon(s: PermState): string {
  switch (s) {
    case "granted": return "✓";
    case "denied": return "✕";
    case "checking": return "…";
    default: return "○";
  }
}
function stateLabel(s: PermState): string {
  switch (s) {
    case "granted": return "granted";
    case "denied": return "denied";
    case "checking": return "checking…";
    default: return "not yet";
  }
}

function Keyboard({ pulsing }: { pulsing?: boolean }) {
  return (
    <svg className="ob-keyboard" viewBox="0 0 320 110" xmlns="http://www.w3.org/2000/svg">
      <rect x="2" y="2" width="316" height="106" rx="14" ry="14" className="ob-kb-frame" />
      {/* Row 1 — top function row */}
      {Array.from({ length: 14 }).map((_, i) => (
        <rect key={`r1-${i}`} x={10 + i * 22} y={10} width={18} height={14} rx="2" className="ob-kb-key small" />
      ))}
      {/* Row 2 — number row */}
      {Array.from({ length: 14 }).map((_, i) => (
        <rect key={`r2-${i}`} x={10 + i * 22} y={28} width={18} height={16} rx="3" className="ob-kb-key" />
      ))}
      {/* Row 3 — qwerty row */}
      {Array.from({ length: 13 }).map((_, i) => (
        <rect key={`r3-${i}`} x={20 + i * 22} y={48} width={18} height={16} rx="3" className="ob-kb-key" />
      ))}
      {/* Row 4 — asdf row */}
      {Array.from({ length: 12 }).map((_, i) => (
        <rect key={`r4-${i}`} x={26 + i * 22} y={68} width={18} height={16} rx="3" className="ob-kb-key" />
      ))}
      {/* Row 5 — bottom row with Fn highlighted */}
      <rect x={10} y={88} width={18} height={16} rx="3" className={`ob-kb-key fn ${pulsing ? "pulse" : ""}`} />
      <text x={19} y={99} textAnchor="middle" className="ob-kb-fn-label">fn</text>
      {Array.from({ length: 4 }).map((_, i) => (
        <rect key={`r5b-${i}`} x={30 + i * 22} y={88} width={18} height={16} rx="3" className="ob-kb-key" />
      ))}
      <rect x={120} y={88} width={92} height={16} rx="3" className="ob-kb-key wide" />
      {Array.from({ length: 4 }).map((_, i) => (
        <rect key={`r5e-${i}`} x={216 + i * 22} y={88} width={18} height={16} rx="3" className="ob-kb-key" />
      ))}
      {/* Pointer */}
      <g className="ob-kb-pointer">
        <line x1="19" y1="84" x2="19" y2="78" />
        <text x="19" y="74" textAnchor="middle">FUN</text>
      </g>
    </svg>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(<App />);
