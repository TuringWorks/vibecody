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
export { Markdown, CopyButton } from "./markdown/Markdown";
export { SettingsView } from "./settings/SettingsView";
export { AccountSection } from "./settings/AccountSection";
export { useTheme, type ThemeMode } from "./hooks/useTheme";
export { useClickAway } from "./hooks/useClickAway";
export {
  useProviderSettings,
  KEYED_PROVIDERS,
  LOCAL_PROVIDERS,
} from "./hooks/useProviderSettings";
export { applyThemeById, getPairedTheme, THEMES, type ThemeDef } from "./theme/themes";
export { splitThinking, isReasoningOnly, visibleAnswer, type SplitTurn } from "./lib/thinking";
export {
  useVoiceInput,
  type VoiceState,
  type UseVoiceInput,
  type UseVoiceInputOptions,
} from "./voice/useVoiceInput";
export { VoiceButton } from "./voice/VoiceButton";
export { ComposerDrawer } from "./composer/ComposerDrawer";
export type {
  ComposerIcon,
  ComposerItem,
  ComposerAction,
  ComposerSwitch,
  ComposerGroup,
  ComposerDrawerProps,
} from "./composer/ComposerDrawer";
export { VoiceTranscript } from "./voice/VoiceTranscript";
export type { VoiceTranscriptProps } from "./voice/VoiceTranscript";
export { useVoiceDuplex, duplexSupported } from "./voice/useVoiceDuplex";
export type {
  DuplexState,
  DuplexTurn,
  DuplexLatency,
  UseVoiceDuplex,
  UseVoiceDuplexOptions,
} from "./voice/useVoiceDuplex";
export { DuplexVoiceButton } from "./voice/DuplexVoiceButton";
export { useVoiceDuplexPreference } from "./voice/useVoiceDuplexPreference";
export type { VoiceDuplexPreference } from "./voice/useVoiceDuplexPreference";
export type { DuplexVoiceButtonProps } from "./voice/DuplexVoiceButton";
export {
  tauriTranscriber,
  daemonTranscriber,
  blobToBase64,
  TranscriptionError,
  type Transcriber,
} from "./voice/transcribers";
export {
  getSpeechRecognition,
  describeSpeechError,
  type SpeechRecognitionLike,
} from "./voice/speech";
