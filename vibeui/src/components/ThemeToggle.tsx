import { useEffect, useState } from 'react';

export const ThemeToggle = () => {
    const [theme, setTheme] = useState<'dark' | 'light'>('dark');

    // Initialize theme from localStorage or default to dark
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
            title="Toggle theme"
            style={{ background: 'none', border: 'none', color: 'inherit', cursor: 'pointer' }}
        >
            {theme === 'dark' ? 'Moon' : 'Sun'}
        </button>
    );
};
