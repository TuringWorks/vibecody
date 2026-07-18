import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { MarkdownPreview } from '../MarkdownPreview';

describe('MarkdownPreview', () => {
    it('renders standard markdown', () => {
        const { container } = render(<MarkdownPreview content="# Hello World" />);
        expect(screen.getByText('Hello World')).toBeInTheDocument();
        expect(container.querySelector('h1')).toBeInTheDocument();
    });

    it('renders a markdown table with our custom logic', () => {
        const markdown = `
# Title
Some text

| Header 1 | Header 2 |
|---|---|
| Row A | Row B |
| Data 1 | Data 2 |
`;
        const { container } = render(<MarkdownPreview content={markdown} />);
        
        // It should render a standard HTML table
        const table = container.querySelector('table');
        expect(table).toBeInTheDocument();
        
        // Check headers
        expect(screen.getByText('Header 1')).toBeInTheDocument();
        expect(screen.getByText('Header 2')).toBeInTheDocument();
        expect(container.querySelectorAll('th').length).toBe(2);

        // Check cells
        expect(screen.getByText('Row A')).toBeInTheDocument();
        expect(screen.getByText('Data 2')).toBeInTheDocument();
        expect(container.querySelectorAll('td').length).toBe(4);
    });

    it('allows inline markdown formatting inside table cells', () => {
        const markdown = `
| Name | Description |
|---|---|
| **Bold** | *Italic* and [Link](#) |
`;
        const { container } = render(<MarkdownPreview content={markdown} />);
        
        // Verify strong tag
        const strong = container.querySelector('strong');
        expect(strong).toBeInTheDocument();
        expect(strong?.textContent).toBe('Bold');
        
        // Verify em tag
        const em = container.querySelector('em');
        expect(em).toBeInTheDocument();
        expect(em?.textContent).toBe('Italic');
        
        // Verify anchor tag
        const a = container.querySelector('a');
        expect(a).toBeInTheDocument();
        expect(a?.textContent).toBe('Link');
    });

    it('handles tables that start without exact pipes on edges', () => {
        const markdown = `
Header A | Header B
---|---
Cell 1 | Cell 2
`;
        render(<MarkdownPreview content={markdown} />);
        
        expect(screen.getByText('Header A')).toBeInTheDocument();
        expect(screen.getByText('Cell 2')).toBeInTheDocument();
    });

    it('strips YAML frontmatter', () => {
        const markdown = `---
layout: page
title: Shannon vs VibeCody
---
# Main Content
This is the main content.`;
        const { container } = render(<MarkdownPreview content={markdown} />);
        
        expect(screen.getByText('Main Content')).toBeInTheDocument();
        expect(container.querySelector('hr')).not.toBeInTheDocument();
        // The word "Shannon vs VibeCody" (part of frontmatter) should not be present
        expect(screen.queryByText(/layout:/)).not.toBeInTheDocument();
    });
});
