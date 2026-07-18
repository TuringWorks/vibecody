import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Star, X } from 'lucide-react';

/**
 * G9.1 — `Working toward: {goal title}` banner that sits above the
 * Chat tab. Makes the pinned goal (auto-link target for new daemon
 * /agent runs and the source of `AgentContext.approved_plan` content
 * after G7.1) visible from the surface users spend the most time on.
 *
 * Polls `exec_goal_current` on mount and re-polls every 15 s so
 * external changes (CLI `/goal pin`, mobile pin toggle) appear here
 * without forcing the user to leave the chat tab. Also listens for the
 * `vibecoder:pin-changed` window event so the GoalPanel can poke this
 * banner immediately on pin/unpin.
 */
export interface PinnedGoalBannerProps {
  workspacePath: string | null;
}

interface CurrentGoalResponse {
  workspace: string | null;
  goal_id: string | null;
  pinned_at?: string;
  goal?: { id: string; title: string; status: string };
}

export function PinnedGoalBanner({ workspacePath }: PinnedGoalBannerProps) {
  const [goalId, setGoalId] = useState<string | null>(null);
  const [goalTitle, setGoalTitle] = useState<string>('');
  const [unpinning, setUnpinning] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const resp = (await invoke('exec_goal_current', {
        workspace: workspacePath || null,
      })) as CurrentGoalResponse;
      setGoalId(resp.goal_id ?? null);
      setGoalTitle(resp.goal?.title ?? '');
    } catch {
      setGoalId(null);
      setGoalTitle('');
    }
  }, [workspacePath]);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 15_000);
    const handler = () => refresh();
    window.addEventListener('vibecoder:pin-changed', handler);
    return () => {
      clearInterval(interval);
      window.removeEventListener('vibecoder:pin-changed', handler);
    };
  }, [refresh]);

  if (!goalId) return null;

  const unpin = async () => {
    setUnpinning(true);
    try {
      await invoke('exec_goal_unpin', { workspace: workspacePath || null });
      setGoalId(null);
      setGoalTitle('');
      window.dispatchEvent(new CustomEvent('vibecoder:pin-changed'));
    } catch {
      // Best-effort — user can retry from the Goals panel.
    } finally {
      setUnpinning(false);
    }
  };

  return (
    <div
      className="panel-card"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '6px 12px',
        margin: '6px 8px 0 8px',
        borderRadius: 6,
        fontSize: 'var(--font-size-sm)',
        background: 'var(--bg-subtle)',
        borderLeft: '3px solid var(--accent-primary, #6b7cff)',
      }}
    >
      <Star size={14} fill="currentColor" style={{ color: 'var(--accent-primary, #6b7cff)' }} />
      <span style={{ color: 'var(--text-secondary)', flexShrink: 0 }}>Working toward:</span>
      <span
        style={{
          flex: 1,
          fontWeight: 500,
          whiteSpace: 'nowrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
        }}
        title={goalTitle}
      >
        {goalTitle || '(unknown)'}
      </span>
      <button
        type="button"
        onClick={unpin}
        disabled={unpinning}
        className="panel-btn"
        style={{ padding: '2px 6px', fontSize: 'var(--font-size-xs)' }}
        title="Unpin — new /agent sessions won't auto-link to this goal"
      >
        <X size={12} />
      </button>
    </div>
  );
}
