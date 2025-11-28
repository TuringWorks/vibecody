import React, { useState, useEffect, useRef } from 'react';
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

    useEffect(() => {
        if (isOpen) {
            setValue(initialValue);
            setTimeout(() => inputRef.current?.focus(), 50);
        }
    }, [isOpen, initialValue]);

    if (!isOpen) return null;

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        onConfirm(value);
    };

    return (
        <div className="modal-overlay">
            <div className="modal-content">
                <h3>{title}</h3>
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
