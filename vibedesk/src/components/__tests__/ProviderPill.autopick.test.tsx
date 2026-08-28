/**
 * The default model has to be one the machine can actually run.
 *
 * The auto-pick chose the first row whose name did not contain "cloud". On a
 * 24 GB machine that was a 19.8 GB model, and Ollama answers that with `model
 * requires 19.7 GiB but only 17.3 GiB are available` — HTTP 500, which the
 * retry classifier does not treat as transient. Every run and every spoken
 * turn then failed on a model nobody had chosen.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
import type { DaemonModel } from "../../hooks/useModels";

let catalog: DaemonModel[] = [];
vi.mock("../../hooks/useModels", async (orig) => ({
  ...(await orig<typeof import("../../hooks/useModels")>()),
  useModels: () => catalog,
}));

import { ProviderPill } from "../ProviderPill";

const pill = (onSelect: (p: string, m: string | undefined) => void) =>
  render(
    <ProviderPill
      daemonUrl="http://127.0.0.1:7878"
      daemonOnline
      provider="ollama"
      model={undefined}
      onSelect={onSelect}
    />,
  );

beforeEach(() => {
  catalog = [];
});

describe("ProviderPill — the model it picks for you", () => {
  it("skips a model the daemon says will not fit", () => {
    catalog = [
      { id: "ollama/muse:30b", name: "muse:30b", provider: "ollama", may_not_load: true },
      { id: "ollama/llama3.2", name: "llama3.2", provider: "ollama" },
    ];
    const onSelect = vi.fn();
    pill(onSelect);
    expect(onSelect).toHaveBeenCalledWith("ollama", "llama3.2");
  });

  it("still prefers a local model over a cloud one", () => {
    catalog = [
      { id: "ollama/gpt-oss:120b-cloud", name: "gpt-oss:120b-cloud", provider: "ollama" },
      { id: "ollama/llama3.2", name: "llama3.2", provider: "ollama" },
    ];
    const onSelect = vi.fn();
    pill(onSelect);
    expect(onSelect).toHaveBeenCalledWith("ollama", "llama3.2");
  });

  it("picks something rather than nothing when every model is doubtful", () => {
    // The budget is one number about one machine and the limit is raisable.
    // Refusing to pick would leave the composer with no model at all.
    catalog = [
      { id: "ollama/muse:30b", name: "muse:30b", provider: "ollama", may_not_load: true },
      { id: "ollama/nemo:30b", name: "nemo:30b", provider: "ollama", may_not_load: true },
    ];
    const onSelect = vi.fn();
    pill(onSelect);
    expect(onSelect).toHaveBeenCalledWith("ollama", "muse:30b");
  });

  it("never overrides a model the user already chose", () => {
    catalog = [{ id: "ollama/llama3.2", name: "llama3.2", provider: "ollama" }];
    const onSelect = vi.fn();
    render(
      <ProviderPill
        daemonUrl="http://127.0.0.1:7878"
        daemonOnline
        provider="ollama"
        model="muse:30b"
        onSelect={onSelect}
      />,
    );
    expect(onSelect).not.toHaveBeenCalled();
  });
});
