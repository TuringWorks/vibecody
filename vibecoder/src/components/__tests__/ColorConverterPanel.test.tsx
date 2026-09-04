import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { ColorConverterPanel } from '../ColorConverterPanel';

describe('ColorConverterPanel', () => {
  it('renders without crashing', () => {
    render(<ColorConverterPanel />);
    expect(screen.getByText('Color Converter')).toBeInTheDocument();
  });

  it('displays default hex value', () => {
    render(<ColorConverterPanel />);
    const input = screen.getByDisplayValue('#89B4FA');
    expect(input).toBeInTheDocument();
  });

  it('switches between sub-tabs', () => {
    render(<ColorConverterPanel />);
    fireEvent.click(screen.getByText('Tints & Shades'));
    fireEvent.click(screen.getByText('Contrast'));
    fireEvent.click(screen.getByText('CSS Snippets'));
    fireEvent.click(screen.getByText('Convert'));
  });

  it('converts a short hex color into synchronized RGB and HSL values', () => {
    render(<ColorConverterPanel />);
    fireEvent.change(screen.getByRole('textbox', { name: 'Hex color' }), { target: { value: '#fff' } });

    expect(screen.getByText('rgb(255, 255, 255)')).toBeInTheDocument();
    expect(screen.getByText('hsl(0, 0%, 100%)')).toBeInTheDocument();
  });

  it('updates alpha-dependent formats and snippets', () => {
    render(<ColorConverterPanel />);
    fireEvent.change(screen.getByRole('slider', { name: 'Alpha' }), { target: { value: '50' } });

    expect(screen.getByText('rgba(137, 180, 250, 0.50)')).toBeInTheDocument();
    fireEvent.click(screen.getByText('CSS Snippets'));
    expect(screen.getByText(/rgba\(137, 180, 250, 0\.50\)/)).toBeInTheDocument();
  });

  it('shows WCAG contrast ratios against white, black, and a custom background', () => {
    render(<ColorConverterPanel />);
    fireEvent.click(screen.getByText('Contrast'));

    expect(screen.getAllByText(/^\d+\.\d{2}:1$/).length).toBe(3);
    expect(screen.getByText(/WCAG 2\.1 thresholds/)).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Custom background color' })).toHaveValue('#1E1E2E');
  });

  it('clicking a tint swatch changes the active color', () => {
    render(<ColorConverterPanel />);
    fireEvent.click(screen.getByText('Tints & Shades'));
    const swatch = screen.getByTitle('#FFFFFF').parentElement!;
    fireEvent.click(swatch);

    expect(screen.getByRole('textbox', { name: 'Hex color' })).toHaveValue('#FFFFFF');
  });
});
