/**
 * G4.6 — Goals tree-view sidebar.
 *
 * Renders durable execution-intent goals from the local daemon as a
 * tree. Top-level rows are roots (no `parent_goal_id`); each row
 * expands to its direct children via `/v1/goals/:id/children`. The
 * provider lazy-fetches each level on expansion so a sprawling tree
 * doesn't pin the daemon up front.
 *
 * Refresh is manual (`vibecli.refreshGoals` command). Mutations stay
 * on the existing palette commands (`vibecli.newGoal`, `vibecli.startGoal`)
 * plus per-row context-menu entries we register in package.json.
 */

import * as vscode from 'vscode';
import type { ExecGoalSummary, VibeCLIClient } from './api-client';

interface ChildrenResponse {
  parent_goal_id: string;
  children: ExecGoalSummary[];
  count: number;
}

const STATUS_ICON: Record<ExecGoalSummary['status'], string> = {
  active: '●',
  paused: '⏸',
  done: '✓',
  abandoned: '⊘',
};

export class GoalTreeItem extends vscode.TreeItem {
  constructor(public readonly goal: ExecGoalSummary, pinned: boolean = false) {
    // G13.1 — prefix the title with ★ when the goal is the current pin
    // (in this workspace or the global slot), matching VibeCoder / Watch /
    // Wear / Mobile / TUI. Pin marker comes before the status glyph so
    // it's the first thing the eye lands on.
    const star = pinned ? '★ ' : '';
    super(
      `${star}${STATUS_ICON[goal.status] ?? '·'} ${goal.title}`,
      vscode.TreeItemCollapsibleState.Collapsed,
    );
    this.id = goal.id;
    this.description = goal.status;
    this.tooltip = goal.statement
      ? `${goal.title}\n\n${goal.statement}`
      : goal.title;
    // contextValue gates the per-row context-menu entries in package.json
    // (`view/item/context` → `when: viewItem == vibecliGoal`). Pinned
    // rows get a distinct value so package.json can hide "Pin" and
    // show "Unpin" if we add those menu entries later.
    this.contextValue = pinned ? 'vibecliGoalPinned' : 'vibecliGoal';
  }
}

export class GoalsTreeProvider implements vscode.TreeDataProvider<GoalTreeItem> {
  private readonly _onDidChange = new vscode.EventEmitter<
    GoalTreeItem | undefined | void
  >();
  readonly onDidChangeTreeData = this._onDidChange.event;

  constructor(private readonly client: VibeCLIClient) {}

  refresh(item?: GoalTreeItem): void {
    this._onDidChange.fire(item);
  }

  getTreeItem(element: GoalTreeItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: GoalTreeItem): Promise<GoalTreeItem[]> {
    // G13.1 — fetch the pin set once per refresh and pass it into both
    // the roots and the children branches so ★ shows up at every depth.
    const pinned = new Set(await this.fetchPinnedIds());
    if (!element) {
      // Top-level: all goals, filtered to roots client-side. The
      // daemon's GET /v1/goals doesn't have a `parent_goal_id IS NULL`
      // filter yet, so we fetch and partition here. Cheap because the
      // default limit is 50.
      const all = await this.client.listGoals();
      const ids = new Set(all.map((g) => g.id));
      const roots = all.filter((g) => {
        const parent = (g as ExecGoalSummary & { parent_goal_id?: string | null })
          .parent_goal_id;
        return !parent || !ids.has(parent);
      });
      // Sort: pinned first, then active, then by updated_at desc. ★
      // lives at the top of the tree to match the "what am I working
      // on now" framing the watch tile uses.
      roots.sort((a, b) => {
        const ap = pinned.has(a.id) ? 1 : 0;
        const bp = pinned.has(b.id) ? 1 : 0;
        if (ap !== bp) return bp - ap;
        if (a.status !== b.status) {
          return a.status === 'active' ? -1 : b.status === 'active' ? 1 : 0;
        }
        return (b.updated_at ?? '').localeCompare(a.updated_at ?? '');
      });
      return roots.map((g) => new GoalTreeItem(g, pinned.has(g.id)));
    }
    const kids = await this.fetchChildren(element.goal.id);
    return kids.map((g) => new GoalTreeItem(g, pinned.has(g.id)));
  }

  private async fetchPinnedIds(): Promise<string[]> {
    const ws = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    return this.client.getPinnedGoalIds(ws);
  }

  private async fetchChildren(parentId: string): Promise<ExecGoalSummary[]> {
    try {
      const url = `http://localhost:${this.port()}/v1/goals/${encodeURIComponent(parentId)}/children`;
      const res = await fetch(url);
      if (!res.ok) return [];
      const data = (await res.json()) as ChildrenResponse;
      return data.children ?? [];
    } catch {
      return [];
    }
  }

  private port(): number {
    return vscode.workspace
      .getConfiguration('vibecli')
      .get<number>('daemonPort', 7878);
  }
}
