import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { JwtPanel } from "../JwtPanel";

function encodeJson(value: unknown): string {
  const bytes = new TextEncoder().encode(JSON.stringify(value));
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function token(payload: unknown, header: unknown = { alg: "HS256", typ: "JWT" }): string {
  return `${encodeJson(header)}.${encodeJson(payload)}.signature`;
}

afterEach(() => vi.useRealTimers());

describe("JwtPanel — decode", () => {
  it("Given a valid JWT, then it displays its header, payload, and signature", () => {
    render(<JwtPanel />);

    expect(screen.getByText("HEADER", { exact: false })).toHaveTextContent("HS256 · JWT");
    expect(screen.getByText('"Alice Smith"')).toBeInTheDocument();
    expect(screen.getByText("SIGNATURE")).toBeInTheDocument();
    expect(screen.getByText("✓ VALID", { exact: false })).toBeInTheDocument();
  });

  it("Given a UTF-8 JWT payload, then it decodes international text without mojibake", () => {
    render(<JwtPanel />);

    fireEvent.change(screen.getByPlaceholderText("Paste JWT here…"), {
      target: { value: token({ name: "José 東京 🚀" }) },
    });

    expect(screen.getByText('"José 東京 🚀"')).toBeInTheDocument();
    expect(screen.getByText("no expiry")).toBeInTheDocument();
  });

  it("Given expiration equal to the current second, then the token is expired", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2030-01-01T00:00:00Z"));
    render(<JwtPanel />);

    fireEvent.change(screen.getByPlaceholderText("Paste JWT here…"), {
      target: { value: token({ exp: 1_893_456_000 }) },
    });

    expect(screen.getByText("✕ EXPIRED", { exact: false })).toBeInTheDocument();
  });

  it("Given a malformed token, then it reports the format and clears decoded sections", () => {
    render(<JwtPanel />);
    fireEvent.change(screen.getByPlaceholderText("Paste JWT here…"), { target: { value: "only.two" } });

    expect(screen.getByText("Expected 3 dot-separated parts")).toBeInTheDocument();
    expect(screen.queryByText("PAYLOAD")).not.toBeInTheDocument();
    expect(screen.queryByText("SIGNATURE")).not.toBeInTheDocument();
  });
});

describe("JwtPanel — sign workflow", () => {
  it("Given valid JSON and a secret, when signed, then the result can be decoded", async () => {
    render(<JwtPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Sign" }));

    fireEvent.change(screen.getByRole("textbox", { name: "JWT payload JSON" }), { target: { value: '{"sub":"unicode","name":"José 東京"}' } });
    fireEvent.click(screen.getAllByRole("button", { name: "Sign" })[1]);

    await waitFor(() => expect(screen.getByText("GENERATED JWT")).toBeInTheDocument());
    const generated = screen.getByText("GENERATED JWT").parentElement!.nextElementSibling!;
    expect(generated.textContent!.split(".")).toHaveLength(3);

    fireEvent.click(within(screen.getByText("GENERATED JWT").parentElement!).getByRole("button", { name: "→ Decode" }));
    expect(screen.getByText('"José 東京"')).toBeInTheDocument();
  });

  it("Given a non-HS256 header, then it refuses to label an HMAC signature as another algorithm", async () => {
    render(<JwtPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Sign" }));
    fireEvent.change(screen.getByRole("textbox", { name: "JWT header JSON" }), { target: { value: '{"alg":"none","typ":"JWT"}' } });

    fireEvent.click(screen.getAllByRole("button", { name: "Sign" })[1]);

    expect(await screen.findByText("Only HS256 headers can be signed by this panel.")).toBeInTheDocument();
    expect(screen.queryByText("GENERATED JWT")).not.toBeInTheDocument();
  });

  it("Given invalid JSON, then it surfaces the parsing error and does not generate a token", async () => {
    render(<JwtPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Sign" }));
    fireEvent.change(screen.getByRole("textbox", { name: "JWT payload JSON" }), { target: { value: "{" } });

    fireEvent.click(screen.getAllByRole("button", { name: "Sign" })[1]);

    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent(/JSON/));
    expect(screen.queryByText("GENERATED JWT")).not.toBeInTheDocument();
  });

  it("Given the claims reference, then it documents registered and common claims", () => {
    render(<JwtPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Claims Ref" }));

    expect(screen.getByRole("cell", { name: "exp" })).toBeInTheDocument();
    expect(screen.getByText(/Expiration — Unix timestamp/)).toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "scope" })).toBeInTheDocument();
  });
});
