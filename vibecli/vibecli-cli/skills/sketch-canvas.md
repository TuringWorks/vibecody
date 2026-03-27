# Sketch Canvas

Freeform drawing canvas that converts hand-drawn sketches, wireframes, and diagrams into production code. Supports output to React components, HTML/CSS, SwiftUI views, and Flutter widgets using AI vision.

## When to Use
- Converting whiteboard sketches or paper wireframes into working UI code
- Rapid prototyping by drawing UI layouts instead of writing code
- Generating React components from hand-drawn mockups
- Creating SwiftUI or Flutter views from visual sketches
- Iterating on UI designs visually before committing to code

## Commands
- `/sketch open` — Open the sketch canvas in VibeUI
- `/sketch import <image>` — Import an image or photo of a sketch
- `/sketch generate <target>` — Generate code from canvas (react, html, swiftui, flutter)
- `/sketch refine <feedback>` — Refine generated code with natural language feedback
- `/sketch export <format>` — Export canvas as PNG, SVG, or PDF
- `/sketch templates` — Browse starter templates for common UI patterns
- `/sketch undo` — Undo last canvas action
- `/sketch clear` — Clear the canvas

## Examples
```
/sketch import whiteboard-photo.jpg
# Imported sketch. Detected elements:
# - Navigation bar with 4 items
# - Card grid (2x3 layout)
# - Floating action button (bottom right)
# - Search bar (top)

/sketch generate react
# Generated 4 components:
# - NavBar.tsx (4 nav items, responsive)
# - CardGrid.tsx (2x3 grid, mapped from data)
# - SearchBar.tsx (with debounced input)
# - FloatingButton.tsx (fixed position)
# Total: 187 lines of TypeScript + Tailwind CSS

/sketch refine "Make the cards rounded with shadows and add hover effects"
# Updated CardGrid.tsx: added rounded-xl, shadow-lg, hover:scale-105
```

## Best Practices
- Draw clear boundaries between UI elements for best recognition
- Label elements in your sketch to help the AI identify purposes
- Start with simple layouts and refine iteratively with feedback
- Use templates for common patterns like dashboards, forms, and lists
- Review generated code for accessibility attributes after generation
