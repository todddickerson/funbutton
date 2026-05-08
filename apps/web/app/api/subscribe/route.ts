import { NextResponse } from "next/server";

export const runtime = "edge";

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export async function POST(req: Request) {
  let email: string | undefined;
  try {
    const body = await req.json();
    email = (body?.email || "").toString().trim().toLowerCase();
  } catch {
    return NextResponse.json({ error: "bad request" }, { status: 400 });
  }

  if (!email || !EMAIL_RE.test(email)) {
    return NextResponse.json({ error: "invalid email" }, { status: 400 });
  }

  const apiKey = process.env.RESEND_API_KEY;
  const audienceId = process.env.RESEND_AUDIENCE_ID;

  if (!apiKey || !audienceId) {
    return NextResponse.json(
      { error: "server not configured" },
      { status: 500 },
    );
  }

  try {
    const res = await fetch(
      `https://api.resend.com/audiences/${audienceId}/contacts`,
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ email, unsubscribed: false }),
      },
    );

    // 422 = already exists. Treat as success for UX (no email enumeration).
    if (res.ok || res.status === 422) {
      return NextResponse.json({ ok: true });
    }

    const text = await res.text();
    console.error("resend error", res.status, text);
    return NextResponse.json(
      { error: "couldn't subscribe — try again later" },
      { status: 502 },
    );
  } catch (e) {
    console.error("resend fetch failed", e);
    return NextResponse.json({ error: "network error" }, { status: 502 });
  }
}
