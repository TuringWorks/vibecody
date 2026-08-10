import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, beforeAll } from 'vitest';
import { HtmlPreview } from '../HtmlPreview';

/**
 * HtmlPreview — sandbox policy and the notices that explain a stalled preview.
 *
 * The bug these cover: a JS-rendered page paints its loading skeleton and stops
 * there, because scripts are off by default and nothing on screen said so. A
 * preview that silently shows a spinner forever is indistinguishable from a
 * crashed one.
 */

// jsdom has no URL.createObjectURL; the component builds a blob URL for the
// iframe src on mount.
beforeAll(() => {
  if (!URL.createObjectURL) {
    Object.defineProperty(URL, 'createObjectURL', { value: () => 'blob:mock', writable: true });
    Object.defineProperty(URL, 'revokeObjectURL', { value: () => {}, writable: true });
  }
});

const STATIC_PAGE = '<html><body><h1>Hello</h1></body></html>';
const SCRIPTED_PAGE = '<html><body><div id="root"></div><script>render()</script></body></html>';
const REMOTE_PAGE =
  '<html><head><script type="module" src="https://cdn.example.com/app.js"></script></head><body></body></html>';

function iframeOf(container: HTMLElement): HTMLIFrameElement {
  const frame = container.querySelector('iframe');
  if (!frame) throw new Error('preview iframe was not rendered');
  return frame as HTMLIFrameElement;
}

describe('HtmlPreview sandboxing', () => {
  it('grants no sandbox permissions while scripts are disabled', () => {
    const { container } = render(<HtmlPreview content={STATIC_PAGE} filePath="/tmp/a.html" />);
    expect(iframeOf(container).getAttribute('sandbox')).toBe('');
  });

  it('never pairs allow-scripts with allow-same-origin', () => {
    // Both together on a blob: URL this app minted hands the previewed
    // document our own origin — it could reach `parent`, our localStorage and
    // the Tauri IPC bridge. That is not a sandbox, and previewed HTML is
    // routinely something the user just downloaded.
    const { container } = render(<HtmlPreview content={SCRIPTED_PAGE} filePath="/tmp/a.html" />);
    fireEvent.click(screen.getByTitle('Enable Scripts'));

    const sandbox = iframeOf(container).getAttribute('sandbox') ?? '';
    expect(sandbox).toContain('allow-scripts');
    expect(sandbox).not.toContain('allow-same-origin');
  });
});

describe('HtmlPreview notices', () => {
  it('explains a page that cannot render because scripts are off', () => {
    render(<HtmlPreview content={SCRIPTED_PAGE} filePath="/tmp/a.html" />);
    expect(screen.getByText(/builds itself with JavaScript/i)).toBeTruthy();
  });

  it('offers a one-click way to enable them, and clears the notice', () => {
    render(<HtmlPreview content={SCRIPTED_PAGE} filePath="/tmp/a.html" />);
    fireEvent.click(screen.getByRole('button', { name: /enable scripts/i }));
    expect(screen.queryByText(/builds itself with JavaScript/i)).toBeNull();
  });

  it('stays quiet for a page that needs nothing', () => {
    render(<HtmlPreview content={STATIC_PAGE} filePath="/tmp/a.html" />);
    expect(screen.queryByText(/builds itself with JavaScript/i)).toBeNull();
    expect(screen.queryByText(/loads its code from the network/i)).toBeNull();
  });

  it('warns that a network-loaded page will not render offline', () => {
    // Enabling scripts is not enough for these: the iframe inherits the app's
    // 'self'-only CSP, so the remote bundle never arrives and the page sits on
    // its loading screen. Saying so beats letting the user retry forever.
    render(<HtmlPreview content={REMOTE_PAGE} filePath="/tmp/a.html" />);
    fireEvent.click(screen.getByTitle('Enable Scripts'));
    expect(screen.getByText(/loads its code from the network/i)).toBeTruthy();
  });

  it('does not warn about the network before scripts are even on', () => {
    // The scripts-off notice is the actionable one; showing both at once buries it.
    render(<HtmlPreview content={REMOTE_PAGE} filePath="/tmp/a.html" />);
    expect(screen.queryByText(/loads its code from the network/i)).toBeNull();
  });
});
