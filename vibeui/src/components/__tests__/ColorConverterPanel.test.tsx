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
});
