import type { ComponentType } from "react";

/**
 * The menu behind a composer's `+`.
 *
 * Progressive disclosure, generalised from the VibeDesk composer: a toolbar
 * that grows one control per feature ends up as a row of equal-weight buttons
 * where nothing says which of them describe the *message* and which describe
 * the *run*. Things you reach for occasionally live here; the toolbar keeps
 * only what you touch every turn.
 *
 * Deliberately content-free — the host supplies the groups, because what
 * belongs behind `+` differs per shell. VibeCoder offers attaching and
 * mentioning files; VibeAIChat, which has no workspace, offers neither.
 *
 * Click-away and Escape are the *host's* job, not this component's: the menu
 * and the button that opens it must be inside one ref, or dismissing fires
 * before the button's own click and reopens the menu on every attempt.
 * [`useClickAway`] is the shared hook for exactly that.
 */

/** Any icon component taking a `size`. Kept structural so this module does not
 *  depend on a particular icon library. */
export type ComposerIcon = ComponentType<{ size?: number | string }>;

interface ItemBase {
  id: string;
  icon: ComposerIcon;
  label: string;
  disabled?: boolean;
  /** Why it is unavailable. Replaces the sub-line rather than hiding in a
   *  tooltip — a disabled row with no reason reads as a bug. */
  disabledHint?: string;
}

export interface ComposerAction extends ItemBase {
  kind?: "action";
  /** One line under the label saying what it does. */
  sub?: string;
  onSelect: () => void;
}

export interface ComposerSwitch extends ItemBase {
  kind: "switch";
  on: boolean;
  /** What to say in each state. `off` sells the feature; `on` says what to do
   *  next, since the switch itself is not the thing that starts it. */
  sub?: { on: string; off: string };
  onChange: (on: boolean) => void;
}

export type ComposerItem = ComposerAction | ComposerSwitch;

export interface ComposerGroup {
  /** Uppercase heading. Omit for an unlabelled run of items. */
  title?: string;
  items: ComposerItem[];
}

export interface ComposerDrawerProps {
  groups: ComposerGroup[];
  /** Close the menu. Called before an action runs; a switch leaves it open, so
   *  you can flip it and read the result without the menu vanishing. */
  onClose: () => void;
  /** Accessible name for the menu itself. */
  label?: string;
}

const isSwitch = (i: ComposerItem): i is ComposerSwitch => i.kind === "switch";

export function ComposerDrawer({ groups, onClose, label = "More actions" }: ComposerDrawerProps) {
  return (
    <div className="vxc-drawer" role="menu" aria-label={label}>
      {groups
        .filter((g) => g.items.length > 0)
        .map((group, gi) => (
          <div key={group.title ?? `group-${gi}`}>
            {group.title && <div className="vxc-drawer__group">{group.title}</div>}
            {group.items.map((item) => {
              const Icon = item.icon;
              const off = item.disabled === true;
              const sub = off
                ? (item.disabledHint ?? "Not available here")
                : isSwitch(item)
                  ? (item.on ? item.sub?.on : item.sub?.off)
                  : item.sub;
              return (
                <button
                  key={item.id}
                  className="vxc-drawer__item"
                  role={isSwitch(item) ? "menuitemcheckbox" : "menuitem"}
                  aria-checked={isSwitch(item) ? item.on && !off : undefined}
                  disabled={off}
                  title={off ? item.disabledHint : undefined}
                  onClick={() => {
                    if (isSwitch(item)) {
                      item.onChange(!item.on);
                      return;
                    }
                    onClose();
                    item.onSelect();
                  }}
                >
                  <Icon size={15} />
                  <span className="vxc-drawer__label">{item.label}</span>
                  {sub && <span className="vxc-drawer__sub">{sub}</span>}
                  {isSwitch(item) && !off && (
                    <span
                      className={`vxc-drawer__switch${item.on ? " is-on" : ""}`}
                      aria-hidden
                    />
                  )}
                </button>
              );
            })}
          </div>
        ))}
    </div>
  );
}
