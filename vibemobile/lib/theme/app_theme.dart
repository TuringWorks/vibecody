import 'package:flutter/material.dart';
import 'vibe_tokens.dart';

/// VibeCody adaptive theme — follows system light/dark preference.
/// Colors sourced from vibe_tokens.dart, which mirrors vibeui/design-system/tokens.css.
class AppTheme {
  // ── Dark ThemeData ──────────────────────────────────────────────────────
  static final dark = ThemeData(
    brightness: Brightness.dark,
    scaffoldBackgroundColor: VibeDarkColors.bgPrimary,
    colorScheme: const ColorScheme.dark(
      primary: VibeDarkColors.accentBlue,
      secondary: VibeDarkColors.accentGreen,
      tertiary: VibeDarkColors.accentPurple,
      error: VibeDarkColors.errorColor,
      surface: VibeDarkColors.bgSecondary,
      onSurface: VibeDarkColors.textPrimary,
      onPrimary: Colors.white,
    ),
    cardTheme: CardThemeData(
      color: VibeDarkColors.bgSecondary,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(VibeRadius.md),
        side: const BorderSide(color: VibeDarkColors.borderColor),
      ),
      elevation: 0,
    ),
    appBarTheme: const AppBarTheme(
      backgroundColor: VibeDarkColors.bgSecondary,
      foregroundColor: VibeDarkColors.textPrimary,
      elevation: 0,
      centerTitle: false,
    ),
    bottomNavigationBarTheme: const BottomNavigationBarThemeData(
      backgroundColor: VibeDarkColors.bgSecondary,
      selectedItemColor: VibeDarkColors.accentBlue,
      unselectedItemColor: VibeDarkColors.textSecondary,
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: VibeDarkColors.bgTertiary,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(VibeRadius.sm),
        borderSide: const BorderSide(color: VibeDarkColors.borderColor),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(VibeRadius.sm),
        borderSide: const BorderSide(color: VibeDarkColors.borderColor),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(VibeRadius.sm),
        borderSide: const BorderSide(color: VibeDarkColors.accentBlue, width: 2),
      ),
      hintStyle: const TextStyle(color: VibeDarkColors.textSecondary, fontSize: VibeFontSize.md),
    ),
    textTheme: const TextTheme(
      headlineLarge:  TextStyle(color: VibeDarkColors.textPrimary, fontWeight: FontWeight.w700, fontSize: VibeFontSize.xl3),
      headlineMedium: TextStyle(color: VibeDarkColors.textPrimary, fontWeight: FontWeight.w600, fontSize: VibeFontSize.xl2),
      titleLarge:     TextStyle(color: VibeDarkColors.textPrimary, fontWeight: FontWeight.w600, fontSize: VibeFontSize.xl),
      titleMedium:    TextStyle(color: VibeDarkColors.textPrimary, fontWeight: FontWeight.w600, fontSize: VibeFontSize.lg),
      bodyLarge:      TextStyle(color: VibeDarkColors.textPrimary,   fontSize: VibeFontSize.md),
      bodyMedium:     TextStyle(color: VibeDarkColors.textSecondary, fontSize: VibeFontSize.base),
      bodySmall:      TextStyle(color: VibeDarkColors.textSecondary, fontSize: VibeFontSize.sm),
      labelSmall:     TextStyle(color: VibeDarkColors.textMuted,     fontSize: VibeFontSize.xs),
    ),
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        backgroundColor: VibeDarkColors.accentBlue,
        foregroundColor: Colors.white,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(VibeRadius.sm)),
        padding: const EdgeInsets.symmetric(horizontal: VibeSpacing.s6, vertical: VibeSpacing.s3),
        textStyle: const TextStyle(fontWeight: FontWeight.w600, fontSize: VibeFontSize.md),
      ),
    ),
    floatingActionButtonTheme: const FloatingActionButtonThemeData(
      backgroundColor: VibeDarkColors.accentBlue,
      foregroundColor: Colors.white,
    ),
    chipTheme: ChipThemeData(
      backgroundColor: VibeDarkColors.bgTertiary,
      labelStyle: const TextStyle(color: VibeDarkColors.textPrimary, fontSize: VibeFontSize.sm),
      side: const BorderSide(color: VibeDarkColors.borderColor),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(VibeRadius.xs)),
    ),
    dividerColor: VibeDarkColors.borderColor,
    useMaterial3: true,
  );

  // ── Light ThemeData ─────────────────────────────────────────────────────
  static final light = ThemeData(
    brightness: Brightness.light,
    scaffoldBackgroundColor: VibeLightColors.bgPrimary,
    colorScheme: const ColorScheme.light(
      primary: VibeLightColors.accentBlue,
      secondary: VibeLightColors.accentGreen,
      tertiary: VibeLightColors.accentPurple,
      error: VibeLightColors.errorColor,
      surface: VibeLightColors.bgSecondary,
      onSurface: VibeLightColors.textPrimary,
      onPrimary: Colors.white,
    ),
    cardTheme: CardThemeData(
      color: VibeLightColors.bgSecondary,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(VibeRadius.md),
        side: const BorderSide(color: VibeLightColors.borderColor),
      ),
      elevation: 0,
    ),
    appBarTheme: const AppBarTheme(
      backgroundColor: VibeLightColors.bgSecondary,
      foregroundColor: VibeLightColors.textPrimary,
      elevation: 0,
      centerTitle: false,
    ),
    bottomNavigationBarTheme: const BottomNavigationBarThemeData(
      backgroundColor: VibeLightColors.bgSecondary,
      selectedItemColor: VibeLightColors.accentBlue,
      unselectedItemColor: VibeLightColors.textSecondary,
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: VibeLightColors.bgTertiary,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(VibeRadius.sm),
        borderSide: const BorderSide(color: VibeLightColors.borderColor),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(VibeRadius.sm),
        borderSide: const BorderSide(color: VibeLightColors.borderColor),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(VibeRadius.sm),
        borderSide: const BorderSide(color: VibeLightColors.accentBlue, width: 2),
      ),
      hintStyle: const TextStyle(color: VibeLightColors.textSecondary, fontSize: VibeFontSize.md),
    ),
    textTheme: const TextTheme(
      headlineLarge:  TextStyle(color: VibeLightColors.textPrimary, fontWeight: FontWeight.w700, fontSize: VibeFontSize.xl3),
      headlineMedium: TextStyle(color: VibeLightColors.textPrimary, fontWeight: FontWeight.w600, fontSize: VibeFontSize.xl2),
      titleLarge:     TextStyle(color: VibeLightColors.textPrimary, fontWeight: FontWeight.w600, fontSize: VibeFontSize.xl),
      titleMedium:    TextStyle(color: VibeLightColors.textPrimary, fontWeight: FontWeight.w600, fontSize: VibeFontSize.lg),
      bodyLarge:      TextStyle(color: VibeLightColors.textPrimary,   fontSize: VibeFontSize.md),
      bodyMedium:     TextStyle(color: VibeLightColors.textSecondary, fontSize: VibeFontSize.base),
      bodySmall:      TextStyle(color: VibeLightColors.textSecondary, fontSize: VibeFontSize.sm),
      labelSmall:     TextStyle(color: VibeLightColors.textMuted,     fontSize: VibeFontSize.xs),
    ),
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        backgroundColor: VibeLightColors.accentBlue,
        foregroundColor: Colors.white,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(VibeRadius.sm)),
        padding: const EdgeInsets.symmetric(horizontal: VibeSpacing.s6, vertical: VibeSpacing.s3),
        textStyle: const TextStyle(fontWeight: FontWeight.w600, fontSize: VibeFontSize.md),
      ),
    ),
    floatingActionButtonTheme: const FloatingActionButtonThemeData(
      backgroundColor: VibeLightColors.accentBlue,
      foregroundColor: Colors.white,
    ),
    chipTheme: ChipThemeData(
      backgroundColor: VibeLightColors.bgTertiary,
      labelStyle: const TextStyle(color: VibeLightColors.textPrimary, fontSize: VibeFontSize.sm),
      side: const BorderSide(color: VibeLightColors.borderColor),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(VibeRadius.xs)),
    ),
    dividerColor: VibeLightColors.borderColor,
    useMaterial3: true,
  );
}

/// Extension to resolve the current theme's VibePalette from context.
/// Usage: `context.vibeColors.accentBlue`
extension VibeCodyColors on BuildContext {
  VibePalette get vibeColors => VibeTokens.of(this);
}
