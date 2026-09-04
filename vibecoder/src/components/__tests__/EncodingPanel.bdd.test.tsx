import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { EncodingPanel } from "../EncodingPanel";

const input = () => screen.getByRole("textbox", { name: "Input text" });

function output(label: string): HTMLElement {
  const heading = screen.getByText(label);
  return heading.parentElement!.nextElementSibling as HTMLElement;
}

describe("EncodingPanel — encoding workflows", () => {
  it("Given Unicode text, then Base64 encoding preserves its UTF-8 bytes", () => {
    render(<EncodingPanel />);
    fireEvent.change(input(), { target: { value: "José 東京 🚀" } });

    expect(output("ENCODED")).toHaveTextContent("Sm9zw6kg5p2x5LqsIPCfmoA=");
  });

  it("Given URL-safe Base64, then decode accepts missing padding", () => {
    render(<EncodingPanel />);
    fireEvent.change(input(), { target: { value: "Sm9zw6kg5p2x5LqsIPCfmoA" } });

    expect(output("DECODED (treat input as Base64)")).toHaveTextContent("José 東京 🚀");
  });

  it("Given URL-safe output is selected, then it removes padding and substitutes reserved alphabet characters", () => {
    render(<EncodingPanel />);
    fireEvent.change(input(), { target: { value: "ÿÿ" } });
    fireEvent.click(screen.getByRole("checkbox"));

    expect(output("ENCODED")).toHaveTextContent("w7_Dvw");
    expect(output("ENCODED")).not.toHaveTextContent(/[+/=]/);
  });

  it("Given malformed Base64 or URL encoding, then it displays a decoding error", () => {
    render(<EncodingPanel />);
    fireEvent.change(input(), { target: { value: "%%%" } });
    expect(output("DECODED (treat input as Base64)")).toHaveTextContent(/^Error:/);

    fireEvent.click(screen.getByRole("button", { name: "URL" }));
    expect(output("URL DECODED (decodeURIComponent)")).toHaveTextContent(/^Error:/);
  });

  it("Given URL content, then it encodes and decodes reserved characters", () => {
    render(<EncodingPanel />);
    fireEvent.click(screen.getByRole("button", { name: "URL" }));
    fireEvent.change(input(), { target: { value: "a/b?x=hello world&emoji=🚀" } });
    expect(output("URL ENCODED (encodeURIComponent)")).toHaveTextContent("a%2Fb%3Fx%3Dhello%20world%26emoji%3D%F0%9F%9A%80");

    fireEvent.change(input(), { target: { value: "hello%20world%21" } });
    expect(output("URL DECODED (decodeURIComponent)")).toHaveTextContent("hello world!");
  });

  it("Given named, decimal, and hexadecimal entities, then HTML decode resolves all forms", () => {
    render(<EncodingPanel />);
    fireEvent.click(screen.getByRole("button", { name: "HTML" }));
    fireEvent.change(input(), { target: { value: "&lt;b&gt;Jos&#233; &#x1F680;&lt;/b&gt;" } });

    expect(output("HTML DECODED (treat input as HTML-encoded)")).toHaveTextContent("<b>José 🚀</b>");
  });

  it("Given unsafe markup, then HTML encode escapes all five syntax characters", () => {
    render(<EncodingPanel />);
    fireEvent.click(screen.getByRole("button", { name: "HTML" }));
    fireEvent.change(input(), { target: { value: `<script a="x">'&</script>` } });

    expect(output("HTML ENCODED")).toHaveTextContent("&lt;script a=&quot;x&quot;&gt;&#39;&amp;&lt;/script&gt;");
  });
});

describe("EncodingPanel — derived tools", () => {
  it("Given text, then it computes standard SHA digests", async () => {
    render(<EncodingPanel />);
    fireEvent.change(input(), { target: { value: "abc" } });
    fireEvent.click(screen.getByRole("button", { name: "Hash" }));

    await waitFor(() => expect(output("SHA-256")).toHaveTextContent("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"));
    expect(output("SHA-1")).toHaveTextContent("a9993e364706816aba3e25717850c26c9cd0d89d");
  });

  it("Given empty hash input, then it shows the hash guidance instead of stale digests", () => {
    render(<EncodingPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Hash" }));
    fireEvent.click(screen.getByRole("button", { name: "Clear" }));

    expect(screen.getByText("Type or paste text above to compute hashes.")).toBeInTheDocument();
    expect(screen.queryByText("SHA-256")).not.toBeInTheDocument();
  });

  it("Given mixed identifier syntax, then each case conversion is available and reusable", () => {
    render(<EncodingPanel />);
    fireEvent.change(input(), { target: { value: "HTTPServer user_name" } });
    fireEvent.click(screen.getByRole("button", { name: "Case" }));

    const snakeRow = screen.getByText("snake_case").parentElement!;
    expect(snakeRow).toHaveTextContent("http_server_user_name");
    fireEvent.click(snakeRow.querySelector("button")!);
    expect(input()).toHaveValue("http_server_user_name");
  });

  it("Given Unicode graphemes, then character and UTF-8 byte counts use different units", () => {
    render(<EncodingPanel />);
    fireEvent.change(input(), { target: { value: "A🚀é" } });
    fireEvent.click(screen.getByRole("button", { name: "Stats" }));

    expect(screen.getByText("Characters").previousElementSibling).toHaveTextContent("3");
    expect(screen.getByText("Bytes (UTF-8)").previousElementSibling).toHaveTextContent("7");
  });

  it("Given multiline prose, then it counts words, lines, sentences, and paragraphs", () => {
    render(<EncodingPanel />);
    fireEvent.change(input(), { target: { value: "Hello world.\n\n🚀 Go!" } });
    fireEvent.click(screen.getByRole("button", { name: "Stats" }));

    expect(screen.getByText("Words").previousElementSibling).toHaveTextContent("4");
    expect(screen.getByText("Lines").previousElementSibling).toHaveTextContent("3");
    expect(screen.getByText("Sentences").previousElementSibling).toHaveTextContent("2");
    expect(screen.getByText("Paragraphs").previousElementSibling).toHaveTextContent("2");
    expect(screen.getByText("TOP CHARACTERS (excluding whitespace)")).toBeInTheDocument();
  });

  it("Given clipboard text, when Paste is selected, then it replaces the input", async () => {
    const readText = vi.fn().mockResolvedValue("from clipboard 🚀");
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { readText } });
    render(<EncodingPanel />);

    fireEvent.click(screen.getByRole("button", { name: "Paste" }));

    await waitFor(() => expect(input()).toHaveValue("from clipboard 🚀"));
    expect(readText).toHaveBeenCalledOnce();
  });
});
