# 07 — VibeUI Usability Improvements

> Comprehensive audit of UI consistency, accessibility, and interaction quality across 235+ panels.
> Generated: 2026-04-10 | Audited by: 5 parallel agents covering spacing, interactivity, responsive, color, and a11y.
>
> Each item is **independently implementable** and parallelizable. Items are grouped into work streams
> that can be assigned to different developers. Each includes a testability note for TDD/BDD coverage.

---

## Work Stream 1: Spacing & Layout Consistency

### S-1: Replace hardcoded padding/margin with design system tokens
- **Scope**: ~200+ inline style occurrences across 40+ panels
- **Issue**: Panels use inconsistent padding values (3px, 5px, 6px, 8px, 10px, 12px, 14px, 16px, 20px) instead of `--space-*` tokens (4/8/12/16/20/24/32px)
- **Examples**:
  - `AdminPanel.tsx:152` — `padding: '10px 12px'` (non-standard)
  - `AgentTeamPanel.tsx:130` — `padding: "16px 12px"` (hardcoded)
  - `BackgroundJobsPanel.tsx:218` — `padding: '8px 10px'` (mixed)
  - `BrowserPanel.tsx:219` — `padding: '3px 10px'` (hardcoded)
- **Fix**: Replace with `var(--space-N)` references or standardize to 4px grid
- **Test**: Visual regression snapshot tests comparing before/after; CSS linter rule disallowing raw px in padding/margin

### S-2: Standardize flex gap values
- **Scope**: ~100+ flex containers across 30+ panels
- **Issue**: Gap values range from 2-16px without following the 4px grid (values like 2, 3, 6, 10 found)
- **Examples**:
  - `AdminPanel.tsx:269` — `gap: 3` (not on grid)
  - `AcpPanel.tsx:237` — `gap: 6` (not on grid)
  - `A2aPanel.tsx:627` — `gap: 10` (not on grid)
  - `AgentTeamPanel.tsx:202` — `gap: 2` (too tight)
- **Fix**: Normalize all gaps to multiples of 4 (4, 8, 12, 16, 20, 24)
- **Test**: Grep-based lint rule; snapshot tests

### S-3: Standardize border-radius to token scale
- **Scope**: ~80+ hardcoded border-radius values
- **Issue**: Values like 4px, 8px used but not in token scale (tokens define xs:3, sm:6, md:10, lg:14, xl:20)
- **Examples**:
  - `AdminPanel.tsx:200` — `borderRadius: 4` (not in tokens)
  - `AgentHostPanel.tsx:142` — `borderRadius: 8` (not in tokens)
  - `AgentTeamPanel.tsx:142` — `borderRadius: 4` (not in tokens)
- **Fix**: Map to nearest token or add `--radius-2xs: 4px` and `--radius-sm-plus: 8px` to token scale
- **Test**: CSS lint rule; visual regression tests

---

## Work Stream 2: Color & Theme Compliance

### C-1: Replace hardcoded hex colors with CSS variables
- **Scope**: 52+ hardcoded hex values across 15+ panels
- **Severity**: HIGH
- **Examples**:
  - `CollabChatPanel.tsx:438,508` — `#fff` -> `var(--btn-primary-fg)`
  - `AgentOSDashboard.tsx:327-343` — `#4fc3f7`, `#ffb74d`, `#81c784`, `#ef5350` -> `var(--info-color)`, `var(--warning-color)`, `var(--success-color)`, `var(--error-color)`
  - `CounselPanel.tsx:54-58` — 5 hardcoded role colors -> semantic tokens
  - `CloudAutofixPanel.tsx:109-110` — status colors -> `var(--success-color)`, `var(--error-color)`, `var(--warning-color)`
  - `A2aPanel.tsx:66-93` — `#6366f1`, `#06b6d4`, `#f97316` -> accent tokens
  - `ArchitectureSpecPanel.tsx:712,1171` — `#f87171`, `#f0a500` -> `var(--error-color)`, `var(--warning-color)`
- **Fix**: Map each hex to nearest design system variable; add missing tokens (--accent-indigo, --accent-cyan)
- **Test**: Grep-based lint rule `/#[0-9a-fA-F]{3,8}/` in .tsx files; theme switching test

### C-2: Standardize overlay/backdrop opacity
- **Scope**: 6+ overlay implementations
- **Issue**: Opacity varies between 0.5, 0.6, 0.85 across modals and overlays
- **Examples**:
  - `Modal.css:7` — `rgba(0, 0, 0, 0.5)`
  - `CommandPalette.css:7` — `rgba(0, 0, 0, 0.6)`
  - `AIChat.css:1583` — `rgba(0, 0, 0, 0.85)`
  - `AgilePanel.tsx:301,875` — `rgba(0,0,0,0.55)` and `rgba(0,0,0,0.5)`
- **Fix**: Add `--overlay-bg: rgba(0, 0, 0, 0.5)` token; apply uniformly
- **Test**: Visual regression test for all modals/overlays

### C-3: Standardize shadow/elevation values
- **Scope**: 4+ shadow definitions
- **Issue**: Different shadow values instead of elevation scale tokens
- **Examples**:
  - `ContextPicker.css:9` — `0 4px 16px rgba(0,0,0,0.4)` -> `var(--elevation-2)`
  - `Toaster.css:26` — `0 4px 12px rgba(0,0,0,0.35)` -> `var(--elevation-2)`
  - `CommandPalette.css:36` — `0 8px 32px rgba(0,0,0,0.4)` -> `var(--elevation-3)`
- **Fix**: Use `--elevation-1` through `--elevation-3` tokens
- **Test**: CSS lint rule for `box-shadow` outside tokens

### C-4: Standardize rgba opacity values
- **Scope**: 15+ inline rgba values
- **Issue**: Opacity ranges from 0.04 to 0.55 with no standard scale
- **Examples**:
  - `AgentTeamsPanel.tsx:362` — `rgba(52,211,153,0.08)`
  - `AutofixPanel.tsx:183` — `rgba(76,175,80,0.1)`
  - `CloudAgentPanel.tsx:217` — `rgba(244,67,54,0.15)`
  - `CollabChatPanel.tsx:481` — `rgba(249,123,34,0.1)`
- **Fix**: Add `--success-bg`, `--error-bg`, `--warning-bg` tokens (some exist, ensure coverage); use `color-mix()` for dynamic values
- **Test**: Grep lint rule for inline rgba

### C-5: Add missing color tokens
- **Scope**: Token scale gaps
- **Issue**: `--text-tertiary` referenced but not defined; `--accent-indigo`, `--accent-cyan` needed
- **Fix**: Add to `design-system/tokens.css`:
  ```css
  --text-tertiary: #888;
  --accent-indigo: #6366f1;
  --accent-cyan: #06b6d4;
  ```
- **Test**: Token completeness check — grep for `var(--` references not defined in tokens.css

---

## Work Stream 3: Interactive Element Quality

### I-1: Add visible focus rings to all interactive elements
- **Scope**: 20+ panels, all buttons and inputs
- **Severity**: HIGH (WCAG 2.4.7)
- **Issue**: Most inline-styled buttons/inputs lack `:focus-visible` styles
- **Examples**:
  - `SpecPanel.tsx:164-175` — inputs lack focus outline
  - `EnvPanel.tsx:256-271` — input + button missing focus
  - `CanvasPanel.tsx:211-217` — input missing focus ring
  - `DatabasePanel.tsx:163-169` — connection string input
- **Fix**: Add global CSS rule:
  ```css
  .panel-btn:focus-visible, .panel-input:focus-visible, .panel-select:focus-visible {
    outline: 2px solid var(--accent-color);
    outline-offset: 2px;
  }
  ```
- **Test**: BDD — `Given a button, When user tabs to it, Then focus ring is visible`; jest-dom `toHaveFocus` + computed style checks

### I-2: Add aria-labels to all icon-only buttons
- **Scope**: ~20 instances across 10+ panels
- **Severity**: HIGH (WCAG 1.1.1, 4.1.2)
- **Examples**:
  - `SpecPanel.tsx:153-158` — refresh `↺` button, no label
  - `EnvPanel.tsx:353-362` — delete `✕` button, no label
  - `CanvasPanel.tsx:313-322` — delete `x` button, no label
  - `NotificationCenter.tsx:143-149` — dismiss X icon, no label
- **Fix**: Add `aria-label="Refresh"`, `aria-label="Delete"`, `aria-label="Dismiss"` etc.
- **Test**: jest-axe automated a11y scan per component; BDD — `Given icon button, Then aria-label exists`

### I-3: Standardize disabled button styling
- **Scope**: 10+ panels with disabled buttons
- **Severity**: HIGH
- **Issue**: Disabled states vary — some opacity only, some color, some cursor-only
- **Examples**:
  - `SpecPanel.tsx:181` — relies on browser default
  - `EnvPanel.tsx:267` — no visual disabled style
  - `CloudAutofixPanel.tsx:179` — inline opacity mixing
  - `DatabasePanel.tsx:237` — no opacity/color change
- **Fix**: Add global CSS:
  ```css
  .panel-btn:disabled, button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }
  ```
- **Test**: Snapshot test disabled state; BDD — `Given disabled button, Then opacity is 0.5 and cursor is not-allowed`

### I-4: Add ARIA tab roles to all tab bars
- **Scope**: 15+ tab bar implementations
- **Severity**: HIGH (WCAG 4.1.2)
- **Issue**: Tab bars use `panel-tab` class but no `role="tablist"`, `role="tab"`, `aria-selected`
- **Examples**:
  - `SpecPanel.tsx:206-217` — no tablist/tab roles
  - `AstEditPanel.tsx:132-135` — no tab semantics
  - `ScriptPanel.tsx:170-200` — buttons as tabs, no ARIA
  - `DatabasePanel.tsx:143-161` — missing tab semantics
  - `SemanticIndexPanel.tsx:147-151` — panel-tab without ARIA
- **Fix**: Wrap in `<div role="tablist">`, add `role="tab"` + `aria-selected={isActive}` to each tab button, `role="tabpanel"` to content
- **Test**: jest-axe per tab component; BDD — `Given tab bar, Then tablist role exists and active tab has aria-selected=true`

### I-5: Increase minimum click target sizes
- **Scope**: 10+ panels with small targets
- **Severity**: MEDIUM (WCAG 2.5.8)
- **Issue**: Buttons with padding < 24px total height
- **Examples**:
  - `ScriptPanel.tsx:175-189` — `padding: "3px 10px"` (~22px)
  - `DatabasePanel.tsx:143-161` — `padding: "3px 0"` (~20px)
  - `EnvPanel.tsx:265-271` — `padding: "4px 12px"` (~24px borderline)
- **Fix**: Set `min-height: 28px` on all `.panel-btn` variants; `min-height: 32px` for primary actions
- **Test**: Computed style test — `expect(button.offsetHeight).toBeGreaterThanOrEqual(28)`

### I-6: Add loading spinners to async buttons
- **Scope**: 8+ panels with async actions
- **Severity**: MEDIUM
- **Issue**: Async buttons show text change ("Running...") but no visual spinner
- **Examples**:
  - `SpecPanel.tsx:179-192` — "Generating..." text only
  - `ScriptPanel.tsx:256-263` — "Running..." text only
  - `EnvPanel.tsx:292-298` — "Saving..." text only
- **Fix**: Add `<Loader2 className="spin" size={14} />` from lucide-react (pattern already used in AgentPanel)
- **Test**: BDD — `Given async button clicked, Then spinner icon is visible`

### I-7: Standardize hover/active states
- **Scope**: 15+ panels with clickable cards/rows
- **Severity**: MEDIUM
- **Issue**: Each panel implements hover differently (JS state, inline style mutation, or CSS class)
- **Examples**:
  - `SpecPanel.tsx:237` — `onMouseEnter` to set background
  - `AgilePanel.tsx:823-824` — manual translateY on hover
  - `HistoryPanel.tsx:96-117` — cursor pointer but no active feedback
- **Fix**: Add standard CSS classes `.panel-card--clickable:hover` and `.panel-card--clickable:active`
- **Test**: Visual regression tests on hover/active states

---

## Work Stream 4: Responsive & Overflow Handling

### R-1: Add horizontal scroll wrappers to all tables
- **Scope**: 8+ panels with data tables
- **Severity**: HIGH
- **Issue**: Tables overflow container without scroll on narrow widths
- **Examples**:
  - `DatabasePanel.tsx:254-275` — table without `overflowX: "auto"` wrapper
  - `CsvPanel.tsx:256-292` — multiple tables missing wrapper
- **Fix**: Wrap all `<table>` elements in `<div style={{ overflowX: "auto" }}>`
- **Test**: BDD — `Given narrow viewport, When table has many columns, Then horizontal scroll is available`

### R-2: Replace fixed heights with dynamic sizing
- **Scope**: 10+ panels with hardcoded maxHeight/height
- **Severity**: MEDIUM
- **Issue**: Hardcoded px heights truncate content on different screen sizes
- **Examples**:
  - `ScriptPanel.tsx:323` — `maxHeight: 220` (fixed)
  - `CsvPanel.tsx:407-425` — `maxHeight: 200` and `maxHeight: 160` (inconsistent)
  - `ArenaPanel.tsx:269` — `minHeight: "150px"` (fixed)
- **Fix**: Use `calc(100vh - Npx)` or flex-based sizing with `min-height: 0` on parents
- **Test**: Responsive test at 768px, 1024px, 1440px viewport widths

### R-3: Add word-break protection to text cells
- **Scope**: 6+ panels with table cells or long text
- **Severity**: MEDIUM
- **Issue**: Long unbroken strings overflow without wrapping
- **Examples**:
  - `CsvPanel.tsx:277` — `whiteSpace: "nowrap"` without `wordBreak`
  - `ArenaPanel.tsx:105` — `pre` without word-break
- **Fix**: Add `word-break: break-word` to cells with dynamic content
- **Test**: Render test with 200-char unbroken string

### R-4: Add flex shrink protection
- **Scope**: 8+ panels with fixed-width sidebars
- **Severity**: MEDIUM
- **Issue**: Fixed-width sidebars don't collapse or protect right-side content
- **Examples**:
  - `CanvasPanel.tsx:354` — `width: 220` properties panel
  - `DatabasePanel.tsx:139` — `width: 200` sidebar, no min-width on right
- **Fix**: Add `flexShrink: 0` on fixed sidebars; `minWidth: 0` on flex children with text
- **Test**: Layout test at 600px width

### R-5: Standardize empty state design
- **Scope**: 10+ panels
- **Severity**: LOW
- **Issue**: Empty states use different centering methods (marginTop vs flex centering)
- **Examples**:
  - `DocumentIngestPanel.tsx:171` — `marginTop: 32` (hardcoded)
  - `HistoryPanel.tsx:91` — `marginTop: "24px"` (different value)
- **Fix**: Create standard `.panel-empty-state` class with flex centering
- **Test**: Snapshot test per panel's empty state

### R-6: Add maxHeight to command palettes and dropdowns
- **Scope**: 3+ overlay components
- **Severity**: MEDIUM
- **Issue**: Lists can expand beyond viewport
- **Examples**:
  - `CommandPalette.tsx:144` — no maxHeight on command list
  - `NotificationCenter.tsx:93` — panel max-height unspecified
  - `AutomationsPanel.tsx:109-127` — dropdown lacks viewport protection
- **Fix**: Set `max-height: min(400px, 60vh)` on overlay lists
- **Test**: Render test with 100+ items

---

## Work Stream 5: Accessibility (WCAG 2.1 AA)

### A-1: Convert div onClick to semantic buttons
- **Scope**: 8+ panels
- **Severity**: HIGH (WCAG 2.1.1, 4.1.2)
- **Issue**: Clickable divs/spans without `role="button"`, `tabIndex`, or keyboard handlers
- **Examples**:
  - `SpecPanel.tsx:234` — `<div onClick={...}>` card without keyboard support
  - `HistoryPanel.tsx:96-117` — session items only clickable by mouse
- **Fix**: Replace with `<button>` or add `role="button" tabIndex={0} onKeyDown={handleKeyboardClick}`
- **Test**: jest-axe; BDD — `Given card element, When user presses Enter, Then click handler fires`

### A-2: Add role="alert" to error containers
- **Scope**: 10+ panels with error display
- **Severity**: MEDIUM (WCAG 3.3.1, 4.1.3)
- **Issue**: Error messages lack ARIA live region announcements
- **Examples**:
  - `A2aPanel.tsx:318-328` — error div without aria-live
  - `DatabasePanel.tsx` — error display without role
- **Fix**: `<div role="alert" aria-live="assertive">{error}</div>`
- **Test**: BDD — `Given error occurs, Then element with role="alert" appears`

### A-3: Associate form labels with inputs
- **Scope**: 8+ panels with form inputs
- **Severity**: MEDIUM (WCAG 1.3.1, 3.3.2)
- **Issue**: Label text exists but not wrapped in `<label>` with `htmlFor`
- **Examples**:
  - `CloudAutofixPanel.tsx:186-191` — "Container Image" label not linked
  - `DocumentIngestPanel.tsx:100-106` — label without htmlFor
  - `EnvPanel.tsx:259-272` — input without associated label
- **Fix**: Add `htmlFor` on labels and matching `id` on inputs
- **Test**: jest-axe label association rule

### A-4: Add ARIA attributes to toggle switches
- **Scope**: 5+ custom toggle implementations
- **Severity**: HIGH (WCAG 4.1.2)
- **Issue**: Custom toggle buttons lack `role="switch"`, `aria-checked`, `aria-label`
- **Examples**:
  - `DocumentIngestPanel.tsx:240-262` — toggle without aria-pressed
  - `EnvPanel.tsx:342-351` — toggle without role
- **Fix**: Add `role="switch" aria-checked={isOn} aria-label="Toggle X"`
- **Test**: BDD — `Given toggle switch, Then role="switch" and aria-checked matches state`

### A-5: Add scope="col" to table headers
- **Scope**: All panels with `<th>` elements
- **Severity**: LOW (WCAG 1.3.1)
- **Fix**: Add `scope="col"` to all `<th>` in `<thead>`
- **Test**: Grep-based lint; jest-axe

### A-6: Add aria-modal to dialog overlays
- **Scope**: Modal.tsx, CommandPalette.tsx, and similar overlays
- **Severity**: MEDIUM (WCAG 4.1.2)
- **Fix**: Add `role="dialog" aria-modal="true" aria-labelledby="modal-title"`
- **Test**: jest-axe dialog rule

---

## Implementation Priority

| Priority | Work Stream | Items | Est. Effort | Parallelizable |
|----------|------------|-------|-------------|----------------|
| P0 | Accessibility | A-1, A-4, I-1, I-2, I-4 | 4-6 hrs | Yes (per panel) |
| P1 | Color compliance | C-1, C-5 | 3-4 hrs | Yes (per panel) |
| P1 | Interactive | I-3, I-5 | 2-3 hrs | Yes (global CSS) |
| P2 | Responsive | R-1, R-2, R-6 | 2-3 hrs | Yes (per panel) |
| P2 | Spacing | S-1, S-2 | 4-6 hrs | Yes (per panel) |
| P3 | Color refinement | C-2, C-3, C-4 | 2-3 hrs | Yes (CSS only) |
| P3 | Layout | S-3, R-3, R-4, R-5 | 3-4 hrs | Yes (per panel) |
| P3 | Polish | I-6, I-7, A-2, A-3, A-5, A-6 | 3-4 hrs | Yes (per panel) |

## Testing Strategy

### TDD Approach
1. **CSS Lint Rules**: ESLint plugin or custom script to flag hardcoded hex colors, px values for padding/margin/gap, and missing ARIA attributes
2. **jest-axe**: Automated WCAG violation detection per component (`expect(await axe(container)).toHaveNoViolations()`)
3. **Computed Style Tests**: Verify min-height, focus outline, disabled opacity via `getComputedStyle()`
4. **Snapshot Tests**: Before/after visual regression for each spacing/color fix

### BDD Scenarios (Cucumber/Playwright)
```gherkin
Feature: Panel Usability Standards
  Scenario: All buttons have visible focus
    Given any panel is rendered
    When user tabs to a button
    Then a focus ring with outline-offset >= 2px is visible

  Scenario: Disabled buttons are visually distinct
    Given a button in disabled state
    Then its opacity is <= 0.5
    And cursor is "not-allowed"

  Scenario: Tables scroll horizontally
    Given a data table with > 6 columns
    When viewport width is 800px
    Then horizontal scrollbar is present

  Scenario: Icon buttons are accessible
    Given a button with only an icon
    Then it has an aria-label attribute
    And the label is descriptive (not empty)

  Scenario: Tab bars use ARIA roles
    Given a tab navigation bar
    Then container has role="tablist"
    And each tab has role="tab"
    And active tab has aria-selected="true"
```

## Summary

| Category | Issues Found | Panels Affected |
|----------|-------------|-----------------|
| Spacing/Layout | ~280+ inline style inconsistencies | 40+ |
| Color/Theme | 65+ hardcoded values | 15+ |
| Interactive Elements | 48+ interaction issues | 30+ |
| Responsive/Overflow | 25+ layout issues | 15+ |
| Accessibility | 55+ WCAG violations | 25+ |
| **Total** | **~470+** | **45+** |
