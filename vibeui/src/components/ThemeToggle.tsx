import { useEffect, useState } from 'react';
import { Sun, Moon } from 'lucide-react';

export const ThemeToggle = () => {
    const [theme, setTheme] = useState<'dark' | 'light'>('dark');

    useEffect(() => {
        const stored = localStorage.getItem('vibeui-theme') as 'dark' | 'light' | null;
        const initial = stored ?? 'dark';
        setTheme(initial);
        document.documentElement.setAttribute('data-theme', initial);
    }, []);

    const toggleTheme = () => {
        const newTheme = theme === 'dark' ? 'light' : 'dark';
        setTheme(newTheme);
        document.documentElement.setAttribute('data-theme', newTheme);
        localStorage.setItem('vibeui-theme', newTheme);
    };

    return (
        <button
            className="icon-button"
            onClick={toggleTheme}
            title={`Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`}
            aria-label={`Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`}
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
            {theme === 'dark' ? (
                <Moon size={16} strokeWidth={2} style={{
                    transition: 'transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1), opacity 0.2s',
                    color: 'var(--accent-blue)',
                }} />
            ) : (
                <Sun size={16} strokeWidth={2} style={{
                    transition: 'transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1), opacity 0.2s',
                    color: 'var(--accent-gold, #d4a017)',
                }} />
            )}
        </button>
    );
};
