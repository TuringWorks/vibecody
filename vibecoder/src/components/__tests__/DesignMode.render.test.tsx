/**
 * Rendering behaviour of DesignMode's tab strip.
 *
 * Two properties this panel did not have:
 *  - the four editor tabs are not mounted until visited (they used to all mount
 *    on open, hidden with `display: none`, each firing its own startup calls)
 *  - a failed generation is reported as a failure, not written into the code
 *    pane as if the model had produced it
 */
import { useEffect } from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// Each editor announces its own mount, so "was it mounted?" is observable.
// Recorded from an effect, not the render body, so a re-render is not counted
// as a second mount.
const mounted: string[] = [];
function stub(name: string) {
  return function Stub() {
    useEffect(() => {
      mounted.push(name);
    }, []);
    return <div data-testid={`${name}-mounted`} />;
  };
}
vi.mock('../DrawioEditorPanel', () => ({ DrawioEditorPanel: stub('drawio') }));
vi.mock('../PencilPanel', () => ({ PencilPanel: stub('pencil') }));
vi.mock('../PenpotPanel', () => ({ PenpotPanel: stub('penpot') }));
vi.mock('../DiagramGeneratorPanel', () => ({ DiagramGeneratorPanel: stub('diagrams') }));

import { DesignMode } from '../DesignMode';

beforeEach(() => {
  mounted.length = 0;
  mockInvoke.mockReset();
  mockInvoke.mockResolvedValue('');
});

const renderPanel = (provider = 'Claude (claude-opus-5)') =>
  render(<DesignMode workspacePath="/tmp/ws" provider={provider} />);

describe('DesignMode — tab mounting', () => {
  it('mounts no editor until its tab is opened', () => {
    renderPanel();
    expect(mounted).toEqual([]);
  });

  it('mounts an editor on first visit and keeps it mounted after switching away', async () => {
    renderPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Draw.io' }));
    await waitFor(() => expect(screen.getByTestId('drawio-mounted')).toBeTruthy());
    expect(mounted).toEqual(['drawio']);

    // Switching away hides it rather than unmounting — editors hold unsaved work.
    fireEvent.click(screen.getByRole('button', { name: 'Preview' }));
    expect(screen.getByTestId('drawio-mounted')).toBeTruthy();

    // And the other three still have not been mounted.
    expect(mounted).toEqual(['drawio']);
  });
});

describe('DesignMode — generate', () => {
  it('reports a generation failure instead of showing the error as generated code', async () => {
    mockInvoke.mockImplementation((cmd: string) =>
      cmd === 'generate_component'
        ? Promise.reject(new Error('Cannot use provider "Claude (x)": Provider not found'))
        : Promise.resolve(''),
    );
    renderPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Generate' }));

    fireEvent.change(screen.getByPlaceholderText(/Describe a component/i), {
      target: { value: 'a login form' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Generate component' }));

    await waitFor(() => expect(screen.getByRole('alert').textContent).toMatch(/Provider not found/));
    // The failure must not have become the contents of the code editor.
    expect(screen.queryByText(/Generated Code/)).toBeNull();
  });

  it('refuses to generate with no provider selected', () => {
    renderPanel('');
    fireEvent.click(screen.getByRole('button', { name: 'Generate' }));
    expect(screen.getByText(/Pick a provider in the toolbar dropdown/i)).toBeTruthy();
  });
});

describe('DesignMode — components', () => {
  it('reports an unscannable workspace rather than an empty component list', async () => {
    mockInvoke.mockImplementation((cmd: string) =>
      cmd === 'design_component_tree'
        ? Promise.reject(new Error('/nope is not a directory.'))
        : Promise.resolve(''),
    );
    renderPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Components' }));
    fireEvent.click(screen.getByRole('button', { name: /Scan workspace/i }));

    await waitFor(() => expect(screen.getByRole('alert').textContent).toMatch(/not a directory/));
    expect(screen.queryByText(/component\(s\) across/)).toBeNull();
  });
});

describe('DesignMode — preview frame', () => {
  it('withholds allow-same-origin from a generated preview', async () => {
    // Model-authored code runs in this frame. With allow-same-origin on a
    // srcdoc frame it would share VibeCoder's origin — its document, its
    // storage and its Tauri bridge.
    mockInvoke.mockImplementation((cmd: string) =>
      cmd === 'generate_component'
        ? Promise.resolve('```tsx\nconst App = () => <div>hi</div>;\n```')
        : Promise.resolve(''),
    );
    const { container } = renderPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Generate' }));
    fireEvent.change(screen.getByPlaceholderText(/Describe a component/i), {
      target: { value: 'a box' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Generate component' }));

    const frame = container.querySelector('iframe') as HTMLIFrameElement;
    await waitFor(() => expect(frame.getAttribute('srcdoc')).toBeTruthy());
    expect(frame.getAttribute('sandbox')).toBe('allow-scripts allow-forms allow-modals');
  });

  it('an external URL keeps allow-same-origin, which it needs to render', () => {
    const { container } = renderPanel();
    const frame = container.querySelector('iframe') as HTMLIFrameElement;
    expect(frame.getAttribute('sandbox')).toContain('allow-same-origin');
  });
});
