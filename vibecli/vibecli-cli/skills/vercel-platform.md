---
triggers: ["Vercel", "vercel deploy", "vercel edge", "vercel serverless", "vercel kv", "vercel postgres", "vercel blob", "vercel preview"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["vercel"]
category: cloud-vercel
---

# Vercel Platform

When working with Vercel:

1. Link a project and deploy with `vercel` for preview or `vercel --prod` for production; configure project settings in `vercel.json` with `{ "buildCommand": "npm run build", "outputDirectory": ".next", "framework": "nextjs" }` and use `vercel pull` to sync environment variables locally.
2. Deploy Next.js applications leveraging automatic framework detection; configure `next.config.js` with `output: 'standalone'` for optimal cold starts, use `generateStaticParams()` for ISR routes, and set revalidation with `export const revalidate = 60` or `revalidatePath('/path')` for on-demand ISR.
3. Create Edge Functions by exporting from `app/api/route.ts` with `export const runtime = 'edge'` for sub-millisecond cold starts at 30+ global regions; use the `next/server` imports: `import { NextRequest, NextResponse } from 'next/server'` and keep bundles under 1MB for edge runtime.
4. Build serverless API routes in `app/api/*/route.ts` with standard Node.js runtime (default); configure function regions with `export const preferredRegion = 'iad1'` and set max duration with `export const maxDuration = 30` (up to 300s on Pro plan).
5. Manage environment variables with `vercel env add SECRET_KEY production` for secrets and `vercel env pull .env.local` to sync to local development; use `NEXT_PUBLIC_` prefix for client-side variables and never expose server-only secrets to the browser bundle.
6. Use Vercel KV (Redis) for caching and rate limiting: `import { kv } from '@vercel/kv'; await kv.set('key', value, { ex: 3600 }); const val = await kv.get('key');` and configure in the Vercel dashboard Storage tab to provision a Upstash-backed Redis instance.
7. Set up Vercel Postgres (Neon-backed) with `import { sql } from '@vercel/postgres'; const { rows } = await sql\`SELECT * FROM users WHERE id = ${id}\`;` using tagged template literals for automatic parameterized queries that prevent SQL injection.
8. Store files with Vercel Blob: `import { put, del, list } from '@vercel/blob'; const blob = await put('avatar.png', file, { access: 'public' }); console.log(blob.url);` for CDN-backed object storage with automatic content-type detection.
9. Configure preview deployments by setting branch-specific environment variables with `vercel env add DB_URL preview` and use `vercel.json` headers/rewrites: `{ "rewrites": [{ "source": "/api/:path*", "destination": "https://backend.example.com/:path*" }] }` for preview-specific routing.
10. Set up custom domains with `vercel domains add example.com` and configure DNS; use `vercel certs issue example.com` for automatic TLS provisioning, and set up redirects in `vercel.json`: `{ "redirects": [{ "source": "/old", "destination": "/new", "permanent": true }] }`.
11. Optimize costs by using Edge Functions for lightweight logic (cheaper than serverless), enabling ISR to reduce function invocations, setting `images: { minimumCacheTTL: 2592000 }` in `next.config.js` for image optimization caching, and monitoring usage with `vercel billing` and the Usage dashboard.
12. Secure deployments by enabling Deployment Protection (Vercel Authentication) for preview URLs, using `vercel.json` headers for security: `{ "headers": [{ "source": "/(.*)", "headers": [{ "key": "X-Frame-Options", "value": "DENY" }] }] }`, setting `VERCEL_AUTOMATION_BYPASS_SECRET` for CI testing, and configuring Trusted IPs on Enterprise plans.
