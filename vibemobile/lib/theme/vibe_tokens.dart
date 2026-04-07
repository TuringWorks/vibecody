// vibe_tokens.dart — VibeCody Design System Token Translation for Flutter
//
// This file is the Flutter equivalent of vibeui/design-system/tokens.css.
// All values are kept in sync with the CSS custom properties defined there.
// When updating tokens in tokens.css, update the corresponding values here.
//
// Usage:
//   import 'vibe_tokens.dart';
//   Container(color: VibeTokens.dark.bgPrimary)
//   Text('hello', style: TextStyle(fontSize: VibeTokens.fontSizeMd))

import 'package:flutter/material.dart';

// ── Color Tokens ────────────────────────────────────────────────────────────

/// Dark theme palette (matches :root in tokens.css)
class VibeDarkColors {
  const VibeDarkColors._();

  // Backgrounds
  static const bgPrimary   = Color(0xFF0F1117); // --bg-primary
  static const bgSecondary = Color(0xFF161821); // --bg-secondary
  static const bgTertiary  = Color(0xFF1C1F2B); // --bg-tertiary
  static const bgElevated  = Color(0xFF222638); // --bg-elevated

  // Text
  static const textPrimary   = Color(0xFFE2E4EA); // --text-primary
  static const textSecondary = Color(0xFF6E7491); // --text-secondary
  static const textMuted     = Color(0xFF4B5068); // --text-muted

  // Accents
  static const accentBlue   = Color(0xFF6C8CFF); // --accent-blue
  static const accentGreen  = Color(0xFF34D399); // --accent-green
  static const accentPurple = Color(0xFFA78BFA); // --accent-purple
  static const accentGold   = Color(0xFFF5C542); // --accent-gold
  static const accentRose   = Color(0xFFF472B6); // --accent-rose

  // Semantic
  static const successColor = accentGreen;
  static const errorColor   = Color(0xFFEF4444); // --error-color
  static const warningColor = accentGold;
  static const infoColor    = accentBlue;

  // Semantic backgrounds (10% opacity fills)
  static const successBg = Color(0x1A34D399); // rgba(52, 211, 153, 0.1)
  static const errorBg   = Color(0x1AEF4444); // rgba(239, 68, 68, 0.1)
  static const warningBg = Color(0x1AF5C542); // rgba(245, 197, 66, 0.1)
  static const infoBg    = Color(0x1A6C8CFF); // rgba(108, 140, 255, 0.1)
  static const accentBg  = Color(0x266C8CFF); // rgba(108, 140, 255, 0.15)

  // Border
  static const borderColor  = Color(0x0FFFFFFF); // rgba(255,255,255,0.06)
  static const borderSubtle = Color(0x08FFFFFF); // rgba(255,255,255,0.03)

  // Git status
  static const gitModified   = accentGold;
  static const gitAdded      = accentGreen;
  static const gitDeleted    = errorColor;
  static const gitIgnored    = textMuted;
  static const gitConflicted = accentRose;
}

/// Light theme palette (matches [data-theme="light"] in tokens.css)
class VibeLightColors {
  const VibeLightColors._();

  // Backgrounds
  static const bgPrimary   = Color(0xFFFAFBFD); // --bg-primary
  static const bgSecondary = Color(0xFFF0F1F5); // --bg-secondary
  static const bgTertiary  = Color(0xFFE6E8EF); // --bg-tertiary
  static const bgElevated  = Color(0xFFFFFFFF); // --bg-elevated

  // Text
  static const textPrimary   = Color(0xFF1A1D2E); // --text-primary
  static const textSecondary = Color(0xFF6B7089); // --text-secondary
  static const textMuted     = Color(0xFF9CA3AF); // --text-muted

  // Accents (adjusted for light-mode contrast)
  static const accentBlue   = Color(0xFF4F6DF5); // --accent-blue
  static const accentGreen  = Color(0xFF10B981); // --accent-green
  static const accentPurple = Color(0xFF8B5CF6); // --accent-purple
  static const accentGold   = Color(0xFFD4A017); // --accent-gold
  static const accentRose   = Color(0xFFEC4899); // --accent-rose

  // Semantic
  static const successColor = accentGreen;
  static const errorColor   = Color(0xFFDC2626); // --error-color
  static const warningColor = accentGold;
  static const infoColor    = accentBlue;

  // Semantic backgrounds
  static const successBg = Color(0x1A10B981);
  static const errorBg   = Color(0x1ADC2626);
  static const warningBg = Color(0x1AD4A017);
  static const infoBg    = Color(0x1A4F6DF5);
  static const accentBg  = Color(0x1A4F6DF5);

  // Border
  static const borderColor  = Color(0x14000000); // rgba(0,0,0,0.08)
  static const borderSubtle = Color(0x0A000000); // rgba(0,0,0,0.04)

  // Git status
  static const gitModified   = accentGold;
  static const gitAdded      = accentGreen;
  static const gitDeleted    = errorColor;
  static const gitIgnored    = Color(0xFF9CA3AF);
  static const gitConflicted = accentRose;
}

// ── Spacing Tokens (4px base grid) ──────────────────────────────────────────

class VibeSpacing {
  const VibeSpacing._();

  static const s1 = 4.0;   // --space-1
  static const s2 = 8.0;   // --space-2
  static const s3 = 12.0;  // --space-3
  static const s4 = 16.0;  // --space-4
  static const s5 = 20.0;  // --space-5
  static const s6 = 24.0;  // --space-6
  static const s8 = 32.0;  // --space-8

  // Convenience EdgeInsets
  static const EdgeInsets cardPadding    = EdgeInsets.all(s3);
  static const EdgeInsets sectionPadding = EdgeInsets.all(s4);
  static const EdgeInsets panelBody      = EdgeInsets.symmetric(horizontal: s4, vertical: s3);
}

// ── Typography Tokens ────────────────────────────────────────────────────────

class VibeFontSize {
  const VibeFontSize._();

  static const xs   = 10.0; // --font-size-xs   timestamps, badges
  static const sm   = 11.0; // --font-size-sm   labels, captions
  static const base = 12.0; // --font-size-base panel body
  static const md   = 13.0; // --font-size-md   primary content
  static const lg   = 14.0; // --font-size-lg   section headings
  static const xl   = 15.0; // --font-size-xl   panel heading
  static const xl2  = 18.0; // --font-size-2xl  key metrics
  static const xl3  = 24.0; // --font-size-3xl  hero stats
}

class VibeFontWeight {
  const VibeFontWeight._();

  static const normal   = FontWeight.w400; // --font-normal
  static const medium   = FontWeight.w500; // --font-medium
  static const semibold = FontWeight.w600; // --font-semibold
  static const bold     = FontWeight.w700; // --font-bold
}

// ── Radius Tokens ────────────────────────────────────────────────────────────

class VibeRadius {
  const VibeRadius._();

  static const xs = 3.0;  // --radius-xs  tags, tiny badges
  static const sm = 6.0;  // --radius-sm  buttons, inputs
  static const md = 10.0; // --radius-md  cards, panels
  static const lg = 14.0; // --radius-lg  modals, sheets
  static const xl = 20.0; // --radius-xl  pill shapes

  static BorderRadius borderXs = BorderRadius.circular(xs);
  static BorderRadius borderSm = BorderRadius.circular(sm);
  static BorderRadius borderMd = BorderRadius.circular(md);
  static BorderRadius borderLg = BorderRadius.circular(lg);
  static BorderRadius borderXl = BorderRadius.circular(xl);
}

// ── Motion Tokens ────────────────────────────────────────────────────────────

class VibeDuration {
  const VibeDuration._();

  static const fast   = Duration(milliseconds: 150); // --transition-fast
  static const smooth = Duration(milliseconds: 250); // --transition-smooth
  static const spring = Duration(milliseconds: 350); // --transition-spring
}

class VibeCurve {
  const VibeCurve._();

  static const standard = Curves.easeInOut;  // cubic-bezier(0.4,0,0.2,1)
  static const spring   = Curves.elasticOut; // approximate spring easing
}

// ── Elevation / Shadow Tokens ────────────────────────────────────────────────

class VibeElevation {
  const VibeElevation._();

  /// Subtle — lists, borders
  static const List<BoxShadow> e1 = [
    BoxShadow(color: Color(0x4D000000), blurRadius: 2, offset: Offset(0, 1)),
  ];

  /// Standard — cards, dropdowns
  static const List<BoxShadow> e2 = [
    BoxShadow(color: Color(0x59000000), blurRadius: 12, offset: Offset(0, 4)),
  ];

  /// Deep — modals, overlays
  static const List<BoxShadow> e3 = [
    BoxShadow(color: Color(0x73000000), blurRadius: 30, offset: Offset(0, 8)),
  ];
}

// ── Convenience accessor ─────────────────────────────────────────────────────

/// Use `VibeTokens.of(context)` to get the right palette for current brightness.
class VibeTokens {
  static VibePalette of(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    return dark ? VibePalette.dark : VibePalette.light;
  }
}

/// Unified palette view combining colors + semantic aliases.
class VibePalette {
  final Color bgPrimary;
  final Color bgSecondary;
  final Color bgTertiary;
  final Color bgElevated;
  final Color textPrimary;
  final Color textSecondary;
  final Color textMuted;
  final Color accentBlue;
  final Color accentGreen;
  final Color accentPurple;
  final Color accentGold;
  final Color accentRose;
  final Color successColor;
  final Color errorColor;
  final Color warningColor;
  final Color infoColor;
  final Color successBg;
  final Color errorBg;
  final Color warningBg;
  final Color infoBg;
  final Color borderColor;

  const VibePalette({
    required this.bgPrimary,
    required this.bgSecondary,
    required this.bgTertiary,
    required this.bgElevated,
    required this.textPrimary,
    required this.textSecondary,
    required this.textMuted,
    required this.accentBlue,
    required this.accentGreen,
    required this.accentPurple,
    required this.accentGold,
    required this.accentRose,
    required this.successColor,
    required this.errorColor,
    required this.warningColor,
    required this.infoColor,
    required this.successBg,
    required this.errorBg,
    required this.warningBg,
    required this.infoBg,
    required this.borderColor,
  });

  static const dark = VibePalette(
    bgPrimary:     VibeDarkColors.bgPrimary,
    bgSecondary:   VibeDarkColors.bgSecondary,
    bgTertiary:    VibeDarkColors.bgTertiary,
    bgElevated:    VibeDarkColors.bgElevated,
    textPrimary:   VibeDarkColors.textPrimary,
    textSecondary: VibeDarkColors.textSecondary,
    textMuted:     VibeDarkColors.textMuted,
    accentBlue:    VibeDarkColors.accentBlue,
    accentGreen:   VibeDarkColors.accentGreen,
    accentPurple:  VibeDarkColors.accentPurple,
    accentGold:    VibeDarkColors.accentGold,
    accentRose:    VibeDarkColors.accentRose,
    successColor:  VibeDarkColors.successColor,
    errorColor:    VibeDarkColors.errorColor,
    warningColor:  VibeDarkColors.warningColor,
    infoColor:     VibeDarkColors.infoColor,
    successBg:     VibeDarkColors.successBg,
    errorBg:       VibeDarkColors.errorBg,
    warningBg:     VibeDarkColors.warningBg,
    infoBg:        VibeDarkColors.infoBg,
    borderColor:   VibeDarkColors.borderColor,
  );

  static const light = VibePalette(
    bgPrimary:     VibeLightColors.bgPrimary,
    bgSecondary:   VibeLightColors.bgSecondary,
    bgTertiary:    VibeLightColors.bgTertiary,
    bgElevated:    VibeLightColors.bgElevated,
    textPrimary:   VibeLightColors.textPrimary,
    textSecondary: VibeLightColors.textSecondary,
    textMuted:     VibeLightColors.textMuted,
    accentBlue:    VibeLightColors.accentBlue,
    accentGreen:   VibeLightColors.accentGreen,
    accentPurple:  VibeLightColors.accentPurple,
    accentGold:    VibeLightColors.accentGold,
    accentRose:    VibeLightColors.accentRose,
    successColor:  VibeLightColors.successColor,
    errorColor:    VibeLightColors.errorColor,
    warningColor:  VibeLightColors.warningColor,
    infoColor:     VibeLightColors.infoColor,
    successBg:     VibeLightColors.successBg,
    errorBg:       VibeLightColors.errorBg,
    warningBg:     VibeLightColors.warningBg,
    infoBg:        VibeLightColors.infoBg,
    borderColor:   VibeLightColors.borderColor,
  );
}
