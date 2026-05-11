import type { Env } from '../types';

export async function sendActivationEmail(
  env: Env,
  args: { to: string; jwt: string; tier: string }
): Promise<void> {
  if (!env.RESEND_API_KEY) {
    console.warn('RESEND_API_KEY missing, skipping activation email send');
    return;
  }
  const activationLink = `${env.ACTIVATION_URL_SCHEME}?jwt=${encodeURIComponent(args.jwt)}`;
  const webFallback = `${env.APP_URL}/activate?jwt=${encodeURIComponent(args.jwt)}`;
  const subject = 'Your FunButton license is ready';
  const html = `<!doctype html>
<html><body style="font-family: -apple-system, system-ui, sans-serif; max-width: 560px; margin: 0 auto; padding: 24px;">
  <h1 style="font-size: 24px; margin: 0 0 12px;">Welcome to FunButton</h1>
  <p>Your <strong>${args.tier.replace('_', ' ')}</strong> license is ready.</p>
  <p><a href="${activationLink}" style="display:inline-block;background:#111;color:#fff;text-decoration:none;padding:12px 18px;border-radius:8px;font-weight:600;">Activate FunButton</a></p>
  <p style="font-size:13px;color:#555;">If the button doesn't work, open the app and paste this link: <code>${activationLink}</code></p>
  <p style="font-size:13px;color:#555;">Or use the web fallback: <a href="${webFallback}">${webFallback}</a></p>
  <hr style="border:none;border-top:1px solid #eee;margin:24px 0;">
  <p style="font-size:12px;color:#888;">You're receiving this because someone purchased a FunButton license with this email. Reply if that wasn't you.</p>
</body></html>`;

  const res = await fetch('https://api.resend.com/emails', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${env.RESEND_API_KEY}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      from: env.ACTIVATION_EMAIL_FROM,
      to: args.to,
      subject,
      html,
    }),
  });
  if (!res.ok) {
    const body = await res.text();
    console.error('Resend send failed', res.status, body.slice(0, 200));
  }
}
