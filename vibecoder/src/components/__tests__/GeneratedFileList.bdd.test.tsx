/**
 * BDD tests for GeneratedFileList — the shared review-and-write list behind
 * Screenshot to App, the Import tab, and the Figma import.
 *
 * Its whole reason to exist is that nothing is written without the user asking,
 * and that a write that fails says so:
 *  1. Rendering the list writes nothing
 *  2. "Write" sends the edited destination, joined to the workspace root
 *  3. A failed write reports the error and offers a retry — it never says "Written"
 *  4. With no workspace open, writing is unavailable and says why
 *  5. "Write All" reports only the files that actually landed
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { GeneratedFileList } from '../design/GeneratedFileList';

const FILES = [
  { path: 'src/App.tsx', content: 'export const App = () => null;', language: 'tsx' },
  { path: 'src/Card.tsx', content: 'export const Card = () => null;', language: 'tsx' },
];

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockResolvedValue(null);
});

describe('GeneratedFileList', () => {
  it('1. rendering the list writes nothing', () => {
    render(<GeneratedFileList files={FILES} workspacePath="/tmp/ws" />);
    expect(screen.getByText(/2 files generated/i)).toBeTruthy();
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('2. Write sends the edited destination joined to the workspace root', async () => {
    render(<GeneratedFileList files={FILES} workspacePath="/tmp/ws" />);
    const destination = screen.getByLabelText(/Destination for generated file 1/i);
    fireEvent.change(destination, { target: { value: 'app/Main.tsx' } });
    fireEvent.click(screen.getAllByRole('button', { name: /^Write$/ })[0]);

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('write_file', {
        path: '/tmp/ws/app/Main.tsx',
        content: FILES[0].content,
      }),
    );
  });

  it('3. a failed write reports the error and offers a retry', async () => {
    const onError = vi.fn();
    mockInvoke.mockRejectedValue(new Error('read-only file system'));
    render(<GeneratedFileList files={FILES} workspacePath="/tmp/ws" onError={onError} />);
    fireEvent.click(screen.getAllByRole('button', { name: /^Write$/ })[0]);

    await waitFor(() => expect(onError).toHaveBeenCalled());
    expect(String(onError.mock.calls[0][0])).toMatch(/read-only file system/);
    expect(screen.getAllByRole('button', { name: /Retry/ }).length).toBe(1);
    expect(screen.queryByRole('button', { name: /^Written$/ })).toBeNull();
  });

  it('4. with no workspace open, writing is unavailable and says why', () => {
    render(<GeneratedFileList files={FILES} workspacePath={null} />);
    expect(screen.getByText(/No workspace folder is open/i)).toBeTruthy();
    const write = screen.getAllByRole('button', { name: /^Write$/ })[0] as HTMLButtonElement;
    expect(write.disabled).toBe(true);
  });

  it('5. Write All reports only the files that actually landed', async () => {
    const onWritten = vi.fn();
    mockInvoke.mockImplementation((_cmd: string, args: { path: string }) =>
      args.path.endsWith('Card.tsx')
        ? Promise.reject(new Error('nope'))
        : Promise.resolve(null),
    );
    render(
      <GeneratedFileList files={FILES} workspacePath="/tmp/ws" onWritten={onWritten} onError={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole('button', { name: /Write All to Project/i }));

    await waitFor(() => expect(onWritten).toHaveBeenCalled());
    expect(onWritten).toHaveBeenCalledWith(['src/App.tsx']);
  });
});
