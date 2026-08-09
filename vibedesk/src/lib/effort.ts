import type { ReasoningEffort } from "../components/ReasoningPill";

/**
 * The `reasoning` field to send with a run, or `undefined` when thinking is off.
 *
 * "Off" must reach the daemon as an *absent* field rather than a value. The
 * daemon maps an unrecognised effort to no thinking budget, but it also
 * publishes a `Reasoning effort: …` system line for whatever value it receives
 * — so sending "off" would announce a tier the user had just turned off.
 */
export function effortParam(effort: ReasoningEffort): string | undefined {
  return effort === "off" ? undefined : effort;
}
