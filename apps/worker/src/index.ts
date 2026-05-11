import { Hono } from 'hono';
import type { Env } from './types';
import license from './routes/license';
import transcribe from './routes/transcribe';
import cleanup from './routes/cleanup';
import usage from './routes/usage';
import checkout from './routes/checkout';
import webhook from './routes/webhook';

export { UsageCounter, LifetimeCounter } from './lib/usage-do';

const app = new Hono<{ Bindings: Env }>();

app.get('/', (c) =>
  c.json({ name: 'funbutton-api', ok: true, ts: Date.now() })
);

app.get('/health', (c) => c.json({ ok: true }));

app.route('/v1/license', license);
app.route('/v1/transcribe', transcribe);
app.route('/v1/cleanup', cleanup);
app.route('/v1/usage', usage);
app.route('/v1/checkout', checkout);
app.route('/v1/portal', checkout); // shares the controller for `/portal`
app.route('/v1/stripe/webhook', webhook);

app.notFound((c) => c.json({ error: 'not_found', path: c.req.path }, 404));
app.onError((err, c) => {
  console.error('unhandled error', err);
  return c.json({ error: 'internal_error' }, 500);
});

export default {
  fetch: app.fetch,
  async scheduled(_event: ScheduledEvent, env: Env, ctx: ExecutionContext) {
    // Monthly reset cron — wipes every license's UsageCounter at 00:00 UTC on the 1st.
    // Iterate KV listing of license_ids and reset each. KV list is paginated.
    ctx.waitUntil(resetAllUsage(env));
  },
};

async function resetAllUsage(env: Env) {
  let cursor: string | undefined = undefined;
  let resetCount = 0;
  do {
    const page: KVNamespaceListResult<unknown> = await env.LICENSE_KV.list({ cursor, limit: 1000 });
    for (const { name } of page.keys) {
      if (name.startsWith('lic_') && !name.includes(':')) {
        const stub = env.USAGE_DO.get(env.USAGE_DO.idFromName(name));
        try {
          await stub.fetch('https://do/reset');
          resetCount++;
        } catch (err) {
          console.error('failed to reset', name, err);
        }
      }
    }
    cursor = page.list_complete ? undefined : page.cursor;
  } while (cursor);
  console.log(`monthly reset: ${resetCount} licenses cleared`);
}
