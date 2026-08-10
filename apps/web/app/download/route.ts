import { NextResponse } from "next/server";

// Version-agnostic download endpoint.
//
// The landing CTA points here instead of at a hardcoded
// `releases/latest/download/FunButton_X.Y.Z_aarch64.dmg` URL — that form 404s
// the moment a new release renames the asset (finding #12). This resolves the
// current release's macOS arm64 .dmg from the GitHub API at request time and
// 302-redirects to it, so the primary CTA keeps working across every future
// release with no page edit and no rebuild required.
//
// Resilience: the resolved asset is cached (revalidate) so we don't hammer the
// unauthenticated GitHub API, and any failure falls back to the always-valid
// `releases/latest` page rather than serving a dead link.

export const runtime = "edge";

const REPO = "todddickerson/funbutton";
const RELEASES_PAGE = `https://github.com/${REPO}/releases/latest`;

function pickDmg(
  assets: { name: string; browser_download_url: string }[],
): string | undefined {
  const dmgs = assets.filter((a) => a.name.toLowerCase().endsWith(".dmg"));
  if (dmgs.length === 0) return undefined;
  // Prefer the Apple Silicon build if the release carries more than one .dmg.
  const arm =
    dmgs.find((a) => /aarch64|arm64/i.test(a.name)) ?? dmgs[0];
  return arm.browser_download_url;
}

export async function GET() {
  try {
    const res = await fetch(
      `https://api.github.com/repos/${REPO}/releases/latest`,
      {
        headers: {
          Accept: "application/vnd.github+json",
          // GitHub rejects unauthenticated API requests without a User-Agent.
          "User-Agent": "funbutton-landing",
        },
        // Cache the resolved release for an hour so a burst of downloads is one
        // upstream call, not one per click.
        next: { revalidate: 3600 },
      },
    );
    if (res.ok) {
      const data = (await res.json()) as {
        assets?: { name: string; browser_download_url: string }[];
      };
      const url = pickDmg(data.assets ?? []);
      if (url) {
        return NextResponse.redirect(url, 302);
      }
    }
  } catch {
    // fall through to the release page
  }
  // Never serve a dead link: the releases page is always valid.
  return NextResponse.redirect(RELEASES_PAGE, 302);
}
