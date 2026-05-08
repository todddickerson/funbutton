"use client";

import { useState } from "react";

export default function ComingSoon() {
  const [email, setEmail] = useState("");
  const [state, setState] = useState<"idle" | "loading" | "ok" | "err">("idle");
  const [errMsg, setErrMsg] = useState("");

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!email || !email.includes("@")) {
      setErrMsg("that's not an email.");
      setState("err");
      return;
    }
    setState("loading");
    try {
      const res = await fetch("/api/subscribe", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email }),
      });
      if (res.ok) {
        setState("ok");
      } else {
        const j = await res.json().catch(() => ({}));
        setErrMsg(j.error || "something broke. try again.");
        setState("err");
      }
    } catch {
      setErrMsg("network error. try again.");
      setState("err");
    }
  }

  return (
    <main className="min-h-screen fb-grid relative overflow-hidden">
      {/* corner mark */}
      <div className="absolute top-6 left-6 flex items-center gap-2 font-mono text-xs text-neutral-500">
        <span className="fb-glyph" aria-hidden />
        <span>FunButton</span>
      </div>
      <div className="absolute top-6 right-6 font-mono text-xs text-neutral-500">
        v0.1.0-alpha
      </div>

      <section className="max-w-3xl mx-auto px-6 pt-32 pb-16 sm:pt-40">
        <p className="font-mono text-xs uppercase tracking-[0.2em] text-red-400 mb-6">
          ▌ coming soon
        </p>

        <h1 className="text-5xl sm:text-7xl font-bold tracking-tight leading-[1.05]">
          Talk fast.
          <br />
          Stay local.
          <br />
          <span className="text-red-400">Pay less.</span>
        </h1>

        <p className="mt-8 text-lg sm:text-xl text-neutral-400 max-w-xl leading-relaxed">
          Voice dictation for people who actually ship. One button, your laptop,
          no cloud tax. <span className="text-neutral-200">Wispr Flow without the SaaS.</span>
        </p>

        {/* Email capture */}
        <div className="mt-12">
          {state === "ok" ? (
            <SuccessState />
          ) : (
            <form onSubmit={onSubmit} className="flex flex-col sm:flex-row gap-3 max-w-lg">
              <input
                type="email"
                required
                value={email}
                onChange={(e) => {
                  setEmail(e.target.value);
                  if (state === "err") setState("idle");
                }}
                placeholder="you@domain.com"
                disabled={state === "loading"}
                className="flex-1 px-4 py-3 bg-neutral-900 border border-neutral-800 rounded-md font-mono text-sm text-neutral-100 placeholder:text-neutral-600 focus:outline-none focus:border-red-500/60 focus:ring-2 focus:ring-red-500/20 transition disabled:opacity-50"
              />
              <button
                type="submit"
                disabled={state === "loading"}
                className="px-5 py-3 bg-red-500 hover:bg-red-400 active:bg-red-600 text-black font-mono text-sm font-bold rounded-md transition disabled:opacity-50 whitespace-nowrap"
              >
                {state === "loading" ? "..." : "Get notified →"}
              </button>
            </form>
          )}

          {state === "err" && (
            <p className="mt-3 text-sm font-mono text-red-400">{errMsg}</p>
          )}
        </div>

        {/* Social proof / build-in-public line */}
        <p className="mt-10 font-mono text-xs text-neutral-500">
          built in public →{" "}
          <a
            href="https://github.com/todddickerson/funbutton"
            target="_blank"
            rel="noopener noreferrer"
            className="text-neutral-300 hover:text-red-400 underline underline-offset-4 decoration-neutral-700 hover:decoration-red-400 transition"
          >
            github.com/todddickerson/funbutton
          </a>
        </p>
      </section>

      {/* footer */}
      <footer className="absolute bottom-6 left-0 right-0 px-6 font-mono text-[10px] text-neutral-600 flex flex-wrap justify-between gap-2">
        <span>© 2026 — no trackers on this page.</span>
        <span>
          one button. your computer. your data.
        </span>
      </footer>
    </main>
  );
}

function SuccessState() {
  return (
    <div className="border border-green-500/30 bg-green-500/5 rounded-md p-5 max-w-lg">
      <p className="font-mono text-sm text-green-400">
        ✓ you&apos;re on the list.
      </p>
      <p className="mt-1 text-sm text-neutral-400">
        Watch this space.
      </p>
      <div className="mt-4 pt-4 border-t border-neutral-800 flex flex-wrap items-center gap-3 text-xs font-mono">
        <a
          href="https://github.com/todddickerson/funbutton"
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-1.5 text-neutral-400 hover:text-neutral-100 transition"
        >
          <img
            src="https://img.shields.io/github/stars/todddickerson/funbutton?style=flat&label=stars&color=ef4444&labelColor=171717"
            alt="GitHub stars"
            className="h-5"
          />
        </a>
        <span className="text-neutral-700">|</span>
        <a
          href="https://github.com/todddickerson/funbutton/releases"
          target="_blank"
          rel="noopener noreferrer"
          className="text-neutral-500 hover:text-red-400 transition underline underline-offset-4 decoration-neutral-800"
        >
          for the brave: download the alpha →
        </a>
      </div>
    </div>
  );
}
