import { createComposite } from "./createComposite";

export const TerminalComposite = createComposite([
  { id: "scripts", label: "Scripts", importFn: () => import("../ScriptPanel"), exportName: "ScriptPanel" },
  { id: "ssh", label: "SSH", importFn: () => import("../SshPanel"), exportName: "SshPanel" },
  { id: "notebook", label: "Notebook", importFn: () => import("../NotebookPanel"), exportName: "NotebookPanel" },
  { id: "logs", label: "Logs", importFn: () => import("../LogPanel"), exportName: "LogPanel" },
  { id: "voicelocal", label: "Voice", importFn: () => import("../VoiceLocalPanel"), exportName: "VoiceLocalPanel" },
]);
