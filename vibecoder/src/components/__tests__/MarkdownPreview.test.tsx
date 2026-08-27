import { fireEvent, render, screen, within } from '@testing-library/react';
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
    it('renders a generated answer as a real click-to-reveal disclosure', () => {
        // A generated study guide hides answers behind <details>; react-markdown
        // has no raw-HTML pipeline, so the tags used to reach the reader as text
        // *and* the answer was visible anyway. Now it is the real element.
        const markdown = `### 1. How does a VLM see an image?

<details><summary><b>Answer</b></summary>

Three components: a **vision encoder**, a projector, and the LLM.

</details>`;
        const { container } = render(<MarkdownPreview content={markdown} />);

        const details = container.querySelector('details');
        expect(details).toBeInTheDocument();
        expect(details).not.toHaveAttribute('open');
        expect(container.textContent).not.toContain('<details>');
        expect(container.textContent).not.toContain('<summary>');
        expect(container.textContent).not.toContain('<b>');
        expect(within(details!).getByText('Answer')).toBeInTheDocument();
        expect(details!.querySelector('strong')?.textContent).toBe('Answer');
        // The answer is inside the disclosure, not loose in the document.
        expect(details!.textContent).toContain('Three components');
    });

    it('reveals the answer when the summary is clicked', () => {
        const markdown = '<details><summary>Answer</summary>\n\nHidden until asked for.\n\n</details>';
        const { container, rerender } = render(<MarkdownPreview content={markdown} />);

        const details = container.querySelector('details')!;
        expect(details.open).toBe(false);
        fireEvent.click(container.querySelector('summary')!);
        expect(details.open).toBe(true);

        // Editing text elsewhere must not snap it shut: the markdown editor
        // re-renders its preview on every keystroke, so the disclosure has to
        // keep its identity across a changed document.
        rerender(<MarkdownPreview content={`Some prose.\n\n${markdown}`} />);
        expect(container.querySelector('details')!.open).toBe(true);
    });

    it('keeps a details example written in code as text', () => {
        const markdown = ['```html', '<details><summary>x</summary>', '</details>', '```'].join('\n');
        const { container } = render(<MarkdownPreview content={markdown} />);

        expect(container.querySelector('details')).not.toBeInTheDocument();
        expect(container.textContent).toContain('<details><summary>x</summary>');
    });

    it('opens a relative link as a workspace file, resolved against the document', () => {
        const opened: Array<[string, string | null]> = [];
        const { container } = render(
            <MarkdownPreview
                content="See [Documentation Index](docs/README.md)."
                basePath="/work/repo/AGENTS.md"
                onOpenFile={(path, fragment) => { opened.push([path, fragment]); }}
            />
        );

        fireEvent.click(container.querySelector('a')!);
        expect(opened).toEqual([['/work/repo/docs/README.md', null]]);
    });

    it('carries the fragment of a cross-file link', () => {
        const opened: Array<[string, string | null]> = [];
        const { container } = render(
            <MarkdownPreview
                content="[Answer Style](AGENTS.md#answer-style)"
                basePath="/work/repo/README.md"
                onOpenFile={(path, fragment) => { opened.push([path, fragment]); }}
            />
        );

        fireEvent.click(container.querySelector('a')!);
        expect(opened).toEqual([['/work/repo/AGENTS.md', 'answer-style']]);
    });

    it('leaves an external link to the browser, not to the file opener', () => {
        const opened: string[] = [];
        const { container } = render(
            <MarkdownPreview
                content="[Site](https://example.com/x)"
                basePath="/work/repo/README.md"
                onOpenFile={(path) => { opened.push(path); }}
            />
        );

        fireEvent.click(container.querySelector('a')!);
        expect(opened).toEqual([]);
    });

    it('gives headings the id their own anchors name, and scrolls to one', () => {
        const scrolled: HTMLElement[] = [];
        const original = Element.prototype.scrollIntoView;
        Element.prototype.scrollIntoView = function (this: HTMLElement) { scrolled.push(this); };
        try {
            const { container } = render(
                <MarkdownPreview content={'[Go](#answer-style)\n\n## Answer Style\n\nBody.'} />
            );

            const heading = container.querySelector('h2')!;
            expect(heading.id).toBe('answer-style');

            fireEvent.click(container.querySelector('a')!);
            expect(scrolled).toEqual([heading]);
        } finally {
            Element.prototype.scrollIntoView = original;
        }
    });

    it('does not follow a local link when the surface has no opener', () => {
        // DocumentViewer and the memory panel render markdown with nowhere to
        // open a file to; the click must stay inert rather than look handled.
        const { container } = render(<MarkdownPreview content="[Docs](docs/README.md)" basePath="/work/repo/README.md" />);
        const link = container.querySelector('a')!;
        expect(link.getAttribute('data-link-kind')).toBe('file');
        expect(() => fireEvent.click(link)).not.toThrow();
    });
});
