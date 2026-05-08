import { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import "./pill.css";

function Pill() {
  const [status, setStatus] = useState("recording");
  const [msg, setMsg] = useState<string | null>(null);

  useEffect(() => {
    const u = listen<{ status: string; message: string | null }>("funbutton:status", (e) => {
      setStatus(e.payload.status);
      setMsg(e.payload.message ?? null);
    });
    return () => { u.then((un) => un()); };
  }, []);

  const label =
    status === "recording" ? "● recording" :
    status === "transcribing" ? "◐ transcribing" :
    status === "cleaning" ? "◑ cleaning" :
    status === "pasting" ? "✎ pasting" :
    status === "error" ? "⚠ error" : "● idle";

  return (
    <div className={`pill pill-${status}`}>
      <span className="pill-label">{label}</span>
      {msg && <span className="pill-msg">{msg}</span>}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(<Pill />);
