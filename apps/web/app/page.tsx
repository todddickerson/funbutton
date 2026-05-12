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
          The key at the bottom-left
          <br />
          of your Mac
          <br />
          <span className="text-red-400">finally does something.</span>
        </h1>

        <p className="mt-8 text-lg sm:text-xl text-neutral-400 max-w-xl leading-relaxed">
          Hold <kbd className="inline-flex items-center justify-center px-2 py-0.5 mx-0.5 font-mono text-base font-bold text-red-300 bg-neutral-900 border border-red-500/40 rounded shadow-[0_0_12px_rgba(239,68,68,0.25)] align-middle">Fn</kbd>. Talk. Release. Get clean text — your dictionary, your tone, your machine.
          <br className="hidden sm:inline" />
          <span className="text-neutral-200">No cloud tax. No subscription. This is the Fun Button.</span>
        </p>

        <p className="mt-4 font-mono text-xs text-neutral-600 max-w-xl">
          → talk fast · stay local · pay less
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

      <PricingSection />

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

function PricingSection() {
  const [busy, setBusy] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function buy(tier: string) {
    setBusy(tier);
    setNotice(null);
    try {
      const res = await fetch("/api/checkout", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tier }),
      });
      if (res.status === 503) {
        setNotice("Checkout opens soon — join the waitlist above and we'll email you the moment it goes live.");
        return;
      }
      if (!res.ok) {
        setNotice("Checkout temporarily unavailable. Try again in a minute.");
        return;
      }
      const json = (await res.json()) as { url?: string };
      if (json.url) {
        window.location.href = json.url;
      } else {
        setNotice("Checkout temporarily unavailable.");
      }
    } catch {
      setNotice("Network error. Try again.");
    } finally {
      setBusy(null);
    }
  }

  return (
    <section id="pricing" className="max-w-5xl mx-auto px-6 pb-24">
      <p className="font-mono text-xs uppercase tracking-[0.2em] text-red-400 mb-3">
        ▌ pricing
      </p>
      <h2 className="text-3xl sm:text-4xl font-bold tracking-tight leading-tight mb-2">
        Pay once. Pay monthly. Or don&apos;t pay at all.
      </h2>
      <p className="text-neutral-400 max-w-2xl mb-10 leading-relaxed">
        Free forever on Groq BYOK or local Ollama. Pro adds premium cleanup models and 50K
        words/mo included. Lifetime is one-time and never goes up after we hit the next ladder rung.
      </p>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <PriceCard
          tier="free"
          name="Free"
          price="$0"
          period="forever"
          features={[
            "Unlimited usage on your own Groq key",
            "Or fully local via Ollama",
            "GPLv3 — open source desktop",
            "No cap, no card, no cloud lock-in",
          ]}
          cta="Download alpha"
          ctaHref="https://github.com/todddickerson/funbutton/releases"
          ctaTarget="_blank"
        />
        <PriceCard
          tier="pro_annual"
          name="Pro"
          price="$79"
          period="/yr"
          subPrice="or $9/mo"
          features={[
            "50K premium cleanup words/mo (Haiku 4.5)",
            "Sonnet, Opus, GPT-4.1 selectable",
            "Metered overage with user-set cap",
            "Auto top-up OFF by default",
          ]}
          cta={busy === "pro_annual" ? "…" : "Get Pro"}
          onCta={() => buy("pro_annual")}
          highlight
        />
        <PriceCard
          tier="lifetime"
          name="Lifetime"
          price="$149"
          period="once"
          subPrice="first 1,000 customers"
          features={[
            "Groq fast tier unlimited forever",
            "Premium cleanup pay-as-you-go",
            "Price climbs to $199 then $249",
            "No recurring charges on the base",
          ]}
          cta={busy === "lifetime" ? "…" : "Get Lifetime"}
          onCta={() => buy("lifetime")}
        />
      </div>

      {notice && (
        <p className="mt-6 text-sm font-mono text-amber-400">
          {notice}
        </p>
      )}

      <p className="mt-8 font-mono text-xs text-neutral-600 max-w-2xl">
        Premium models priced per 10K words: Haiku $0.40 · Sonnet $0.60 · Opus $0.99 · GPT-4.1 $0.50.
        Cap defaults to $0 (hard stop, fast-tier fallback). You can raise the cap up to $100/mo at any time.
      </p>
    </section>
  );
}

interface PriceCardProps {
  tier: string;
  name: string;
  price: string;
  period: string;
  subPrice?: string;
  features: string[];
  cta: string;
  ctaHref?: string;
  ctaTarget?: string;
  onCta?: () => void;
  highlight?: boolean;
}

function PriceCard({
  name,
  price,
  period,
  subPrice,
  features,
  cta,
  ctaHref,
  ctaTarget,
  onCta,
  highlight,
}: PriceCardProps) {
  const ringClass = highlight
    ? "border-red-500/60 shadow-[0_0_24px_rgba(239,68,68,0.15)]"
    : "border-neutral-800";

  return (
    <div className={`rounded-lg border ${ringClass} bg-neutral-950/50 p-6 flex flex-col`}>
      <div className="flex items-baseline justify-between mb-1">
        <h3 className="text-lg font-bold">{name}</h3>
        {highlight && (
          <span className="font-mono text-[10px] uppercase tracking-wider text-red-400">
            most popular
          </span>
        )}
      </div>
      <div className="flex items-baseline gap-1 mb-1">
        <span className="text-3xl font-bold">{price}</span>
        <span className="text-neutral-500 text-sm">{period}</span>
      </div>
      {subPrice && (
        <p className="font-mono text-xs text-neutral-500 mb-4">{subPrice}</p>
      )}
      {!subPrice && <div className="mb-4" />}

      <ul className="space-y-2 mb-6 flex-1">
        {features.map((f) => (
          <li key={f} className="text-sm text-neutral-300 flex gap-2">
            <span className="text-red-400 mt-0.5">→</span>
            <span>{f}</span>
          </li>
        ))}
      </ul>

      {ctaHref ? (
        <a
          href={ctaHref}
          target={ctaTarget}
          rel={ctaTarget === "_blank" ? "noopener noreferrer" : undefined}
          className={`w-full text-center px-4 py-2.5 rounded-md font-mono text-sm font-bold transition ${
            highlight
              ? "bg-red-500 hover:bg-red-400 text-black"
              : "border border-neutral-700 hover:border-red-500/60 hover:text-red-300 text-neutral-200"
          }`}
        >
          {cta}
        </a>
      ) : (
        <button
          onClick={onCta}
          className={`w-full text-center px-4 py-2.5 rounded-md font-mono text-sm font-bold transition ${
            highlight
              ? "bg-red-500 hover:bg-red-400 text-black"
              : "border border-neutral-700 hover:border-red-500/60 hover:text-red-300 text-neutral-200"
          }`}
        >
          {cta}
        </button>
      )}
    </div>
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
