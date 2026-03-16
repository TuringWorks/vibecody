import { useEffect, useState } from 'react';
import { Sun, Moon } from 'lucide-react';
import { getPairedTheme, applyThemeById } from './SettingsPanel';

export const ThemeToggle = () => {
    const [mode, setMode] = useState<'dark' | 'light'>('dark');

    useEffect(() => {
        const stored = localStorage.getItem('vibeui-theme') as 'dark' | 'light' | null;
        const initial = stored ?? 'dark';
        setMode(initial);
        document.documentElement.setAttribute('data-theme', initial);
    }, []);

    const toggleTheme = () => {
        const currentId = localStorage.getItem('vibeui-theme-id') || (mode === 'dark' ? 'dark-default' : 'light-default');
        const paired = getPairedTheme(currentId);
        if (paired) {
            applyThemeById(paired.id);
            setMode(paired.mode);
        } else {
            // Fallback: simple dark/light toggle with default pair
            const newMode = mode === 'dark' ? 'light' : 'dark';
            const fallbackId = newMode === 'dark' ? 'dark-default' : 'light-default';
            applyThemeById(fallbackId);
            setMode(newMode);
        }
    };

    return (
        <button
            className="icon-button"
            onClick={toggleTheme}
            title={`Switch to ${mode === 'dark' ? 'light' : 'dark'} mode`}
            aria-label={`Switch to ${mode === 'dark' ? 'light' : 'dark'} mode`}
            style={{
                background: 'none',
                border: 'none',
                color: 'inherit',
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition: 'transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1)',
                width: 32,
                height: 32,
            }}
        >
            {mode === 'dark' ? (
                <Moon size={16} strokeWidth={2} style={{
                    transition: 'transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1), opacity 0.2s',
                    color: 'var(--accent-blue)',
                }} />
            ) : (
                <Sun size={16} strokeWidth={2} style={{
                    transition: 'transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1), opacity 0.2s',
                    color: 'var(--accent-gold)',
                }} />
            )}
        </button>
    );
};
