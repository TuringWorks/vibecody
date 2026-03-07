---
triggers: ["Netlify", "netlify deploy", "netlify functions", "netlify edge", "netlify build plugin", "netlify forms", "netlify identity"]
tools_allowed: ["read_file", "write_file", "bash"]
category: cloud-netlify
---

# Netlify Platform

When working with Netlify:

1. Deploy sites with `netlify deploy --prod --dir=dist` for manual deploys or link a Git repository with `netlify init` for automatic CI/CD on every push; use `netlify deploy` (without `--prod`) to create draft deploy URLs for previewing changes before going live.
2. Create serverless functions in `netlify/functions/` using the modern streaming format: `export default async (req: Request) => { return new Response(JSON.stringify({ ok: true }), { headers: { 'content-type': 'application/json' } }); }` which maps to `/.netlify/functions/functionName` automatically.
3. Deploy Edge Functions in `netlify/edge-functions/` with Deno runtime for geo-aware, low-latency responses: `export default async (request: Request, context: Context) => { const country = context.geo.country?.code; return new Response(\`Hello from \${country}\`); }` and configure routes in `netlify.toml` under `[[edge_functions]]`.
4. Configure build settings in `netlify.toml`: `[build] command = "npm run build" publish = "dist" [build.environment] NODE_VERSION = "20"` and define context-specific overrides with `[context.production]`, `[context.deploy-preview]`, and `[context.branch-deploy]` sections.
5. Set up Netlify Forms by adding `data-netlify="true"` to HTML forms or `netlify` attribute in JSX; handle submissions with serverless functions using `submission-created` event triggers, and configure spam protection with `data-netlify-honeypot="bot-field"`.
6. Enable Netlify Identity for authentication by running `netlify identity:enable` and integrating the GoTrue client: `import netlifyIdentity from 'netlify-identity-widget'; netlifyIdentity.init(); netlifyIdentity.on('login', user => console.log(user));` for passwordless, OAuth, and email/password auth flows.
7. Create build plugins in `plugins/` with `onPreBuild`, `onBuild`, `onPostBuild` hooks: `export const onPreBuild = ({ utils }) => { console.log('Running pre-build checks'); }` and register in `netlify.toml`: `[[plugins]] package = "./plugins/my-plugin"` for custom build pipeline logic.
8. Configure redirects and rewrites in `netlify.toml`: `[[redirects]] from = "/api/*" to = "/.netlify/functions/:splat" status = 200` for API proxying, or `[[redirects]] from = "/*" to = "/index.html" status = 200` for SPA client-side routing fallback.
9. Set up split testing (A/B) by creating branch deploys and configuring traffic distribution in the Netlify dashboard under Split Testing; deploy feature branches that automatically get unique URLs at `branch-name--site-name.netlify.app` for stakeholder review.
10. Manage environment variables with `netlify env:set API_KEY value` and scope them to contexts: `netlify env:set DB_URL "postgres://prod" --context production` and `netlify env:set DB_URL "postgres://staging" --context deploy-preview` to isolate secrets per environment.
11. Use Netlify Blobs for key-value storage from functions: `import { getStore } from '@netlify/blobs'; export default async (req: Request) => { const store = getStore('mystore'); await store.set('key', 'value'); const data = await store.get('key'); return new Response(data); }` for persistent data without external databases.
12. Secure deployments by configuring custom headers in `netlify.toml`: `[[headers]] for = "/*" [headers.values] X-Frame-Options = "DENY" Content-Security-Policy = "default-src 'self'"`, enabling branch deploy passwords with `NETLIFY_DEPLOY_PASSWORD`, using signed function URLs for webhook verification, and rotating deploy keys regularly.
