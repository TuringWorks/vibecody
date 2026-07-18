/**
 * Productivity panel DTOs — mirror the serde-serialized Rust types returned
 * by `productivity_*` Tauri commands.
 *
 * Field names are snake_case because Tauri serializes Rust structs as-is
 * (no automatic camelCase conversion configured).
 */

export interface Email {
  id: string;
  from: string;
  to: string;
  subject: string;
  snippet: string;
  date: string;
  is_read: boolean;
  labels: string[];
}

export interface EmailBody {
  id: string;
  from: string;
  to: string;
  cc: string;
  subject: string;
  date: string;
  body_text: string;
  body_html: string;
  is_read: boolean;
  labels: string[];
}

export interface EmailLabel {
  id: string;
  name: string;
  message_count: number | null;
}

/**
 * Auth/config status for the provider status strip.
 * `connected: false` is a legitimate "not signed in" state, not an error.
 */
export interface ProviderStatus {
  connected: boolean;
  provider: string | null;
  account: string | null;
  message: string | null;
}

// ── Calendar ────────────────────────────────────────────────────────────────

export interface CalendarEvent {
  id: string;
  summary: string;
  start: string;
  end: string;
  location: string | null;
  description: string | null;
  attendees: string[];
  status: string;
}

export interface FreeSlot {
  start: string;
  end: string;
}

// ── Tasks (Todoist) ─────────────────────────────────────────────────────────

export interface TodoistTask {
  id: string;
  content: string;
  description: string;
  due: string | null;
  priority: number; // 1 (normal) - 4 (urgent) per Todoist API
  project_id: string | null;
  is_completed: boolean;
}

// ── Notion ──────────────────────────────────────────────────────────────────

export interface NotionPage {
  id: string;
  title: string;
  url: string;
  last_edited: string;
  icon: string | null;
}

// ── Jira ────────────────────────────────────────────────────────────────────

export interface JiraIssue {
  key: string;
  summary: string;
  status: string;
  priority: string;
  assignee: string | null;
  issue_type: string;
  url: string;
}

// ── Home Assistant ──────────────────────────────────────────────────────────

export interface HaEntity {
  entity_id: string;
  state: string;
  // `attributes` is arbitrary JSON from HA (friendly_name, brightness, etc).
  attributes: Record<string, unknown>;
}
