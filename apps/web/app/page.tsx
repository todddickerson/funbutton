import Link from "next/link";

const REPO = "https://github.com/todddickerson/funbutton";
const RELEASE = `${REPO}/releases/latest`;

export default function Home() {
  return (
    <main className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-50">
      {/* Nav */}
      <nav className="mx-auto max-w-6xl flex items-center justify-between px-6 py-5">
        <div className="flex items-center gap-2">
          <span className="text-2xl text-pink-500">●</span>
          <span className="font-bold text-lg tracking-tight">FunButton</span>
          <span className="ml-2 text-[10px] uppercase tracking-widest text-zinc-500 border border-zinc-300 dark:border-zinc-700 px-1.5 py-0.5 rounded">v0.1.0 · alpha</span>
        </div>
        <div className="flex items-center gap-5 text-sm">
          <Link href="/showdown" className="hover:text-pink-500 font-semibold text-pink-500">showdown</Link>
          <Link href="#how" className="hover:text-pink-500">how</Link>
          <Link href="#vs" className="hover:text-pink-500">vs wispr</Link>
          <Link href="#price" className="hover:text-pink-500">price</Link>
          <a href={REPO} target="_blank" rel="noreferrer" className="hover:text-pink-500">github</a>
        </div>
      </nav>

      {/* Hero */}
      <section className="mx-auto max-w-6xl px-6 pt-20 pb-24 sm:pt-28 sm:pb-32 text-center">
        <p className="text-[11px] uppercase tracking-[0.3em] text-pink-500 mb-6">no cloud · no api key · no electron tax</p>
        <h1 className="text-5xl sm:text-7xl font-black tracking-tighter leading-[0.95]">
          Press the<br />
          <span className="text-pink-500">fun button.</span><br />
          Talk to your computer<br />like it&apos;s a friend.
        </h1>
        <p className="mt-8 max-w-2xl mx-auto text-lg sm:text-xl text-zinc-600 dark:text-zinc-400 leading-relaxed">
          A dev-grade voice dictation tool. Press <kbd className="rounded border border-zinc-300 dark:border-zinc-700 px-2 py-1 text-sm font-mono">Right Option</kbd>, talk, watch your cleaned-up text land at the cursor.
          Local-first. Code-aware. Half the price of Wispr.
        </p>
        <div className="mt-10 flex flex-col sm:flex-row items-center justify-center gap-3">
          <a href={RELEASE} className="bg-pink-500 hover:bg-pink-600 text-white font-bold px-7 py-3.5 rounded-full text-base transition">
            Download for Mac (arm64)
          </a>
          <a href={REPO} target="_blank" rel="noreferrer" className="border-2 border-zinc-900 dark:border-zinc-100 px-6 py-3 rounded-full font-semibold hover:bg-zinc-900 hover:text-zinc-50 dark:hover:bg-zinc-100 dark:hover:text-zinc-900 transition">
            Star on GitHub →
          </a>
        </div>
        <p className="mt-4 text-xs text-zinc-500">unsigned alpha · macOS 12+ · Windows + Linux next</p>

        {/* Animated demo */}
        <Demo />

        <p className="mt-6 text-sm text-zinc-600 dark:text-zinc-400">
          See the wedge in action: <Link href="/showdown" className="text-pink-500 font-semibold underline underline-offset-2 hover:no-underline">/showdown</Link> — five real captured outputs, side by side.
        </p>
      </section>

      {/* How it works */}
      <section id="how" className="mx-auto max-w-6xl px-6 py-20 border-t border-zinc-200 dark:border-zinc-800">
        <h2 className="text-3xl sm:text-4xl font-black tracking-tight mb-12">How it works</h2>
        <div className="grid sm:grid-cols-3 gap-6">
          <Step n="01" title="Hold the button">
            Hold <kbd className="rounded border border-zinc-300 dark:border-zinc-700 px-1.5 py-0.5 text-xs font-mono">Right Option</kbd>. Tray icon goes red. A tiny pill drops in the corner.
          </Step>
          <Step n="02" title="Talk normally">
            Ramble. Change your mind. Stutter. FunButton handles it. Up to ~60 seconds per take.
          </Step>
          <Step n="03" title="Release">
            Whisper Turbo transcribes. Llama 3.3 cleans (or your local Ollama if it&apos;s running). Cmd+V pastes. Total: ~1.5s.
          </Step>
        </div>

        {/* Code mode demo */}
        <div className="mt-16 grid md:grid-cols-2 gap-4">
          <ScenarioCard label="You say" tone="raw">
            &ldquo;open paren camelCase user name comma age close paren arrow user name plus age dot to string open paren close paren&rdquo;
          </ScenarioCard>
          <ScenarioCard label="In Cursor" tone="cleaned">
            <code>(userName, age) =&gt; userName + age.toString()</code>
          </ScenarioCard>
          <ScenarioCard label="You say" tone="raw">
            &ldquo;um so like new function snake_case fetch user data takes id returns promise of user&rdquo;
          </ScenarioCard>
          <ScenarioCard label="Code mode in VS Code" tone="cleaned">
            <code>function fetch_user_data(id): Promise&lt;User&gt;</code>
          </ScenarioCard>
        </div>
        <p className="text-xs text-zinc-500 mt-3">Code mode auto-activates in Cursor, VS Code, JetBrains, Vim, Terminal, Xcode.</p>
      </section>

      {/* vs Wispr */}
      <section id="vs" className="mx-auto max-w-6xl px-6 py-20 border-t border-zinc-200 dark:border-zinc-800">
        <h2 className="text-3xl sm:text-4xl font-black tracking-tight mb-2">vs Wispr Flow</h2>
        <p className="text-zinc-600 dark:text-zinc-400 mb-10 text-sm">Wispr is great. We just don&apos;t want to send our voice through OpenAI to type a Slack message.</p>
        <div className="overflow-x-auto rounded-lg border border-zinc-200 dark:border-zinc-800">
          <table className="w-full text-sm">
            <thead className="bg-zinc-100 dark:bg-zinc-900 text-left">
              <tr>
                <th className="px-4 py-3 font-semibold"></th>
                <th className="px-4 py-3 font-semibold">Wispr Flow</th>
                <th className="px-4 py-3 font-semibold text-pink-500">FunButton</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-zinc-200 dark:divide-zinc-800">
              <Row label="Bundle / RAM" wispr="800 MB Electron, 8% CPU idle" us="~10 MB Tauri 2, ~50 MB" />
              <Row label="Local mode" wispr="No — cloud always" us="Yes — Ollama default" />
              <Row label="API key required" wispr="Yes" us="No (in local mode)" />
              <Row label="Code mode" wispr="Generic auto-cleanup" us="Spoken symbols + casing taxonomy" />
              <Row label="Linux" wispr="None" us="First-class (post-MVP)" />
              <Row label="Source" wispr="Closed" us="GPLv3 desktop core" />
              <Row label="Pricing" wispr="$144/yr, no lifetime" us="$99 lifetime (founders, planned)" />
              <Row label="Screen capture" wispr="Yes (privacy controversy)" us="Never" />
            </tbody>
          </table>
        </div>
      </section>

      {/* Pricing */}
      <section id="price" className="mx-auto max-w-6xl px-6 py-20 border-t border-zinc-200 dark:border-zinc-800">
        <h2 className="text-3xl sm:text-4xl font-black tracking-tight mb-2">Pricing</h2>
        <p className="text-zinc-600 dark:text-zinc-400 mb-10 text-sm">During alpha: everything is free. Post-1.0:</p>
        <div className="grid sm:grid-cols-3 gap-4">
          <Card title="Free / BYOK" price="$0">
            <li>Unlimited local mode (Ollama)</li>
            <li>Bring your own Groq key</li>
            <li>All code-mode profiles</li>
            <li>GPLv3 source — build it yourself</li>
          </Card>
          <Card title="Pro" price="$7" suffix="/mo annual" highlight>
            <li>Cloud cleanup quota included</li>
            <li>Sync across devices</li>
            <li>Priority Groq queue</li>
            <li>Premium voices (Claude Haiku tier)</li>
          </Card>
          <Card title="Lifetime" price="$99" suffix="one-time" badge="founders, first 1k">
            <li>Pro forever — no subscription</li>
            <li>All future features</li>
            <li>Direct line to the dev</li>
            <li>Locked in before $149</li>
          </Card>
        </div>
        <p className="text-xs text-zinc-500 mt-4">Wispr won&apos;t ever offer lifetime. They literally can&apos;t — the unit economics don&apos;t work when every dictation hits cloud. Local-first changes the math.</p>
      </section>

      {/* Footer */}
      <footer className="border-t border-zinc-200 dark:border-zinc-800 py-12">
        <div className="mx-auto max-w-6xl px-6 flex flex-col sm:flex-row items-center justify-between gap-4 text-sm text-zinc-500">
          <div className="flex items-center gap-2">
            <span className="text-pink-500">●</span>
            <span className="font-semibold text-zinc-700 dark:text-zinc-300">FunButton.ai</span>
            <span>· talk fast · stay local · pay less</span>
          </div>
          <div className="flex items-center gap-5">
            <a href={REPO} target="_blank" rel="noreferrer" className="hover:text-pink-500">github</a>
            <a href={`${REPO}/blob/main/PRD.md`} target="_blank" rel="noreferrer" className="hover:text-pink-500">prd</a>
            <a href={`${REPO}/blob/main/RESEARCH.md`} target="_blank" rel="noreferrer" className="hover:text-pink-500">research</a>
            <a href="https://groq.com" target="_blank" rel="noreferrer" className="hover:text-pink-500">powered by groq</a>
          </div>
        </div>
      </footer>
    </main>
  );
}

function Demo() {
  return (
    <div className="mt-14 mx-auto max-w-3xl">
      <div className="relative rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 shadow-2xl overflow-hidden">
        {/* Mock window chrome */}
        <div className="flex items-center gap-2 px-4 py-3 bg-zinc-100 dark:bg-zinc-800 border-b border-zinc-200 dark:border-zinc-700">
          <div className="flex gap-1.5">
            <span className="w-3 h-3 rounded-full bg-red-400" />
            <span className="w-3 h-3 rounded-full bg-yellow-400" />
            <span className="w-3 h-3 rounded-full bg-green-400" />
          </div>
          <span className="text-xs text-zinc-500 ml-2 font-mono">Cursor — main.tsx</span>
        </div>

        {/* Mock editor surface */}
        <div className="p-6 font-mono text-sm leading-relaxed text-left min-h-[200px] bg-white dark:bg-zinc-900">
          <div className="text-zinc-400 dark:text-zinc-500">// Press <span className="text-pink-500 font-bold">Right Option</span>, then say:</div>
          <div className="text-zinc-400 dark:text-zinc-500 italic mb-4">// &quot;open paren camelCase user data comma options close paren fat arrow…&quot;</div>
          <div className="fb-typing">
            <span className="text-pink-500">const</span>{" "}
            <span className="text-blue-600 dark:text-blue-400">filter</span>{" "}
            <span className="text-zinc-500">=</span>{" "}
            <span className="text-zinc-700 dark:text-zinc-300">(userData, options)</span>{" "}
            <span className="text-zinc-500">=&gt;</span>{" "}
            <span className="text-zinc-700 dark:text-zinc-300">Object.keys(userData).filter(k =&gt; !k.startsWith(</span>
            <span className="text-green-600 dark:text-green-400">&apos;_&apos;</span>
            <span className="text-zinc-700 dark:text-zinc-300">))</span>
          </div>
          <span className="fb-cursor">▍</span>
        </div>

        {/* Floating pill */}
        <div className="fb-pill-demo absolute top-3 right-3 flex items-center gap-2 px-3 py-2 rounded-full text-xs font-bold shadow-lg">
          <span className="fb-pill-dot" />
          <span className="fb-pill-text">recording</span>
        </div>
      </div>
    </div>
  );
}

function Step({ n, title, children }: { n: string; title: string; children: React.ReactNode }) {
  return (
    <div className="border border-zinc-200 dark:border-zinc-800 rounded-lg p-6 bg-white dark:bg-zinc-900">
      <div className="text-pink-500 font-mono text-xs mb-2">{n}</div>
      <h3 className="font-bold mb-2">{title}</h3>
      <p className="text-sm text-zinc-600 dark:text-zinc-400 leading-relaxed">{children}</p>
    </div>
  );
}

function ScenarioCard({ label, tone, children }: { label: string; tone: "raw" | "cleaned"; children: React.ReactNode }) {
  return (
    <div className={`rounded-lg p-5 border ${tone === "raw" ? "border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 text-zinc-700 dark:text-zinc-300" : "border-pink-300 dark:border-pink-900 bg-pink-50 dark:bg-pink-950/30 text-zinc-900 dark:text-zinc-50"}`}>
      <div className="text-[10px] uppercase tracking-widest mb-2 opacity-60">{label}</div>
      <div className="text-sm leading-relaxed font-mono">{children}</div>
    </div>
  );
}

function Row({ label, wispr, us }: { label: string; wispr: string; us: string }) {
  return (
    <tr>
      <td className="px-4 py-3 font-semibold">{label}</td>
      <td className="px-4 py-3 text-zinc-600 dark:text-zinc-400">{wispr}</td>
      <td className="px-4 py-3 font-medium text-pink-700 dark:text-pink-400">{us}</td>
    </tr>
  );
}

function Card({ title, price, suffix, highlight, badge, children }: { title: string; price: string; suffix?: string; highlight?: boolean; badge?: string; children: React.ReactNode }) {
  return (
    <div className={`rounded-lg p-6 border ${highlight ? "border-pink-500 bg-pink-50 dark:bg-pink-950/20" : "border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900"}`}>
      <div className="flex items-center justify-between mb-2">
        <h3 className="font-bold text-lg">{title}</h3>
        {badge && <span className="text-[10px] uppercase tracking-widest text-pink-600 dark:text-pink-400 border border-pink-300 dark:border-pink-800 px-2 py-0.5 rounded">{badge}</span>}
      </div>
      <div className="mb-5">
        <span className="text-4xl font-black">{price}</span>
        {suffix && <span className="text-zinc-500 text-sm ml-1">{suffix}</span>}
      </div>
      <ul className="space-y-2 text-sm text-zinc-700 dark:text-zinc-300">
        {children}
      </ul>
    </div>
  );
}
