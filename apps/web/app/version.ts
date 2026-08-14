// Single source of truth for the FunButton version label shown across the site
// (nav, hero release-notes link, OG social card).
//
// scripts/update-landing-version.sh rewrites this one line after each release,
// so the visible version can never drift between surfaces. The download CTA is
// deliberately NOT derived from this — it uses the version-agnostic /download
// route so the primary CTA keeps working even if this label lags a release.
export const APP_VERSION = "0.1.6";
