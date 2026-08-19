"use client";

import { useEffect, useRef, useState } from "react";
import { APP_VERSION } from "./version";

// The download CTA points at our own version-agnostic /download route, which
// resolves the current release's macOS arm64 .dmg at request time (see
// app/download/route.ts). This intentionally carries no version and no asset
// filename so a new release never breaks the primary CTA (finding #12).
const DMG_URL = "/download";
// `releases/latest` always redirects to the newest release page — never 404s.
const RELEASE_URL = "https://github.com/todddickerson/funbutton/releases/latest";
const REPO_URL = "https://github.com/todddickerson/funbutton";

export default function Home() {
  return (
    <main className="min-h-screen fb-grid">
      <Nav />
      <Hero />
      <TerminalDemo />
      <WorksWhere />
      <HowItWorks />
      <Comparison />
      <OpenSourceInstall />
      <EmailCapture />
      <PricingSection />
      <Footer />
    </main>
  );
}

/* ------------------------------ github mark ------------------------------ */

/** Inline GitHub mark — no third-party requests, keeps the footer's
 *  "no trackers on this page" claim literally true. */
function GitHubMark({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden className={className}>
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
    </svg>
  );
}

/* ---------------------------------- nav ---------------------------------- */

function Nav() {
  return (
    <nav className="sticky top-0 z-50 border-b border-neutral-900 bg-[#0a0a0a]/85 backdrop-blur">
      <div className="max-w-5xl mx-auto px-6 h-14 flex items-center justify-between">
        <a href="#" className="flex items-center gap-2 font-mono text-xs text-neutral-300">
          <span className="fb-glyph" aria-hidden />
          <span className="font-bold text-neutral-100">FunButton</span>
          <span className="hidden sm:inline text-neutral-600">v{APP_VERSION}-alpha</span>
        </a>
        <div className="flex items-center gap-5 font-mono text-xs">
          <a href="#how" className="hidden sm:inline text-neutral-500 hover:text-red-400 transition">
            how it works
          </a>
          <a href="#compare" className="hidden sm:inline text-neutral-500 hover:text-red-400 transition">
            compare
          </a>
          <a href="#pricing" className="text-neutral-500 hover:text-red-400 transition">
            pricing
          </a>
          <a
            href={REPO_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 text-neutral-300 hover:text-red-400 transition"
          >
            <GitHubMark className="w-4 h-4" />
            <span>source →</span>
          </a>
        </div>
      </div>
    </nav>
  );
}

/* -------------------------------- eyebrow -------------------------------- */

/** Section eyebrow — bar glyph in its own flex cell so wrapped lines hang-indent
 *  under the text, not under the bar (390px wrap fix). */
function Eyebrow({
  children,
  className = "mb-3",
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <p
      className={`flex items-start gap-2 font-mono text-xs uppercase tracking-[0.2em] text-red-400 ${className}`}
    >
      <span aria-hidden className="select-none">
        ▌
      </span>
      <span>{children}</span>
    </p>
  );
}

/* ---------------------------------- hero --------------------------------- */

function Hero() {
  return (
    <section className="max-w-5xl mx-auto px-6 pt-20 pb-14 sm:pt-28">
      <Eyebrow className="mb-6">the dictation button for developers</Eyebrow>

      <h1 className="text-5xl sm:text-7xl font-bold tracking-tight leading-[1.05] max-w-3xl">
        The key at the <span className="whitespace-nowrap">bottom-left</span> of{" "}
        <span className="whitespace-nowrap">your Mac</span>
        <br />
        <span className="text-red-400">finally does something.</span>
      </h1>

      <p className="mt-8 text-lg sm:text-xl text-neutral-400 max-w-xl leading-relaxed">
        Hold{" "}
        <kbd className="inline-flex items-center justify-center px-2 py-0.5 mx-0.5 font-mono text-base font-bold text-red-300 bg-neutral-900 border border-red-500/40 rounded align-middle">
          Fn
        </kbd>
        . Say the thing. Release. Clean text lands at your cursor — in your terminal, your
        editor, your coding agent.
        <br />
        <span className="text-neutral-200">
          Transcribed and cleaned entirely on your Mac. No API key. No account. No cloud.
        </span>
      </p>

      <div className="mt-10 flex flex-col sm:flex-row items-start sm:items-center gap-4">
        <a
          href={DMG_URL}
          className="inline-flex items-center gap-2 px-6 py-3.5 bg-red-500 hover:bg-red-400 active:bg-red-600 text-black font-mono text-sm font-bold rounded-md transition"
        >
          <span className="whitespace-nowrap">↓ Download for macOS</span>
          <span className="whitespace-nowrap text-black/70 font-normal">(Apple Silicon)</span>
        </a>
        <a
          href={RELEASE_URL}
          target="_blank"
          rel="noopener noreferrer"
          className="font-mono text-xs text-neutral-500 hover:text-red-400 transition underline underline-offset-4 decoration-neutral-700"
        >
          v{APP_VERSION} release notes →
        </a>
      </div>

      <p className="mt-4 font-mono text-[11px] text-neutral-600 max-w-xl leading-relaxed">
        A <span className="text-neutral-300">~16 MB</span> download — not the 1.2 GB
        we used to ship (or the ~1 GB the bundle-a-model apps ship). The on-device models
        (Whisper + a cleanup LLM, ~1.1 GB) fetch once on first run, SHA-256-verified and
        resumable, then it&apos;s fully offline forever. GPLv3 — read every line before it
        reads you.
      </p>

      {/* Brew-first happy path. The unsigned-alpha "damaged" dialog only hits the
          MANUAL .dmg download; brew and the curl one-liner clear quarantine for
          you, so most people never see a warning or touch a terminal. Framing
          inverted on purpose — lead with the one command, not the scary xattr. */}
      <div className="mt-5 max-w-xl rounded-md border border-neutral-800 bg-neutral-900/40 p-3.5">
        <p className="font-mono text-[11px] leading-relaxed text-neutral-300">
          <span className="font-bold text-neutral-100">One command, no warnings.</span>{" "}
          Install with Homebrew — no &ldquo;damaged&rdquo; dialog, no{" "}
          <code className="text-neutral-200">xattr</code>, no Apple account:
        </p>
        <pre className="mt-2.5 overflow-x-auto rounded bg-black/40 p-2.5 font-mono text-[11px] leading-relaxed text-neutral-200">
          <code>
            <span className="text-red-400 select-none">$ </span>brew install --cask todddickerson/funbutton/funbutton
          </code>
        </pre>
        <p className="mt-2 font-mono text-[10px] text-neutral-500">
          No Homebrew? A one-line{" "}
          <code className="text-neutral-400">curl</code> installer and the manual
          .dmg are{" "}
          <a
            href="#install"
            className="underline underline-offset-2 decoration-neutral-700 hover:text-neutral-300"
          >
            right below
          </a>
          .
        </p>
      </div>

      {/* the Fn-key gag, rendered */}
      <div className="mt-12 flex flex-wrap items-end gap-1.5 select-none" aria-hidden>
        <div className="fb-keycap fb-keycap-fn w-14 h-14 flex items-end justify-start p-2 font-mono text-sm font-bold">
          fn
        </div>
        <div className="fb-keycap w-14 h-14 flex items-end justify-start p-2 font-mono text-[10px] text-neutral-500">
          ctrl
        </div>
        <div className="fb-keycap w-14 h-14 flex items-end justify-start p-2 font-mono text-[10px] text-neutral-500">
          opt
        </div>
        <div className="fb-keycap w-[70px] h-14 flex items-end justify-start p-2 font-mono text-[10px] text-neutral-500">
          cmd
        </div>
        <p className="w-full mt-3 sm:w-auto sm:mt-0 sm:ml-3 sm:pb-1 font-mono text-xs text-neutral-600">
          ← bottom-left corner. you know the one.
        </p>
      </div>
    </section>
  );
}

/* ----------------------------- terminal demo ----------------------------- */

function TermLine({
  delay,
  children,
  className = "",
}: {
  delay: number;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={`fb-line ${className}`} style={{ animationDelay: `${delay}ms` }}>
      {children}
    </div>
  );
}

function TerminalDemo() {
  const ref = useRef<HTMLElement | null>(null);
  const [play, setPlay] = useState(false);

  // Start the line stagger when the demo actually scrolls into view, so the
  // animation doesn't finish below the fold before anyone sees it.
  useEffect(() => {
    const el = ref.current;
    if (!el || typeof IntersectionObserver === "undefined") {
      setPlay(true);
      return;
    }
    const obs = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setPlay(true);
          obs.disconnect();
        }
      },
      { threshold: 0.3 }
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, []);

  return (
    <section
      ref={ref}
      id="demo"
      className={`max-w-5xl mx-auto px-6 pb-20 scroll-mt-20${play ? " fb-demo-play" : ""}`}
    >
      {/* no-JS fallback: show the terminal's end state */}
      <noscript>
        <style>{`.fb-line { opacity: 1; }`}</style>
      </noscript>
      <div className="rounded-lg border border-neutral-800 bg-[#0d0d0d] overflow-hidden">
        {/* window chrome */}
        <div className="flex items-center gap-2 px-4 py-2.5 border-b border-neutral-800 bg-neutral-900/60">
          <span className="w-3 h-3 rounded-full bg-neutral-700" aria-hidden />
          <span className="w-3 h-3 rounded-full bg-neutral-700" aria-hidden />
          <span className="w-3 h-3 rounded-full bg-neutral-700" aria-hidden />
          <span className="ml-2 font-mono text-[11px] text-neutral-500">
            zsh — funbutton is listening
          </span>
          <span className="ml-auto inline-flex items-center gap-1.5 font-mono text-[11px] text-red-400">
            <span className="fb-rec" aria-hidden /> rec
          </span>
        </div>

        <div className="p-4 sm:p-6 font-mono text-[13px] sm:text-sm leading-relaxed space-y-1.5 overflow-x-auto">
          <TermLine delay={100} className="text-neutral-600">
            <span className="text-neutral-500">~/src/api</span>{" "}
            <span className="text-red-400">$</span>{" "}
            <span className="italic"># hold fn, ramble like a human…</span>
          </TermLine>
          <TermLine delay={350} className="text-neutral-500">
            <span className="text-neutral-600 select-none">you said › </span>
            <span className="italic">
              “okay so um, git commit dash m… fix the race condition in the retry handler — no
              wait, in the backoff logic”
            </span>
          </TermLine>
          <TermLine delay={750} className="text-neutral-100">
            <span className="text-red-400 select-none">funbutton typed › </span>
            <span className="fb-cursor">{`git commit -m "fix race condition in backoff logic"`}</span>
          </TermLine>

          <div className="fb-line pt-4" style={{ animationDelay: "1150ms" }}>
            <div className="text-neutral-600">
              <span className="text-neutral-500">claude code</span>{" "}
              <span className="text-red-400">$</span>{" "}
              <span className="italic"># same button, prompting your agent…</span>
            </div>
          </div>
          <TermLine delay={1400} className="text-neutral-500">
            <span className="text-neutral-600 select-none">you said › </span>
            <span className="italic">
              “uh refactor the auth middleware to use the new session store and and write the
              tests first”
            </span>
          </TermLine>
          <TermLine delay={1800} className="text-neutral-100">
            <span className="text-red-400 select-none">funbutton typed › </span>
            Refactor the auth middleware to use the new session store. Write the tests first.
          </TermLine>
        </div>
      </div>

      <p className="mt-4 font-mono text-xs text-neutral-600 max-w-2xl leading-relaxed">
        Self-corrections resolved. Fillers gone. “dash m” becomes <code>-m</code>. Code mode
        knows you meant a flag; prose mode exists for the humans you Slack. It also reads the
        window you&apos;re in, so your identifiers and jargon come out spelled right — and that
        context stays on your Mac, never logged.
      </p>
    </section>
  );
}

/* ----------------------------- works where ------------------------------- */

function WorksWhere() {
  const places = ["Cursor", "VS Code", "Claude Code", "Terminal", "Slack", "any cursor"];
  return (
    <section className="border-y border-neutral-900 bg-neutral-950/50">
      <div className="max-w-5xl mx-auto px-6 py-8 flex flex-col sm:flex-row sm:items-center gap-4 sm:gap-6">
        <p className="font-mono text-xs uppercase tracking-[0.2em] text-neutral-500 shrink-0">
          works where you work
        </p>
        <ul className="flex flex-wrap gap-2">
          {places.map((p) => (
            <li
              key={p}
              className="px-3 py-1.5 rounded border border-neutral-800 bg-neutral-900/60 font-mono text-xs text-neutral-300"
            >
              {p}
            </li>
          ))}
        </ul>
      </div>
      <div className="max-w-5xl mx-auto px-6 pb-8 -mt-2">
        <p className="font-mono text-xs text-neutral-600">
          No integrations. No plugins. It types wherever your cursor blinks.
        </p>
      </div>
    </section>
  );
}

/* ------------------------------ how it works ----------------------------- */

function HowItWorks() {
  const steps = [
    {
      n: "01",
      title: "Hold Fn",
      body: "Or Right Option. Push-to-talk, so there's no toggle left running while you think out loud.",
    },
    {
      n: "02",
      title: "Whisper transcribes",
      body: "On your Mac, with an on-device model you grab once on first run. Zero key. Works on a plane after that.",
    },
    {
      n: "03",
      title: "A local LLM cleans it up",
      body: "An on-device model kills the fillers, resolves your self-corrections, and reads what's on your screen — the focused window, the file, your selection — so identifiers like getUserById and brand names come out spelled the way your code already spells them. Pin the mode per app or force Terminal; swap the model any time.",
    },
    {
      n: "04",
      title: "It types",
      body: "Pastes at your cursor and restores your clipboard. Feels like you typed it, minus the typing.",
    },
  ];
  return (
    <section id="how" className="max-w-5xl mx-auto px-6 py-20 scroll-mt-20">
      <Eyebrow>how it works</Eyebrow>
      <h2 className="text-3xl sm:text-4xl font-bold tracking-tight mb-10">
        Four steps. All of them on your machine.
      </h2>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {steps.map((s) => (
          <div key={s.n} className="rounded-lg border border-neutral-800 bg-neutral-950/50 p-5">
            <p className="font-mono text-xs text-red-400 mb-3">{s.n}</p>
            <h3 className="font-bold mb-2">{s.title}</h3>
            <p className="text-sm text-neutral-400 leading-relaxed">{s.body}</p>
          </div>
        ))}
      </div>

      <p className="mt-6 font-mono text-xs text-neutral-600 max-w-2xl leading-relaxed">
        Want more horsepower? Point cleanup at your own Ollama, bring a Groq key, or use our
        paid cloud models. All optional. The default path never phones home.
      </p>
    </section>
  );
}

/* ------------------------------- comparison ------------------------------ */

function Comparison() {
  const rows: [string, string, string, string, string][] = [
    ["Price", "$0 · $149 lifetime", "$0", "$25–49 lifetime", "$12–15/mo, forever"],
    ["License", "GPLv3", "MIT", "GPLv3", "closed"],
    ["Download size", "~16 MB download", "app + model", "app + model", "cloud"],
    ["Works offline", "after first-run models", "yes", "yes", "no"],
    ["AI cleanup on-device", "on-device, default", "optional", "optional (cloud text)", "cloud only"],
    ["API key or account", "never", "never", "never", "account required"],
    ["Dev-first defaults", "the whole point", "generic dictation", "generic Power Mode", "generic prose"],
    ["Stack", "Tauri + Rust", "Tauri + Rust", "Swift", "closed"],
  ];

  return (
    <section id="compare" className="max-w-5xl mx-auto px-6 pb-20 scroll-mt-20">
      <Eyebrow>the honest table</Eyebrow>
      <h2 className="text-3xl sm:text-4xl font-bold tracking-tight mb-2">
        Where this sits, exactly.
      </h2>
      <p className="text-neutral-400 max-w-2xl mb-8 leading-relaxed">
        Handy and VoiceInk are excellent open-source projects. Go star them. We exist because
        nobody wakes up thinking about terminals, commit messages, and coding agents.
      </p>

      <div className="overflow-x-auto rounded-lg border border-neutral-800">
        <table className="w-full min-w-[640px] text-sm border-collapse">
          <thead>
            <tr className="border-b border-neutral-800 bg-neutral-900/60 font-mono text-xs">
              <th className="text-left p-3 text-neutral-500 font-normal" />
              <th className="text-left p-3 text-red-400">FunButton</th>
              <th className="text-left p-3 text-neutral-300">
                <a
                  href="https://handy.computer"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-red-400 transition"
                >
                  Handy
                </a>
              </th>
              <th className="text-left p-3 text-neutral-300">
                <a
                  href="https://tryvoiceink.com"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-red-400 transition"
                >
                  VoiceInk
                </a>
              </th>
              <th className="text-left p-3 text-neutral-300">Wispr Flow</th>
            </tr>
          </thead>
          <tbody>
            {rows.map(([label, fb, handy, vi, wispr]) => (
              <tr key={label} className="border-b border-neutral-900 last:border-0">
                <td className="p-3 font-mono text-xs text-neutral-500 whitespace-nowrap">
                  {label}
                </td>
                <td className="p-3 text-neutral-100">{fb}</td>
                <td className="p-3 text-neutral-400">{handy}</td>
                <td className="p-3 text-neutral-400">{vi}</td>
                <td className="p-3 text-neutral-500">{wispr}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <p className="mt-4 font-mono text-xs text-neutral-600 max-w-2xl leading-relaxed">
        Wispr Flow raised ~$334M and still can&apos;t work on a plane. We hold that against
        them slightly less than the subscription.
      </p>
    </section>
  );
}

/* --------------------------- open source + install ------------------------ */

function OpenSourceInstall() {
  return (
    <section id="install" className="max-w-5xl mx-auto px-6 pb-20 scroll-mt-20">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 items-start">
        <div className="min-w-0">
          <Eyebrow>gplv3, built in public</Eyebrow>
          <h2 className="text-3xl sm:text-4xl font-bold tracking-tight mb-4">
            A dictation app hears everything you say. Read its source.
          </h2>
          <p className="text-neutral-400 leading-relaxed mb-4">
            The desktop app is GPLv3. Every commit is public, including the embarrassing ones.
            If we ever do something shady with your audio, you can catch us in the diff.
          </p>
          <a
            href={REPO_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="font-mono text-xs text-neutral-300 hover:text-red-400 underline underline-offset-4 decoration-neutral-700 hover:decoration-red-400 transition"
          >
            github.com/todddickerson/funbutton →
          </a>
        </div>

        <div className="min-w-0 space-y-3">
          <p className="font-mono text-[11px] uppercase tracking-[0.2em] text-neutral-500">
            install — pick one
          </p>

          {/* 1 — Homebrew: primary path, clears quarantine for you */}
          <InstallMethod
            n="1"
            title="Homebrew"
            badge="recommended"
            command="brew install --cask todddickerson/funbutton/funbutton"
            note="No “damaged” dialog, no xattr, no Apple account. Update anytime with brew upgrade --cask funbutton."
          />

          {/* 2 — curl one-liner: same result without Homebrew */}
          <InstallMethod
            n="2"
            title="curl one-liner"
            command="curl -fsSL https://funbutton.ai/install.sh | bash"
            note={
              <>
                No Homebrew? One line, same result — and you can read exactly what
                it does first at{" "}
                <a
                  href="/install.sh"
                  className="underline underline-offset-2 decoration-neutral-700 hover:text-neutral-300"
                >
                  funbutton.ai/install.sh
                </a>
                .
              </>
            }
          />

          {/* 3 — manual .dmg: the ONLY path that still needs the xattr step */}
          <div className="rounded-lg border border-neutral-800 bg-[#0d0d0d] p-4">
            <div className="flex items-center gap-2 mb-2.5">
              <span className="inline-flex h-5 w-5 items-center justify-center rounded bg-neutral-800 font-mono text-[11px] text-neutral-300">
                3
              </span>
              <span className="font-mono text-[13px] text-neutral-200">Manual .dmg</span>
              <span className="font-mono text-[10px] text-neutral-600">the DIY path</span>
            </div>
            <a
              href={DMG_URL}
              className="inline-flex items-center gap-2 px-3 py-2 bg-neutral-800 hover:bg-neutral-700 text-neutral-100 font-mono text-[12px] rounded transition"
            >
              ↓ Download .dmg <span className="text-neutral-400">(Apple Silicon)</span>
            </a>
            <p className="mt-3 font-mono text-[11px] leading-relaxed text-neutral-500">
              This build is unsigned, so macOS mislabels the download as
              &ldquo;damaged&rdquo; until you clear quarantine on both the .dmg and
              the app:
            </p>
            <pre className="mt-2 overflow-x-auto rounded bg-black/40 p-2.5 font-mono text-[11px] leading-relaxed text-neutral-200">
              <code>
                <span className="text-red-400 select-none">$ </span>xattr -cr ~/Downloads/FunButton_{APP_VERSION}_aarch64.dmg{"\n"}
                <span className="text-red-400 select-none">$ </span>xattr -cr /Applications/FunButton.app
              </code>
            </pre>
            <p className="mt-2 font-mono text-[10px] leading-relaxed text-neutral-600">
              Brew and the one-liner already do this for you — this step is{" "}
              <span className="text-neutral-500">only</span> for the manual
              download. Real signing is on the way (Developer ID pending): see{" "}
              <a
                href={`${REPO_URL}/blob/main/SIGNING.md`}
                target="_blank"
                rel="noopener noreferrer"
                className="underline underline-offset-2 decoration-neutral-700 hover:text-neutral-400"
              >
                SIGNING.md
              </a>
              .
            </p>
          </div>

          <p className="font-mono text-[10px] leading-relaxed text-neutral-600">
            Then: grant Microphone · Accessibility · Input Monitoring, pick your
            hotkey, hold it, talk. The on-device models (~1.1 GB) fetch once on
            first run.
          </p>

          <AgentInstall />
        </div>
      </div>
    </section>
  );
}

/** Compact secondary path: hand the install to a coding agent. Deliberately quieter
 *  than the numbered methods above — brew stays the primary human path. The copy
 *  button drops a short, model-agnostic prompt that points the agent at the
 *  machine-readable contract (/install.json) and AGENT-INSTALL.md. */
const AGENT_PROMPT =
  "Install FunButton on my Mac (macOS, Apple Silicon). Follow " +
  "https://funbutton.ai/install.json (machine-readable) and " +
  "https://github.com/todddickerson/funbutton/blob/main/AGENT-INSTALL.md. " +
  "Prefer the Homebrew cask, verify the app launches, surface the ~1.1 GB " +
  "first-run model download, and hand the macOS permission grants (Microphone, " +
  "Accessibility, Input Monitoring) back to me — you can't grant those yourself.";

function AgentInstall() {
  return (
    <div className="mt-1 rounded-lg border border-neutral-800/70 bg-[#0b0b0b] p-4">
      <div className="flex items-center justify-between gap-2 mb-2">
        <span className="font-mono text-[11px] uppercase tracking-[0.15em] text-neutral-400">
          installing with a coding agent?
        </span>
        <CopyButton text={AGENT_PROMPT} label="copy prompt" />
      </div>
      <p className="font-mono text-[11px] leading-relaxed text-neutral-500 mb-2.5">
        Paste this into Claude Code, OpenClaw, Cursor, Codex, or Hermes — it installs
        and verifies, then hands the macOS permission grants back to you.
      </p>
      <pre className="overflow-x-auto rounded bg-black/40 p-2.5 font-mono text-[11px] leading-relaxed text-neutral-300 whitespace-pre-wrap break-words">
        <code>{AGENT_PROMPT}</code>
      </pre>
      <p className="mt-2.5 font-mono text-[10px] leading-relaxed text-neutral-600">
        Or point the agent straight at the machine-readable contract:{" "}
        <a
          href="/install.json"
          className="underline underline-offset-2 decoration-neutral-700 hover:text-neutral-400"
        >
          funbutton.ai/install.json
        </a>{" "}
        ·{" "}
        <a
          href="/llms.txt"
          className="underline underline-offset-2 decoration-neutral-700 hover:text-neutral-400"
        >
          llms.txt
        </a>
      </p>
    </div>
  );
}

/** Clipboard button styled for the terminal aesthetic — text label, no icon dep,
 *  no emoji. Flips to a confirmation for ~1.5s after a successful copy. */
function CopyButton({ text, label }: { text: string; label: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      onClick={async () => {
        try {
          await navigator.clipboard.writeText(text);
          setCopied(true);
          setTimeout(() => setCopied(false), 1500);
        } catch {
          // Clipboard API can be blocked (insecure context / denied permission);
          // leave the visible prompt so it can still be selected and copied by hand.
        }
      }}
      className="shrink-0 rounded border border-neutral-700 px-2 py-1 font-mono text-[10px] text-neutral-300 hover:border-red-400 hover:text-red-400 transition"
      aria-label={copied ? "Copied to clipboard" : label}
    >
      {copied ? "copied" : label}
    </button>
  );
}

/** One install method row — a numbered command block with a one-line note.
 *  Used for the brew and curl paths; the manual .dmg gets its own block because
 *  it carries the download button and the demoted xattr step. */
function InstallMethod({
  n,
  title,
  badge,
  command,
  note,
}: {
  n: string;
  title: string;
  badge?: string;
  command: string;
  note: React.ReactNode;
}) {
  return (
    <div className="rounded-lg border border-neutral-800 bg-[#0d0d0d] p-4">
      <div className="flex items-center gap-2 mb-2.5">
        <span className="inline-flex h-5 w-5 items-center justify-center rounded bg-neutral-800 font-mono text-[11px] text-neutral-300">
          {n}
        </span>
        <span className="font-mono text-[13px] text-neutral-200">{title}</span>
        {badge && (
          <span className="rounded bg-red-500/15 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-red-300">
            {badge}
          </span>
        )}
      </div>
      <pre className="overflow-x-auto rounded bg-black/40 p-2.5 font-mono text-[12px] leading-relaxed text-neutral-100">
        <code>
          <span className="text-red-400 select-none">$ </span>
          {command}
        </code>
      </pre>
      <p className="mt-2 font-mono text-[11px] leading-relaxed text-neutral-500">{note}</p>
    </div>
  );
}

/* ------------------------------ email capture ---------------------------- */

function EmailCapture() {
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
    <section className="border-y border-neutral-900 bg-neutral-950/50">
      <div className="max-w-5xl mx-auto px-6 py-14">
        <h2 className="text-2xl sm:text-3xl font-bold tracking-tight mb-2">
          Release notes in your inbox. Nothing else.
        </h2>
        <p className="text-neutral-400 mb-6 max-w-xl">
          New builds, breaking changes, and the occasional apology. No drip campaign — we
          don&apos;t have a marketing team.
        </p>

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
              className="flex-1 px-4 py-3 bg-neutral-900 border border-neutral-800 rounded-md font-mono text-base text-neutral-100 placeholder:text-neutral-600 focus:outline-none focus:border-red-500/60 focus:ring-2 focus:ring-red-500/20 transition disabled:opacity-50"
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

        {state === "err" && <p className="mt-3 text-sm font-mono text-red-400">{errMsg}</p>}
      </div>
    </section>
  );
}

function SuccessState() {
  return (
    <div className="border border-green-500/30 bg-green-500/5 rounded-md p-5 max-w-lg">
      <p className="font-mono text-sm text-green-400">✓ you&apos;re on the list.</p>
      <p className="mt-1 text-sm text-neutral-400">Watch this space.</p>
      <div className="mt-4 pt-4 border-t border-neutral-800 flex flex-wrap items-center gap-3 text-xs font-mono">
        <a
          href={REPO_URL}
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-1.5 text-neutral-400 hover:text-neutral-100 transition"
        >
          <GitHubMark className="w-3.5 h-3.5" />
          <span>star the repo →</span>
        </a>
        <span className="text-neutral-700">|</span>
        <a
          href={DMG_URL}
          className="text-neutral-500 hover:text-red-400 transition underline underline-offset-4 decoration-neutral-800"
        >
          for the brave: download the alpha →
        </a>
      </div>
    </div>
  );
}

/* --------------------------------- pricing ------------------------------- */

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
        setNotice(
          "Checkout opens soon — join the waitlist above and we'll email you the moment it goes live."
        );
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
    <section id="pricing" className="max-w-5xl mx-auto px-6 py-20 scroll-mt-20">
      <Eyebrow>pricing</Eyebrow>
      <h2 className="text-3xl sm:text-4xl font-bold tracking-tight leading-tight mb-2">
        Pay once. Pay monthly. Or don&apos;t pay at all.
      </h2>
      <p className="text-neutral-400 max-w-2xl mb-10 leading-relaxed">
        Free forever on the bundled local models, your own Groq key, or Ollama. Pro adds
        premium cleanup models and 50K words/mo included. Lifetime is one-time and never goes
        up after we hit the next ladder rung.
      </p>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <PriceCard
          tier="free"
          name="Free"
          price="$0"
          period="forever"
          features={[
            "Fully local: on-device Whisper + LLM",
            "~16 MB download · models fetch on first run",
            "Unlimited usage, or BYO Groq / Ollama",
            "GPLv3 — open source desktop",
          ]}
          cta="Download alpha"
          ctaHref={DMG_URL}
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

      {notice && <p className="mt-6 text-sm font-mono text-amber-400">{notice}</p>}

      <p className="mt-8 font-mono text-xs text-neutral-600 max-w-2xl">
        Premium models priced per 10K words: Haiku $0.40 · Sonnet $0.60 · Opus $0.99 · GPT-4.1
        $0.50. Cap defaults to $0 (hard stop, fast-tier fallback). You can raise the cap up to
        $100/mo at any time.
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
  return (
    <div
      className={`rounded-lg border p-6 flex flex-col ${
        highlight
          ? "border-red-500/60 bg-neutral-900/60"
          : "border-neutral-800 bg-neutral-950/50"
      }`}
    >
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
      {subPrice && <p className="font-mono text-xs text-neutral-500 mb-4">{subPrice}</p>}
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

/* --------------------------------- footer -------------------------------- */

function Footer() {
  return (
    <footer className="border-t border-neutral-900">
      <div className="max-w-5xl mx-auto px-6 py-10 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div className="flex items-center gap-2 font-mono text-xs text-neutral-500">
          <span className="fb-glyph" aria-hidden />
          <span>FunButton — one button. your computer. your data.</span>
        </div>
        <div className="flex flex-wrap items-center gap-4 font-mono text-[11px] text-neutral-600">
          <a href={REPO_URL} target="_blank" rel="noopener noreferrer" className="hover:text-red-400 transition">
            source
          </a>
          <a href={RELEASE_URL} target="_blank" rel="noopener noreferrer" className="hover:text-red-400 transition">
            releases
          </a>
          <a
            href="https://www.gnu.org/licenses/gpl-3.0.html"
            target="_blank"
            rel="noopener noreferrer"
            className="hover:text-red-400 transition"
          >
            gplv3
          </a>
          <span>© 2026 · no trackers on this page.</span>
        </div>
      </div>
    </footer>
  );
}
