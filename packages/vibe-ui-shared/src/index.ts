/**
 * UI shared by the VibeCody Tauri shells.
 *
 * Consumed as source, not as a built artifact: each host app aliases
 * `@vibe/shared` at its own `vite.config.ts` and compiles these files with its
 * own toolchain. That keeps the package build-step-free — there is no `dist`
 * to rebuild and no version to bump when something here changes.
 *
 * What belongs here: anything two shells would otherwise keep in sync by hand.
 * The settings screens and the reasoning parser both got copied between apps
 * before this existed, and the copies drifted — a fix to one left the other
 * rendering raw `<thinking>` markup.
 *
 * What does not belong here: anything that reaches for a host app's own state,
 * routing, or layout. These modules talk to the daemon through Tauri commands
 * that both shells register, and to nothing else.
 */
export { SettingsView } from "./settings/SettingsView";
export { AccountSection } from "./settings/AccountSection";
export { useTheme, type ThemeMode } from "./hooks/useTheme";
export {
  useProviderSettings,
  KEYED_PROVIDERS,
  LOCAL_PROVIDERS,
} from "./hooks/useProviderSettings";
export { applyThemeById, getPairedTheme, THEMES, type ThemeDef } from "./theme/themes";
export { splitThinking, isReasoningOnly, visibleAnswer, type SplitTurn } from "./lib/thinking";
