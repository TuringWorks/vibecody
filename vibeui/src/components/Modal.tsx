import React, { useState, useEffect, useRef, useCallback } from 'react';
import './Modal.css';

interface ModalProps {
    isOpen: boolean;
    title: string;
    message?: string;
    placeholder?: string;
    initialValue?: string;
    onConfirm: (value: string) => void;
    onCancel: () => void;
}

const Modal: React.FC<ModalProps> = ({
    isOpen,
    title,
    message,
    placeholder,
    initialValue = '',
    onConfirm,
    onCancel,
}) => {
    const [value, setValue] = useState(initialValue);
    const inputRef = useRef<HTMLInputElement>(null);
    const overlayRef = useRef<HTMLDivElement>(null);
    const previousFocusRef = useRef<Element | null>(null);

    useEffect(() => {
        if (isOpen) {
            previousFocusRef.current = document.activeElement;
            setValue(initialValue);
            setTimeout(() => inputRef.current?.focus(), 50);
        } else if (previousFocusRef.current instanceof HTMLElement) {
            previousFocusRef.current.focus();
            previousFocusRef.current = null;
        }
    }, [isOpen, initialValue]);

    const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
        if (e.key === 'Escape') {
            e.preventDefault();
            onCancel();
            return;
        }
        // Focus trap: cycle Tab within modal
        if (e.key === 'Tab' && overlayRef.current) {
            const focusable = overlayRef.current.querySelectorAll<HTMLElement>(
                'input, button, [tabindex]:not([tabindex="-1"])'
            );
            if (focusable.length === 0) return;
            const first = focusable[0];
            const last = focusable[focusable.length - 1];
            if (e.shiftKey && document.activeElement === first) {
                e.preventDefault();
                last.focus();
            } else if (!e.shiftKey && document.activeElement === last) {
                e.preventDefault();
                first.focus();
            }
        }
    }, [onCancel]);

    if (!isOpen) return null;

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        onConfirm(value);
    };

    return (
        <div
            className="modal-overlay"
            ref={overlayRef}
            role="dialog"
            aria-modal="true"
            aria-labelledby="modal-title"
            onKeyDown={handleKeyDown}
        >
            <div className="modal-content">
                <h3 id="modal-title">{title}</h3>
                {message && <p>{message}</p>}
                <form onSubmit={handleSubmit}>
                    <input
                        ref={inputRef}
                        type="text"
                        value={value}
                        onChange={(e) => setValue(e.target.value)}
                        placeholder={placeholder}
                        className="modal-input"
                    />
                    <div className="modal-actions">
                        <button type="button" className="btn-secondary" onClick={onCancel}>
                            Cancel
                        </button>
                        <button type="submit" className="btn-primary">
                            Confirm
                        </button>
                    </div>
                </form>
            </div>
        </div>
    );
};

export default Modal;
