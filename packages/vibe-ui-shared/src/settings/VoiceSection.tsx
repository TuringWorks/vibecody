import { useVoiceSettings } from "../voice/useVoiceSettings";

/**
 * The Voice settings pane — engine, language, voice — backed by the daemon's
 * `/voice/settings`.
 *
 * Its own file, and exported, because VibeCoder does not use the shared
 * `SettingsView`: it has an older sidebar of its own, so a section defined
 * privately inside `SettingsView` reached two of the three shells and the
 * third silently had no Voice tab at all.
 */

/** Language codes the recogniser reports, named for a human. */
const LANG_NAMES: Record<string, string> = {
  auto: "Detect per turn", en: "English", "en-gb": "English (UK)", es: "Spanish",
  fr: "French", hi: "Hindi", it: "Italian", ja: "Japanese", pt: "Portuguese",
  zh: "Chinese", de: "German", nl: "Dutch", ru: "Russian", ar: "Arabic",
  ko: "Korean", ta: "Tamil", te: "Telugu",
};
const langName = (c: string) => LANG_NAMES[c] ?? LANG_NAMES[c.split("-")[0]] ?? c;

export function VoiceSection() {
  const { settings, error, saving, update } = useVoiceSettings();

  // Only a *load* failure leaves nothing to show. A failed save also sets
  // `error`, and returning early on it used to replace the whole pane with one
  // line of error text — the settings vanished, and a save problem read as a
  // load problem. Keep the controls, put the message above them.
  if (!settings) {
    return (
      <div className="vx-set-section">
        <p className="vx-set-hint">{error ?? "Loading…"}</p>
      </div>
    );
  }

  // Voices are grouped by language because a voice belongs to one: an English
  // voice reading Hindi is not accented, it is the wrong sounds. A flat list of
  // 28 names hides that.
  const byLang = settings.voices.reduce<Record<string, typeof settings.voices>>((acc, v) => {
    (acc[v.lang] ||= []).push(v);
    return acc;
  }, {});

  // Named rows only: the catalog's first entry is the active provider itself,
  // which has no addressable model name and would select nothing.
  const modelsByProvider = (settings.models ?? []).reduce<Record<string, string[]>>((acc, m) => {
    if (m.name) (acc[m.provider] ||= []).push(m.name);
    return acc;
  }, {});

  return (
    <div className="vx-set-section">
      {error && (
        <p className="vx-set-error" role="status">
          {error}
        </p>
      )}

      <h4 className="vx-set-h">Speech engine</h4>
      <p className="vx-set-hint">Which engine speaks the assistant's replies.</p>
      <div className="vx-voice-engines">
        {settings.engines.map((e) => (
          <button
            key={e.id}
            className={`vx-voice-engine${settings.engine === e.id ? " is-active" : ""}`}
            disabled={!e.available || saving}
            // Disabled with the reason in the row, not a tooltip: a greyed-out
            // option with no explanation reads as a bug in the app.
            onClick={() => update({ engine: e.id })}
          >
            <span className="vx-voice-engine__label">{e.label}</span>
            <span className="vx-voice-engine__detail">{e.detail}</span>
          </button>
        ))}
      </div>

      <h4 className="vx-set-h">Language</h4>
      <p className="vx-set-hint">
        Detect per turn unless you pin one. Pinning does not merely bias the
        recogniser — it suppresses its detection, so a pinned language is
        answered in that language whatever you actually said.
      </p>
      <select
        className="vx-set-select"
        value={settings.language}
        disabled={saving}
        onChange={(e) => update({ language: e.target.value })}
      >
        <option value="auto">{langName("auto")}</option>
        {settings.languages.map((c) => (
          <option key={c} value={c}>{langName(c)}</option>
        ))}
      </select>

      <h4 className="vx-set-h">Model for spoken replies</h4>
      <p className="vx-set-hint">
        A spoken turn is silence while it waits, and the model is nearly all of
        it: measured here, a 20B answered in 5.0s warm and 42s cold, against
        0.58s to recognise the speech and milliseconds to say the reply. Pick a
        small model and speech stays quick however large a model the app is set
        to for writing code.
      </p>
      <select
        className="vx-set-select"
        value={settings.provider && settings.model ? `${settings.provider}/${settings.model}` : ""}
        disabled={saving}
        onChange={(e) => {
          // Provider and model travel together — the daemon refuses half a
          // pair, and "" for both is how the choice goes back to the app.
          const [provider, ...rest] = e.target.value.split("/");
          update(e.target.value ? { provider, model: rest.join("/") } : { provider: "", model: "" });
        }}
      >
        <option value="">Use the model the app has selected</option>
        {Object.entries(modelsByProvider).map(([prov, ms]) => (
          <optgroup key={prov} label={prov}>
            {ms.map((m) => (
              <option key={`${prov}/${m}`} value={`${prov}/${m}`}>{m}</option>
            ))}
          </optgroup>
        ))}
      </select>

      <h4 className="vx-set-h">Voice</h4>
      {settings.voices.length === 0 ? (
        <p className="vx-set-hint">
          No voices to choose from — the selected engine could not be asked.
        </p>
      ) : (
        <select
          className="vx-set-select"
          value={settings.voice}
          disabled={saving}
          onChange={(e) => update({ voice: e.target.value })}
        >
          {Object.entries(byLang).map(([lang, vs]) => (
            <optgroup key={lang} label={langName(lang)}>
              {vs.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.name}
                  {v.quality !== "default" && v.quality !== "neural" ? ` (${v.quality})` : ""}
                </option>
              ))}
            </optgroup>
          ))}
        </select>
      )}
    </div>
  );
}

export default VoiceSection;
