import { NextResponse } from "next/server";

// Machine-readable install contract, served at https://funbutton.ai/install.json.
//
// This is what a coding agent should fetch instead of scraping the marketing page
// (see AGENT-INSTALL.md at the repo root for the human-facing prompt that points
// agents here). The install.sh route serves a *static* body; this one is dynamic
// on purpose: the latest version and the .dmg's SHA-256 change every release, and
// GitHub already publishes both — the release tag and each asset's `digest`. We
// resolve them from the API at request time (same source as /download) so an agent
// always gets the true current values with no per-release edit and no drift. If the
// API is unreachable we fall back to the last-known-good pin below rather than
// serving a broken contract.
//
// Everything else in the contract (install methods, required TCC permissions, the
// verify block, model download facts) is stable and lives in STATIC_CONTRACT.

export const runtime = "edge";

const REPO = "todddickerson/funbutton";
const RELEASES_PAGE = `https://github.com/${REPO}/releases/latest`;

// Last-known-good release pin — the fallback when the GitHub API can't be reached.
// Kept in sync by the same release flow that runs scripts/update-cask.sh.
const FALLBACK = {
  version: "0.1.8",
  asset_name: "FunButton_0.1.8_aarch64.dmg",
  sha256: "c186f57dec651c9a43dd104a287985bf6a8dae9beed1b9794ae940fc24b1285a",
  size_bytes: 17050262,
  asset_url:
    "https://github.com/todddickerson/funbutton/releases/download/v0.1.8/FunButton_0.1.8_aarch64.dmg",
};

type Release = {
  tag_name?: string;
  assets?: { name: string; browser_download_url: string; size?: number; digest?: string | null }[];
};

// Pick the Apple Silicon .dmg for this release, preferring the versioned name if a
// legacy-named compat asset lives alongside it (mirrors app/download/route.ts).
function pickDmg(rel: Release) {
  const dmgs = (rel.assets ?? []).filter((a) => a.name.toLowerCase().endsWith(".dmg"));
  if (dmgs.length === 0) return undefined;
  const arm = dmgs.filter((a) => /aarch64|arm64/i.test(a.name));
  const pool = arm.length > 0 ? arm : dmgs;
  const version = (rel.tag_name ?? "").replace(/^v/, "");
  const versioned = version ? pool.find((a) => a.name.includes(version)) : undefined;
  return versioned ?? pool[0];
}

// GitHub asset `digest` is formatted "sha256:<hex>". Return the bare hex, or null
// if the field is absent (older releases) so callers fall back cleanly.
function sha256From(digest?: string | null): string | null {
  if (!digest) return null;
  const m = /^sha256:([0-9a-f]{64})$/i.exec(digest.trim());
  return m ? m[1].toLowerCase() : null;
}

async function resolveRelease() {
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: {
        Accept: "application/vnd.github+json",
        "User-Agent": "funbutton-landing",
      },
      next: { revalidate: 3600 },
    });
    if (res.ok) {
      const rel = (await res.json()) as Release;
      const dmg = pickDmg(rel);
      const version = (rel.tag_name ?? "").replace(/^v/, "");
      if (dmg && version) {
        return {
          version,
          asset_name: dmg.name,
          sha256: sha256From(dmg.digest), // may be null if GitHub omits it
          size_bytes: dmg.size ?? null,
          asset_url: dmg.browser_download_url,
        };
      }
    }
  } catch {
    // fall through to the pin
  }
  return null;
}

function buildContract(rel: {
  version: string;
  asset_name: string;
  sha256: string | null;
  size_bytes: number | null;
  asset_url: string;
}) {
  return {
    schema_version: "1",
    name: "FunButton",
    tagline: "Local-first push-to-talk dictation for macOS.",
    homepage: "https://funbutton.ai",
    repo: RELEASES_PAGE.replace("/releases/latest", ""),
    license: "GPL-3.0-or-later",

    latest_version: rel.version,

    platform: {
      os: "macOS",
      min_macos: "12.0",
      arch: "arm64",
      note: "Apple Silicon only. There is no Intel/x86_64 build.",
    },

    signed: false,
    signing_note:
      "The alpha is unsigned (no Apple Developer ID yet), so left untouched macOS mislabels it as \"damaged.\" The brew cask and the curl installer strip the com.apple.quarantine flag from FunButton.app only. Never disable Gatekeeper globally (no `spctl --master-disable`).",

    download: {
      // Version-agnostic 302 to the current release's arm64 .dmg — prefer this.
      url: "https://funbutton.ai/download",
      asset_name: rel.asset_name,
      asset_url: rel.asset_url,
      sha256: rel.sha256,
      size_bytes: rel.size_bytes,
    },

    install_methods: [
      {
        id: "brew",
        preferred: true,
        title: "Homebrew cask",
        commands: ["brew install --cask todddickerson/funbutton/funbutton"],
        upgrade: "brew upgrade --cask funbutton",
        clears_quarantine: true,
        success_contains: "was successfully installed",
        note: "The cask's postflight strips com.apple.quarantine from FunButton.app only. brew does NOT strip quarantine by default.",
      },
      {
        id: "curl",
        preferred: false,
        title: "curl one-liner",
        commands: ["curl -fsSL https://funbutton.ai/install.sh | bash"],
        clears_quarantine: true,
        success_contains: "FunButton is installed",
        readable_source: "https://funbutton.ai/install.sh",
        note: "No sudo, no global Gatekeeper change. Readable at the URL before you run it.",
      },
      {
        id: "manual",
        preferred: false,
        title: "Manual .dmg (last resort)",
        commands: [
          "curl -fL -o /tmp/FunButton.dmg https://funbutton.ai/download",
          "mkdir -p /tmp/fb-mnt",
          "hdiutil attach /tmp/FunButton.dmg -nobrowse -noautoopen -mountpoint /tmp/fb-mnt",
          "rm -rf /Applications/FunButton.app",
          "ditto /tmp/fb-mnt/FunButton.app /Applications/FunButton.app",
          "hdiutil detach /tmp/fb-mnt",
          "xattr -dr com.apple.quarantine /Applications/FunButton.app",
        ],
        clears_quarantine: false,
        note: "The only path that does NOT clear quarantine for you — the final xattr line is required. Never run `spctl --master-disable`.",
      },
    ],

    required_permissions: [
      {
        name: "Microphone",
        why: "capture your voice while you hold the button",
        settings_pane: "System Settings > Privacy & Security > Microphone",
      },
      {
        name: "Accessibility",
        why: "paste the cleaned text at your cursor",
        settings_pane: "System Settings > Privacy & Security > Accessibility",
      },
      {
        name: "Input Monitoring",
        why: "detect your push-to-talk key press",
        settings_pane: "System Settings > Privacy & Security > Input Monitoring",
      },
    ],
    permissions_grantable_by_agent: false,
    permissions_note:
      "These are macOS TCC permissions. macOS reserves them for a human at an unlocked screen — no agent, script, tccutil, or MDM profile can grant them silently. Install and verify, then hand the three toggles off to the user.",

    hotkey: {
      user_selectable: true,
      options: ["Fn", "Right Control", "Left Control", "Right Command", "Left Command", "Right Option", "Caps Lock"],
      note: "The app detects the keyboard. On extended Apple keyboards the bottom-left key is Control, not Fn.",
    },

    models: {
      bundled: false,
      when: "first run",
      download_size_human: "~1.1 GB",
      download_size_bytes: 1202206944,
      location: "~/Library/Application Support/ai.funbutton.desktop/models/",
      resumable: true,
      verified: "sha-256",
      files: ["qwen2.5-1.5b-instruct-q4_k_m.gguf", "whisper-base.en-Q8_0.gguf"],
      note: "Not bundled in the app. Surface this before it happens so a first-run network pull isn't a surprise. After it downloads, transcription and cleanup run fully offline.",
    },

    verify: [
      {
        id: "app_present",
        command: "test -d /Applications/FunButton.app && echo OK",
        expect: "OK",
      },
      {
        id: "no_quarantine",
        command: "xattr /Applications/FunButton.app",
        expect_absent: "com.apple.quarantine",
        note: "Output must NOT contain com.apple.quarantine.",
      },
      {
        id: "version",
        command:
          "defaults read /Applications/FunButton.app/Contents/Info CFBundleShortVersionString",
        expect: rel.version,
        note: "Should equal latest_version.",
      },
      {
        id: "launch",
        command:
          "/Applications/FunButton.app/Contents/MacOS/funbutton >/tmp/fb-launch.log 2>&1 & FB=$!; sleep 25; grep -iE 'embedded STT model loaded|llama-server ready|cleanup engine reloaded' /tmp/fb-launch.log; kill \"$FB\" 2>/dev/null",
        expect_log_any: ["embedded STT model loaded", "llama-server ready", "cleanup engine reloaded"],
        note: "Runs the binary directly; env_logger prints to stderr at info level. On first run, models (~1.1 GB) download before these lines appear.",
      },
    ],

    docs: {
      agent_prompt: `${RELEASES_PAGE.replace("/releases/latest", "")}/blob/main/AGENT-INSTALL.md`,
      llms_txt: "https://funbutton.ai/llms.txt",
      install_sh: "https://funbutton.ai/install.sh",
      signing: `${RELEASES_PAGE.replace("/releases/latest", "")}/blob/main/SIGNING.md`,
    },
  };
}

export async function GET() {
  const resolved = await resolveRelease();
  // If the API gave us a version but omitted the digest, keep the resolved version
  // but backfill the sha256 from the pin when the asset name matches.
  let rel = resolved ?? FALLBACK;
  if (resolved && !resolved.sha256 && resolved.asset_name === FALLBACK.asset_name) {
    rel = { ...resolved, sha256: FALLBACK.sha256 };
  }

  const body = JSON.stringify(buildContract(rel), null, 2) + "\n";
  return new NextResponse(body, {
    headers: {
      "content-type": "application/json; charset=utf-8",
      // Same cache posture as /download: one upstream call per hour, not per fetch.
      "cache-control": "public, max-age=300, s-maxage=3600",
    },
  });
}
