import { lazy, Suspense, type ComponentType } from "react";
import { TabbedPanel } from "../TabbedPanel";

const Loading = () => (
  <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>
);

/** Definition for a single tab in a composite panel */
export interface TabDef {
  id: string;
  label: string;
  /** Lazy import function, e.g. () => import("../FooPanel") */
  importFn: () => Promise<{ default: ComponentType<any> } | Record<string, ComponentType<any>>>;
  /** Named export to extract (if not default) */
  exportName?: string;
  /** Props to forward (merged with composite props) */
  extraProps?: Record<string, unknown>;
}

/** Standard props passed to all composite panels */
export interface CompositeProps {
  workspacePath?: string | null;
  provider?: string;
  onOpenFile?: (path: string, line?: number) => void;
}

/**
 * Factory that generates a composite panel from a list of tab definitions.
 *
 * Replaces the 33 nearly-identical composite files with a one-liner:
 * ```ts
 * export const MyComposite = createComposite([
 *   { id: "foo", label: "Foo", importFn: () => import("../FooPanel"), exportName: "FooPanel" },
 *   { id: "bar", label: "Bar", importFn: () => import("../BarPanel"), exportName: "BarPanel" },
 * ]);
 * ```
 */
export function createComposite(tabs: TabDef[]) {
  // Pre-create lazy components once (not on every render)
  const lazyComponents = tabs.map((tab) => {
    const LazyComp = lazy(() =>
      tab.importFn().then((mod) => {
        if (tab.exportName && tab.exportName in mod) {
          return { default: (mod as Record<string, ComponentType<any>>)[tab.exportName] };
        }
        if ("default" in mod) {
          return mod as { default: ComponentType<any> };
        }
        // Fallback: pick first export
        const first = Object.values(mod)[0];
        return { default: first as ComponentType<any> };
      })
    ) as ComponentType<any>;
    return { ...tab, LazyComp };
  });

  return function CompositePanel(props: CompositeProps) {
    const wp = props.workspacePath ?? null;
    return (
      <TabbedPanel
        tabs={lazyComponents.map((t) => ({
          id: t.id,
          label: t.label,
          content: (
            <Suspense fallback={<Loading />}>
              <t.LazyComp
                workspacePath={wp}
                provider={props.provider}
                onOpenFile={props.onOpenFile}
                {...(t.extraProps || {})}
              />
            </Suspense>
          ),
        }))}
      />
    );
  };
}
