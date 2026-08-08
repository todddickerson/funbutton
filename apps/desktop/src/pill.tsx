// Recording pill — the floating HUD that IS the product while you talk.
//
// Design notes (peg: Wispr Flow's pill, unramble's HUD):
// - recording: a LIVE scrolling waveform driven by the real mic signal
//   (getUserMedia + AnalyserNode — same proven pattern as onboarding Step7).
//   Torn down completely the instant recording ends: tracks stopped, context
//   closed, RAF cancelled. Zero idle CPU, no lingering orange mic dot.
// - transcribing / cleaning: lib.rs only emits status events at the pipeline
//   edges; the mid-pipeline flips live in AppState. We poll get_status (an
//   existing invoke command) ~9x/sec ONLY while a dictation is in flight so
//   the pill tracks whisper → cleanup in real time.
// - pasting / error / too-short: the Rust side hides the pill the moment the
//   pipeline returns, BEFORE those emits. The webview re-shows itself for a
//   short flash — "✓ code mode · Cursor" is the dev-first proof and deserves
//   its beat on screen. Errors hold longer and say how to retry.
// - placement: bottom-center of the work area (above the Dock), Wispr-style,
//   re-resolved at the start of EVERY dictation against the monitor the
//   cursor is on (cursorPosition + monitorFromPoint) — multi-monitor setups
//   get the pill where the user is actually working, and Dock/resolution
//   changes are picked up on the next hold. The window is taller than the
//   capsule so the drop shadow renders fully inside its own bounds.
// - overlap: Rust doesn't gate a new hold while a previous pipeline is in
//   flight, and it hides+re-emits around this window unconditionally. While
//   current === "recording" the HUD is the user's only proof the mic is hot,
//   so late pasting/error/idle events from the PREVIOUS dictation must never
//   kill the waveform — we swallow their flash and re-show the window.
//
// Contract: listens to `funbutton:status` exactly as before; never emits.
import { useEffect, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { Check } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  getCurrentWindow,
  currentMonitor,
  primaryMonitor,
  monitorFromPoint,
  cursorPosition,
  PhysicalPosition,
  type Monitor,
} from "@tauri-apps/api/window";
import "./pill.css";

const BAR_COUNT = 26;
const BAR_MIN = 0.08; // resting scaleY so the strip reads as a track, not a void
const HISTORY_STEP_MS = 45; // one bar ≈ 45ms → the strip shows ~1.2s of you

type Phase =
  | "hidden"
  | "recording"
  | "transcribing"
  | "cleaning"
  | "pasted"
  | "error"
  | "hint";

const ACTIVE: ReadonlySet<Phase> = new Set(["recording", "transcribing", "cleaning"]);

function Pill() {
  const [phase, setPhase] = useState<Phase>("hidden");
  const [detail, setDetail] = useState("");
  const [leaving, setLeaving] = useState(false);
  const barsRef = useRef<(HTMLSpanElement | null)[]>([]);

  useEffect(() => {
    let disposed = false;
    const win = getCurrentWindow();

    // ---- placement: bottom-center of the work area, on the monitor the
    // user is actually on. Runs on mount AND at the start of every dictation
    // (the hidden window idles on whatever screen it last showed on, so
    // mount-time state is stale by definition on multi-monitor setups). ----
    const place = async () => {
      try {
        let mon: Monitor | null = null;
        try {
          // Physical cursor coords → the monitor being worked on right now.
          const cur = await cursorPosition();
          mon = await monitorFromPoint(cur.x, cur.y);
        } catch {
          // older runtime / denied permission — fall through
        }
        if (!mon) mon = (await currentMonitor()) ?? (await primaryMonitor());
        if (!mon || disposed) return;
        const size = await win.outerSize();
        const wa = mon.workArea ?? { position: mon.position, size: mon.size };
        // Small margin: the window already carries ~36px of internal shadow
        // clearance below the capsule (see pill.css #root), so the capsule
        // itself floats ~40px above the work-area bottom — Wispr territory.
        const margin = Math.round(4 * mon.scaleFactor);
        const x = Math.round(wa.position.x + (wa.size.width - size.width) / 2);
        const y = Math.round(wa.position.y + wa.size.height - size.height - margin);
        await win.setPosition(new PhysicalPosition(x, y));
      } catch {
        // keep the current position — placement is a nicety
      }
    };
    void place();

    // ---- live waveform: direct DOM writes, no per-frame React renders ----
    let audio: { ctx: AudioContext; stream: MediaStream } | null = null;
    let analyser: AnalyserNode | null = null;
    let data: Uint8Array | null = null;
    let raf = 0;
    let lastShift = 0;
    let micGen = 0; // bumped by every start/stop; stale getUserMedia awaits self-destruct
    const history = new Float32Array(BAR_COUNT);

    const paint = () => {
      const bars = barsRef.current;
      for (let i = 0; i < BAR_COUNT; i++) {
        const el = bars[i];
        if (el) el.style.transform = `scaleY(${BAR_MIN + history[i] * (1 - BAR_MIN)})`;
      }
    };

    const tick = (t: number) => {
      if (!analyser || !data) return;
      analyser.getByteTimeDomainData(data);
      let sum = 0;
      for (let i = 0; i < data.length; i++) {
        const d = (data[i] - 128) / 128;
        sum += d * d;
      }
      const rms = Math.sqrt(sum / data.length);
      const level = Math.min(1, Math.pow(rms * 3.4, 0.72)); // gain + gamma: speech reads big
      if (t - lastShift >= HISTORY_STEP_MS) {
        history.copyWithin(0, 1);
        history[BAR_COUNT - 1] = level;
        lastShift = t;
      } else if (level > history[BAR_COUNT - 1]) {
        history[BAR_COUNT - 1] = level; // catch transients between shifts
      }
      paint();
      raf = requestAnimationFrame(tick);
    };

    const stopMic = () => {
      micGen++; // invalidate any in-flight getUserMedia
      if (raf) cancelAnimationFrame(raf);
      raf = 0;
      analyser = null;
      data = null;
      if (audio) {
        audio.stream.getTracks().forEach((tr) => tr.stop());
        void audio.ctx.close().catch(() => {});
        audio = null;
      }
      history.fill(0);
      paint();
    };

    const startMic = async () => {
      if (audio || disposed) return;
      const gen = ++micGen;
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        // Recording may already be over — or a newer recording may already own
        // the mic — by the time this resolves (quick tap, rapid re-press).
        // A stale generation always stops ITS OWN stream: no orphaned tracks,
        // no lingering orange mic dot, ever.
        if (disposed || gen !== micGen || current !== "recording" || audio) {
          stream.getTracks().forEach((tr) => tr.stop());
          return;
        }
        const ctx = new AudioContext();
        const src = ctx.createMediaStreamSource(stream);
        const an = ctx.createAnalyser();
        an.fftSize = 1024;
        src.connect(an);
        audio = { ctx, stream };
        analyser = an;
        data = new Uint8Array(an.fftSize);
        lastShift = 0;
        raf = requestAnimationFrame(tick);
      } catch {
        // no mic for the visual — recording itself happens in Rust regardless
      }
    };

    // ---- phase machine ----
    let current: Phase = "hidden";
    let hideTimer: number | undefined;
    let outTimer: number | undefined;

    const clearTimers = () => {
      if (hideTimer) window.clearTimeout(hideTimer);
      if (outTimer) window.clearTimeout(outTimer);
      hideTimer = outTimer = undefined;
    };

    const go = (p: Phase, d = "") => {
      current = p;
      setLeaving(false);
      setPhase(p);
      setDetail(d);
    };

    const dismiss = (afterMs: number) => {
      hideTimer = window.setTimeout(() => {
        setLeaving(true);
        outTimer = window.setTimeout(() => {
          current = "hidden";
          setPhase("hidden");
          setLeaving(false);
          void win.hide().catch(() => {});
        }, 190);
      }, afterMs);
    };

    // ---- mid-pipeline status poll (active dictations only) ----
    let poll: number | undefined;
    const stopPoll = () => {
      if (poll) window.clearInterval(poll);
      poll = undefined;
    };
    const startPoll = () => {
      if (poll) return;
      poll = window.setInterval(async () => {
        if (!ACTIVE.has(current)) {
          stopPoll();
          return;
        }
        try {
          const s = await invoke<string>("get_status");
          if ((s === "transcribing" || s === "cleaning") && current !== s) {
            stopMic();
            go(s as Phase);
          }
        } catch {
          // command unavailable — events still drive the edges
        }
      }, 110);
    };

    // Overlapped dictations: Rust spawns each pipeline detached, so a new
    // hold can start while the previous one is still transcribing/pasting.
    // Late events from that previous dictation must NEVER kill the live
    // waveform — for push-to-talk the HUD IS the recording confirmation.
    // Rust also hides this window unconditionally when the old pipeline
    // returns, so the guard re-shows it. (The receipt for the older
    // dictation is skipped; the one for the hold in progress will land.)
    const holdIsHot = () => current === "recording";

    const onStatus = (status: string, message: string | null) => {
      switch (status) {
        case "recording":
          clearTimers();
          void place(); // the cursor's monitor, fresh for every dictation
          go("recording");
          startPoll();
          void startMic();
          break;
        case "transcribing":
        case "cleaning":
          stopMic();
          clearTimers();
          go(status as Phase);
          break;
        case "pasting":
          if (holdIsHot()) {
            void win.show().catch(() => {});
            break;
          }
          stopMic();
          stopPoll();
          clearTimers();
          go("pasted", message ?? "");
          void win.show().catch(() => {});
          dismiss(1500);
          break;
        case "error":
          if (holdIsHot() && message !== null && !message.startsWith("encode:")) {
            // A previous dictation's pipeline choked mid-hold. Keep the
            // waveform; this hold's own outcome will surface on release.
            // ("encode:" errors are the exception — they're emitted
            // synchronously for the hold that JUST ended, never late.)
            void win.show().catch(() => {});
            break;
          }
          stopMic();
          stopPoll();
          clearTimers();
          go("error", message ?? "pipeline choked");
          void win.show().catch(() => {});
          dismiss(5200);
          break;
        case "idle":
          if (message === "too short") {
            // Emitted synchronously on release of the hold that just ended
            // (never late from an overlapped pipeline) — always flash it.
            stopMic();
            stopPoll();
            clearTimers();
            go("hint");
            void win.show().catch(() => {});
            dismiss(1400);
          } else if (holdIsHot()) {
            // Previous pipeline wound down mid-hold; Rust hid the window.
            void win.show().catch(() => {});
          } else if (ACTIVE.has(current)) {
            // pipeline ended without a flash-worthy payload — fold quietly
            stopMic();
            stopPoll();
            clearTimers();
            current = "hidden";
            setPhase("hidden");
          }
          // if a pasted/error flash is on screen, let its timer finish
          break;
      }
    };

    const un = listen<{ status: string; message: string | null }>(
      "funbutton:status",
      (e) => {
        if (!disposed) onStatus(e.payload.status, e.payload.message ?? null);
      },
    );

    return () => {
      disposed = true;
      clearTimers();
      stopPoll();
      stopMic();
      un.then((u) => u());
    };
  }, []);

  if (phase === "hidden") return null;

  return (
    <div className={`pill p-${phase}${leaving ? " leaving" : ""}`}>
      {phase === "recording" && (
        <div className="layer">
          <span className="rec-dot" aria-hidden="true" />
          <div className="wave" aria-label="recording">
            {Array.from({ length: BAR_COUNT }, (_, i) => (
              <span
                key={i}
                className="wave-bar"
                ref={(el) => {
                  barsRef.current[i] = el;
                }}
              />
            ))}
          </div>
        </div>
      )}
      {(phase === "transcribing" || phase === "cleaning") && (
        <div className="layer" key={phase}>
          <span className="spin" aria-hidden="true" />
          <span className="t-main">
            {phase === "transcribing" ? "transcribing" : "cleaning up"}
          </span>
          <span className="t-sub">
            {phase === "transcribing" ? "whisper" : "mode-aware"}
          </span>
        </div>
      )}
      {phase === "pasted" && (
        <div className="layer">
          <Check className="ok" size={13} strokeWidth={3} aria-hidden="true" />
          <span className="t-msg">{detail || "done"}</span>
          <span className="t-tag">pasted</span>
        </div>
      )}
      {phase === "error" && (
        <div className="layer err">
          <span className="err-main" title={detail}>{detail}</span>
          <span className="err-hint">hold the button · run it back</span>
        </div>
      )}
      {phase === "hint" && (
        <div className="layer">
          <span className="t-hint">too quick · keep holding</span>
        </div>
      )}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(<Pill />);
