/**
 * Temporary harness: mounts the real DocumentViewer, with the app's own tokens
 * and stylesheets, against real files — so the two-page spread can be looked at
 * in a browser rather than only asserted about in jsdom.
 */
import React from "react";
import ReactDOM from "react-dom/client";

import "../design-system/tokens.css";
import { DocumentViewer } from "./components/DocumentViewer";

const DOCX_TEXT = `# Chapter Two: Leading the Team You Inherit

Most managers do not build their teams. They inherit them, usually in the middle
of something, from someone who has already left. The team has a history you were
not part of, habits nobody wrote down, and at least one long-running argument
that everyone has stopped naming out loud.

## The first ninety days

Map your first ninety days. Identify three to five priority conversations you
need to have in your first weeks — with your manager, with key stakeholders,
with your direct reports.

- Write down the questions you actually want answered.
- Schedule the conversations before the work fills the calendar.
- After each one, note what you learned and how it changed your picture.

**The point is not the plan.** It is that you will be asked to make decisions
before you have context, and the only way to shorten that window is to go and
get the context deliberately rather than waiting for it to arrive.

## Communication architecture

What are the current norms in your team? What works and what does not? Write
them down, share them, and ask for the corrections you will certainly get.

> A norm nobody has stated is a norm only the people who already know it can
> follow.

Teams do not fail for lack of talent nearly as often as they fail for lack of a
shared idea of what "done" means, who decides, and where things are written.
`;

(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
  invoke: async (cmd: string) => {
    if (cmd === "read_document_text") {
      return {
        format: "docx",
        language: "markdown",
        text: DOCX_TEXT,
        sections: 1,
        warnings: [],
        writable: true,
      };
    }
    return null;
  },
  transformCallback: (cb: unknown) => cb,
  convertFileSrc: (p: string) => p,
};

function Frame({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={{ height: "100vh", display: "flex", flexDirection: "column" }}>
      <div style={{ padding: 6, font: "12px system-ui", color: "var(--text-secondary)" }}>
        {title}
      </div>
      <div style={{ flex: 1, minHeight: 0, display: "flex" }}>{children}</div>
    </div>
  );
}

async function main() {
  const which = new URLSearchParams(location.search).get("doc") ?? "pdf";
  const root = ReactDOM.createRoot(document.getElementById("root")!);

  if (which === "docx") {
    root.render(
      <Frame title="DOCX — the real viewer">
        <DocumentViewer filePath="/docs/stepping-up.docx" base64Data="" />
      </Frame>,
    );
    return;
  }

  const bytes = new Uint8Array(await (await fetch("/preview-sample.pdf")).arrayBuffer());
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  root.render(
    <Frame title="PDF — the real viewer">
      <DocumentViewer filePath="/docs/sample.pdf" base64Data={btoa(binary)} />
    </Frame>,
  );
}

void main();
