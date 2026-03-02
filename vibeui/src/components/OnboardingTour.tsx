import { useState, useEffect, useCallback, useRef } from 'react';

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
  // aria-live announcement text — updated on every step change
  const [announcement, setAnnouncement] = useState('');
  const nextBtnRef = useRef<HTMLButtonElement>(null);

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

  // Announce step change to screen readers and focus the Next button
  useEffect(() => {
    const s = STEPS[step];
    setAnnouncement(`Step ${step + 1} of ${STEPS.length}: ${s.title}. ${s.description}`);
    // Move focus to the Next/Get Started button so keyboard users can advance
    nextBtnRef.current?.focus();
  }, [step]);

  // Global keyboard handler for the tour
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onComplete();
      } else if (e.key === 'ArrowRight' || e.key === 'Enter') {
        setStep(prev => {
          if (prev === STEPS.length - 1) { onComplete(); return prev; }
          return prev + 1;
        });
      } else if (e.key === 'ArrowLeft') {
        setStep(prev => Math.max(0, prev - 1));
      }
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [onComplete]);

  const current = STEPS[step];
  const isLast = step === STEPS.length - 1;

  // Position tooltip near target
  const tooltipStyle: React.CSSProperties = {};
  if (rect) {
    const pad = 12;
    if (rect.bottom + 200 < window.innerHeight) {
      tooltipStyle.top = rect.bottom + pad;
    } else {
      tooltipStyle.bottom = window.innerHeight - rect.top + pad;
    }
    tooltipStyle.left = Math.max(16, Math.min(rect.left, window.innerWidth - 340));
  }

  return (
    <>
      {/* Screen-reader live region announces step changes */}
      <div aria-live="polite" aria-atomic="true" className="sr-only">
        {announcement}
      </div>

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
      <div
        className="tour-tooltip"
        style={tooltipStyle}
        role="dialog"
        aria-modal="true"
        aria-labelledby="tour-title"
        aria-describedby="tour-desc"
      >
        <h4 id="tour-title">{current.title}</h4>
        <p id="tour-desc">{current.description}</p>

        {/* Visually rendered step counter */}
        <div className="tour-steps" aria-hidden="true">
          Step {step + 1} of {STEPS.length}
        </div>
        {/* Screen-reader only counter (redundant with aria-live, but ensures AT parse it) */}
        <span className="sr-only">Step {step + 1} of {STEPS.length}</span>

        <div className="tour-actions">
          {step > 0 && (
            <button
              className="btn-secondary"
              onClick={() => setStep(step - 1)}
              style={{ fontSize: '12px', padding: '4px 12px' }}
              aria-label="Previous step"
            >
              ← Back
            </button>
          )}
          <button
            className="btn-secondary"
            onClick={onComplete}
            style={{ fontSize: '12px', padding: '4px 12px' }}
            aria-label="Skip tour (Escape)"
          >
            Skip Tour
          </button>
          <button
            ref={nextBtnRef}
            className="btn-primary"
            onClick={() => {
              if (isLast) {
                onComplete();
              } else {
                setStep(step + 1);
              }
            }}
            style={{ fontSize: '12px', padding: '4px 12px' }}
            aria-label={isLast ? 'Finish tour' : `Next step (${step + 2} of ${STEPS.length})`}
          >
            {isLast ? 'Get Started' : 'Next →'}
          </button>
        </div>

        {/* Keyboard hint */}
        <div style={{ fontSize: '11px', color: 'var(--text-muted, #888)', marginTop: '6px', textAlign: 'center' }}>
          ← → to navigate · Esc to skip
        </div>
      </div>
    </>
  );
}
