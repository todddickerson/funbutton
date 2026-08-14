import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AlertTriangle, Check, Keyboard as KeyboardIcon, RefreshCw, Radio } from "lucide-react";
import {
  bottomRow,
  extraKeys,
  hotkeyMeta,
  layoutHeadline,
  type DiagramKey,
  type HotkeyKind,
  type KeyboardInfo,
} from "./hotkeys";
import "./HotkeyPicker.css";

type CaptureState = "idle" | "listening" | "captured" | "timeout";

const GENERIC: KeyboardInfo = {
  model: "Unknown keyboard",
  layout: "generic",
  is_builtin: false,
  fn_bottom_left: false,
  default_hotkey: "right_option",
  vendor_id: null,
  product_id: null,
  source: "none",
};

/**
 * Visual keyboard picker. Detects the connected keyboard, draws its real
 * bottom row, and lets the user click the key they want as the Fun Button — or
 * just press it ("press the key you want" capture). Reused in onboarding and
 * settings. Selecting persists via `onPick`, which the parent wires to
 * save_settings (re-arming the listener with no restart — the shared armed
 * atomic already handles hot-swap).
 */
export function HotkeyPicker({
  selected,
  onPick,
  variant = "settings",
}: {
  selected: HotkeyKind;
  onPick: (kind: HotkeyKind) => void;
  variant?: "onboarding" | "settings";
}) {
  const [kb, setKb] = useState<KeyboardInfo | null>(null);
  const [capture, setCapture] = useState<CaptureState>("idle");
  const [captured, setCaptured] = useState<HotkeyKind | null>(null);
  const capturingRef = useRef(false);

  const detect = useCallback(async () => {
    try {
      const info = await invoke<KeyboardInfo>("detect_keyboard");
      setKb(info);
    } catch {
      // Never strand the picker — a generic layout is always usable.
      setKb(GENERIC);
    }
  }, []);

  useEffect(() => {
    detect();
  }, [detect]);

  // Capture-mode event wiring lives for the component's life; the Rust side is
  // only armed while the user asks for it (invoke capture_hotkey).
  useEffect(() => {
    const subs = [
      listen<HotkeyKind>("funbutton:hotkey-captured", (e) => {
        if (!capturingRef.current) return;
        capturingRef.current = false;
        setCaptured(e.payload);
        setCapture("captured");
        onPick(e.payload);
      }).catch(() => null),
      listen("funbutton:hotkey-capture-timeout", () => {
        if (!capturingRef.current) return;
        capturingRef.current = false;
        setCapture("timeout");
      }).catch(() => null),
    ];
    return () => {
      subs.forEach((p) => p.then((un) => { if (un) un(); }));
      // Stand down any in-flight capture when the picker unmounts.
      if (capturingRef.current) {
        capturingRef.current = false;
        invoke("cancel_hotkey_capture").catch(() => {});
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function startCapture() {
    setCaptured(null);
    setCapture("listening");
    capturingRef.current = true;
    try {
      await invoke("capture_hotkey", { timeoutMs: 8000 });
    } catch {
      capturingRef.current = false;
      setCapture("idle");
    }
  }

  async function cancelCapture() {
    capturingRef.current = false;
    setCapture("idle");
    await invoke("cancel_hotkey_capture").catch(() => {});
  }

  function pick(kind: HotkeyKind) {
    setCapture("idle");
    onPick(kind);
  }

  const info = kb ?? GENERIC;
  const row = bottomRow(info.layout);
  const extras = extraKeys(info.layout);
  const sel = hotkeyMeta(selected);

  return (
    <div className={`hkp hkp-${variant}`}>
      <div className="hkp-detected">
        <KeyboardIcon size={14} aria-hidden />
        <span className="hkp-detected-text">{layoutHeadline(info)}</span>
        <button className="hkp-redetect" onClick={detect} title="re-detect keyboard">
          <RefreshCw size={12} aria-hidden />
        </button>
      </div>
      {info.source !== "none" && (
        <div className="hkp-model">{info.model}</div>
      )}

      {/* Bottom-row diagram. Clickable modifier keys highlight; space/arrows are
          just context so the row reads as a real keyboard. */}
      <div className="hkp-board" role="group" aria-label="keyboard bottom row">
        {row.map((k) => (
          <DiagramKeycap key={k.id} k={k} selected={selected} onPick={pick} />
        ))}
      </div>

      {extras.length > 0 && (
        <div className="hkp-extras">
          <span className="hkp-extras-label">also on your board:</span>
          {extras.map(({ kind, note }) => {
            const m = hotkeyMeta(kind);
            const on = selected === kind;
            return (
              <button
                key={kind}
                className={`hkp-chip ${on ? "on" : ""}`}
                onClick={() => pick(kind)}
                title={m.hint}
              >
                <span className="hkp-chip-glyph">{m.glyph}</span>
                <span className="hkp-chip-name">{m.name}</span>
                <span className="hkp-chip-note">{note}</span>
              </button>
            );
          })}
        </div>
      )}

      {/* Press-to-capture — the table-stakes alternative to clicking. */}
      <div className="hkp-capture">
        {capture === "listening" ? (
          <div className="hkp-listening">
            <Radio size={14} className="hkp-pulse" aria-hidden />
            <span>hold the key you want to use…</span>
            <button className="hkp-cancel" onClick={cancelCapture}>cancel</button>
          </div>
        ) : (
          <button className="hkp-capture-btn" onClick={startCapture}>
            <Radio size={13} aria-hidden /> Press the key you want
          </button>
        )}
        {capture === "captured" && captured && (
          <span className="hkp-capture-msg ok">
            <Check size={13} aria-hidden /> got it — {hotkeyMeta(captured).name}
          </span>
        )}
        {capture === "timeout" && (
          <span className="hkp-capture-msg warn">
            didn&apos;t catch a key — needs Input Monitoring. try again or click one above.
          </span>
        )}
      </div>

      {/* Current selection + any caveat. */}
      <div className="hkp-selected">
        <span className="hkp-selected-tag">armed</span>
        <span className="hkp-selected-name">{sel.name}</span>
        <span className="hkp-selected-hint">{sel.hint}</span>
      </div>
      {sel.warn && (
        <div className="hkp-warn">
          <AlertTriangle size={13} aria-hidden />
          <span>{sel.warn}</span>
        </div>
      )}
    </div>
  );
}

function DiagramKeycap({
  k,
  selected,
  onPick,
}: {
  k: DiagramKey;
  selected: HotkeyKind;
  onPick: (kind: HotkeyKind) => void;
}) {
  const pickable = !!k.kind;
  const on = pickable && k.kind === selected;
  const isSpace = k.id === "space";
  const isArrows = k.id === "arrows";
  return (
    <button
      type="button"
      className={`hkp-key ${pickable ? "pickable" : "static"} ${on ? "on" : ""} ${isSpace ? "space" : ""} ${isArrows ? "arrows" : ""}`}
      style={{ flexGrow: k.flex }}
      disabled={!pickable}
      aria-pressed={pickable ? on : undefined}
      onClick={() => k.kind && onPick(k.kind)}
      title={k.kind ? hotkeyMeta(k.kind).hint : undefined}
    >
      <span className="hkp-key-glyph">{k.glyph}</span>
      {k.sub && <span className="hkp-key-sub">{k.sub}</span>}
    </button>
  );
}
