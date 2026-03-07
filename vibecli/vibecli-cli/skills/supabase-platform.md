---
triggers: ["Supabase", "supabase", "supabase auth", "supabase realtime", "supabase edge function", "supabase storage", "row level security", "supabase postgres"]
tools_allowed: ["read_file", "write_file", "bash"]
category: cloud-supabase
---

# Supabase Platform

When working with Supabase:

1. Initialize a local Supabase project with `supabase init` and start the local dev stack with `supabase start` which launches Postgres, Auth, Storage, Edge Functions, and Studio locally; link to a remote project with `supabase link --project-ref $PROJECT_ID` and push migrations with `supabase db push`.
2. Use the JavaScript client for CRUD operations: `import { createClient } from '@supabase/supabase-js'; const supabase = createClient(url, anonKey); const { data, error } = await supabase.from('posts').select('*, author:users(name)').eq('published', true).order('created_at', { ascending: false }).range(0, 9);` leveraging PostgREST's powerful query syntax.
3. Implement authentication with `const { data, error } = await supabase.auth.signUp({ email, password })` and configure OAuth providers (Google, GitHub, Apple) in the dashboard; use `supabase.auth.onAuthStateChange((event, session) => { ... })` to react to login/logout events in real time.
4. Enable Row Level Security on every table with `ALTER TABLE posts ENABLE ROW LEVEL SECURITY;` and create policies: `CREATE POLICY "Users can read own posts" ON posts FOR SELECT USING (auth.uid() = user_id);` and `CREATE POLICY "Users can insert own posts" ON posts FOR INSERT WITH CHECK (auth.uid() = user_id);` to enforce data isolation.
5. Subscribe to real-time changes with `const channel = supabase.channel('posts').on('postgres_changes', { event: 'INSERT', schema: 'public', table: 'posts' }, (payload) => { console.log('New post:', payload.new); }).subscribe();` and enable replication in the dashboard for each table that needs real-time updates.
6. Create Edge Functions with `supabase functions new my-function` and deploy with `supabase functions deploy my-function`; write Deno-based handlers: `Deno.serve(async (req) => { const { name } = await req.json(); return new Response(JSON.stringify({ message: \`Hello \${name}\` }), { headers: { 'Content-Type': 'application/json' } }); });`.
7. Manage file storage with `const { data, error } = await supabase.storage.from('avatars').upload(\`\${userId}/avatar.png\`, file, { cacheControl: '3600', upsert: true });` and create signed URLs for private access: `const { data } = await supabase.storage.from('avatars').createSignedUrl('path/file.png', 3600);`.
8. Use pgvector for AI embeddings: `CREATE EXTENSION vector; CREATE TABLE documents (id bigserial PRIMARY KEY, content text, embedding vector(1536));` then query nearest neighbors with `SELECT * FROM documents ORDER BY embedding <=> $1 LIMIT 5;` and create an IVFFlat index for performance: `CREATE INDEX ON documents USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);`.
9. Write database migrations with `supabase migration new add_posts_table` which creates a timestamped SQL file in `supabase/migrations/`; apply locally with `supabase db reset` and push to production with `supabase db push`; use `supabase db diff` to auto-generate migration files from schema changes.
10. Configure database webhooks in the dashboard or via SQL to trigger Edge Functions on row changes: `CREATE TRIGGER on_new_user AFTER INSERT ON auth.users FOR EACH ROW EXECUTE FUNCTION supabase_functions.http_request('https://project.supabase.co/functions/v1/on-signup', 'POST', '{}', '{}', '1000');` for event-driven architectures.
11. Use Supabase branching for preview environments: `supabase branches create feature-auth` creates an isolated Postgres instance with its own migrations, allowing safe schema experimentation per pull request without affecting production data.
12. Secure your project by never exposing the `service_role` key in client code (use `anon` key only), enforcing RLS on all tables, validating JWTs in Edge Functions with `const { data: { user } } = await supabase.auth.getUser(req.headers.get('Authorization')?.replace('Bearer ', ''))`, setting restrictive CORS origins, and enabling MFA with `supabase.auth.mfa.enroll({ factorType: 'totp' })`.
