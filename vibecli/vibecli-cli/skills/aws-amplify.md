---
triggers: ["Amplify", "aws amplify", "amplify gen2", "amplify data", "amplify auth", "amplify hosting", "amplify function", "amplify sandbox"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["npx"]
category: cloud-aws
---

# AWS Amplify (Gen 2)

When working with AWS Amplify Gen 2:

1. Initialize a Gen 2 project with `npm create amplify@latest`; the `amplify/` directory contains TypeScript backend definitions (`auth.ts`, `data.ts`, `storage.ts`) that compile to AWS CDK constructs.
2. Define data models in `amplify/data/resource.ts` using the schema builder: `a.model({ Post: a.model({ title: a.string().required(), content: a.string() }).authorization(allow => [allow.owner()]) })`.
3. Configure auth in `amplify/auth/resource.ts` with `defineAuth({ loginWith: { email: true } })`; customize sign-up attributes, MFA settings, and external identity providers (Google, Apple, SAML) declaratively.
4. Set up file storage in `amplify/storage/resource.ts` with `defineStorage({ name: 'media', access: allow => ({ 'photos/*': [allow.authenticated.to(['read', 'write'])] }) })` for S3-backed storage with path-level auth.
5. Create serverless functions in `amplify/functions/` with `defineFunction({ entry: './handler.ts' })`; use them as data resolvers, auth triggers, or standalone API endpoints with typed event parameters.
6. Run `npx ampx sandbox` to deploy a personal cloud sandbox environment; each developer gets an isolated backend stack for testing without affecting teammates or shared environments.
7. Use the generated `amplify_outputs.json` client config file; import it in your frontend with `Amplify.configure(outputs)` and use `generateClient<Schema>()` for fully typed data operations.
8. Query and mutate data with the typed client: `const { data } = await client.models.Post.list()`, `await client.models.Post.create({ title: 'Hello' })`; subscriptions work via `client.models.Post.observeQuery()`.
9. Integrate Amplify UI components (`@aws-amplify/ui-react`) for pre-built auth flows (`<Authenticator>`), storage file uploaders (`<StorageManager>`), and form builders that connect directly to data models.
10. Deploy to Amplify Hosting by connecting a Git repo in the Amplify console; configure `amplify.yml` build settings, set environment variables per branch, and enable preview deployments for pull requests.
11. Add custom CDK resources in `amplify/backend.ts` by accessing the underlying CDK stacks: `backend.data.resources.cfnResources` for fine-grained CloudFormation overrides and custom AWS service integrations.
12. Manage environments with Git branch-based deployments; `main` maps to production, feature branches create ephemeral stacks; use `npx ampx generate outputs --branch main` to pull production config locally.
