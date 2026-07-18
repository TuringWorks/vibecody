// TS mirror of `vibecli/vibecli-cli/src/recap.rs` wire shape.
// Bumped with that file. Spec: docs/design/recap-resume/01-session.md.

export type RecapKind = "session" | "job" | "diff_chain";

export type ArtifactKind = "file" | "diff" | "job" | "url";

export interface RecapArtifact {
  kind: ArtifactKind;
  label: string;
  locator: string;
}

export type RecapGenerator =
  | { type: "heuristic" }
  | { type: "user_edited" }
  | { type: "llm"; provider: string; model: string };

export type ResumeTarget =
  | { type: "session"; id: string }
  | { type: "job"; id: string }
  | { type: "diff_chain"; id: string };

export interface ResumeHint {
  target: ResumeTarget;
  from_message?: number | null;
  from_step?: number | null;
  from_diff_index?: number | null;
  seed_instruction?: string | null;
  branch_on_resume: boolean;
}

export interface RecapTokenUsage {
  input: number;
  output: number;
}

export interface Recap {
  id: string;
  kind: RecapKind;
  subject_id: string;
  last_message_id?: number | null;
  workspace?: string | null;
  generated_at: string;
  generator: RecapGenerator;
  headline: string;
  bullets: string[];
  next_actions: string[];
  artifacts: RecapArtifact[];
  resume_hint?: ResumeHint | null;
  token_usage?: RecapTokenUsage | null;
  schema_version: number;
}
