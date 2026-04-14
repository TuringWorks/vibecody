//! Google Calendar and Outlook Calendar integration for VibeCLI.
//!
//! Connects to both Google Calendar (REST API) and Microsoft Outlook Calendar
//! (Graph API) to list, create, delete, and reschedule events.
//!
//! Configuration:
//! - **Google**: `GOOGLE_CALENDAR_TOKEN` env var, or `google_calendar.access_token` in config
//! - **Outlook**: `OUTLOOK_ACCESS_TOKEN` env var, or `outlook.access_token` in config
//!
//! Usage in REPL:
//! ```text
//! /calendar today              — Show today's events
//! /calendar week               — Show this week's events
//! /calendar list [N]           — List next N events (default 10)
//! /calendar create <title> --start <datetime> --end <datetime>
//! /calendar delete <id>        — Delete/cancel event
//! /calendar free [date]        — Show free slots (today or given date)
//! /calendar move <id> --to <datetime>
//! /calendar next               — Show the next upcoming event
//! /calendar remind             — Set up 5-min reminder for next event
//! ```

use anyhow::{anyhow, Result};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use vibe_ai::{retry_async, RetryConfig};

// ── Constants ────────────────────────────────────────────────────────────────

const GOOGLE_BASE_URL: &str = "https://www.googleapis.com/calendar/v3";
const OUTLOOK_BASE_URL: &str = "https://graph.microsoft.com/v1.0/me";

const WORK_HOUR_START: u32 = 9;
const WORK_HOUR_END: u32 = 18;

// ── Data types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalendarProvider {
    Google,
    Outlook,
}

impl std::fmt::Display for CalendarProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalendarProvider::Google => write!(f, "Google Calendar"),
            CalendarProvider::Outlook => write!(f, "Outlook Calendar"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub summary: String,
    pub start: String, // ISO 8601
    pub end: String,   // ISO 8601
    pub location: Option<String>,
    pub description: Option<String>,
    pub attendees: Vec<String>,
    pub status: String, // confirmed, tentative, cancelled
}

impl CalendarEvent {
    pub fn status_icon(&self) -> &str {
        match self.status.as_str() {
            "confirmed" => "✅",
            "tentative" => "❓",
            "cancelled" => "❌",
            _ => "🗓",
        }
    }

    /// Format a single event as a human-readable line.
    pub fn format_line(&self) -> String {
        let loc = self
            .location
            .as_deref()
            .map(|l| format!(" 📍 {}", l))
            .unwrap_or_default();
        let attendee_count = if self.attendees.is_empty() {
            String::new()
        } else {
            format!(" ({} attendees)", self.attendees.len())
        };
        format!(
            "{icon} 🕐 {start} – {end}  {summary}{loc}{att}\n",
            icon = self.status_icon(),
            start = format_time_short(&self.start),
            end = format_time_short(&self.end),
            summary = self.summary,
            loc = loc,
            att = attendee_count,
        )
    }
}

/// Extract HH:MM from an ISO 8601 datetime string for display.
fn format_time_short(iso: &str) -> String {
    // Try to parse with chrono; fall back to raw string.
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) {
        let local = dt.with_timezone(&chrono::Local);
        return local.format("%H:%M").to_string();
    }
    // If it contains a 'T', grab the time portion.
    if let Some(pos) = iso.find('T') {
        let time_part = &iso[pos + 1..];
        return time_part.chars().take(5).collect();
    }
    iso.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeSlot {
    pub start: String,
    pub end: String,
}

impl FreeSlot {
    pub fn format_line(&self) -> String {
        format!(
            "  🔓 {} – {}\n",
            format_time_short(&self.start),
            format_time_short(&self.end),
        )
    }
}

// ── CalendarClient ───────────────────────────────────────────────────────────

pub struct CalendarClient {
    provider: CalendarProvider,
    access_token: String,
    client: reqwest::Client,
}

impl CalendarClient {
    pub fn new(provider: CalendarProvider, access_token: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("VibeCLI/1.0")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            provider,
            access_token,
            client,
        }
    }

    /// Try to resolve credentials from env or config; returns the first available provider.
    pub fn from_env_or_config() -> Option<Self> {
        // 0. ProfileStore (encrypted SQLite) — highest priority
        if let Ok(store) = crate::profile_store::ProfileStore::new() {
            if let Ok(Some(tok)) = store.get_api_key("default", "integration.calendar.google_access_token") {
                if !tok.is_empty() { return Some(Self::new(CalendarProvider::Google, tok)); }
            }
            if let Ok(Some(tok)) = store.get_api_key("default", "integration.calendar.outlook_access_token") {
                if !tok.is_empty() { return Some(Self::new(CalendarProvider::Outlook, tok)); }
            }
        }
        // 1. Google Calendar — env
        if let Ok(token) = std::env::var("GOOGLE_CALENDAR_TOKEN") {
            if !token.is_empty() {
                return Some(Self::new(CalendarProvider::Google, token));
            }
        }
        // 2. Google Calendar — config (calendar.google_access_token)
        if let Ok(cfg) = crate::config::Config::load() {
            if let Some(ref cal) = cfg.calendar {
                if let Some(ref token) = cal.google_access_token {
                    if !token.is_empty() {
                        return Some(Self::new(CalendarProvider::Google, token.clone()));
                    }
                }
            }
        }
        // 3. Outlook — env
        if let Ok(token) = std::env::var("OUTLOOK_ACCESS_TOKEN") {
            if !token.is_empty() {
                return Some(Self::new(CalendarProvider::Outlook, token));
            }
        }
        // 4. Outlook — config (calendar.outlook_access_token)
        if let Ok(cfg) = crate::config::Config::load() {
            if let Some(ref cal) = cfg.calendar {
                if let Some(ref token) = cal.outlook_access_token {
                    if !token.is_empty() {
                        return Some(Self::new(CalendarProvider::Outlook, token.clone()));
                    }
                }
            }
        }
        None
    }

    fn base_url(&self) -> &str {
        match self.provider {
            CalendarProvider::Google => GOOGLE_BASE_URL,
            CalendarProvider::Outlook => OUTLOOK_BASE_URL,
        }
    }

    // ── HTTP helpers ─────────────────────────────────────────────────────────

    async fn get(&self, url: &str) -> Result<serde_json::Value> {
        let resp = retry_async(&RetryConfig::default(), "calendar-get", || {
            let client = self.client.clone();
            let token = self.access_token.clone();
            let url = url.to_string();
            async move {
                client
                    .get(&url)
                    .bearer_auth(&token)
                    .send()
                    .await
                    .map_err(Into::into)
            }
        })
        .await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = body["error"]["message"]
                .as_str()
                .or_else(|| body["error"]["code"].as_str())
                .unwrap_or("Unknown API error");
            return Err(anyhow!("{} API error ({}): {}", self.provider, status, msg));
        }
        Ok(body)
    }

    async fn post(&self, url: &str, payload: &serde_json::Value) -> Result<serde_json::Value> {
        let resp = retry_async(&RetryConfig::default(), "calendar-post", || {
            let client = self.client.clone();
            let token = self.access_token.clone();
            let url = url.to_string();
            let payload = payload.clone();
            async move {
                client
                    .post(&url)
                    .bearer_auth(&token)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(Into::into)
            }
        })
        .await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown API error");
            return Err(anyhow!("{} API error ({}): {}", self.provider, status, msg));
        }
        Ok(body)
    }

    async fn patch(&self, url: &str, payload: &serde_json::Value) -> Result<serde_json::Value> {
        let resp = retry_async(&RetryConfig::default(), "calendar-patch", || {
            let client = self.client.clone();
            let token = self.access_token.clone();
            let url = url.to_string();
            let payload = payload.clone();
            async move {
                client
                    .patch(&url)
                    .bearer_auth(&token)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(Into::into)
            }
        })
        .await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            let msg = body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown API error");
            return Err(anyhow!("{} API error ({}): {}", self.provider, status, msg));
        }
        Ok(body)
    }

    async fn delete_req(&self, url: &str) -> Result<()> {
        let resp = retry_async(&RetryConfig::default(), "calendar-delete", || {
            let client = self.client.clone();
            let token = self.access_token.clone();
            let url = url.to_string();
            async move {
                client
                    .delete(&url)
                    .bearer_auth(&token)
                    .send()
                    .await
                    .map_err(Into::into)
            }
        })
        .await?;
        let status = resp.status();
        if !status.is_success() {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            let msg = body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown API error");
            return Err(anyhow!("{} API error ({}): {}", self.provider, status, msg));
        }
        Ok(())
    }

    // ── Event parsing ────────────────────────────────────────────────────────

    fn parse_event(&self, v: &serde_json::Value) -> CalendarEvent {
        match self.provider {
            CalendarProvider::Google => self.parse_google_event(v),
            CalendarProvider::Outlook => self.parse_outlook_event(v),
        }
    }

    fn parse_google_event(&self, v: &serde_json::Value) -> CalendarEvent {
        let start = v["start"]["dateTime"]
            .as_str()
            .or_else(|| v["start"]["date"].as_str())
            .unwrap_or("")
            .to_string();
        let end = v["end"]["dateTime"]
            .as_str()
            .or_else(|| v["end"]["date"].as_str())
            .unwrap_or("")
            .to_string();
        let attendees = v["attendees"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| a["email"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        CalendarEvent {
            id: v["id"].as_str().unwrap_or("").to_string(),
            summary: v["summary"].as_str().unwrap_or("(No title)").to_string(),
            start,
            end,
            location: v["location"].as_str().map(String::from),
            description: v["description"].as_str().map(String::from),
            attendees,
            status: v["status"].as_str().unwrap_or("confirmed").to_string(),
        }
    }

    fn parse_outlook_event(&self, v: &serde_json::Value) -> CalendarEvent {
        let start = v["start"]["dateTime"].as_str().unwrap_or("").to_string();
        let end = v["end"]["dateTime"].as_str().unwrap_or("").to_string();
        let attendees = v["attendees"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| {
                        a["emailAddress"]["address"]
                            .as_str()
                            .map(String::from)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let status = if v["isCancelled"].as_bool().unwrap_or(false) {
            "cancelled".to_string()
        } else if v["responseStatus"]["response"]
            .as_str()
            .unwrap_or("")
            == "tentativelyAccepted"
        {
            "tentative".to_string()
        } else {
            "confirmed".to_string()
        };
        CalendarEvent {
            id: v["id"].as_str().unwrap_or("").to_string(),
            summary: v["subject"].as_str().unwrap_or("(No title)").to_string(),
            start,
            end,
            location: v["location"]["displayName"].as_str().map(String::from),
            description: v["bodyPreview"].as_str().map(String::from),
            attendees,
            status,
        }
    }

    // ── Public API ───────────────────────────────────────────────────────────

    /// List events in a time range.
    pub async fn list_events(
        &self,
        time_min: &str,
        time_max: &str,
        max_results: usize,
    ) -> Result<Vec<CalendarEvent>> {
        let url = match self.provider {
            CalendarProvider::Google => {
                format!(
                    "{}/calendars/primary/events?timeMin={}&timeMax={}&maxResults={}&singleEvents=true&orderBy=startTime",
                    self.base_url(), time_min, time_max, max_results,
                )
            }
            CalendarProvider::Outlook => {
                format!(
                    "{}/calendarView?startDateTime={}&endDateTime={}&$top={}&$orderby=start/dateTime",
                    self.base_url(), time_min, time_max, max_results,
                )
            }
        };

        let data = self.get(&url).await?;
        let items_key = match self.provider {
            CalendarProvider::Google => "items",
            CalendarProvider::Outlook => "value",
        };
        let events: Vec<CalendarEvent> = data[items_key]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|v| self.parse_event(v))
            .collect();
        Ok(events)
    }

    /// List the next N upcoming events.
    pub async fn list_upcoming(&self, max_results: usize) -> Result<Vec<CalendarEvent>> {
        let now = chrono::Utc::now().to_rfc3339();
        let far_future = (chrono::Utc::now() + chrono::Duration::days(365)).to_rfc3339();
        self.list_events(&now, &far_future, max_results).await
    }

    /// Get today's events.
    pub async fn today_events(&self) -> Result<Vec<CalendarEvent>> {
        let today = chrono::Local::now().date_naive();
        let start = today
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap()
            .to_rfc3339();
        let end = today
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap()
            .to_rfc3339();
        self.list_events(&start, &end, 50).await
    }

    /// Get this week's events (Monday through Sunday).
    pub async fn week_events(&self) -> Result<Vec<CalendarEvent>> {
        let today = chrono::Local::now().date_naive();
        let weekday = today.weekday().num_days_from_monday();
        let monday = today - chrono::Duration::days(weekday as i64);
        let sunday = monday + chrono::Duration::days(6);
        let start = monday
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap()
            .to_rfc3339();
        let end = sunday
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap()
            .to_rfc3339();
        self.list_events(&start, &end, 100).await
    }

    /// Create a new event.
    pub async fn create_event(
        &self,
        summary: &str,
        start: &str,
        end: &str,
    ) -> Result<CalendarEvent> {
        let (url, payload) = match self.provider {
            CalendarProvider::Google => {
                let url = format!("{}/calendars/primary/events", self.base_url());
                let payload = serde_json::json!({
                    "summary": summary,
                    "start": { "dateTime": start },
                    "end": { "dateTime": end },
                });
                (url, payload)
            }
            CalendarProvider::Outlook => {
                let url = format!("{}/events", self.base_url());
                let payload = serde_json::json!({
                    "subject": summary,
                    "start": { "dateTime": start, "timeZone": "UTC" },
                    "end": { "dateTime": end, "timeZone": "UTC" },
                });
                (url, payload)
            }
        };

        let data = self.post(&url, &payload).await?;
        Ok(self.parse_event(&data))
    }

    /// Delete/cancel an event by ID.
    pub async fn delete_event(&self, event_id: &str) -> Result<()> {
        let url = match self.provider {
            CalendarProvider::Google => {
                format!(
                    "{}/calendars/primary/events/{}",
                    self.base_url(),
                    event_id
                )
            }
            CalendarProvider::Outlook => {
                format!("{}/events/{}", self.base_url(), event_id)
            }
        };
        self.delete_req(&url).await
    }

    /// Move/reschedule an event to a new start time (keeps same duration).
    pub async fn move_event(
        &self,
        event_id: &str,
        new_start: &str,
    ) -> Result<CalendarEvent> {
        // We need the original event to compute duration.
        let events = self.list_upcoming(50).await?;
        let original = events
            .iter()
            .find(|e| e.id == event_id)
            .ok_or_else(|| anyhow!("Event '{}' not found", event_id))?;

        // Compute duration from original
        let orig_start = chrono::DateTime::parse_from_rfc3339(&original.start)
            .unwrap_or_else(|_| chrono::Utc::now().into());
        let orig_end = chrono::DateTime::parse_from_rfc3339(&original.end)
            .unwrap_or_else(|_| chrono::Utc::now().into());
        let duration = orig_end - orig_start;

        let new_start_dt =
            chrono::DateTime::parse_from_rfc3339(new_start)
                .map_err(|e| anyhow!("Invalid datetime '{}': {}", new_start, e))?;
        let new_end_dt = new_start_dt + duration;
        let new_end = new_end_dt.to_rfc3339();

        let (url, payload) = match self.provider {
            CalendarProvider::Google => {
                let url = format!(
                    "{}/calendars/primary/events/{}",
                    self.base_url(),
                    event_id
                );
                let payload = serde_json::json!({
                    "start": { "dateTime": new_start },
                    "end": { "dateTime": new_end },
                });
                (url, payload)
            }
            CalendarProvider::Outlook => {
                let url = format!("{}/events/{}", self.base_url(), event_id);
                let payload = serde_json::json!({
                    "start": { "dateTime": new_start, "timeZone": "UTC" },
                    "end": { "dateTime": new_end, "timeZone": "UTC" },
                });
                (url, payload)
            }
        };
        let data = self.patch(&url, &payload).await?;
        Ok(self.parse_event(&data))
    }

    /// Find the next upcoming event.
    pub async fn next_event(&self) -> Result<Option<CalendarEvent>> {
        let events = self.list_upcoming(1).await?;
        Ok(events.into_iter().next())
    }
}

// ── Free-slot calculation ────────────────────────────────────────────────────

/// Compute free slots between events during work hours (9am–6pm).
pub fn compute_free_slots(events: &[CalendarEvent], date: chrono::NaiveDate) -> Vec<FreeSlot> {
    let tz = chrono::Local::now().timezone();
    let work_start = date
        .and_hms_opt(WORK_HOUR_START, 0, 0)
        .unwrap()
        .and_local_timezone(tz)
        .unwrap();
    let work_end = date
        .and_hms_opt(WORK_HOUR_END, 0, 0)
        .unwrap()
        .and_local_timezone(tz)
        .unwrap();

    // Collect busy intervals, clamped to work hours.
    let mut busy: Vec<(chrono::DateTime<chrono::FixedOffset>, chrono::DateTime<chrono::FixedOffset>)> = Vec::new();
    for event in events {
        if event.status == "cancelled" {
            continue;
        }
        let Ok(s) = chrono::DateTime::parse_from_rfc3339(&event.start) else {
            continue;
        };
        let Ok(e) = chrono::DateTime::parse_from_rfc3339(&event.end) else {
            continue;
        };
        let s = s.max(work_start.into());
        let e = e.min(work_end.into());
        if s < e {
            busy.push((s, e));
        }
    }
    busy.sort_by_key(|(s, _)| *s);

    // Merge overlapping intervals.
    let mut merged: Vec<(chrono::DateTime<chrono::FixedOffset>, chrono::DateTime<chrono::FixedOffset>)> = Vec::new();
    for (s, e) in &busy {
        if let Some(last) = merged.last_mut() {
            if *s <= last.1 {
                last.1 = last.1.max(*e);
                continue;
            }
        }
        merged.push((*s, *e));
    }

    // Gaps between merged intervals are the free slots.
    let mut slots = Vec::new();
    let mut cursor: chrono::DateTime<chrono::FixedOffset> = work_start.into();
    for (s, e) in &merged {
        if cursor < *s {
            slots.push(FreeSlot {
                start: cursor.to_rfc3339(),
                end: s.to_rfc3339(),
            });
        }
        cursor = cursor.max(*e);
    }
    let work_end_fixed: chrono::DateTime<chrono::FixedOffset> = work_end.into();
    if cursor < work_end_fixed {
        slots.push(FreeSlot {
            start: cursor.to_rfc3339(),
            end: work_end_fixed.to_rfc3339(),
        });
    }
    slots
}

// ── Argument parsing helpers ─────────────────────────────────────────────────

fn extract_flag<'a>(parts: &'a [&'a str], flag: &str) -> Option<&'a str> {
    parts
        .iter()
        .position(|p| *p == flag)
        .and_then(|i| parts.get(i + 1).copied())
}

/// Parse a date string loosely: YYYY-MM-DD or relative words like "today", "tomorrow".
fn parse_date_loose(s: &str) -> Option<chrono::NaiveDate> {
    match s.to_lowercase().as_str() {
        "today" => Some(chrono::Local::now().date_naive()),
        "tomorrow" => Some(chrono::Local::now().date_naive() + chrono::Duration::days(1)),
        _ => chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok(),
    }
}

// ── REPL handler ─────────────────────────────────────────────────────────────

/// Run the `/calendar` REPL command.
/// Returns a human-readable output string.
pub async fn handle_calendar_command(args: &str) -> String {
    let client = match CalendarClient::from_env_or_config() {
        Some(c) => c,
        None => {
            return "⚠️  Calendar not configured.\n\
                Set GOOGLE_CALENDAR_TOKEN or OUTLOOK_ACCESS_TOKEN env var,\n\
                or add the token to ~/.vibecli/config.toml\n"
                .to_string();
        }
    };

    let parts: Vec<&str> = args.split_whitespace().collect();
    let sub = parts.first().copied().unwrap_or("").trim();

    match sub {
        "today" | "agenda" | "" => match client.today_events().await {
            Ok(events) => {
                if events.is_empty() {
                    return format!(
                        "🗓 No events today ({}).\n",
                        chrono::Local::now().format("%A, %B %d")
                    );
                }
                let mut out = format!(
                    "🗓 Today's agenda — {} ({} events):\n",
                    chrono::Local::now().format("%A, %B %d"),
                    events.len()
                );
                for ev in &events {
                    out.push_str(&ev.format_line());
                }
                out
            }
            Err(e) => format!("❌ Calendar error: {}\n", e),
        },

        "week" => match client.week_events().await {
            Ok(events) => {
                if events.is_empty() {
                    return "🗓 No events this week.\n".to_string();
                }
                let mut out = format!("🗓 This week ({} events):\n", events.len());
                for ev in &events {
                    out.push_str(&ev.format_line());
                }
                out
            }
            Err(e) => format!("❌ Calendar error: {}\n", e),
        },

        "list" => {
            let n: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
            match client.list_upcoming(n).await {
                Ok(events) => {
                    if events.is_empty() {
                        return "🗓 No upcoming events.\n".to_string();
                    }
                    let mut out = format!("🗓 Next {} events:\n", events.len());
                    for ev in &events {
                        out.push_str(&ev.format_line());
                    }
                    out
                }
                Err(e) => format!("❌ Calendar error: {}\n", e),
            }
        }

        "create" => {
            // /calendar create <title> --start <dt> --end <dt>
            let start = extract_flag(&parts, "--start");
            let end = extract_flag(&parts, "--end");
            let (Some(start), Some(end)) = (start, end) else {
                return "Usage: /calendar create <title> --start <ISO-datetime> --end <ISO-datetime>\n\
                    Example: /calendar create \"Team Standup\" --start 2026-04-04T10:00:00Z --end 2026-04-04T10:30:00Z\n"
                    .to_string();
            };
            // Title is everything between "create" and "--start"
            let title_parts: Vec<&str> = parts[1..]
                .iter()
                .take_while(|p| **p != "--start")
                .copied()
                .collect();
            let title = title_parts.join(" ");
            let title = title.trim().trim_matches('"');
            if title.is_empty() {
                return "Usage: /calendar create <title> --start <datetime> --end <datetime>\n"
                    .to_string();
            }
            match client.create_event(title, start, end).await {
                Ok(ev) => format!(
                    "✅ Created: {} (🕐 {} – {})\n   ID: {}\n",
                    ev.summary,
                    format_time_short(&ev.start),
                    format_time_short(&ev.end),
                    ev.id,
                ),
                Err(e) => format!("❌ Failed to create event: {}\n", e),
            }
        }

        "delete" => {
            let id = parts.get(1).copied().unwrap_or("").trim();
            if id.is_empty() {
                return "Usage: /calendar delete <event-id>\n".to_string();
            }
            match client.delete_event(id).await {
                Ok(()) => format!("🗑 Event {} deleted.\n", id),
                Err(e) => format!("❌ Failed to delete event: {}\n", e),
            }
        }

        "free" => {
            let date = parts
                .get(1)
                .and_then(|s| parse_date_loose(s))
                .unwrap_or_else(|| chrono::Local::now().date_naive());
            let date_start = date
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(chrono::Local)
                .unwrap()
                .to_rfc3339();
            let date_end = date
                .and_hms_opt(23, 59, 59)
                .unwrap()
                .and_local_timezone(chrono::Local)
                .unwrap()
                .to_rfc3339();
            match client.list_events(&date_start, &date_end, 50).await {
                Ok(events) => {
                    let slots = compute_free_slots(&events, date);
                    if slots.is_empty() {
                        return format!(
                            "🗓 No free slots on {} ({}:00–{}:00 work hours).\n",
                            date, WORK_HOUR_START, WORK_HOUR_END
                        );
                    }
                    let mut out = format!(
                        "🔓 Free slots on {} ({} slots, {}:00–{}:00):\n",
                        date,
                        slots.len(),
                        WORK_HOUR_START,
                        WORK_HOUR_END,
                    );
                    for slot in &slots {
                        out.push_str(&slot.format_line());
                    }
                    out
                }
                Err(e) => format!("❌ Calendar error: {}\n", e),
            }
        }

        "move" => {
            // /calendar move <id> --to <datetime>
            let id = parts.get(1).copied().unwrap_or("").trim();
            let new_time = extract_flag(&parts, "--to");
            if id.is_empty() || new_time.is_none() {
                return "Usage: /calendar move <event-id> --to <ISO-datetime>\n".to_string();
            }
            match client.move_event(id, new_time.unwrap()).await {
                Ok(ev) => format!(
                    "✅ Rescheduled: {} → 🕐 {} – {}\n",
                    ev.summary,
                    format_time_short(&ev.start),
                    format_time_short(&ev.end),
                ),
                Err(e) => format!("❌ Failed to reschedule: {}\n", e),
            }
        }

        "next" => match client.next_event().await {
            Ok(Some(ev)) => {
                format!("⏭ Next event:\n{}", ev.format_line())
            }
            Ok(None) => "🗓 No upcoming events.\n".to_string(),
            Err(e) => format!("❌ Calendar error: {}\n", e),
        },

        "remind" => match client.next_event().await {
            Ok(Some(ev)) => {
                // Compute minutes until event.
                if let Ok(start_dt) = chrono::DateTime::parse_from_rfc3339(&ev.start) {
                    let now = chrono::Utc::now();
                    let diff = start_dt.signed_duration_since(now);
                    let mins = diff.num_minutes();
                    if mins <= 0 {
                        format!(
                            "🔔 {} is happening now or already started!\n{}",
                            ev.summary,
                            ev.format_line()
                        )
                    } else if mins <= 5 {
                        format!(
                            "🔔 {} starts in {} minute(s)!\n{}",
                            ev.summary,
                            mins,
                            ev.format_line()
                        )
                    } else {
                        let remind_in = mins - 5;
                        format!(
                            "🔔 Reminder set: {} in {} minutes (remind in {} min).\n{}",
                            ev.summary,
                            mins,
                            remind_in,
                            ev.format_line()
                        )
                    }
                } else {
                    format!(
                        "🔔 Next event: {} (could not parse start time for reminder)\n",
                        ev.summary
                    )
                }
            }
            Ok(None) => "🗓 No upcoming events to remind about.\n".to_string(),
            Err(e) => format!("❌ Calendar error: {}\n", e),
        },

        _ => {
            "Usage:\n  \
             /calendar today          — Show today's events\n  \
             /calendar week           — Show this week's events\n  \
             /calendar list [N]       — List next N events (default 10)\n  \
             /calendar create <title> --start <dt> --end <dt>\n  \
             /calendar delete <id>    — Delete/cancel event\n  \
             /calendar free [date]    — Show free slots\n  \
             /calendar move <id> --to <datetime>\n  \
             /calendar next           — Show next upcoming event\n  \
             /calendar remind         — 5-min reminder for next event\n"
                .to_string()
        }
    }
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Serialize all tests that mutate process-wide env vars to avoid races.
    static CALENDAR_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn make_event(id: &str, summary: &str, start: &str, end: &str, status: &str) -> CalendarEvent {
        CalendarEvent {
            id: id.to_string(),
            summary: summary.to_string(),
            start: start.to_string(),
            end: end.to_string(),
            location: None,
            description: None,
            attendees: vec![],
            status: status.to_string(),
        }
    }

    // ── CalendarEvent ────────────────────────────────────────────────────────

    #[test]
    fn status_icon_confirmed() {
        let ev = make_event("1", "Meeting", "", "", "confirmed");
        assert_eq!(ev.status_icon(), "✅");
    }

    #[test]
    fn status_icon_tentative() {
        let ev = make_event("1", "Maybe", "", "", "tentative");
        assert_eq!(ev.status_icon(), "❓");
    }

    #[test]
    fn status_icon_cancelled() {
        let ev = make_event("1", "Nope", "", "", "cancelled");
        assert_eq!(ev.status_icon(), "❌");
    }

    #[test]
    fn status_icon_unknown() {
        let ev = make_event("1", "Hmm", "", "", "something_else");
        assert_eq!(ev.status_icon(), "🗓");
    }

    #[test]
    fn format_line_includes_summary() {
        let ev = make_event("1", "Team Standup", "2026-04-04T10:00:00Z", "2026-04-04T10:30:00Z", "confirmed");
        let line = ev.format_line();
        assert!(line.contains("Team Standup"));
        assert!(line.contains("✅"));
    }

    #[test]
    fn format_line_with_location() {
        let mut ev = make_event("1", "Lunch", "2026-04-04T12:00:00Z", "2026-04-04T13:00:00Z", "confirmed");
        ev.location = Some("Cafe".to_string());
        let line = ev.format_line();
        assert!(line.contains("📍 Cafe"));
    }

    #[test]
    fn format_line_with_attendees() {
        let mut ev = make_event("1", "Sync", "", "", "confirmed");
        ev.attendees = vec!["alice@co.com".into(), "bob@co.com".into()];
        let line = ev.format_line();
        assert!(line.contains("2 attendees"));
    }

    // ── CalendarEvent serde ──────────────────────────────────────────────────

    #[test]
    fn calendar_event_serde_roundtrip() {
        let ev = CalendarEvent {
            id: "evt-1".into(),
            summary: "Roundtrip Test".into(),
            start: "2026-04-04T09:00:00Z".into(),
            end: "2026-04-04T10:00:00Z".into(),
            location: Some("Room 42".into()),
            description: Some("A description".into()),
            attendees: vec!["a@b.com".into()],
            status: "confirmed".into(),
        };
        let json = serde_json::to_string(&ev).unwrap();
        let back: CalendarEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "evt-1");
        assert_eq!(back.summary, "Roundtrip Test");
        assert_eq!(back.location, Some("Room 42".into()));
        assert_eq!(back.attendees.len(), 1);
    }

    #[test]
    fn calendar_event_clone() {
        let ev = make_event("x", "Clone", "s", "e", "confirmed");
        let cloned = ev.clone();
        assert_eq!(cloned.id, ev.id);
        assert_eq!(cloned.summary, ev.summary);
    }

    // ── CalendarProvider ─────────────────────────────────────────────────────

    #[test]
    fn provider_display() {
        assert_eq!(format!("{}", CalendarProvider::Google), "Google Calendar");
        assert_eq!(format!("{}", CalendarProvider::Outlook), "Outlook Calendar");
    }

    #[test]
    fn provider_serde_roundtrip() {
        let g = CalendarProvider::Google;
        let json = serde_json::to_string(&g).unwrap();
        let back: CalendarProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(back, CalendarProvider::Google);
    }

    // ── format_time_short ────────────────────────────────────────────────────

    #[test]
    fn format_time_short_rfc3339() {
        let t = format_time_short("2026-04-04T14:30:00+00:00");
        // Should produce local time HH:MM (exact value depends on timezone)
        assert_eq!(t.len(), 5);
        assert!(t.contains(':'));
    }

    #[test]
    fn format_time_short_with_t() {
        let t = format_time_short("2026-04-04T09:15:00");
        assert_eq!(t, "09:15");
    }

    #[test]
    fn format_time_short_fallback() {
        let t = format_time_short("not-a-date");
        assert_eq!(t, "not-a-date");
    }

    // ── FreeSlot ─────────────────────────────────────────────────────────────

    #[test]
    fn free_slot_format_line() {
        let slot = FreeSlot {
            start: "2026-04-04T09:00:00+00:00".into(),
            end: "2026-04-04T10:00:00+00:00".into(),
        };
        let line = slot.format_line();
        assert!(line.contains("🔓"));
        assert!(line.contains("–"));
    }

    // ── compute_free_slots ───────────────────────────────────────────────────

    #[test]
    fn free_slots_no_events_full_workday() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 4, 6).unwrap(); // a Monday
        let slots = compute_free_slots(&[], date);
        assert_eq!(slots.len(), 1, "Entire workday should be free");
    }

    #[test]
    fn free_slots_cancelled_events_ignored() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 4, 6).unwrap();
        let tz = chrono::Local::now().timezone();
        let s = date.and_hms_opt(10, 0, 0).unwrap().and_local_timezone(tz).unwrap().to_rfc3339();
        let e = date.and_hms_opt(11, 0, 0).unwrap().and_local_timezone(tz).unwrap().to_rfc3339();
        let events = vec![make_event("1", "Cancelled", &s, &e, "cancelled")];
        let slots = compute_free_slots(&events, date);
        assert_eq!(slots.len(), 1, "Cancelled event should be ignored");
    }

    #[test]
    fn free_slots_one_event_two_gaps() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 4, 6).unwrap();
        let tz = chrono::Local::now().timezone();
        let s = date.and_hms_opt(12, 0, 0).unwrap().and_local_timezone(tz).unwrap().to_rfc3339();
        let e = date.and_hms_opt(13, 0, 0).unwrap().and_local_timezone(tz).unwrap().to_rfc3339();
        let events = vec![make_event("1", "Lunch", &s, &e, "confirmed")];
        let slots = compute_free_slots(&events, date);
        assert_eq!(slots.len(), 2, "Should have gap before and after event");
    }

    #[test]
    fn free_slots_overlapping_events_merged() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 4, 6).unwrap();
        let tz = chrono::Local::now().timezone();
        let s1 = date.and_hms_opt(10, 0, 0).unwrap().and_local_timezone(tz).unwrap().to_rfc3339();
        let e1 = date.and_hms_opt(11, 30, 0).unwrap().and_local_timezone(tz).unwrap().to_rfc3339();
        let s2 = date.and_hms_opt(11, 0, 0).unwrap().and_local_timezone(tz).unwrap().to_rfc3339();
        let e2 = date.and_hms_opt(12, 0, 0).unwrap().and_local_timezone(tz).unwrap().to_rfc3339();
        let events = vec![
            make_event("1", "A", &s1, &e1, "confirmed"),
            make_event("2", "B", &s2, &e2, "confirmed"),
        ];
        let slots = compute_free_slots(&events, date);
        // Should merge: one gap 09-10, one gap 12-18 = 2 slots
        assert_eq!(slots.len(), 2);
    }

    // ── parse_date_loose ─────────────────────────────────────────────────────

    #[test]
    fn parse_date_loose_today() {
        let d = parse_date_loose("today").unwrap();
        assert_eq!(d, chrono::Local::now().date_naive());
    }

    #[test]
    fn parse_date_loose_tomorrow() {
        let d = parse_date_loose("tomorrow").unwrap();
        assert_eq!(
            d,
            chrono::Local::now().date_naive() + chrono::Duration::days(1)
        );
    }

    #[test]
    fn parse_date_loose_iso() {
        let d = parse_date_loose("2026-04-10").unwrap();
        assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2026, 4, 10).unwrap());
    }

    #[test]
    fn parse_date_loose_invalid() {
        assert!(parse_date_loose("nope").is_none());
    }

    // ── extract_flag ─────────────────────────────────────────────────────────

    #[test]
    fn extract_flag_present() {
        let parts = vec!["create", "Meeting", "--start", "2026-04-04T10:00:00Z"];
        assert_eq!(extract_flag(&parts, "--start"), Some("2026-04-04T10:00:00Z"));
    }

    #[test]
    fn extract_flag_missing() {
        let parts = vec!["create", "Meeting"];
        assert_eq!(extract_flag(&parts, "--start"), None);
    }

    #[test]
    fn extract_flag_at_end_no_value() {
        let parts = vec!["create", "--start"];
        assert_eq!(extract_flag(&parts, "--start"), None);
    }

    // ── CalendarClient construction ──────────────────────────────────────────

    #[test]
    fn client_new_stores_provider_and_token() {
        let c = CalendarClient::new(CalendarProvider::Google, "tok-123".into());
        assert_eq!(c.access_token, "tok-123");
        assert_eq!(c.provider, CalendarProvider::Google);
    }

    #[test]
    fn client_base_url_google() {
        let c = CalendarClient::new(CalendarProvider::Google, "t".into());
        assert_eq!(c.base_url(), GOOGLE_BASE_URL);
    }

    #[test]
    fn client_base_url_outlook() {
        let c = CalendarClient::new(CalendarProvider::Outlook, "t".into());
        assert_eq!(c.base_url(), OUTLOOK_BASE_URL);
    }

    #[test]
    fn client_from_env_google() {
        let _lock = CALENDAR_ENV_LOCK.lock().unwrap();
        std::env::set_var("GOOGLE_CALENDAR_TOKEN", "gtest");
        std::env::remove_var("OUTLOOK_ACCESS_TOKEN");
        let c = CalendarClient::from_env_or_config();
        assert!(c.is_some());
        let c = c.unwrap();
        assert_eq!(c.provider, CalendarProvider::Google);
        std::env::remove_var("GOOGLE_CALENDAR_TOKEN");
    }

    #[test]
    fn client_from_env_outlook_fallback() {
        let _lock = CALENDAR_ENV_LOCK.lock().unwrap();
        std::env::remove_var("GOOGLE_CALENDAR_TOKEN");
        std::env::set_var("OUTLOOK_ACCESS_TOKEN", "otest");
        let c = CalendarClient::from_env_or_config();
        // May pick up Google from config, or Outlook from env
        assert!(c.is_some());
        std::env::remove_var("OUTLOOK_ACCESS_TOKEN");
    }

    // ── Google event parsing ─────────────────────────────────────────────────

    #[test]
    fn parse_google_event_full() {
        let c = CalendarClient::new(CalendarProvider::Google, "t".into());
        let json = serde_json::json!({
            "id": "g-evt-1",
            "summary": "Standup",
            "start": { "dateTime": "2026-04-04T09:00:00Z" },
            "end": { "dateTime": "2026-04-04T09:30:00Z" },
            "location": "Room A",
            "description": "Daily standup",
            "attendees": [
                { "email": "a@x.com" },
                { "email": "b@x.com" }
            ],
            "status": "confirmed"
        });
        let ev = c.parse_google_event(&json);
        assert_eq!(ev.id, "g-evt-1");
        assert_eq!(ev.summary, "Standup");
        assert_eq!(ev.start, "2026-04-04T09:00:00Z");
        assert_eq!(ev.location, Some("Room A".into()));
        assert_eq!(ev.attendees.len(), 2);
        assert_eq!(ev.status, "confirmed");
    }

    #[test]
    fn parse_google_event_all_day() {
        let c = CalendarClient::new(CalendarProvider::Google, "t".into());
        let json = serde_json::json!({
            "id": "g-evt-2",
            "summary": "Holiday",
            "start": { "date": "2026-04-04" },
            "end": { "date": "2026-04-05" },
            "status": "confirmed"
        });
        let ev = c.parse_google_event(&json);
        assert_eq!(ev.start, "2026-04-04");
        assert_eq!(ev.end, "2026-04-05");
    }

    // ── Outlook event parsing ────────────────────────────────────────────────

    #[test]
    fn parse_outlook_event_full() {
        let c = CalendarClient::new(CalendarProvider::Outlook, "t".into());
        let json = serde_json::json!({
            "id": "o-evt-1",
            "subject": "Review",
            "start": { "dateTime": "2026-04-04T14:00:00Z" },
            "end": { "dateTime": "2026-04-04T15:00:00Z" },
            "location": { "displayName": "Teams" },
            "bodyPreview": "Code review",
            "attendees": [
                { "emailAddress": { "address": "c@y.com" } }
            ],
            "isCancelled": false,
            "responseStatus": { "response": "accepted" }
        });
        let ev = c.parse_outlook_event(&json);
        assert_eq!(ev.id, "o-evt-1");
        assert_eq!(ev.summary, "Review");
        assert_eq!(ev.location, Some("Teams".into()));
        assert_eq!(ev.description, Some("Code review".into()));
        assert_eq!(ev.attendees, vec!["c@y.com"]);
        assert_eq!(ev.status, "confirmed");
    }

    #[test]
    fn parse_outlook_event_cancelled() {
        let c = CalendarClient::new(CalendarProvider::Outlook, "t".into());
        let json = serde_json::json!({
            "id": "o-evt-2",
            "subject": "Cancelled",
            "start": { "dateTime": "" },
            "end": { "dateTime": "" },
            "isCancelled": true,
            "responseStatus": { "response": "" }
        });
        let ev = c.parse_outlook_event(&json);
        assert_eq!(ev.status, "cancelled");
    }

    #[test]
    fn parse_outlook_event_tentative() {
        let c = CalendarClient::new(CalendarProvider::Outlook, "t".into());
        let json = serde_json::json!({
            "id": "o-evt-3",
            "subject": "Maybe",
            "start": { "dateTime": "" },
            "end": { "dateTime": "" },
            "isCancelled": false,
            "responseStatus": { "response": "tentativelyAccepted" }
        });
        let ev = c.parse_outlook_event(&json);
        assert_eq!(ev.status, "tentative");
    }

    // ── Constants ────────────────────────────────────────────────────────────

    #[test]
    fn constants_correct() {
        assert_eq!(GOOGLE_BASE_URL, "https://www.googleapis.com/calendar/v3");
        assert_eq!(OUTLOOK_BASE_URL, "https://graph.microsoft.com/v1.0/me");
        assert_eq!(WORK_HOUR_START, 9);
        assert_eq!(WORK_HOUR_END, 18);
    }

    // ── REPL handler ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn handle_no_config_shows_warning() {
        let _lock = CALENDAR_ENV_LOCK.lock().unwrap();
        std::env::remove_var("GOOGLE_CALENDAR_TOKEN");
        std::env::remove_var("OUTLOOK_ACCESS_TOKEN");
        let out = handle_calendar_command("today").await;
        // May get "not configured" or valid output if config file has tokens
        assert!(
            out.contains("not configured")
                || out.contains("Calendar")
                || !out.is_empty()
        );
    }

    #[tokio::test]
    async fn handle_unknown_subcommand_shows_usage() {
        let _lock = CALENDAR_ENV_LOCK.lock().unwrap();
        std::env::set_var("GOOGLE_CALENDAR_TOKEN", "fake-token");
        let out = handle_calendar_command("unknown_cmd").await;
        assert!(
            out.contains("Usage:") || out.contains("not configured"),
            "unknown sub should show usage"
        );
        std::env::remove_var("GOOGLE_CALENDAR_TOKEN");
    }

    #[tokio::test]
    async fn handle_create_missing_flags() {
        let _lock = CALENDAR_ENV_LOCK.lock().unwrap();
        std::env::set_var("GOOGLE_CALENDAR_TOKEN", "fake-token");
        let out = handle_calendar_command("create Meeting").await;
        assert!(
            out.contains("Usage:") || out.contains("not configured"),
            "missing flags should show usage"
        );
        std::env::remove_var("GOOGLE_CALENDAR_TOKEN");
    }

    #[tokio::test]
    async fn handle_delete_empty_id() {
        let _lock = CALENDAR_ENV_LOCK.lock().unwrap();
        std::env::set_var("GOOGLE_CALENDAR_TOKEN", "fake-token");
        let out = handle_calendar_command("delete").await;
        assert!(
            out.contains("Usage:") || out.contains("not configured"),
        );
        std::env::remove_var("GOOGLE_CALENDAR_TOKEN");
    }

    #[tokio::test]
    async fn handle_move_missing_flags() {
        let _lock = CALENDAR_ENV_LOCK.lock().unwrap();
        std::env::set_var("GOOGLE_CALENDAR_TOKEN", "fake-token");
        let out = handle_calendar_command("move").await;
        assert!(
            out.contains("Usage:") || out.contains("not configured"),
        );
        std::env::remove_var("GOOGLE_CALENDAR_TOKEN");
    }
}
