---
triggers: ["home assistant", "smart home", "lights", "thermostat", "climate", "smart lights", "home automation", "hass", "HA", "scene", "automation", "switch", "sensor"]
tools_allowed: ["read_file", "write_file", "bash"]
category: smart-home
---

# Home Assistant Integration

VibeCLI connects to your local Home Assistant instance via `/home` (alias `/ha`).

## Setup

```toml
# ~/.vibecli/config.toml
[home_assistant]
url = "http://homeassistant.local:8123"   # or IP address
token = "eyJ0..."                           # Long-Lived Access Token
```

Or set environment variables:
- `HA_URL` — base URL of your Home Assistant instance
- `HA_TOKEN` — long-lived access token (Settings → Profile → Long-Lived Access Tokens)

**Tailscale users**: Use the Tailscale IP (`100.x.x.x`) to reach Home Assistant remotely without port-forwarding.

## REPL Commands

| Command | Description |
|---------|-------------|
| `/ha status` | Summary of all entity states |
| `/ha lights` | List all light entities |
| `/ha on <entity>` | Turn entity on |
| `/ha off <entity>` | Turn entity off |
| `/ha toggle <entity>` | Toggle entity state |
| `/ha set <entity> <key> <value>` | Set attribute (brightness, color_temp, etc.) |
| `/ha scene <name>` | Activate a scene |
| `/ha climate <entity> <temp>` | Set thermostat target temperature |
| `/ha history <entity> [hours]` | Get state history (default 24h) |
| `/ha automation <name> trigger` | Manually trigger an automation |

## Entity Addressing

Entities can be referenced by:
- Full ID: `light.living_room_ceiling`
- Friendly name (fuzzy match): `living room ceiling`
- Domain wildcard: `all lights`, `all switches`

## Effective Usage Patterns

1. **Goodnight routine**: `/ha scene goodnight` activates your pre-configured goodnight scene — locks doors, dims lights, lowers thermostat.
2. **Conversational control**: Ask the AI "turn off all lights on the second floor" — it translates to individual `ha_off` calls for each entity in the `light.*_floor2` domain.
3. **Status dashboards**: `/ha status` returns a structured summary usable in reports — pipe it to a markdown file for a periodic home log.
4. **Energy monitoring**: Use `/ha history sensor.energy_consumption 168` to get a week of power data and ask the AI to identify high-usage patterns.
5. **Context-aware automation**: Combine calendar + home: "when my work day ends (cal shows last event done), run /ha scene relax".
6. **Temperature control**: `/ha climate thermostat.main 72` sets the temperature. The AI can also suggest energy-saving setpoints based on the forecast.
7. **Bulk operations**: Entity domains let you control groups: `/ha off all lights`, `/ha on all switches in basement`.
8. **History analysis**: Feed `/ha history` output to the AI to detect anomalies — a door sensor that opened 40 times overnight may indicate a faulty sensor.
9. **SSL/HTTPS**: If your Home Assistant uses HTTPS (via Let's Encrypt or Nabu Casa), set `url = "https://..."` — the client validates the certificate by default. Set `ha_insecure = true` in config for self-signed certs on a trusted local network.
10. **Remote access via Nabu Casa**: Set the Nabu Casa remote URL in `url` and your token works identically from anywhere without VPN.
