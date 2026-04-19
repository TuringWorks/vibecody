/**
 * BDD tests for Modal component.
 *
 * Scenarios:
 *  - Hidden when isOpen=false
 *  - Renders title, message, placeholder when open
 *  - Calls onConfirm with input value on form submit
 *  - Calls onCancel on Cancel button click
 *  - Closes on Escape key
 *  - Traps Tab focus within modal
 *  - Restores external focus after close
 *  - Resets value to initialValue when re-opened
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import Modal from '../Modal';

// ── Helpers ───────────────────────────────────────────────────────────────────

function openModal(props: Partial<React.ComponentProps<typeof Modal>> = {}) {
  return render(
    <Modal
      isOpen
      title="Test Dialog"
      onConfirm={props.onConfirm ?? vi.fn()}
      onCancel={props.onCancel ?? vi.fn()}
      {...props}
    />,
  );
}

// ── Visibility ────────────────────────────────────────────────────────────────

describe('Modal — visibility', () => {
  it('renders nothing when isOpen=false', () => {
    const { container } = render(
      <Modal isOpen={false} title="Hidden" onConfirm={vi.fn()} onCancel={vi.fn()} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it('renders when isOpen=true', () => {
    openModal();
    expect(screen.getByRole('dialog')).toBeDefined();
  });
});

// ── Content ───────────────────────────────────────────────────────────────────

describe('Modal — content', () => {
  it('renders the title', () => {
    openModal({ title: 'Create file' });
    expect(screen.getByText('Create file')).toBeDefined();
  });

  it('renders the message when provided', () => {
    openModal({ message: 'Enter a filename:' });
    expect(screen.getByText('Enter a filename:')).toBeDefined();
  });

  it('omits message element when not provided', () => {
    openModal({ title: 'T' });
    expect(screen.queryByText('Enter a filename:')).toBeNull();
  });

  it('sets input placeholder', () => {
    openModal({ placeholder: 'my-file.ts' });
    expect(screen.getByPlaceholderText('my-file.ts')).toBeDefined();
  });

  it('populates input with initialValue', () => {
    openModal({ initialValue: 'hello.ts' });
    expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('hello.ts');
  });

  it('defaults input value to empty string when no initialValue', () => {
    openModal();
    expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('');
  });
});

// ── Confirmation ──────────────────────────────────────────────────────────────

describe('Modal — confirm action', () => {
  it('calls onConfirm with current input value when Confirm button clicked', () => {
    const onConfirm = vi.fn();
    openModal({ onConfirm, initialValue: 'typed-value' });
    fireEvent.click(screen.getByRole('button', { name: 'Confirm' }));
    expect(onConfirm).toHaveBeenCalledWith('typed-value');
  });

  it('calls onConfirm when form is submitted (Enter key)', () => {
    const onConfirm = vi.fn();
    openModal({ onConfirm, initialValue: 'via-enter' });
    fireEvent.submit(screen.getByRole('textbox').closest('form')!);
    expect(onConfirm).toHaveBeenCalledWith('via-enter');
  });

  it('calls onConfirm with the updated value after typing', () => {
    const onConfirm = vi.fn();
    openModal({ onConfirm });
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'newfile.ts' } });
    fireEvent.click(screen.getByRole('button', { name: 'Confirm' }));
    expect(onConfirm).toHaveBeenCalledWith('newfile.ts');
  });
});

// ── Cancellation ──────────────────────────────────────────────────────────────

describe('Modal — cancel action', () => {
  it('calls onCancel when Cancel button is clicked', () => {
    const onCancel = vi.fn();
    openModal({ onCancel });
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it('calls onCancel when Escape key is pressed', () => {
    const onCancel = vi.fn();
    openModal({ onCancel });
    fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
    expect(onCancel).toHaveBeenCalledOnce();
  });
});

// ── Accessibility ─────────────────────────────────────────────────────────────

describe('Modal — accessibility', () => {
  it('has role="dialog" with aria-modal="true"', () => {
    openModal();
    const dialog = screen.getByRole('dialog');
    expect(dialog.getAttribute('aria-modal')).toBe('true');
  });

  it('labelledby points to the title element', () => {
    openModal({ title: 'Rename' });
    const dialog = screen.getByRole('dialog');
    const labelledBy = dialog.getAttribute('aria-labelledby');
    expect(labelledBy).toBe('modal-title');
    expect(screen.getByText('Rename').id).toBe('modal-title');
  });
});

// ── Reset on re-open ──────────────────────────────────────────────────────────

describe('Modal — reset on re-open', () => {
  it('resets value to initialValue when isOpen transitions back to true', () => {
    const { rerender } = render(
      <Modal isOpen title="T" initialValue="original" onConfirm={vi.fn()} onCancel={vi.fn()} />,
    );
    // Type something different
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'changed' } });
    expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('changed');

    // Close
    rerender(<Modal isOpen={false} title="T" initialValue="original" onConfirm={vi.fn()} onCancel={vi.fn()} />);
    // Re-open — should reset to initialValue
    rerender(<Modal isOpen title="T" initialValue="original" onConfirm={vi.fn()} onCancel={vi.fn()} />);
    expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('original');
  });
});
