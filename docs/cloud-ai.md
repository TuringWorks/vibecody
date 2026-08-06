---
layout: page
title: Cloud AI Backends — Serving, Training, Eval & Routing
permalink: /cloud-ai/
---

VibeCody talks to the major cloud AI platforms through one configuration-driven
layer. Adding a cloud, repointing an endpoint, or changing which backend serves
a request is a config edit — not a code change, and not a rebuild.

Every path below performs real work against the vendor's API. There are no
simulated responses, no estimated scores, and no mock backends anywhere in this
surface. When an eval case cannot be executed, it is reported as a failure with
the reason — never counted as a pass.

---

## Supported backends

| Backend | Cloud | Stages | Auth | Endpoint verified against |
|---|---|---|---|---|
| `digitalocean` | DigitalOcean Gradient AI | serve | Bearer token | `docs.digitalocean.com/products/inference` |
| `azure` | Azure OpenAI / AI Foundry | serve, train | `api-key` header | `learn.microsoft.com` azure-openai rest |
| `google` | Google Vertex AI | serve, train | OAuth access token | `cloud.google.com/vertex-ai` (openai + tuningJobs) |
| `aws` | AWS Bedrock | serve, train | Signature V4 | `docs.aws.amazon.com/bedrock` rest |
| `oracle` | OCI Generative AI | serve | OCI session token | `docs.oracle.com` generative-ai-inference |
| `ibm` | IBM watsonx.ai | serve, train | IBM Cloud API key → IAM token | `cloud.ibm.com/apidocs/watsonx-ai` |
| `akamai` | Akamai Cloud Inference (Linode GPU) | serve | Bearer token | per-deployment — you supply `base_url` |
| `custom` | Any OpenAI-compatible endpoint | serve | Bearer token (optional) | n/a — you supply `base_url` |

`custom` is the escape hatch: vLLM, SGLang, llama.cpp, TGI, Ollama, a corporate
gateway, or a cloud not listed above. Eval and routing run on top of **any**
serve-capable backend, so they work with `custom` too.

Run `vibecli --cloud-ai list` for the live table — it reflects your catalog,
which may differ from the shipped default.

---

## The five-minute version

```bash
# 1. See what exists
vibecli --cloud-ai list

# 2. Configure one backend (values differ per cloud — see the table below)
vibecli --cloud-ai set azure resource    my-resource
vibecli --cloud-ai set azure deployment  gpt-4o
vibecli --cloud-ai set azure api_version 2026-02-01

# 3. Store the credential in the encrypted profile store
vibecli set-key azure_openai <your-key>

# 4. Confirm it is actually ready
vibecli --cloud-ai status

# 5. Use it
vibecli --cloud-ai chat azure gpt-4o "Say hello"
```

`status` is the diagnostic to reach for first. It lists every backend with
either `[ready]` and the concrete URL it will call, or `[config]` and the exact
command that fixes it:

```
Readiness for the serve stage:

  [ready]   custom         http://localhost:11434/v1/chat/completions
  [config]  azure          backend 'azure' needs 'deployment' — set it with
                           `vibecli --cloud-ai set azure deployment <value>`
  [config]  aws            no credential for 'aws' — store one with
                           `vibecli set-key aws <value>`

1 backend(s) ready.
```

---

## Credentials

Credentials go in the encrypted `ProfileStore` (`~/.vibecli/profile_settings.db`),
never in a config file and never in an environment variable. Store them with
`vibecli set-key`, remove them with `vibecli unset-key`.

| Backend | `set-key` name | Value shape |
|---|---|---|
| `digitalocean` | `digitalocean` | Model access key or personal access token |
| `azure` | `azure_openai` | Azure OpenAI API key (shared with serving and fine-tuning) |
| `google` | `google_vertex` | OAuth access token; falls back to `gcloud auth print-access-token` |
| `aws` | `aws` | `ACCESS_KEY_ID:SECRET_ACCESS_KEY` (optionally `:SESSION_TOKEN`) |
| `oracle` | `oracle_oci` | OCI session token from `oci session authenticate` |
| `ibm` | `ibm_watsonx` | IBM Cloud API key — exchanged for an IAM bearer token at request time |
| `akamai` | `akamai` | Bearer token for your inference host |
| `custom` | `custom_cloud` | Whatever your endpoint expects; leave empty for unauthenticated local servers |

The AWS credential is used to compute a real Signature V4 over each request. The
IBM key is exchanged for an IAM token over HTTP and cached until it expires.
Neither is a placeholder.

---

## Configuration variables

Each backend declares the variables its URLs need. Set them with
`vibecli --cloud-ai set <backend> <var> <value>`; clear one by passing an empty
string, which returns the backend to "needs configuration" rather than leaving a
half-built URL behind.

| Backend | Required variables |
|---|---|
| `azure` | `resource` (without `.openai.azure.com`), `deployment`, `api_version` |
| `google` | `project`, `region` |
| `aws` | `region` |
| `oracle` | `region`, `compartment_id` |
| `ibm` | `region`, `project_id`, `api_version` |
| `akamai`, `custom` | `base_url` |

One optional variable applies to every backend: `price_per_mtok`, used by the
`cheapest` routing policy.

---

## Routing

Routing picks a backend for a stage from what is actually ready, and explains
both the choice and every skip.

```bash
vibecli --cloud-ai route serve                        # first ready (default)
vibecli --cloud-ai route serve cheapest               # lowest price_per_mtok
vibecli --cloud-ai route serve ordered:aws,azure,ibm  # first ready in your order
vibecli --cloud-ai route train                        # same, for the train stage
```

```
-> custom (cheapest configured at $0.25/Mtok)

Skipped:
  aws            no credential for 'aws' — store one with `vibecli set-key aws <value>`
  azure          backend 'azure' needs 'resource' — set it with `vibecli --cloud-ai set azure resource <value>`
```

If no ready backend has a `price_per_mtok`, `cheapest` falls back to first-ready
and says so — it does not silently pretend a price comparison happened.

---

## Training

Four backends expose a training stage: `azure`, `google`, `aws`, and `ibm`.

```bash
vibecli --cloud-ai train <backend> <base-model> <training-data> [suffix]
vibecli --cloud-ai job   <backend> <job-id>
```

`train` submits a real fine-tuning job to the vendor and returns the vendor's
job id; `job` polls that vendor for its real status. VibeCody is the control
plane here — the training itself runs on the cloud you selected, under your
account and your quota.

---

## Eval

An eval suite is a TOML file. Each case sends a real prompt to a real model and
scores the real output.

```toml
name  = "smoke"
model = "llama3.2:latest"

[[case]]
name   = "capital"
prompt = "What is the capital of France? Answer with the single word only."
expect = { contains = "Paris" }

[[case]]
name   = "no-unwraps"
prompt = "Write a Rust function that parses a port number."
expect = { excludes = ["unwrap(", "expect("] }

[[case]]
name   = "graded-by-execution"
prompt = "Output only a shell line that prints hello."
expect = { file = "gen.sh", command = "sh gen.sh | grep -q hello" }
```

```bash
vibecli --cloud-ai eval <backend> <suite.toml> [workdir]
```

```
  [pass] capital                      substring found (379ms)
  [FAIL] no-unwraps                   contained forbidden "unwrap(" (1.2s)
  [FAIL] graded-by-execution          command exited 1: gen.sh: line 1: hello: command not found

smoke: 1/3 passed (33%) - 2 failed, 0 errored, on custom
```

### Expectations

An `expect` table sets exactly one check:

| Key | Passes when |
|---|---|
| `exact` | Output equals the value after trimming |
| `contains` | Output contains the value |
| `excludes` | Output contains none of the listed values |
| `json_pointer` | Output parses as JSON and has a non-null value at the pointer |
| `command` | Output is written to `file`, then `command` runs and exits 0 |

`command` is how a code-generation case is graded: by executing it. It takes an
optional `file` (default `output.txt`) naming where the model's output is
written inside the working directory. **A case that cannot be executed — no
working directory given — fails with "command cases need a working directory".
It is never counted as a pass.**

---

## Overriding the catalog

The backend list ships embedded, and is overridden wholesale by
`~/.vibecli/cloud_ai.toml` when that file exists. Copy
`vibecli/vibecli-cli/src/cloud_ai_catalog.toml` as your starting point to add a
cloud, repoint a `base_url`, or pin a different API version — all without
touching Rust.

Each `[[backend]]` declares its id, auth kind, credential name, stages, and one
endpoint table per stage. Every `base_url` is a template: `{braces}` are
resolved from your stored configuration, so nothing is baked in.

---

## Relationship to chat providers

VibeCody has a second, older path to some of these clouds: the chat providers
listed under [Providers]({{ site.baseurl }}/providers/), including
[AWS Bedrock]({{ site.baseurl }}/providers/bedrock/) and
[Azure OpenAI]({{ site.baseurl }}/providers/azure-openai/). They overlap on
serving only. Use whichever fits:

| | Chat providers | Cloud AI backends |
|---|---|---|
| Covers | Serving | Serving, training, eval, routing |
| Used by | The toolbar model selector, panels, the agent loop | The `--cloud-ai` CLI |
| Adding one | A Rust implementation per provider | A `[[backend]]` entry in a TOML catalog |
| Training | Not supported | Real jobs on Azure, Google, AWS, IBM |

If you only want to chat with a model, pick it in the toolbar — that goes
through the chat providers. Reach for `--cloud-ai` when you need the training,
eval, or routing stages, or when you want to add a cloud without writing Rust.

---

## Related

- [Configuration]({{ site.baseurl }}/configuration/) — the wider settings surface
- [Model Comparison]({{ site.baseurl }}/model-comparison/) — choosing a model
- [Security]({{ site.baseurl }}/security/) — how the encrypted stores work
