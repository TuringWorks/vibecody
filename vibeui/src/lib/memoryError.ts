/**
 * Classify a raw memory-system error string into a user-actionable
 * { message, hint } pair. Same shape as the diffcomplete classifier
 * — keeps hint cards consistent across panels.
 *
 * The Tauri / HTTP boundary surfaces errors as plain strings (we don't
 * carry typed errors across IPC). This mapper inspects stable
 * substrings and attaches a short next-step hint so users see "what to
 * do about this" instead of just a stack-trace fragment.
 */
export function classifyMemoryError(raw: string): { message: string; hint?: string } {
  const lower = raw.toLowerCase();

  if (lower.includes("permission denied") || lower.includes("eacces") || lower.includes("readonly")) {
    return {
      message: raw,
      hint: "VibeCody can't write to its memory store. Check permissions on ~/.local/share/vibecli/openmemory/ (Linux) or ~/Library/Application Support/vibecli/openmemory/ (macOS).",
    };
  }
  if (lower.includes("no space left") || lower.includes("disk full") || lower.includes("enospc")) {
    return {
      message: raw,
      hint: "Disk is full. Free up space or run memory.run_decay() to purge low-salience memories.",
    };
  }
  if (lower.includes("invalid json") || lower.includes("expected value") || lower.includes("trailing characters") || lower.includes("eof while parsing")) {
    return {
      message: raw,
      hint: "The memory store file is corrupt. Restore from backup, or delete the openmemory directory to start fresh (you'll lose existing memories).",
    };
  }
  if (lower.includes("not found") || lower.includes("404")) {
    return {
      message: raw,
      hint: "The memory ID no longer exists — it may have been decayed or deleted in another session. Refresh the list.",
    };
  }
  if (lower.includes("connection refused") || lower.includes("network") || lower.includes("timeout") || lower.includes("daemon")) {
    return {
      message: raw,
      hint: "The vibecli daemon isn't reachable. Start it with `vibecli serve` and retry.",
    };
  }
  if (lower.includes("encryption") || lower.includes("decrypt") || lower.includes("passphrase")) {
    return {
      message: raw,
      hint: "Encryption / decryption failed. If you recently changed the passphrase, run `vibecli set-key openmemory_passphrase <value>` with the original passphrase to recover.",
    };
  }
  if (lower.includes("import")) {
    return {
      message: raw,
      hint: "The import file format wasn't recognized. Supported formats: openmemory JSON, mem0 JSON, Zep JSON. Check the file or try the auto-detect mode.",
    };
  }
  return { message: raw };
}
