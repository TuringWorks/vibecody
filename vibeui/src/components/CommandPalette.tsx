import { useState, useEffect, useRef } from 'react';
import Fuse from 'fuse.js';
import './CommandPalette.css';

export interface Command {
    id: string;
    label: string;
    category: string;
    icon?: string;
    shortcut?: string;
    action: () => void;
}

interface CommandPaletteProps {
    isOpen: boolean;
    onClose: () => void;
    commands: Command[];
}

export const CommandPalette = ({ isOpen, onClose, commands }: CommandPaletteProps) => {
    const [searchQuery, setSearchQuery] = useState('');
    const [selectedIndex, setSelectedIndex] = useState(0);
    const [filteredCommands, setFilteredCommands] = useState<Command[]>(commands);
    const inputRef = useRef<HTMLInputElement>(null);
    const listRef = useRef<HTMLDivElement>(null);

    // Initialize Fuse.js for fuzzy search
    const fuse = useRef(
        new Fuse(commands, {
            keys: ['label', 'category'],
            threshold: 0.3,
            includeScore: true,
        })
    );

    // Update fuse instance when commands change
    useEffect(() => {
        fuse.current = new Fuse(commands, {
            keys: ['label', 'category'],
            threshold: 0.3,
            includeScore: true,
        });
    }, [commands]);

    // Filter commands based on search query
    useEffect(() => {
        if (searchQuery.trim() === '') {
            setFilteredCommands(commands);
        } else {
            const results = fuse.current.search(searchQuery);
            setFilteredCommands(results.map(result => result.item));
        }
        setSelectedIndex(0);
    }, [searchQuery, commands]);

    // Focus input when opened
    useEffect(() => {
        if (isOpen && inputRef.current) {
            inputRef.current.focus();
            setSearchQuery('');
            setSelectedIndex(0);
        }
    }, [isOpen]);

    // Keyboard navigation
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (!isOpen) return;

            switch (e.key) {
                case 'ArrowDown':
                    e.preventDefault();
                    setSelectedIndex(prev =>
                        prev < filteredCommands.length - 1 ? prev + 1 : prev
                    );
                    break;
                case 'ArrowUp':
                    e.preventDefault();
                    setSelectedIndex(prev => (prev > 0 ? prev - 1 : 0));
                    break;
                case 'Enter':
                    e.preventDefault();
                    if (filteredCommands[selectedIndex]) {
                        executeCommand(filteredCommands[selectedIndex]);
                    }
                    break;
                case 'Escape':
                    e.preventDefault();
                    onClose();
                    break;
            }
        };

        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [isOpen, selectedIndex, filteredCommands, onClose]);

    // Scroll selected item into view
    useEffect(() => {
        if (listRef.current) {
            const selectedElement = listRef.current.children[selectedIndex] as HTMLElement;
            if (selectedElement) {
                selectedElement.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
            }
        }
    }, [selectedIndex]);

    const executeCommand = (command: Command) => {
        command.action();
        onClose();
    };

    const groupedCommands = filteredCommands.reduce((acc, command) => {
        if (!acc[command.category]) {
            acc[command.category] = [];
        }
        acc[command.category].push(command);
        return acc;
    }, {} as Record<string, Command[]>);

    if (!isOpen) return null;

    const activeDescendant = filteredCommands[selectedIndex]?.id
        ? `cmd-${filteredCommands[selectedIndex].id}`
        : undefined;

    return (
        <div className="command-palette-overlay" role="dialog" aria-modal="true" aria-label="Command Palette" onClick={onClose}>
            <div className="command-palette" onClick={(e) => e.stopPropagation()}>
                <div className="command-palette-header">
                    <input
                        ref={inputRef}
                        type="text"
                        className="command-palette-input"
                        placeholder="Type a command or search..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        role="combobox"
                        aria-expanded="true"
                        aria-controls="command-palette-listbox"
                        aria-activedescendant={activeDescendant}
                        aria-autocomplete="list"
                    />
                </div>
                <div className="command-palette-list" ref={listRef} id="command-palette-listbox" role="listbox">
                    {Object.keys(groupedCommands).length === 0 ? (
                        <div className="command-palette-empty">No commands found</div>
                    ) : (
                        Object.entries(groupedCommands).map(([category, categoryCommands]) => (
                            <div key={category} className="command-category">
                                <div className="command-category-header">{category}</div>
                                {categoryCommands.map((command) => {
                                    const globalIndex = filteredCommands.indexOf(command);
                                    return (
                                        <div
                                            key={command.id}
                                            id={`cmd-${command.id}`}
                                            role="option"
                                            aria-selected={globalIndex === selectedIndex}
                                            className={`command-item ${globalIndex === selectedIndex ? 'selected' : ''}`}
                                            onClick={() => executeCommand(command)}
                                            onMouseEnter={() => setSelectedIndex(globalIndex)}
                                        >
                                            <div className="command-item-left">
                                                {command.icon && <span className="command-icon">{command.icon}</span>}
                                                <span className="command-label">{command.label}</span>
                                            </div>
                                            {command.shortcut && (
                                                <span className="command-shortcut">{command.shortcut}</span>
                                            )}
                                        </div>
                                    );
                                })}
                            </div>
                        ))
                    )}
                </div>
                <div className="command-palette-footer">
                    <span>↑↓ Navigate</span>
                    <span>↵ Execute</span>
                    <span>Esc Close</span>
                </div>
            </div>
        </div>
    );
};
