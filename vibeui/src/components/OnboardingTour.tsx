import { useState, useEffect, useCallback } from 'react';

interface TourStep {
  target: string; // CSS selector
  title: string;
  description: string;
}

const STEPS: TourStep[] = [
  {
    target: '.activity-bar',
    title: 'Activity Bar',
    description: 'Switch between Explorer, Search, and Source Control. Use keyboard shortcuts like ⌘⇧E and ⌘⇧G for quick access.',
  },
  {
    target: '.ai-selector',
    title: 'AI Provider',
    description: 'Select your AI provider here — Ollama, Claude, ChatGPT, Gemini, Grok, and more are supported.',
  },
  {
    target: '.btn-secondary',
    title: 'AI Panel',
    description: 'Toggle the AI panel with ⌘J. It has 30+ tabs: Chat, Agent, Code Review, Deploy, and more.',
  },
  {
    target: '.status-bar',
    title: 'Status Bar',
    description: 'Quick access to Terminal (⌘`), Browser, theme toggle, and the Command Palette (⌘K).',
  },
];

interface OnboardingTourProps {
  onComplete: () => void;
}

export function OnboardingTour({ onComplete }: OnboardingTourProps) {
  const [step, setStep] = useState(0);
  const [rect, setRect] = useState<DOMRect | null>(null);

  const updateRect = useCallback(() => {
    const el = document.querySelector(STEPS[step].target);
    if (el) {
      setRect(el.getBoundingClientRect());
    }
  }, [step]);

  useEffect(() => {
    updateRect();
    window.addEventListener('resize', updateRect);
    return () => window.removeEventListener('resize', updateRect);
  }, [updateRect]);

  const current = STEPS[step];
  const isLast = step === STEPS.length - 1;

  // Position tooltip near target
  const tooltipStyle: React.CSSProperties = {};
  if (rect) {
    const pad = 12;
    // Try to place below, fallback to above
    if (rect.bottom + 200 < window.innerHeight) {
      tooltipStyle.top = rect.bottom + pad;
    } else {
      tooltipStyle.bottom = window.innerHeight - rect.top + pad;
    }
    // Horizontally: align left edge with target, but clamp
    tooltipStyle.left = Math.max(16, Math.min(rect.left, window.innerWidth - 340));
  }

  return (
    <>
      {/* Overlay */}
      <div className="tour-overlay" />
      {/* Spotlight */}
      {rect && (
        <div
          className="tour-spotlight"
          style={{
            top: rect.top - 4,
            left: rect.left - 4,
            width: rect.width + 8,
            height: rect.height + 8,
          }}
        />
      )}
      {/* Tooltip */}
      <div className="tour-tooltip" style={tooltipStyle} role="dialog" aria-label="Onboarding tour">
        <h4>{current.title}</h4>
        <p>{current.description}</p>
        <div className="tour-steps">Step {step + 1} of {STEPS.length}</div>
        <div className="tour-actions">
          <button
            className="btn-secondary"
            onClick={onComplete}
            style={{ fontSize: '12px', padding: '4px 12px' }}
          >
            Skip Tour
          </button>
          <button
            className="btn-primary"
            onClick={() => {
              if (isLast) {
                onComplete();
              } else {
                setStep(step + 1);
              }
            }}
            style={{ fontSize: '12px', padding: '4px 12px' }}
          >
            {isLast ? 'Get Started' : 'Next'}
          </button>
        </div>
      </div>
    </>
  );
}
