import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { DrawioPreview } from '../DrawioPreview';

describe('DrawioPreview', () => {
    const mockContent = '<mxfile><diagram><mxCell/></diagram></mxfile>';
    
    beforeEach(() => {
        // Clear mock calls and restore implementations
        vi.clearAllMocks();
    });

    it('renders the iframe pointing to the correct viewer URL', () => {
        render(<DrawioPreview content={mockContent} filePath="test.drawio" />);
        
        const iframe = document.querySelector('.html-preview-iframe') as HTMLIFrameElement;
        expect(iframe).toBeInTheDocument();
        expect(iframe.src).toContain('https://viewer.diagrams.net/');
        expect(iframe.src).toContain('title=test.drawio');
    });

    it('asks the viewer to speak the embed protocol at all', () => {
        // Without these the viewer never emits `init`, so the `init` handler
        // below — which was already here — could not run and the preview showed
        // an empty frame with a spinner and no error.
        render(<DrawioPreview content={mockContent} filePath="test.drawio" />);

        const iframe = document.querySelector('.html-preview-iframe') as HTMLIFrameElement;
        expect(iframe.src).toContain('embed=1');
        expect(iframe.src).toContain('proto=json');
    });

    it('acknowledges configure, which is what unblocks init', () => {
        // With `configure=1` the viewer blocks until the host replies. No
        // reply, no `init`; no `init`, no diagram.
        render(<DrawioPreview content={mockContent} filePath="test.drawio" />);

        const iframe = document.querySelector('.html-preview-iframe') as HTMLIFrameElement;
        const postMessageMock = vi.fn();
        Object.defineProperty(iframe, 'contentWindow', {
            value: { postMessage: postMessageMock },
            writable: true
        });

        fireEvent(window, new MessageEvent('message', {
            source: iframe.contentWindow,
            data: JSON.stringify({ event: 'configure' })
        }));

        expect(postMessageMock).toHaveBeenCalledTimes(1);
        expect(JSON.parse(postMessageMock.mock.calls[0][0]).action).toBe('configure');
    });

    it('does not put the diagram in the URL, which has a length limit', () => {
        // A `#R…` fragment carrying the whole diagram stops working somewhere
        // past a few thousand shapes, and fails silently.
        const big = '<mxfile>' + 'x'.repeat(20000) + '</mxfile>';
        render(<DrawioPreview content={big} filePath="big.drawio" />);

        const iframe = document.querySelector('.html-preview-iframe') as HTMLIFrameElement;
        expect(iframe.src.length).toBeLessThan(500);
        expect(iframe.src).not.toContain('#R');
    });

    it('sends the correct postMessage payload when init event is received', () => {
        render(<DrawioPreview content={mockContent} filePath="test.drawio" />);
        
        const iframe = document.querySelector('.html-preview-iframe') as HTMLIFrameElement;
        expect(iframe).toBeInTheDocument();
        
        // Mock the iframe's contentWindow and its postMessage method
        const postMessageMock = vi.fn();
        Object.defineProperty(iframe, 'contentWindow', {
            value: { postMessage: postMessageMock },
            writable: true
        });

        // Simulate the init message from the Draw.io viewer
        fireEvent(window, new MessageEvent('message', {
            source: iframe.contentWindow,
            data: JSON.stringify({ event: 'init' })
        }));

        // The component should respond with the 'load' action and our content
        expect(postMessageMock).toHaveBeenCalledTimes(1);
        const payload = JSON.parse(postMessageMock.mock.calls[0][0]);
        expect(payload.action).toBe('load');
        expect(payload.xml).toBe(mockContent);
    });

    it('ignores unrelated messages', () => {
        render(<DrawioPreview content={mockContent} />);
        
        const iframe = document.querySelector('.html-preview-iframe') as HTMLIFrameElement;
        
        const postMessageMock = vi.fn();
        Object.defineProperty(iframe, 'contentWindow', {
            value: { postMessage: postMessageMock },
            writable: true
        });

        // Simulate a message from a different source
        fireEvent(window, new MessageEvent('message', {
            source: window, // Not from our iframe
            data: JSON.stringify({ event: 'init' })
        }));

        expect(postMessageMock).not.toHaveBeenCalled();

        // Simulate an unrelated message from the iframe
        fireEvent(window, new MessageEvent('message', {
            source: iframe.contentWindow,
            data: JSON.stringify({ event: 'unrelated_event' })
        }));

        expect(postMessageMock).not.toHaveBeenCalled();
    });

    it('renders device toggle buttons allowing preview resizing', () => {
        render(<DrawioPreview content={mockContent} />);
        
        // Desktop button should exist
        const desktopBtn = screen.getByTitle('Desktop');
        expect(desktopBtn).toBeInTheDocument();
        
        fireEvent.click(desktopBtn);
        
        const wrapper = document.querySelector('.html-preview-frame-wrapper');
        expect(wrapper).toHaveClass('device-desktop');
    });
});
