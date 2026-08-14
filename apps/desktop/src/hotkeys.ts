// Single source of truth for hotkey + keyboard metadata shared across the
// onboarding wizard, the settings window, and the picker. Mirrors
// src-tauri/src/state.rs `HotkeyKind` and src-tauri/src/keyboard.rs. Keeping
// every user-facing label here is what kills the class of "the copy says fn but
// the listener is watching something else" bugs.

export type HotkeyKind =
  | "fn"
  | "right_option"
  | "left_option"
  | "right_control"
  | "left_control"
  | "right_command"
  | "left_command"
  | "caps_lock";

export interface HotkeyMeta {
  kind: HotkeyKind;
  /** Title-case full name, e.g. "Right Control". */
  name: string;
  /** Lowercase inline form for prose, e.g. "right control". */
  hold: string;
  /** Compact tag for tight chrome, e.g. "right ⌃". */
  short: string;
  /** Glyph drawn on the keycap (never an emoji — system UI). */
  glyph: string;
  /** Small caption under the glyph, e.g. "control". */
  sub: string;
  /** One-line description. */
  hint: string;
  /** Present when picking this key has a real downside worth flagging. */
  warn?: string;
}

export const HOTKEYS: Record<HotkeyKind, HotkeyMeta> = {
  fn: {
    kind: "fn",
    name: "Fn",
    hold: "fn",
    short: "fn",
    glyph: "fn",
    sub: "function",
    hint: "the Fun Button. bottom-left on MacBooks & compact keyboards.",
  },
  right_option: {
    kind: "right_option",
    name: "Right Option",
    hold: "right option",
    short: "right ⌥",
    glyph: "⌥",
    sub: "option",
    hint: "right of the spacebar. safe everywhere, easy to find.",
  },
  left_option: {
    kind: "left_option",
    name: "Left Option",
    hold: "left option",
    short: "left ⌥",
    glyph: "⌥",
    sub: "option",
    hint: "left of the spacebar.",
  },
  right_control: {
    kind: "right_control",
    name: "Right Control",
    hold: "right control",
    short: "right ⌃",
    glyph: "⌃",
    sub: "control",
    hint: "right of the spacebar, if your board has one.",
  },
  left_control: {
    kind: "left_control",
    name: "Left Control",
    hold: "left control",
    short: "left ⌃",
    glyph: "⌃",
    sub: "control",
    hint: "bottom-left on full-size Apple keyboards.",
    warn: "you use ⌃ in terminals and shortcuts — quick taps are ignored, but heads up.",
  },
  right_command: {
    kind: "right_command",
    name: "Right Command",
    hold: "right command",
    short: "right ⌘",
    glyph: "⌘",
    sub: "command",
    hint: "right of the spacebar.",
  },
  left_command: {
    kind: "left_command",
    name: "Left Command",
    hold: "left command",
    short: "left ⌘",
    glyph: "⌘",
    sub: "command",
    hint: "left of the spacebar.",
    warn: "intercepts every ⌘ shortcut (⌘C, ⌘V, ⌘Tab…). not recommended.",
  },
  caps_lock: {
    kind: "caps_lock",
    name: "Caps Lock",
    hold: "caps lock",
    short: "caps",
    glyph: "⇪",
    sub: "caps lock",
    hint: "tap to start, tap again to stop (it's a toggle, not a hold).",
  },
};

export function hotkeyMeta(k: HotkeyKind): HotkeyMeta {
  return HOTKEYS[k] ?? HOTKEYS.fn;
}
export function hotkeyName(k: HotkeyKind): string {
  return hotkeyMeta(k).name;
}
/** Lowercase inline form for prose — the single source that replaced the
 *  hardcoded "fn" strings scattered through onboarding + settings. */
export function hotkeyHold(k: HotkeyKind): string {
  return hotkeyMeta(k).hold;
}
export function hotkeyShort(k: HotkeyKind): string {
  return hotkeyMeta(k).short;
}

// ---- Keyboard layout (mirrors keyboard.rs) --------------------------------

export type KeyboardLayout =
  | "builtin_macbook"
  | "magic_compact"
  | "magic_extended"
  | "generic";

export interface KeyboardInfo {
  model: string;
  layout: KeyboardLayout;
  is_builtin: boolean;
  fn_bottom_left: boolean;
  default_hotkey: HotkeyKind;
  vendor_id: number | null;
  product_id: number | null;
  source: string;
}

/** A drawn key in the bottom-row diagram. `kind` set ⇒ pickable. */
export interface DiagramKey {
  id: string;
  glyph: string;
  sub?: string;
  kind?: HotkeyKind;
  /** flex-grow weight — sets relative width. */
  flex: number;
}

/**
 * The physical bottom row for a detected layout, left→right. This is the
 * accuracy-critical bit: on `magic_extended` the bottom-left key is Control,
 * NOT Fn — exactly what surprised the tester on a Magic Keyboard with Numeric
 * Keypad. Fn still exists on that board, just up in the nav cluster, so it's
 * offered as an extra chip rather than drawn bottom-left.
 */
export function bottomRow(layout: KeyboardLayout): DiagramKey[] {
  const space: DiagramKey = { id: "space", glyph: "space", flex: 5 };
  const arrows: DiagramKey = { id: "arrows", glyph: "◂ ▴▾ ▸", flex: 1.6 };
  const lCtrl: DiagramKey = { id: "lctrl", glyph: "⌃", sub: "control", kind: "left_control", flex: 1 };
  const lOpt: DiagramKey = { id: "lopt", glyph: "⌥", sub: "option", kind: "left_option", flex: 1 };
  const lCmd: DiagramKey = { id: "lcmd", glyph: "⌘", sub: "command", kind: "left_command", flex: 1.3 };
  const rCmd: DiagramKey = { id: "rcmd", glyph: "⌘", sub: "command", kind: "right_command", flex: 1.3 };
  const rOpt: DiagramKey = { id: "ropt", glyph: "⌥", sub: "option", kind: "right_option", flex: 1 };
  const rCtrl: DiagramKey = { id: "rctrl", glyph: "⌃", sub: "control", kind: "right_control", flex: 1 };
  const fn: DiagramKey = { id: "fn", glyph: "fn", sub: "function", kind: "fn", flex: 1 };

  switch (layout) {
    case "builtin_macbook":
    case "magic_compact":
      // Fn is the bottom-left key — the brand holds here.
      return [fn, lCtrl, lOpt, lCmd, space, rCmd, rOpt, arrows];
    case "magic_extended":
      // Bottom-left is Control. No Fn down here.
      return [lCtrl, lOpt, lCmd, space, rCmd, rOpt, arrows];
    case "generic":
    default:
      // Best-guess full-size PC/Mac row; both sides carry Control.
      return [lCtrl, lOpt, lCmd, space, rCmd, rOpt, rCtrl];
  }
}

/** Pickable keys that don't appear in the drawn bottom row, offered as chips. */
export function extraKeys(layout: KeyboardLayout): { kind: HotkeyKind; note: string }[] {
  const caps = { kind: "caps_lock" as HotkeyKind, note: "top-left · toggle" };
  switch (layout) {
    case "builtin_macbook":
    case "magic_compact":
      return [caps];
    case "magic_extended":
      return [{ kind: "fn", note: "up in the nav cluster" }, caps];
    case "generic":
    default:
      return [{ kind: "fn", note: "if your board has one" }, caps];
  }
}

/** Human phrase for the detected board, used in copy. */
export function layoutHeadline(info: KeyboardInfo): string {
  switch (info.layout) {
    case "builtin_macbook":
      return "Built-in keyboard — Fn is your bottom-left key.";
    case "magic_compact":
      return "Magic Keyboard — Fn is your bottom-left key.";
    case "magic_extended":
      return "Full-size keyboard — your bottom-left key is Control, not Fn.";
    case "generic":
    default:
      return "Couldn't identify your keyboard — pick a key below, or just press one.";
  }
}
