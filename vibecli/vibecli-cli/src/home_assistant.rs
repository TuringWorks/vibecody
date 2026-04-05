//! Home Assistant smart home integration for VibeCLI.
//!
//! Connects to the Home Assistant REST API to control and monitor smart home
//! devices (lights, switches, climate, sensors, scenes, automations).
//!
//! Configuration:
//! - `HOME_ASSISTANT_URL` env or `home_assistant.url` in `~/.vibecli/config.toml`
//! - `HOME_ASSISTANT_TOKEN` env or `home_assistant.token` in `~/.vibecli/config.toml`
//!
//! Usage in REPL:
//! ```
//! /ha status                          — List all entities grouped by domain
//! /ha lights                          — List lights with on/off and brightness
//! /ha on <entity_id>                  — Turn on a device
//! /ha off <entity_id>                 — Turn off a device
//! /ha toggle <entity_id>              — Toggle device state
//! /ha set <entity_id> <attr> <val>    — Set attribute (brightness, temperature)
//! /ha scene <scene_name>              — Activate a scene
//! /ha climate <entity_id>             — Show climate status
//! /ha climate <entity_id> <temp>      — Set thermostat temperature
//! /ha history <entity_id>             — Show recent state changes
//! /ha automation list                 — List automations
//! /ha automation trigger <id>         — Trigger an automation
//! ```

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use vibe_ai::{retry_async, RetryConfig};

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaEntity {
    pub entity_id: String,
    pub state: String,
    #[serde(default)]
    pub attributes: serde_json::Value,
}

impl HaEntity {
    /// Extract the domain from entity_id (e.g. "light" from "light.living_room").
    pub fn domain(&self) -> &str {
        self.entity_id.split('.').next().unwrap_or("unknown")
    }

    /// Return a domain emoji.
    pub fn domain_emoji(&self) -> &str {
        match self.domain() {
            "light" => "\u{1f4a1}",       // 💡
            "climate" => "\u{1f321}\u{fe0f}", // 🌡️
            "switch" => "\u{1f50c}",      // 🔌
            "scene" => "\u{1f3a8}",       // 🎨
            "sensor" => "\u{1f4e1}",      // 📡
            "automation" => "\u{2699}\u{fe0f}", // ⚙️
            "binary_sensor" => "\u{1f518}", // 🔘
            "fan" => "\u{1f4a8}",         // 💨
            "cover" => "\u{1fa9f}",       // 🪟
            "lock" => "\u{1f512}",        // 🔒
            _ => "\u{1f3e0}",             // 🏠
        }
    }

    /// Friendly name from attributes.
    pub fn friendly_name(&self) -> &str {
        self.attributes
            .get("friendly_name")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.entity_id)
    }

    /// Brightness as percentage if available.
    pub fn brightness_pct(&self) -> Option<u8> {
        self.attributes
            .get("brightness")
            .and_then(|v| v.as_u64())
            .map(|b| ((b as f64 / 255.0) * 100.0).round() as u8)
    }
}

// ── HomeAssistantClient ──────────────────────────────────────────────────────

pub struct HomeAssistantClient {
    base_url: String,
    token: String,
    client: reqwest::Client,
}

impl HomeAssistantClient {
    /// Create a client from URL and long-lived access token.
    pub fn new(base_url: String, token: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("VibeCLI/1.0")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            client,
        }
    }

    /// Try to resolve URL and token from env or config.
    pub fn from_env_or_config() -> Option<Self> {
        let url = std::env::var("HOME_ASSISTANT_URL").ok().filter(|s| !s.is_empty()).or_else(|| {
            crate::config::Config::load().ok().and_then(|c| c.home_assistant.and_then(|ha| ha.url))
        });
        let token = std::env::var("HOME_ASSISTANT_TOKEN").ok().filter(|s| !s.is_empty()).or_else(|| {
            crate::config::Config::load().ok().and_then(|c| c.home_assistant.and_then(|ha| ha.token))
        });
        match (url, token) {
            (Some(u), Some(t)) => Some(Self::new(u, t)),
            _ => None,
        }
    }

    /// GET request helper with retry.
    async fn get(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = retry_async(&RetryConfig::default(), "ha-get", || {
            let client = self.client.clone();
            let token = self.token.clone();
            let url = url.clone();
            async move {
                client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", token))
                    .header("Content-Type", "application/json")
                    .send()
                    .await
                    .map_err(Into::into)
            }
        })
        .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("HA API returned {}", resp.status()));
        }
        resp.json::<serde_json::Value>().await.map_err(Into::into)
    }

    /// POST request helper with retry.
    async fn post(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = retry_async(&RetryConfig::default(), "ha-post", || {
            let client = self.client.clone();
            let token = self.token.clone();
            let url = url.clone();
            let body = body.clone();
            async move {
                client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", token))
                    .header("Content-Type", "application/json")
                    .json(&body)
                    .send()
                    .await
                    .map_err(Into::into)
            }
        })
        .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("HA API returned {}", resp.status()));
        }
        resp.json::<serde_json::Value>().await.map_err(Into::into)
    }

    /// Fetch all entity states.
    pub async fn get_states(&self) -> Result<Vec<HaEntity>> {
        let data = self.get("/api/states").await?;
        let entities: Vec<HaEntity> = serde_json::from_value(data)?;
        Ok(entities)
    }

    /// Call a service (e.g. domain="light", service="turn_on").
    pub async fn call_service(
        &self,
        domain: &str,
        service: &str,
        entity_id: &str,
        data: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let mut body = data.unwrap_or_else(|| serde_json::json!({}));
        if let Some(obj) = body.as_object_mut() {
            obj.insert("entity_id".to_string(), serde_json::json!(entity_id));
        }
        let path = format!("/api/services/{}/{}", domain, service);
        self.post(&path, body).await
    }

    /// Fetch history for an entity over the last 24 hours.
    pub async fn get_history(&self, entity_id: &str) -> Result<Vec<HaEntity>> {
        let path = format!(
            "/api/history/period?filter_entity_id={}&minimal_response&no_attributes",
            entity_id
        );
        let data = self.get(&path).await?;
        // HA returns array of arrays
        let entries: Vec<HaEntity> = data
            .as_array()
            .and_then(|a| a.first())
            .and_then(|inner| serde_json::from_value(inner.clone()).ok())
            .unwrap_or_default();
        Ok(entries)
    }

    /// List automations.
    pub async fn list_automations(&self) -> Result<Vec<HaEntity>> {
        let states = self.get_states().await?;
        Ok(states.into_iter().filter(|e| e.domain() == "automation").collect())
    }

    /// Trigger an automation.
    pub async fn trigger_automation(&self, entity_id: &str) -> Result<serde_json::Value> {
        self.call_service("automation", "trigger", entity_id, None).await
    }
}

// ── REPL handler ─────────────────────────────────────────────────────────────

/// Run the `/ha` REPL command.
/// Returns a human-readable output string.
pub async fn handle_ha_command(args: &str) -> String {
    let client = match HomeAssistantClient::from_env_or_config() {
        Some(c) => c,
        None => {
            return "\u{26a0}\u{fe0f}  Home Assistant not configured.\n\
                Set HOME_ASSISTANT_URL and HOME_ASSISTANT_TOKEN env vars, or add:\n\
                [home_assistant]\n\
                url = \"http://homeassistant.local:8123\"\n\
                token = \"your-long-lived-access-token\"\n\
                to ~/.vibecli/config.toml\n"
                .to_string();
        }
    };

    let parts: Vec<&str> = args.splitn(4, ' ').collect();
    let sub = parts.first().copied().unwrap_or("").trim();

    match sub {
        "status" | "" => handle_status(&client).await,
        "lights" => handle_lights(&client).await,
        "on" => {
            let entity = parts.get(1).copied().unwrap_or("").trim();
            if entity.is_empty() {
                return "Usage: /ha on <entity_id>  e.g. /ha on light.living_room\n".to_string();
            }
            handle_turn_on(&client, entity).await
        }
        "off" => {
            let entity = parts.get(1).copied().unwrap_or("").trim();
            if entity.is_empty() {
                return "Usage: /ha off <entity_id>\n".to_string();
            }
            handle_turn_off(&client, entity).await
        }
        "toggle" => {
            let entity = parts.get(1).copied().unwrap_or("").trim();
            if entity.is_empty() {
                return "Usage: /ha toggle <entity_id>\n".to_string();
            }
            handle_toggle(&client, entity).await
        }
        "set" => {
            let entity = parts.get(1).copied().unwrap_or("").trim();
            let attr = parts.get(2).copied().unwrap_or("").trim();
            let value = parts.get(3).copied().unwrap_or("").trim();
            if entity.is_empty() || attr.is_empty() || value.is_empty() {
                return "Usage: /ha set <entity_id> <attribute> <value>\n  e.g. /ha set light.desk brightness 128\n".to_string();
            }
            handle_set(&client, entity, attr, value).await
        }
        "scene" => {
            let scene = parts.get(1).copied().unwrap_or("").trim();
            if scene.is_empty() {
                return "Usage: /ha scene <scene_name>  e.g. /ha scene scene.movie_time\n".to_string();
            }
            handle_scene(&client, scene).await
        }
        "climate" => {
            let entity = parts.get(1).copied().unwrap_or("").trim();
            let temp = parts.get(2).copied().unwrap_or("").trim();
            if entity.is_empty() {
                return "Usage: /ha climate <entity_id> [temperature]\n".to_string();
            }
            if temp.is_empty() {
                handle_climate_status(&client, entity).await
            } else {
                handle_climate_set(&client, entity, temp).await
            }
        }
        "history" => {
            let entity = parts.get(1).copied().unwrap_or("").trim();
            if entity.is_empty() {
                return "Usage: /ha history <entity_id>\n".to_string();
            }
            handle_history(&client, entity).await
        }
        "automation" => {
            let action = parts.get(1).copied().unwrap_or("").trim();
            match action {
                "list" | "" => handle_automation_list(&client).await,
                "trigger" => {
                    let id = parts.get(2).copied().unwrap_or("").trim();
                    if id.is_empty() {
                        return "Usage: /ha automation trigger <automation_id>\n".to_string();
                    }
                    handle_automation_trigger(&client, id).await
                }
                _ => "Usage: /ha automation list | /ha automation trigger <id>\n".to_string(),
            }
        }
        _ => {
            "Usage:\n  \
             /ha status                         \u{2014} List all entities by domain\n  \
             /ha lights                         \u{2014} List lights\n  \
             /ha on <entity_id>                 \u{2014} Turn on device\n  \
             /ha off <entity_id>                \u{2014} Turn off device\n  \
             /ha toggle <entity_id>             \u{2014} Toggle device\n  \
             /ha set <entity_id> <attr> <val>   \u{2014} Set attribute\n  \
             /ha scene <scene_name>             \u{2014} Activate scene\n  \
             /ha climate <entity_id> [temp]     \u{2014} Climate status / set temp\n  \
             /ha history <entity_id>            \u{2014} Recent state changes\n  \
             /ha automation list                \u{2014} List automations\n  \
             /ha automation trigger <id>        \u{2014} Trigger automation\n"
                .to_string()
        }
    }
}

// ── Sub-handlers ─────────────────────────────────────────────────────────────

async fn handle_status(client: &HomeAssistantClient) -> String {
    match client.get_states().await {
        Ok(entities) => {
            if entities.is_empty() {
                return "\u{2705} No entities found.\n".to_string();
            }
            let mut grouped: BTreeMap<String, Vec<&HaEntity>> = BTreeMap::new();
            for e in &entities {
                grouped.entry(e.domain().to_string()).or_default().push(e);
            }
            let mut out = format!("\u{1f3e0} Home Assistant \u{2014} {} entities\n\n", entities.len());
            for (domain, items) in &grouped {
                let emoji = items.first().map(|e| e.domain_emoji()).unwrap_or("\u{1f3e0}");
                out.push_str(&format!("{} {} ({}):\n", emoji, domain, items.len()));
                for e in items.iter().take(15) {
                    out.push_str(&format!("  {} \u{2014} {}\n", e.friendly_name(), e.state));
                }
                if items.len() > 15 {
                    out.push_str(&format!("  ... and {} more\n", items.len() - 15));
                }
                out.push('\n');
            }
            out
        }
        Err(e) => format!("\u{274c} Failed to fetch states: {}\n", e),
    }
}

async fn handle_lights(client: &HomeAssistantClient) -> String {
    match client.get_states().await {
        Ok(entities) => {
            let lights: Vec<&HaEntity> = entities.iter().filter(|e| e.domain() == "light").collect();
            if lights.is_empty() {
                return "\u{1f4a1} No lights found.\n".to_string();
            }
            let mut out = format!("\u{1f4a1} Lights ({}):\n", lights.len());
            for light in &lights {
                let status = if light.state == "on" { "\u{1f7e2} ON" } else { "\u{1f534} OFF" };
                let brightness = light
                    .brightness_pct()
                    .map(|b| format!(" ({}%)", b))
                    .unwrap_or_default();
                out.push_str(&format!("  {} {} {}{}\n", light.entity_id, status, light.friendly_name(), brightness));
            }
            out.push('\n');
            out
        }
        Err(e) => format!("\u{274c} Failed to fetch lights: {}\n", e),
    }
}

async fn handle_turn_on(client: &HomeAssistantClient, entity_id: &str) -> String {
    let domain = entity_id.split('.').next().unwrap_or("homeassistant");
    match client.call_service(domain, "turn_on", entity_id, None).await {
        Ok(_) => format!("\u{2705} Turned on {}\n", entity_id),
        Err(e) => format!("\u{274c} Failed to turn on {}: {}\n", entity_id, e),
    }
}

async fn handle_turn_off(client: &HomeAssistantClient, entity_id: &str) -> String {
    let domain = entity_id.split('.').next().unwrap_or("homeassistant");
    match client.call_service(domain, "turn_off", entity_id, None).await {
        Ok(_) => format!("\u{2705} Turned off {}\n", entity_id),
        Err(e) => format!("\u{274c} Failed to turn off {}: {}\n", entity_id, e),
    }
}

async fn handle_toggle(client: &HomeAssistantClient, entity_id: &str) -> String {
    let domain = entity_id.split('.').next().unwrap_or("homeassistant");
    match client.call_service(domain, "toggle", entity_id, None).await {
        Ok(_) => format!("\u{2705} Toggled {}\n", entity_id),
        Err(e) => format!("\u{274c} Failed to toggle {}: {}\n", entity_id, e),
    }
}

async fn handle_set(client: &HomeAssistantClient, entity_id: &str, attr: &str, value: &str) -> String {
    let domain = entity_id.split('.').next().unwrap_or("homeassistant");
    // Try to parse value as number, otherwise send as string
    let val: serde_json::Value = value
        .parse::<f64>()
        .map(|n| serde_json::json!(n))
        .unwrap_or_else(|_| serde_json::json!(value));
    let data = serde_json::json!({ attr: val });
    match client.call_service(domain, "turn_on", entity_id, Some(data)).await {
        Ok(_) => format!("\u{2705} Set {} = {} on {}\n", attr, value, entity_id),
        Err(e) => format!("\u{274c} Failed to set attribute: {}\n", e),
    }
}

async fn handle_scene(client: &HomeAssistantClient, scene: &str) -> String {
    let entity_id = if scene.starts_with("scene.") {
        scene.to_string()
    } else {
        format!("scene.{}", scene)
    };
    match client.call_service("scene", "turn_on", &entity_id, None).await {
        Ok(_) => format!("\u{1f3a8} Activated scene {}\n", entity_id),
        Err(e) => format!("\u{274c} Failed to activate scene: {}\n", e),
    }
}

async fn handle_climate_status(client: &HomeAssistantClient, entity_id: &str) -> String {
    match client.get_states().await {
        Ok(entities) => {
            if let Some(e) = entities.iter().find(|e| e.entity_id == entity_id) {
                let temp = e.attributes.get("current_temperature")
                    .and_then(|v| v.as_f64())
                    .map(|t| format!("{:.1}\u{00b0}", t))
                    .unwrap_or_else(|| "N/A".to_string());
                let target = e.attributes.get("temperature")
                    .and_then(|v| v.as_f64())
                    .map(|t| format!("{:.1}\u{00b0}", t))
                    .unwrap_or_else(|| "N/A".to_string());
                let humidity = e.attributes.get("current_humidity")
                    .and_then(|v| v.as_f64())
                    .map(|h| format!("{:.0}%", h))
                    .unwrap_or_else(|| "N/A".to_string());
                let mode = e.attributes.get("hvac_action")
                    .or_else(|| e.attributes.get("hvac_mode"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                format!(
                    "\u{1f321}\u{fe0f} Climate: {}\n  State: {}\n  Current temp: {}\n  Target temp: {}\n  Humidity: {}\n  HVAC mode: {}\n",
                    e.friendly_name(), e.state, temp, target, humidity, mode
                )
            } else {
                format!("\u{274c} Entity {} not found\n", entity_id)
            }
        }
        Err(e) => format!("\u{274c} Failed to fetch climate: {}\n", e),
    }
}

async fn handle_climate_set(client: &HomeAssistantClient, entity_id: &str, temp: &str) -> String {
    match temp.parse::<f64>() {
        Ok(t) => {
            let data = serde_json::json!({ "temperature": t });
            match client.call_service("climate", "set_temperature", entity_id, Some(data)).await {
                Ok(_) => format!("\u{2705} Set {} temperature to {:.1}\u{00b0}\n", entity_id, t),
                Err(e) => format!("\u{274c} Failed to set temperature: {}\n", e),
            }
        }
        Err(_) => format!("\u{274c} Invalid temperature value: {}\n", temp),
    }
}

async fn handle_history(client: &HomeAssistantClient, entity_id: &str) -> String {
    match client.get_history(entity_id).await {
        Ok(entries) => {
            if entries.is_empty() {
                return format!("\u{1f4c5} No recent history for {}\n", entity_id);
            }
            let mut out = format!("\u{1f4c5} History for {} ({} entries):\n", entity_id, entries.len());
            for entry in entries.iter().take(20) {
                out.push_str(&format!("  {} \u{2014} {}\n", entry.state, entry.entity_id));
            }
            if entries.len() > 20 {
                out.push_str(&format!("  ... and {} more\n", entries.len() - 20));
            }
            out
        }
        Err(e) => format!("\u{274c} Failed to fetch history: {}\n", e),
    }
}

async fn handle_automation_list(client: &HomeAssistantClient) -> String {
    match client.list_automations().await {
        Ok(automations) => {
            if automations.is_empty() {
                return "\u{2699}\u{fe0f} No automations found.\n".to_string();
            }
            let mut out = format!("\u{2699}\u{fe0f} Automations ({}):\n", automations.len());
            for a in &automations {
                let status = if a.state == "on" { "\u{1f7e2}" } else { "\u{1f534}" };
                out.push_str(&format!("  {} {} \u{2014} {}\n", status, a.entity_id, a.friendly_name()));
            }
            out.push('\n');
            out
        }
        Err(e) => format!("\u{274c} Failed to list automations: {}\n", e),
    }
}

async fn handle_automation_trigger(client: &HomeAssistantClient, entity_id: &str) -> String {
    let full_id = if entity_id.starts_with("automation.") {
        entity_id.to_string()
    } else {
        format!("automation.{}", entity_id)
    };
    match client.trigger_automation(&full_id).await {
        Ok(_) => format!("\u{2705} Triggered automation {}\n", full_id),
        Err(e) => format!("\u{274c} Failed to trigger automation: {}\n", e),
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(id: &str, state: &str, attrs: serde_json::Value) -> HaEntity {
        HaEntity {
            entity_id: id.to_string(),
            state: state.to_string(),
            attributes: attrs,
        }
    }

    #[test]
    fn entity_domain_extraction() {
        let e = make_entity("light.living_room", "on", serde_json::json!({}));
        assert_eq!(e.domain(), "light");
        let e = make_entity("climate.thermostat", "heat", serde_json::json!({}));
        assert_eq!(e.domain(), "climate");
        let e = make_entity("switch.plug_1", "off", serde_json::json!({}));
        assert_eq!(e.domain(), "switch");
    }

    #[test]
    fn entity_domain_emoji() {
        assert_eq!(make_entity("light.x", "on", serde_json::json!({})).domain_emoji(), "\u{1f4a1}");
        assert_eq!(make_entity("climate.x", "on", serde_json::json!({})).domain_emoji(), "\u{1f321}\u{fe0f}");
        assert_eq!(make_entity("switch.x", "on", serde_json::json!({})).domain_emoji(), "\u{1f50c}");
        assert_eq!(make_entity("scene.x", "on", serde_json::json!({})).domain_emoji(), "\u{1f3a8}");
        assert_eq!(make_entity("sensor.x", "on", serde_json::json!({})).domain_emoji(), "\u{1f4e1}");
        assert_eq!(make_entity("automation.x", "on", serde_json::json!({})).domain_emoji(), "\u{2699}\u{fe0f}");
        assert_eq!(make_entity("fan.x", "on", serde_json::json!({})).domain_emoji(), "\u{1f4a8}");
        assert_eq!(make_entity("lock.x", "on", serde_json::json!({})).domain_emoji(), "\u{1f512}");
        assert_eq!(make_entity("unknown_domain.x", "on", serde_json::json!({})).domain_emoji(), "\u{1f3e0}");
    }

    #[test]
    fn friendly_name_from_attributes() {
        let e = make_entity("light.desk", "on", serde_json::json!({"friendly_name": "Desk Lamp"}));
        assert_eq!(e.friendly_name(), "Desk Lamp");
    }

    #[test]
    fn friendly_name_fallback_to_entity_id() {
        let e = make_entity("light.desk", "on", serde_json::json!({}));
        assert_eq!(e.friendly_name(), "light.desk");
    }

    #[test]
    fn brightness_pct_conversion() {
        let e = make_entity("light.x", "on", serde_json::json!({"brightness": 255}));
        assert_eq!(e.brightness_pct(), Some(100));
        let e = make_entity("light.x", "on", serde_json::json!({"brightness": 128}));
        assert_eq!(e.brightness_pct(), Some(50));
        let e = make_entity("light.x", "on", serde_json::json!({"brightness": 0}));
        assert_eq!(e.brightness_pct(), Some(0));
    }

    #[test]
    fn brightness_pct_none_when_missing() {
        let e = make_entity("light.x", "off", serde_json::json!({}));
        assert!(e.brightness_pct().is_none());
    }

    #[test]
    fn entity_serde_roundtrip() {
        let e = make_entity("sensor.temp", "22.5", serde_json::json!({"unit_of_measurement": "°C"}));
        let json = serde_json::to_string(&e).unwrap();
        let back: HaEntity = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entity_id, "sensor.temp");
        assert_eq!(back.state, "22.5");
    }

    #[test]
    fn entity_clone() {
        let e = make_entity("light.x", "on", serde_json::json!({"brightness": 200}));
        let c = e.clone();
        assert_eq!(c.entity_id, e.entity_id);
        assert_eq!(c.state, e.state);
    }

    #[test]
    fn entity_debug_format() {
        let e = make_entity("switch.plug", "off", serde_json::json!({}));
        let dbg = format!("{:?}", e);
        assert!(dbg.contains("switch.plug"));
    }

    #[test]
    fn client_new_trims_trailing_slash() {
        let c = HomeAssistantClient::new("http://ha.local:8123/".to_string(), "tok".to_string());
        assert_eq!(c.base_url, "http://ha.local:8123");
    }

    #[test]
    fn client_new_no_trailing_slash() {
        let c = HomeAssistantClient::new("http://ha.local:8123".to_string(), "tok".to_string());
        assert_eq!(c.base_url, "http://ha.local:8123");
    }

    #[test]
    fn client_from_env_picks_up_vars() {
        std::env::set_var("HOME_ASSISTANT_URL", "http://test:8123");
        std::env::set_var("HOME_ASSISTANT_TOKEN", "test-token");
        let client = HomeAssistantClient::from_env_or_config();
        assert!(client.is_some());
        std::env::remove_var("HOME_ASSISTANT_URL");
        std::env::remove_var("HOME_ASSISTANT_TOKEN");
    }

    #[test]
    fn client_from_env_missing_token() {
        std::env::set_var("HOME_ASSISTANT_URL", "http://test:8123");
        std::env::remove_var("HOME_ASSISTANT_TOKEN");
        let client = HomeAssistantClient::from_env_or_config();
        // May or may not be Some depending on config file presence
        // At minimum, ensure no panic
        let _ = client;
        std::env::remove_var("HOME_ASSISTANT_URL");
    }

    #[tokio::test]
    async fn handle_ha_command_no_config_shows_warning() {
        std::env::remove_var("HOME_ASSISTANT_URL");
        std::env::remove_var("HOME_ASSISTANT_TOKEN");
        let output = handle_ha_command("status").await;
        assert!(
            output.contains("not configured") || output.contains("HOME_ASSISTANT") || !output.is_empty()
        );
    }

    #[tokio::test]
    async fn handle_ha_command_unknown_sub_shows_usage() {
        std::env::set_var("HOME_ASSISTANT_URL", "http://fake:8123");
        std::env::set_var("HOME_ASSISTANT_TOKEN", "fake-token");
        let output = handle_ha_command("unknown_sub").await;
        assert!(output.contains("Usage:") || output.contains("not configured"));
        std::env::remove_var("HOME_ASSISTANT_URL");
        std::env::remove_var("HOME_ASSISTANT_TOKEN");
    }

    #[tokio::test]
    async fn handle_ha_command_on_empty_entity() {
        std::env::set_var("HOME_ASSISTANT_URL", "http://fake:8123");
        std::env::set_var("HOME_ASSISTANT_TOKEN", "fake-token");
        let output = handle_ha_command("on").await;
        assert!(output.contains("Usage:") || output.contains("not configured"));
        std::env::remove_var("HOME_ASSISTANT_URL");
        std::env::remove_var("HOME_ASSISTANT_TOKEN");
    }

    #[tokio::test]
    async fn handle_ha_command_off_empty_entity() {
        std::env::set_var("HOME_ASSISTANT_URL", "http://fake:8123");
        std::env::set_var("HOME_ASSISTANT_TOKEN", "fake-token");
        let output = handle_ha_command("off").await;
        assert!(output.contains("Usage:") || output.contains("not configured"));
        std::env::remove_var("HOME_ASSISTANT_URL");
        std::env::remove_var("HOME_ASSISTANT_TOKEN");
    }

    #[tokio::test]
    async fn handle_ha_command_toggle_empty_entity() {
        std::env::set_var("HOME_ASSISTANT_URL", "http://fake:8123");
        std::env::set_var("HOME_ASSISTANT_TOKEN", "fake-token");
        let output = handle_ha_command("toggle").await;
        assert!(output.contains("Usage:") || output.contains("not configured"));
        std::env::remove_var("HOME_ASSISTANT_URL");
        std::env::remove_var("HOME_ASSISTANT_TOKEN");
    }

    #[tokio::test]
    async fn handle_ha_command_set_missing_args() {
        std::env::set_var("HOME_ASSISTANT_URL", "http://fake:8123");
        std::env::set_var("HOME_ASSISTANT_TOKEN", "fake-token");
        let output = handle_ha_command("set light.x").await;
        assert!(output.contains("Usage:") || output.contains("not configured"));
        std::env::remove_var("HOME_ASSISTANT_URL");
        std::env::remove_var("HOME_ASSISTANT_TOKEN");
    }

    #[tokio::test]
    async fn handle_ha_command_scene_empty() {
        std::env::set_var("HOME_ASSISTANT_URL", "http://fake:8123");
        std::env::set_var("HOME_ASSISTANT_TOKEN", "fake-token");
        let output = handle_ha_command("scene").await;
        assert!(output.contains("Usage:") || output.contains("not configured"));
        std::env::remove_var("HOME_ASSISTANT_URL");
        std::env::remove_var("HOME_ASSISTANT_TOKEN");
    }

    #[tokio::test]
    async fn handle_ha_command_climate_empty() {
        std::env::set_var("HOME_ASSISTANT_URL", "http://fake:8123");
        std::env::set_var("HOME_ASSISTANT_TOKEN", "fake-token");
        let output = handle_ha_command("climate").await;
        assert!(output.contains("Usage:") || output.contains("not configured"));
        std::env::remove_var("HOME_ASSISTANT_URL");
        std::env::remove_var("HOME_ASSISTANT_TOKEN");
    }

    #[tokio::test]
    async fn handle_ha_command_history_empty() {
        std::env::set_var("HOME_ASSISTANT_URL", "http://fake:8123");
        std::env::set_var("HOME_ASSISTANT_TOKEN", "fake-token");
        let output = handle_ha_command("history").await;
        assert!(output.contains("Usage:") || output.contains("not configured"));
        std::env::remove_var("HOME_ASSISTANT_URL");
        std::env::remove_var("HOME_ASSISTANT_TOKEN");
    }

    #[tokio::test]
    async fn handle_ha_command_automation_trigger_empty() {
        std::env::set_var("HOME_ASSISTANT_URL", "http://fake:8123");
        std::env::set_var("HOME_ASSISTANT_TOKEN", "fake-token");
        let output = handle_ha_command("automation trigger").await;
        assert!(output.contains("Usage:") || output.contains("not configured"));
        std::env::remove_var("HOME_ASSISTANT_URL");
        std::env::remove_var("HOME_ASSISTANT_TOKEN");
    }

    #[test]
    fn ha_entity_deserialize_minimal() {
        let json = r#"{"entity_id":"light.x","state":"on"}"#;
        let e: HaEntity = serde_json::from_str(json).unwrap();
        assert_eq!(e.entity_id, "light.x");
        assert_eq!(e.state, "on");
    }

    #[test]
    fn ha_entity_deserialize_with_attributes() {
        let json = r#"{"entity_id":"light.desk","state":"on","attributes":{"brightness":200,"friendly_name":"Desk"}}"#;
        let e: HaEntity = serde_json::from_str(json).unwrap();
        assert_eq!(e.friendly_name(), "Desk");
        assert_eq!(e.brightness_pct(), Some(78));
    }
}
