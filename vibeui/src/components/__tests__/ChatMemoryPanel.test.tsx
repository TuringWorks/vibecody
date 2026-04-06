import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ChatMemoryPanel } from '../ChatMemoryPanel';
import type { MemoryFact } from '../../hooks/useSessionMemory';

// ── Helpers ───────────────────────────────────────────────────────────────────

function makeFact(overrides: Partial<MemoryFact> = {}): MemoryFact {
  return {
    id: overrides.id ?? 'fact-1',
    text: overrides.text ?? 'A test memory fact with enough text',
    source: overrides.source ?? 'extracted',
    pinned: overrides.pinned ?? false,
    tabId: overrides.tabId ?? 'tab-1',
    createdAt: overrides.createdAt ?? 1000,
  };
}

function renderPanel(
  facts: MemoryFact[] = [],
  handlers: Partial<{
    onPin: (id: string) => void;
    onUnpin: (id: string) => void;
    onDelete: (id: string) => void;
    onEdit: (id: string, text: string) => void;
    onAddManual: (text: string) => void;
  }> = {},
) {
  const onPin = handlers.onPin ?? vi.fn();
  const onUnpin = handlers.onUnpin ?? vi.fn();
  const onDelete = handlers.onDelete ?? vi.fn();
  const onEdit = handlers.onEdit ?? vi.fn();
  const onAddManual = handlers.onAddManual ?? vi.fn();

  const result = render(
    <ChatMemoryPanel
      facts={facts}
      tabId="tab-1"
      onPin={onPin}
      onUnpin={onUnpin}
      onDelete={onDelete}
      onEdit={onEdit}
      onAddManual={onAddManual}
    />
  );
  return { ...result, onPin, onUnpin, onDelete, onEdit, onAddManual };
}

function openPanel() {
  fireEvent.click(screen.getByText('Memory'));
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('ChatMemoryPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ── Rendering (collapsed state) ──────────────────────────────────────────

  it('renders the Memory toggle button', () => {
    renderPanel();
    expect(screen.getByText('Memory')).toBeInTheDocument();
  });

  it('is collapsed by default (body not visible)', () => {
    renderPanel([makeFact()]);
    expect(screen.queryByText('Add a note to memory...')).not.toBeInTheDocument();
  });

  it('shows count badge when facts exist', () => {
    const facts = [makeFact({ id: '1' }), makeFact({ id: '2' })];
    renderPanel(facts);
    expect(screen.getByText('2')).toBeInTheDocument();
  });

  it('does not show count badge when no facts', () => {
    renderPanel([]);
    expect(screen.queryByText('0')).not.toBeInTheDocument();
  });

  it('shows "N pinned · in every message" when pinned facts exist', () => {
    const pinned = makeFact({ id: 'p1', pinned: true, tabId: '__pinned__' });
    renderPanel([pinned]);
    expect(screen.getByText(/1 pinned · in every message/)).toBeInTheDocument();
  });

  it('does not show pinned indicator when no pinned facts', () => {
    renderPanel([makeFact()]);
    expect(screen.queryByText(/pinned · in every message/)).not.toBeInTheDocument();
  });

  // ── Open/close toggle ────────────────────────────────────────────────────

  it('opens the panel on click', () => {
    renderPanel();
    openPanel();
    expect(screen.getByPlaceholderText('Add a note to memory...')).toBeInTheDocument();
  });

  it('closes the panel on second click', () => {
    renderPanel();
    openPanel();
    fireEvent.click(screen.getByText('Memory'));
    expect(screen.queryByPlaceholderText('Add a note to memory...')).not.toBeInTheDocument();
  });

  // ── Empty state ──────────────────────────────────────────────────────────

  it('shows empty state message when no facts', () => {
    renderPanel([]);
    openPanel();
    expect(screen.getByText(/No facts yet/)).toBeInTheDocument();
  });

  it('does not show empty state when facts exist', () => {
    renderPanel([makeFact()]);
    openPanel();
    expect(screen.queryByText(/No facts yet/)).not.toBeInTheDocument();
  });

  // ── Session facts ────────────────────────────────────────────────────────

  it('displays session facts for the current tab', () => {
    const fact = makeFact({ text: 'Session fact for current tab', tabId: 'tab-1' });
    renderPanel([fact]);
    openPanel();
    expect(screen.getByText('Session fact for current tab')).toBeInTheDocument();
  });

  it('does not display facts for other tabs', () => {
    const otherFact = makeFact({ text: 'Other tab fact text here', tabId: 'tab-2', pinned: false });
    renderPanel([otherFact]);
    openPanel();
    expect(screen.queryByText('Other tab fact text here')).not.toBeInTheDocument();
  });

  // ── Pinned facts ─────────────────────────────────────────────────────────

  it('displays pinned facts section label', () => {
    const pinned = makeFact({ id: 'p1', pinned: true, tabId: '__pinned__', text: 'Pinned fact text here' });
    renderPanel([pinned]);
    openPanel();
    expect(screen.getByText(/PINNED — injected into every message/)).toBeInTheDocument();
  });

  it('displays pinned fact text', () => {
    const pinned = makeFact({ pinned: true, tabId: '__pinned__', text: 'Pinned fact text content' });
    renderPanel([pinned]);
    openPanel();
    expect(screen.getByText('Pinned fact text content')).toBeInTheDocument();
  });

  it('shows "THIS SESSION" label when both pinned and session facts exist', () => {
    const pinned = makeFact({ id: 'p1', pinned: true, tabId: '__pinned__', text: 'Pinned fact text' });
    const session = makeFact({ id: 's1', pinned: false, tabId: 'tab-1', text: 'Session fact text' });
    renderPanel([pinned, session]);
    openPanel();
    expect(screen.getByText('THIS SESSION')).toBeInTheDocument();
  });

  it('does not show "THIS SESSION" label when only session facts exist', () => {
    renderPanel([makeFact()]);
    openPanel();
    expect(screen.queryByText('THIS SESSION')).not.toBeInTheDocument();
  });

  // ── Pin / Unpin ──────────────────────────────────────────────────────────

  it('calls onPin when pin button clicked for unpinned fact', () => {
    const fact = makeFact({ id: 'fact-x', pinned: false });
    const { onPin } = renderPanel([fact]);
    openPanel();
    // Find the pin button (📌 emoji) for this fact
    const pinBtn = screen.getAllByTitle('Pin to every message')[0];
    fireEvent.click(pinBtn);
    expect(onPin).toHaveBeenCalledWith('fact-x');
  });

  it('calls onUnpin when unpin button clicked for pinned fact', () => {
    const fact = makeFact({ id: 'fact-y', pinned: true, tabId: '__pinned__', text: 'Pinned fact to unpin here' });
    const { onUnpin } = renderPanel([fact]);
    openPanel();
    const unpinBtn = screen.getByTitle('Unpin');
    fireEvent.click(unpinBtn);
    expect(onUnpin).toHaveBeenCalledWith('fact-y');
  });

  // ── Delete ───────────────────────────────────────────────────────────────

  it('calls onDelete when × button clicked', () => {
    const fact = makeFact({ id: 'fact-del' });
    const { onDelete } = renderPanel([fact]);
    openPanel();
    fireEvent.click(screen.getByTitle('Delete'));
    expect(onDelete).toHaveBeenCalledWith('fact-del');
  });

  // ── Inline editing ───────────────────────────────────────────────────────

  it('enters edit mode when fact text is clicked', () => {
    const fact = makeFact({ text: 'Click me to edit this fact text' });
    renderPanel([fact]);
    openPanel();
    fireEvent.click(screen.getByText('Click me to edit this fact text'));
    expect(screen.getByDisplayValue('Click me to edit this fact text')).toBeInTheDocument();
  });

  it('calls onEdit when Enter pressed in edit input', () => {
    const fact = makeFact({ id: 'edit-fact', text: 'Original fact text content here' });
    const { onEdit } = renderPanel([fact]);
    openPanel();
    fireEvent.click(screen.getByText('Original fact text content here'));
    const input = screen.getByDisplayValue('Original fact text content here');
    fireEvent.change(input, { target: { value: 'Updated fact text content here' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onEdit).toHaveBeenCalledWith('edit-fact', 'Updated fact text content here');
  });

  it('calls onEdit when edit input loses focus', () => {
    const fact = makeFact({ id: 'edit-blur', text: 'Blur test fact text content here' });
    const { onEdit } = renderPanel([fact]);
    openPanel();
    fireEvent.click(screen.getByText('Blur test fact text content here'));
    const input = screen.getByDisplayValue('Blur test fact text content here');
    fireEvent.change(input, { target: { value: 'Changed by blur event here' } });
    fireEvent.blur(input);
    expect(onEdit).toHaveBeenCalledWith('edit-blur', 'Changed by blur event here');
  });

  it('cancels edit mode when Escape pressed', () => {
    const fact = makeFact({ text: 'Fact text that will not be edited' });
    const { onEdit } = renderPanel([fact]);
    openPanel();
    fireEvent.click(screen.getByText('Fact text that will not be edited'));
    const input = screen.getByDisplayValue('Fact text that will not be edited');
    fireEvent.keyDown(input, { key: 'Escape' });
    expect(onEdit).not.toHaveBeenCalled();
    // Back to display mode
    expect(screen.getByText('Fact text that will not be edited')).toBeInTheDocument();
  });

  // ── Add manual note ──────────────────────────────────────────────────────

  it('calls onAddManual when Add button clicked', () => {
    const { onAddManual } = renderPanel();
    openPanel();
    const input = screen.getByPlaceholderText('Add a note to memory...');
    fireEvent.change(input, { target: { value: 'A manually entered note fact' } });
    fireEvent.click(screen.getByText('Add'));
    expect(onAddManual).toHaveBeenCalledWith('A manually entered note fact');
  });

  it('calls onAddManual when Enter pressed in note input', () => {
    const { onAddManual } = renderPanel();
    openPanel();
    const input = screen.getByPlaceholderText('Add a note to memory...');
    fireEvent.change(input, { target: { value: 'Enter key adds this note' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onAddManual).toHaveBeenCalledWith('Enter key adds this note');
  });

  it('Add button is disabled when input is empty', () => {
    renderPanel();
    openPanel();
    expect(screen.getByText('Add')).toBeDisabled();
  });

  it('Add button is enabled when input has text', () => {
    renderPanel();
    openPanel();
    fireEvent.change(screen.getByPlaceholderText('Add a note to memory...'), {
      target: { value: 'some text' },
    });
    expect(screen.getByText('Add')).not.toBeDisabled();
  });

  it('does not call onAddManual for empty input via button click', () => {
    const { onAddManual } = renderPanel();
    openPanel();
    fireEvent.click(screen.getByText('Add'));
    expect(onAddManual).not.toHaveBeenCalled();
  });

  // ── Multiple facts ───────────────────────────────────────────────────────

  it('renders multiple session facts', () => {
    const facts = [
      makeFact({ id: '1', text: 'First fact displayed in memory panel' }),
      makeFact({ id: '2', text: 'Second fact displayed in memory panel' }),
      makeFact({ id: '3', text: 'Third fact displayed in memory panel' }),
    ];
    renderPanel(facts);
    openPanel();
    expect(screen.getByText('First fact displayed in memory panel')).toBeInTheDocument();
    expect(screen.getByText('Second fact displayed in memory panel')).toBeInTheDocument();
    expect(screen.getByText('Third fact displayed in memory panel')).toBeInTheDocument();
  });

  it('count badge reflects both pinned and session facts', () => {
    const facts = [
      makeFact({ id: '1', pinned: true, tabId: '__pinned__', text: 'Pinned count test' }),
      makeFact({ id: '2', tabId: 'tab-1', text: 'Session count test here' }),
      makeFact({ id: '3', tabId: 'tab-1', text: 'Another session count test' }),
    ];
    renderPanel(facts);
    // 1 pinned + 2 session = 3 total
    expect(screen.getByText('3')).toBeInTheDocument();
  });
});
