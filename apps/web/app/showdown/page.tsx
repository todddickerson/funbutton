import data from "./data.json";
import Link from "next/link";

export const metadata = {
  title: "FunButton — Showdown · raw transcription vs AI cleanup",
  description:
    "Real captured outputs. Same messy spoken sentence in. Other tools paste the raw mess. FunButton pastes the rewrite. Five scenarios, no cherry-picking.",
};

interface Scenario {
  label: string;
  mode: string;
  utterance: string;
  raw: string;
  cleaned: string;
}

const SCENARIOS = data as Scenario[];

const TITLES: Record<string, { title: string; sub: string; surface: string }> = {
  "rambling-email": {
    title: "The rambling email follow-up",
    sub: "Real meeting context, real fillers, real \"um you know\".",
    surface: "Mail.app · Email mode",
  },
  "slack-correction": {
    title: "The Slack message you change mid-sentence",
    sub: "Said one thing, immediately changed your mind. The raw transcript is wrong. The cleanup is right.",
    surface: "Slack · Slack mode",
  },
  "code-arrow-fn": {
    title: "The arrow function with spoken symbols",
    sub: "\"open paren camelCase user data comma options close paren fat arrow…\" Try this in Whisper alone.",
    surface: "Cursor · Code mode",
  },
  "snake-case-fn": {
    title: "The function signature with types",
    sub: "Speak the function. Get the syntax. Plus types.",
    surface: "VS Code · Code mode",
  },
  "mid-sentence-redirect": {
    title: "The \"actually no scratch that\" pivot",
    sub: "The wedge in one sample. Nobody else solves this.",
    surface: "Notes · Auto mode",
  },
};

export default function Showdown() {
  return (
    <main className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-50">
      <nav className="mx-auto max-w-6xl flex items-center justify-between px-6 py-5">
        <Link href="/" className="flex items-center gap-2">
          <span className="text-2xl text-pink-500">●</span>
          <span className="font-bold text-lg tracking-tight">FunButton</span>
        </Link>
        <div className="flex items-center gap-5 text-sm">
          <Link href="/" className="hover:text-pink-500">home</Link>
          <Link href="/#price" className="hover:text-pink-500">price</Link>
          <a href="https://github.com/todddickerson/funbutton" target="_blank" rel="noreferrer" className="hover:text-pink-500">github</a>
        </div>
      </nav>

      {/* Hero */}
      <section className="mx-auto max-w-6xl px-6 pt-12 pb-16 text-center">
        <p className="text-[11px] uppercase tracking-[0.3em] text-pink-500 mb-4">the showdown · real outputs · zero cherry-picking</p>
        <h1 className="text-5xl sm:text-6xl font-black tracking-tighter leading-[0.95]">
          Other tools transcribe.<br />
          <span className="text-pink-500">FunButton rewrites.</span>
        </h1>
        <p className="mt-6 max-w-2xl mx-auto text-lg text-zinc-600 dark:text-zinc-400 leading-relaxed">
          Same messy spoken input. Five scenarios. The left column is what Whisper hears — what most apps paste.
          The right column is what FunButton sends to your cursor.
        </p>
        <p className="mt-3 text-xs text-zinc-500">
          Captured live from Groq Whisper Large v3 Turbo + Llama 3.3 70B with the exact prompts in the app source.
          Re-run yourself: <code className="bg-zinc-200 dark:bg-zinc-800 px-1.5 py-0.5 rounded text-[11px]">scripts/capture_showdown.sh</code>
        </p>
      </section>

      {/* Scenarios */}
      <section className="mx-auto max-w-6xl px-6 pb-20 space-y-12">
        {SCENARIOS.map((s, i) => {
          const meta = TITLES[s.label];
          return (
            <div key={s.label} className="space-y-3">
              <div className="flex items-baseline justify-between flex-wrap gap-3">
                <div>
                  <div className="flex items-center gap-3 mb-1">
                    <span className="font-mono text-pink-500 text-xs">scene {String(i + 1).padStart(2, "0")}</span>
                    <span className="text-[10px] uppercase tracking-widest text-zinc-500 border border-zinc-300 dark:border-zinc-700 px-1.5 py-0.5 rounded">{meta?.surface ?? s.mode}</span>
                  </div>
                  <h2 className="text-2xl sm:text-3xl font-black tracking-tight">{meta?.title ?? s.label}</h2>
                  {meta?.sub && <p className="text-sm text-zinc-500 mt-1 max-w-2xl">{meta.sub}</p>}
                </div>
              </div>

              <div className="grid md:grid-cols-2 gap-4">
                <Panel kind="raw" title="What Whisper heard (raw transcript)">
                  {s.raw}
                </Panel>
                <Panel kind="cleaned" title="What FunButton pasted">
                  {s.cleaned}
                </Panel>
              </div>

              <details className="text-xs text-zinc-500 mt-2">
                <summary className="cursor-pointer hover:text-pink-500">what we actually said into the mic →</summary>
                <p className="mt-2 italic font-mono leading-relaxed bg-zinc-100 dark:bg-zinc-900 p-3 rounded border border-zinc-200 dark:border-zinc-800">
                  &ldquo;{s.utterance}&rdquo;
                </p>
              </details>
            </div>
          );
        })}
      </section>

      {/* CTA */}
      <section className="mx-auto max-w-6xl px-6 py-20 border-t border-zinc-200 dark:border-zinc-800 text-center">
        <h2 className="text-3xl sm:text-4xl font-black tracking-tight">Stop typing. Start talking.</h2>
        <p className="mt-4 text-zinc-600 dark:text-zinc-400 max-w-xl mx-auto">
          Your voice, your machine, your cursor. Local-first, dev-grade, no API key required (with Ollama).
        </p>
        <div className="mt-8 flex flex-col sm:flex-row items-center justify-center gap-3">
          <a href="https://github.com/todddickerson/funbutton/releases/latest" className="bg-pink-500 hover:bg-pink-600 text-white font-bold px-7 py-3.5 rounded-full text-base transition">
            Download for Mac
          </a>
          <Link href="/" className="border-2 border-zinc-900 dark:border-zinc-100 px-6 py-3 rounded-full font-semibold hover:bg-zinc-900 hover:text-zinc-50 dark:hover:bg-zinc-100 dark:hover:text-zinc-900 transition">
            ← Back to home
          </Link>
        </div>
      </section>
    </main>
  );
}

function Panel({ kind, title, children }: { kind: "raw" | "cleaned"; title: string; children: React.ReactNode }) {
  const styles = kind === "raw"
    ? "border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 text-zinc-700 dark:text-zinc-300"
    : "border-pink-400 dark:border-pink-700 bg-pink-50 dark:bg-pink-950/30 text-zinc-900 dark:text-zinc-50";
  const dotColor = kind === "raw" ? "bg-zinc-400" : "bg-pink-500";
  return (
    <div className={`rounded-lg p-5 border-2 ${styles} relative`}>
      <div className="flex items-center gap-2 mb-3">
        <span className={`w-2 h-2 rounded-full ${dotColor}`} />
        <span className="text-[10px] uppercase tracking-widest opacity-70 font-semibold">{title}</span>
      </div>
      <div className="text-base leading-relaxed font-mono whitespace-pre-wrap">{children}</div>
    </div>
  );
}
