/**
 * What a Sandbox-mode run may touch outside its workspace.
 *
 * Mirrors `SandboxPolicy` in the daemon's `sandbox_policy.rs`, field for field —
 * the daemon is the enforcer and this is only the request shape. Two properties
 * hold there no matter what is sent from here, and the UI should not imply
 * otherwise:
 *
 *  - credential paths (`~/.ssh`, `~/.aws`, `~/.vibecli`, `id_rsa`, …) are never
 *    reachable, not even via an explicit allowed root;
 *  - denied roots are checked before allowed roots.
 */
export interface SandboxPolicy {
  readOutside: boolean;
  writeOutside: boolean;
  execOutside: boolean;
  network: boolean;
  allowRoots: string[];
  denyRoots: string[];
}

/** Deny-everything — identical to the daemon's `SandboxPolicy::locked()`. */
export const LOCKED_SANDBOX: SandboxPolicy = {
  readOutside: false,
  writeOutside: false,
  execOutside: false,
  network: false,
  allowRoots: [],
  denyRoots: [],
};

/** True when the policy grants nothing, so Sandbox mode behaves like Agent. */
export function isLocked(p: SandboxPolicy): boolean {
  return !p.readOutside && !p.writeOutside && !p.execOutside && !p.network;
}

/** The daemon's wire shape: snake_case, and roots trimmed of blanks. */
export function toWire(p: SandboxPolicy): Record<string, unknown> {
  const roots = (xs: string[]) => xs.map((x) => x.trim()).filter(Boolean);
  return {
    read_outside: p.readOutside,
    write_outside: p.writeOutside,
    exec_outside: p.execOutside,
    network: p.network,
    allow_roots: roots(p.allowRoots),
    deny_roots: roots(p.denyRoots),
  };
}

/** Short human summary for the composer pill — what is actually granted. */
export function describe(p: SandboxPolicy): string {
  if (isLocked(p)) return "no access outside the workspace";
  const granted = [
    p.readOutside && "read",
    p.writeOutside && "write",
    p.execOutside && "run commands",
    p.network && "network",
  ].filter(Boolean) as string[];
  const scope = p.allowRoots.length ? ` in ${p.allowRoots.length} allowed root(s)` : " anywhere";
  return `${granted.join(" · ")}${scope}`;
}
