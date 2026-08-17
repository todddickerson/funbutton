import installer from "./installer.json";

// Serves the one-line installer at https://funbutton.ai/install.sh, so
//   curl -fsSL https://funbutton.ai/install.sh | bash
// works. The body is the SINGLE SOURCE OF TRUTH at scripts/install.sh in the repo
// root — installer.json is regenerated from it by scripts/sync-install-sh.sh (run
// that after any edit to install.sh). We serve text/plain, not application/x-sh:
// a human should be able to open this URL in a browser and read exactly what the
// pipe-to-bash will run before trusting it.
export const dynamic = "force-static";

export function GET() {
  return new Response(installer.script, {
    headers: {
      "content-type": "text/plain; charset=utf-8",
      "cache-control": "public, max-age=300, s-maxage=300",
    },
  });
}
