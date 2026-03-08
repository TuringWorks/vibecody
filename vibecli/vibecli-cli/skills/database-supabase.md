---
triggers: ["Supabase", "supabase database", "supabase postgres", "supabase query", "supabase rpc", "supabase realtime", "supabase edge"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# Supabase Database

When working with Supabase's PostgreSQL database:

1. Supabase provides a full PostgreSQL database with auto-generated REST/GraphQL APIs, real-time subscriptions, and Row Level Security — connect via PostgREST API or direct PostgreSQL connection string.
2. Auto-generated APIs: every table gets REST endpoints automatically; `supabase.from('users').select('*, posts(*)').eq('active', true)` — supports filtering, ordering, pagination, and nested relations.
3. Row Level Security (RLS): `ALTER TABLE posts ENABLE ROW LEVEL SECURITY; CREATE POLICY "Users can see own posts" ON posts FOR SELECT USING (auth.uid() = user_id)` — enforced at the database level for all access paths.
4. Realtime subscriptions: `supabase.channel('changes').on('postgres_changes', { event: 'INSERT', schema: 'public', table: 'messages' }, (payload) => { ... }).subscribe()` — listen for INSERT/UPDATE/DELETE in real-time.
5. Database functions as RPC: `CREATE FUNCTION search_posts(query TEXT) RETURNS SETOF posts AS $$ SELECT * FROM posts WHERE title ILIKE '%' || query || '%' $$ LANGUAGE sql`. Call via `supabase.rpc('search_posts', { query: 'hello' })`.
6. Use `pgvector` for AI embeddings: `CREATE EXTENSION vector; ALTER TABLE docs ADD COLUMN embedding vector(1536)`. Similarity search: `SELECT * FROM docs ORDER BY embedding <=> '[...]'::vector LIMIT 10`.
7. Edge Functions for server-side logic: `supabase functions deploy my-function` — Deno-based functions that can query the database with the service role key for admin operations.
8. Storage + database integration: store file metadata in PostgreSQL tables with Supabase Storage URLs; use triggers to clean up storage when records are deleted.
9. Migrations: use `supabase migration new create_users` to create migration files; `supabase db push` to apply; `supabase db diff` to auto-generate migrations from schema changes.
10. Use database webhooks: `CREATE TRIGGER notify_webhook AFTER INSERT ON orders FOR EACH ROW EXECUTE FUNCTION supabase_functions.http_request(...)` — call external APIs on data changes.
11. Branching (preview): `supabase branches create preview-123` creates isolated database instances for PR previews; includes schema + seed data from migrations.
12. Performance: use `pg_stat_statements` for query analysis; create indexes for filtered columns; use connection pooling (Supavisor) for serverless workloads; enable `pg_plan_filter` to block expensive queries.
