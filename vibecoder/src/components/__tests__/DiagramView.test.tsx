/**
 * Diagrams on screen: the picture when it draws, and what is shown when it does
 * not.
 *
 * The renderer is mocked — Mermaid measures text against a real layout engine
 * and PlantUML is a subprocess, neither of which exists in jsdom. What is worth
 * pinning here is the part that is ours: that a fenced block in markdown becomes
 * a diagram at all, and that a diagram which fails shows the reason *and the
 * source*, rather than an empty rectangle.
 */
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const renderDiagram = vi.fn();

vi.mock('../../lib/diagrams', async () => {
  const actual = await vi.importActual<typeof import('../../lib/diagrams')>('../../lib/diagrams');
  return {
    ...actual,
    renderDiagram: (...args: unknown[]) => renderDiagram(...args),
    plantUmlRenderer: vi.fn(async () => '/usr/bin/plantuml'),
  };
});

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));

import { DiagramView } from '../DiagramView';
import { MarkdownPreview } from '../MarkdownPreview';

const SVG = '<svg data-testid="drawn"><text>Alice</text></svg>';

beforeEach(() => {
  renderDiagram.mockReset();
  renderDiagram.mockResolvedValue(SVG);
});

describe('DiagramView', () => {
  it('draws the diagram it is given', async () => {
    render(<DiagramView kind="mermaid" source="graph TD; A-->B;" />);

    expect(await screen.findByText('Alice')).toBeInTheDocument();
    expect(renderDiagram).toHaveBeenCalledWith('mermaid', 'graph TD; A-->B;');
  });

  it('shows why a diagram failed, and the source that failed', async () => {
    renderDiagram.mockRejectedValue('Parse error on line 2: expected a node');
    render(<DiagramView kind="mermaid" source="graph TD; A-->" />);

    expect(await screen.findByText(/Parse error on line 2/)).toBeInTheDocument();
    // The source is the thing that has to change, so it stays on screen.
    expect(screen.getByText('graph TD; A-->')).toBeInTheDocument();
  });

  it("passes the renderer's own words through, including how to install it", async () => {
    renderDiagram.mockRejectedValue(
      'PlantUML is not installed. Install it with `brew install plantuml`.',
    );
    render(<DiagramView kind="plantuml" source="@startuml\nA -> B\n@enduml" />);

    expect(await screen.findByText(/brew install plantuml/)).toBeInTheDocument();
  });

  it('says so rather than drawing nothing for an empty diagram', async () => {
    render(<DiagramView kind="mermaid" source="   " />);

    expect(await screen.findByText(/empty/i)).toBeInTheDocument();
    expect(renderDiagram).not.toHaveBeenCalled();
  });

  it('keeps the last picture up while the next one is drawn', async () => {
    const view = render(<DiagramView kind="mermaid" source="graph TD; A-->B;" />);
    await screen.findByText('Alice');

    // A keystroke: the next render is in flight, and the old diagram must not
    // blink out of existence between characters.
    let resolve: (svg: string) => void = () => {};
    renderDiagram.mockReturnValue(new Promise<string>((r) => (resolve = r)));
    view.rerender(<DiagramView kind="mermaid" source="graph TD; A-->B; B-->C;" />);

    expect(screen.getByText('Alice')).toBeInTheDocument();
    resolve('<svg><text>Bob</text></svg>');
    expect(await screen.findByText('Bob')).toBeInTheDocument();
  });
});

describe('diagrams inside markdown', () => {
  it('draws a ```mermaid fence instead of printing it', async () => {
    render(<MarkdownPreview content={'# Title\n\n```mermaid\ngraph TD; A-->B;\n```\n'} />);

    expect(await screen.findByText('Alice')).toBeInTheDocument();
    await waitFor(() => expect(renderDiagram).toHaveBeenCalledWith('mermaid', 'graph TD; A-->B;'));
  });

  it('draws a ```plantuml fence, and its aliases', async () => {
    render(<MarkdownPreview content={'```puml\n@startuml\nA -> B\n@enduml\n```\n'} />);

    await waitFor(() =>
      expect(renderDiagram).toHaveBeenCalledWith('plantuml', '@startuml\nA -> B\n@enduml'),
    );
  });

  it('leaves ordinary code blocks as code', async () => {
    const { container } = render(
      <MarkdownPreview content={'```ts\nconst a: number = 1;\n```\n'} />,
    );

    await waitFor(() => expect(container.querySelector('pre')).not.toBeNull());
    expect(renderDiagram).not.toHaveBeenCalled();
    expect(screen.getByText(/const a: number = 1;/)).toBeInTheDocument();
  });
});
