import { useMemo } from "react";
import { SkillsView } from "@vibe/shared/skills/SkillsView";
import { skilllensCatalog } from "@vibe/shared/skills/catalog";
import { openPanelTab } from "../lib/panelDeepLink";
import "@vibe/shared/skills/skills.css";

/**
 * The daemon's skill catalogue, with the picked skills written into the chat
 * composer.
 *
 * The catalogue route has been served all along and VibeCoder already
 * registered the two Tauri commands for it, but the only thing rendering them
 * was SkillForge, which scores skills rather than letting you use one. Reading
 * a skill and putting it to work were two different apps.
 *
 * Selection is deliberately just composer text: `AgentRequest` has no `skills`
 * field, so a skill reaches a run as prompt text or not at all. Anything that
 * looked like "arm this skill for the next run" would be describing a daemon
 * feature that does not exist.
 */
export function SkillsPanel() {
  const catalog = useMemo(() => skilllensCatalog(), []);

  return (
    <SkillsView
      catalog={catalog}
      hint="Picked skills are written into the chat composer"
      onUse={(text) => {
        // Inject first: the chat tab is already mounted (PanelHost keeps it
        // alive from launch), and its listener has to exist when the event
        // fires — a switch-then-inject order would race the render.
        window.dispatchEvent(new CustomEvent("vibecoder:inject-context", { detail: text }));
        openPanelTab("chat");
      }}
    />
  );
}
