import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  ArchiveExtractModal,
  describeExtraction,
  type ArchiveExtractPlan,
  type ArchiveExtractResult,
} from '../ArchiveExtractModal';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const plan: ArchiveExtractPlan = {
  archive: '/w/dist.zip',
  archive_name: 'dist.zip',
  member: 'src/main.rs',
  destination: '/w/dist',
  destination_name: 'dist',
  member_destination: '/w/dist/src/main.rs',
  renamed_to_avoid_collision: false,
};

const result: ArchiveExtractResult = {
  destination: '/w/dist',
  files: 3,
  directories: 1,
  skipped: 0,
  bytes: 2048,
  opened_path: '/w/dist/src/main.rs',
};

beforeEach(() => {
  mockInvoke.mockReset();
});

describe('ArchiveExtractModal', () => {
  it('renders nothing until a path is given', () => {
    const { container } = render(
      <ArchiveExtractModal path={null} onCancel={() => {}} onExtracted={() => {}} />,
    );
    expect(container).toBeEmptyDOMElement();
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('names the destination folder before anything is written', async () => {
    mockInvoke.mockResolvedValueOnce(plan);
    render(
      <ArchiveExtractModal
        path="/w/dist.zip!/src/main.rs"
        onCancel={() => {}}
        onExtracted={() => {}}
      />,
    );
    await screen.findByText('/w/dist');
    // Planning must not extract: the user has agreed to nothing yet.
    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith('plan_archive_extraction', {
      path: '/w/dist.zip!/src/main.rs',
    });
  });

  it('says when a previous extraction forced a suffixed folder', async () => {
    mockInvoke.mockResolvedValueOnce({
      ...plan,
      destination: '/w/dist-1',
      destination_name: 'dist-1',
      renamed_to_avoid_collision: true,
    });
    render(
      <ArchiveExtractModal path="/w/dist.zip!/src/main.rs" onCancel={() => {}} onExtracted={() => {}} />,
    );
    await screen.findByText('/w/dist-1');
    expect(screen.getByText(/already exists/)).toBeTruthy();
  });

  it('extracts on confirm and reports the result', async () => {
    mockInvoke.mockResolvedValueOnce(plan).mockResolvedValueOnce(result);
    const onExtracted = vi.fn();
    render(
      <ArchiveExtractModal
        path="/w/dist.zip!/src/main.rs"
        onCancel={() => {}}
        onExtracted={onExtracted}
      />,
    );
    fireEvent.click(await screen.findByText('Extract and edit'));
    await waitFor(() => expect(onExtracted).toHaveBeenCalledWith(result));
    expect(mockInvoke).toHaveBeenLastCalledWith('extract_archive', {
      path: '/w/dist.zip!/src/main.rs',
    });
  });

  it('keeps the file read-only when the prompt is declined', async () => {
    mockInvoke.mockResolvedValueOnce(plan);
    const onCancel = vi.fn();
    render(
      <ArchiveExtractModal path="/w/dist.zip!/src/main.rs" onCancel={onCancel} onExtracted={() => {}} />,
    );
    fireEvent.click(await screen.findByText('Keep read-only'));
    expect(onCancel).toHaveBeenCalled();
    // One call — the plan. Declining writes nothing.
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });

  it('shows a failed extraction instead of claiming success', async () => {
    mockInvoke
      .mockResolvedValueOnce(plan)
      .mockRejectedValueOnce('destination already exists: /w/dist');
    const onExtracted = vi.fn();
    render(
      <ArchiveExtractModal
        path="/w/dist.zip!/src/main.rs"
        onCancel={() => {}}
        onExtracted={onExtracted}
      />,
    );
    fireEvent.click(await screen.findByText('Extract and edit'));
    await screen.findByText('destination already exists: /w/dist');
    expect(onExtracted).not.toHaveBeenCalled();
  });
});

describe('describeExtraction', () => {
  it('counts what came out', () => {
    expect(describeExtraction(result)).toBe('Extracted 3 files, 1 folder, 2.0 KB');
  });

  it('reports skipped entries rather than hiding them', () => {
    // Symlinks and zip-slip paths are not extracted; saying so is the
    // difference between "3 files" and "3 files, and one we refused".
    expect(describeExtraction({ ...result, skipped: 2 })).toContain(
      '2 unsafe or non-file entries skipped',
    );
  });

  it('does not pluralize a single file', () => {
    expect(describeExtraction({ ...result, files: 1, directories: 0, bytes: 12 })).toBe(
      'Extracted 1 file, 12 B',
    );
  });
});
