import { ImageResponse } from "next/og";

export const alt =
  "FunButton — the dictation button for developers. No API key. Ever.";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default function OpengraphImage() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          padding: "60px 72px",
          backgroundColor: "#0a0a0a",
          backgroundImage:
            "linear-gradient(rgba(255,255,255,0.05) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.05) 1px, transparent 1px)",
          backgroundSize: "48px 48px",
          color: "#ededed",
          fontFamily: "sans-serif",
        }}
      >
        {/* brand row */}
        <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
          <div
            style={{
              width: 30,
              height: 30,
              borderRadius: 9999,
              backgroundColor: "#ef4444",
              boxShadow: "inset 0 -5px 0 rgba(0,0,0,0.35)",
            }}
          />
          <div
            style={{ display: "flex", fontSize: 30, fontWeight: 700, color: "#f5f5f5" }}
          >
            FunButton
          </div>
          <div style={{ display: "flex", fontSize: 22, color: "#525252" }}>
            v0.1.4-alpha
          </div>
        </div>

        {/* main: red Fn keycap + headline */}
        <div style={{ display: "flex", alignItems: "center", gap: 56 }}>
          <div
            style={{
              width: 190,
              height: 190,
              borderRadius: 28,
              backgroundColor: "#ef4444",
              border: "3px solid #b91c1c",
              boxShadow:
                "inset 0 -16px 0 rgba(0,0,0,0.35), inset 0 3px 0 rgba(255,255,255,0.25)",
              display: "flex",
              alignItems: "flex-end",
              padding: 22,
              color: "#0a0a0a",
              fontSize: 46,
              fontWeight: 700,
              flexShrink: 0,
            }}
          >
            fn
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 20, flex: 1 }}>
            <div
              style={{
                display: "flex",
                fontSize: 64,
                fontWeight: 700,
                lineHeight: 1.08,
                color: "#fafafa",
                letterSpacing: "-0.02em",
              }}
            >
              The dictation button for developers.
            </div>
            <div
              style={{
                display: "flex",
                fontSize: 36,
                fontWeight: 700,
                color: "#f87171",
              }}
            >
              No API key. Ever.
            </div>
          </div>
        </div>

        {/* footer row */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            fontSize: 22,
            color: "#737373",
          }}
        >
          <div style={{ display: "flex" }}>funbutton.ai</div>
          <div style={{ display: "flex" }}>
            on-device Whisper + local cleanup · GPLv3 · macOS
          </div>
        </div>
      </div>
    ),
    size
  );
}
