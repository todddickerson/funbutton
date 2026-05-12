import { NextResponse } from "next/server";

export const runtime = "edge";

const WORKER_BASE =
  process.env.FUNBUTTON_API_BASE ??
  "https://funbutton-api.todd-e03.workers.dev";

const ALLOWED_TIERS = new Set([
  "pro_monthly",
  "pro_annual",
  "lifetime",
  "lifetime_149",
  "lifetime_199",
  "lifetime_249",
]);

export async function POST(req: Request) {
  let body: { tier?: string; email?: string };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "bad_request" }, { status: 400 });
  }

  const tier = (body.tier ?? "").toString();
  if (!ALLOWED_TIERS.has(tier)) {
    return NextResponse.json({ error: "invalid_tier" }, { status: 400 });
  }

  try {
    const res = await fetch(`${WORKER_BASE}/v1/checkout/create-session`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ tier, email: body.email }),
    });
    if (res.status === 503) {
      // Stripe not configured yet — surface a clean message.
      return NextResponse.json(
        { error: "checkout_unavailable", reason: "stripe_not_configured" },
        { status: 503 },
      );
    }
    if (!res.ok) {
      const text = await res.text().catch(() => "");
      console.error("worker checkout failed", res.status, text);
      return NextResponse.json(
        { error: "checkout_failed", status: res.status },
        { status: 502 },
      );
    }
    const json = (await res.json()) as { url?: string; id?: string };
    if (!json.url) {
      return NextResponse.json({ error: "no_url" }, { status: 502 });
    }
    return NextResponse.json({ url: json.url });
  } catch (e) {
    console.error("checkout proxy error", e);
    return NextResponse.json({ error: "network_error" }, { status: 502 });
  }
}
