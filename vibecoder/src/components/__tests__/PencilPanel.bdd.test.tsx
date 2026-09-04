/**
 * BDD tests for PencilPanel.
 *
 * The contract these pin, all of which the panel used to break:
 *  1. Generating passes the clicked template id and reports the real page and
 *     shape counts the backend returned — not a fabricated list
 *  2. A failing generate is shown, not swallowed into "nothing happened"
 *  3. Generating works with no workspace open (workspacePath: null)
 *  4. Exporting `.ep` writes the ZIP bytes the backend sent, under the
 *     filename it chose — not the UTF-8 text of an error message
 *  5. A failed export downloads nothing and says why
 *  6. HTML preview renders the backend's HTML in a sandboxed iframe
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('../Icon', () => ({
  Icon: ({ name }: { name: string }) => <span data-testid={`icon-${name}`} />,
}));

import { PencilPanel } from '../PencilPanel';

const WIREFRAME = {
  title: 'Login Form',
  template: 'login_form',
  pages: [{ name: 'Login', shapes: 17, width: 1440, height: 900 }],
  epXml: '<?xml version="1.0"?><Document name="Login Form" id="d"></Document>',
};

/** Records what `downloadPayload` handed the browser. */
let lastDownload: { name: string; type: string; size: number } | null = null;
let createdUrls = 0;
let clickedAnchors = 0;

beforeEach(() => {
  mockInvoke.mockReset();
  lastDownload = null;
  createdUrls = 0;
  clickedAnchors = 0;
  vi.spyOn(URL, 'createObjectURL').mockImplementation((blob: Blob | MediaSource) => {
    createdUrls += 1;
    const b = blob as Blob;
    lastDownload = { name: '', type: b.type, size: b.size };
    return `blob:pencil/${createdUrls}`;
  });
  vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});
  vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (
    this: HTMLAnchorElement,
  ) {
    clickedAnchors += 1;
    if (lastDownload) lastDownload.name = this.download;
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

async function generate(panelProps: { workspacePath: string | null; provider: string }) {
  render(<PencilPanel {...panelProps} />);
  fireEvent.click(screen.getByLabelText('Generate Login Form wireframe'));
  await waitFor(() => expect(mockInvoke).toHaveBeenCalled());
}

describe('PencilPanel - Templates', () => {
  it('sends the clicked template id and reports the counts the backend returned', async () => {
    mockInvoke.mockResolvedValue(WIREFRAME);
    await generate({ workspacePath: '/w', provider: 'anthropic' });

    const [cmd, args] = mockInvoke.mock.calls[0] as [string, Record<string, unknown>];
    expect(cmd).toBe('generate_pencil_wireframe');
    expect(args.templateId).toBe('login_form');
    expect(args.title).toBe('Login Form');

    expect(await screen.findByText(/Generated: Login Form/)).toBeTruthy();
    // The real shape count, not the "0 shapes" the stub reported.
    expect(screen.getByText('17 shapes · 1440×900')).toBeTruthy();
  });

  it('generates with no workspace open - the command must accept a null path', async () => {
    mockInvoke.mockResolvedValue(WIREFRAME);
    await generate({ workspacePath: null, provider: '' });
    const [, args] = mockInvoke.mock.calls[0] as [string, Record<string, unknown>];
    expect(args.workspacePath).toBeNull();
    expect(await screen.findByText(/Generated: Login Form/)).toBeTruthy();
  });

  it('surfaces a failing generate instead of swallowing it', async () => {
    mockInvoke.mockRejectedValue('unknown wireframe template `login_form`');
    await generate({ workspacePath: '/w', provider: 'anthropic' });
    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('unknown wireframe template');
    expect(screen.queryByText(/Generated: Login Form/)).toBeNull();
  });
});

describe('PencilPanel - Export', () => {
  it('downloads the .ep archive bytes under the backend filename', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'generate_pencil_wireframe') return Promise.resolve(WIREFRAME);
      // "PK\x03\x04\n", base64 - the first bytes of a ZIP local file header.
      return Promise.resolve({
        filename: 'login-form.ep',
        mimeType: 'application/zip',
        encoding: 'base64',
        data: 'UEsDBAo=',
      });
    });
    await generate({ workspacePath: '/w', provider: 'anthropic' });

    fireEvent.click(screen.getByRole('tab', { name: 'Export' }));
    fireEvent.click(await screen.findByText('Download Export'));

    await waitFor(() => expect(clickedAnchors).toBe(1));
    expect(lastDownload).toEqual({ name: 'login-form.ep', type: 'application/zip', size: 5 });

    const exportCall = mockInvoke.mock.calls.find((c) => c[0] === 'export_pencil_wireframe');
    expect((exportCall?.[1] as Record<string, unknown>).format).toBe('ep');
    expect((exportCall?.[1] as Record<string, unknown>).xml).toBe(WIREFRAME.epXml);
  });

  it('downloads nothing and reports the reason when the export fails', async () => {
    mockInvoke.mockImplementation((cmd: string) =>
      cmd === 'generate_pencil_wireframe'
        ? Promise.resolve(WIREFRAME)
        : Promise.reject('Select a provider to convert the wireframe'),
    );
    await generate({ workspacePath: '/w', provider: '' });

    fireEvent.click(screen.getByRole('tab', { name: 'Export' }));
    fireEvent.click(await screen.findByRole('radio', { name: 'React Component' }));
    fireEvent.click(screen.getByText('Download Export'));

    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('Select a provider');
    expect(clickedAnchors).toBe(0);
    expect(createdUrls).toBe(0);
  });

  it('previews the HTML export in a sandboxed iframe', async () => {
    const html = '<!DOCTYPE html><html><body><div class="wf-shape"></div></body></html>';
    mockInvoke.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'generate_pencil_wireframe') return Promise.resolve(WIREFRAME);
      expect(args.format).toBe('html');
      return Promise.resolve({
        filename: 'login-form.html',
        mimeType: 'text/html',
        encoding: 'utf8',
        data: html,
      });
    });
    await generate({ workspacePath: '/w', provider: 'anthropic' });

    fireEvent.click(screen.getByRole('tab', { name: 'Export' }));
    fireEvent.click(await screen.findByText('Preview'));

    const frame = (await screen.findByTitle('Wireframe preview')) as HTMLIFrameElement;
    expect(frame.getAttribute('sandbox')).toBe('');
    expect(frame.getAttribute('srcdoc')).toBe(html);
    // Previewing must not download anything.
    expect(clickedAnchors).toBe(0);
  });
});

describe('PencilPanel - Import', () => {
  it('renders the parsed document rather than a hardcoded name', async () => {
    mockInvoke.mockResolvedValue({
      name: 'Checkout Flow',
      id: 'doc-1',
      pages: [{ name: 'Cart', shapes: 12, width: 1280, height: 800 }],
      page_count: 1,
      total_shapes: 12,
    });
    render(<PencilPanel workspacePath="/w" provider="anthropic" />);
    fireEvent.click(screen.getByRole('tab', { name: 'Import' }));
    fireEvent.change(screen.getByLabelText('Pencil EP XML to parse'), {
      target: { value: '<Document name="Checkout Flow"/>' },
    });
    fireEvent.click(screen.getByText('Parse EP XML'));

    expect(await screen.findByText('Checkout Flow')).toBeTruthy();
    expect(screen.getByText('1 page(s) · 12 shapes')).toBeTruthy();
  });

  it('reports a parse failure instead of inventing a page', async () => {
    mockInvoke.mockRejectedValue('[NO_DOCUMENT] no <Document> element');
    render(<PencilPanel workspacePath="/w" provider="anthropic" />);
    fireEvent.click(screen.getByRole('tab', { name: 'Import' }));
    fireEvent.change(screen.getByLabelText('Pencil EP XML to parse'), {
      target: { value: 'not xml' },
    });
    fireEvent.click(screen.getByText('Parse EP XML'));

    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('NO_DOCUMENT');
  });
});

describe('PencilPanel - MCP', () => {
  it('shows the built request and does not claim it was dispatched', async () => {
    mockInvoke.mockResolvedValue(
      JSON.stringify({ operation: 'get_editor_state', status: 'not_dispatched' }, null, 2),
    );
    render(<PencilPanel workspacePath="/w" provider="anthropic" />);
    fireEvent.click(screen.getByRole('tab', { name: 'Pencil MCP' }));
    fireEvent.click(screen.getByText('Build get_editor_state request'));

    expect(await screen.findByText(/not_dispatched/)).toBeTruthy();
  });
});
