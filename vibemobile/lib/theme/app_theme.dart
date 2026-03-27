import 'package:flutter/material.dart';

/// VibeCody adaptive theme — follows system light/dark preference.
class AppTheme {
  // ── Accent colors (shared across both themes) ───────────────────────
  static const accentBlue = Color(0xFF6C8EEF);
  static const accentGreen = Color(0xFF4EC9B0);
  static const accentRed = Color(0xFFE06C75);
  static const accentOrange = Color(0xFFE5C07B);

  // ── Dark palette ────────────────────────────────────────────────────
  static const _darkBgPrimary = Color(0xFF1E1E2E);
  static const _darkBgSecondary = Color(0xFF252536);
  static const _darkBgTertiary = Color(0xFF2A2A3C);
  static const _darkTextPrimary = Color(0xFFE4E4EF);
  static const _darkTextSecondary = Color(0xFF9999AA);
  static const _darkBorder = Color(0xFF3A3A4C);

  // ── Light palette ───────────────────────────────────────────────────
  static const _lightBgPrimary = Color(0xFFF8F8FA);
  static const _lightBgSecondary = Color(0xFFFFFFFF);
  static const _lightBgTertiary = Color(0xFFF0F0F4);
  static const _lightTextPrimary = Color(0xFF1E1E2E);
  static const _lightTextSecondary = Color(0xFF6B6B80);
  static const _lightBorder = Color(0xFFDDDDE4);

  // Slightly adjusted accents for light mode readability.
  static const _lightAccentBlue = Color(0xFF4A6FD9);
  static const _lightAccentGreen = Color(0xFF2E9A82);
  static const _lightAccentRed = Color(0xFFD04A54);

  // ── Dark ThemeData ──────────────────────────────────────────────────

  static final dark = ThemeData(
    brightness: Brightness.dark,
    scaffoldBackgroundColor: _darkBgPrimary,
    colorScheme: const ColorScheme.dark(
      primary: accentBlue,
      secondary: accentGreen,
      error: accentRed,
      surface: _darkBgSecondary,
      onSurface: _darkTextPrimary,
    ),
    cardTheme: CardThemeData(
      color: _darkBgSecondary,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: const BorderSide(color: _darkBorder),
      ),
      elevation: 0,
    ),
    appBarTheme: const AppBarTheme(
      backgroundColor: _darkBgSecondary,
      foregroundColor: _darkTextPrimary,
      elevation: 0,
      centerTitle: false,
    ),
    bottomNavigationBarTheme: const BottomNavigationBarThemeData(
      backgroundColor: _darkBgSecondary,
      selectedItemColor: accentBlue,
      unselectedItemColor: _darkTextSecondary,
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: _darkBgTertiary,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: _darkBorder),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: _darkBorder),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: accentBlue, width: 2),
      ),
      hintStyle: const TextStyle(color: _darkTextSecondary),
    ),
    textTheme: const TextTheme(
      headlineLarge: TextStyle(color: _darkTextPrimary, fontWeight: FontWeight.bold),
      headlineMedium: TextStyle(color: _darkTextPrimary, fontWeight: FontWeight.w600),
      bodyLarge: TextStyle(color: _darkTextPrimary),
      bodyMedium: TextStyle(color: _darkTextSecondary),
      labelSmall: TextStyle(color: _darkTextSecondary, fontSize: 11),
    ),
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        backgroundColor: accentBlue,
        foregroundColor: Colors.white,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
      ),
    ),
    floatingActionButtonTheme: const FloatingActionButtonThemeData(
      backgroundColor: accentBlue,
      foregroundColor: Colors.white,
    ),
    chipTheme: ChipThemeData(
      backgroundColor: _darkBgTertiary,
      labelStyle: const TextStyle(color: _darkTextPrimary, fontSize: 12),
      side: const BorderSide(color: _darkBorder),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
    ),
    dividerColor: _darkBorder,
    useMaterial3: true,
  );

  // ── Light ThemeData ─────────────────────────────────────────────────

  static final light = ThemeData(
    brightness: Brightness.light,
    scaffoldBackgroundColor: _lightBgPrimary,
    colorScheme: const ColorScheme.light(
      primary: _lightAccentBlue,
      secondary: _lightAccentGreen,
      error: _lightAccentRed,
      surface: _lightBgSecondary,
      onSurface: _lightTextPrimary,
    ),
    cardTheme: CardThemeData(
      color: _lightBgSecondary,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: const BorderSide(color: _lightBorder),
      ),
      elevation: 0,
    ),
    appBarTheme: const AppBarTheme(
      backgroundColor: _lightBgSecondary,
      foregroundColor: _lightTextPrimary,
      elevation: 0,
      centerTitle: false,
    ),
    bottomNavigationBarTheme: const BottomNavigationBarThemeData(
      backgroundColor: _lightBgSecondary,
      selectedItemColor: _lightAccentBlue,
      unselectedItemColor: _lightTextSecondary,
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: _lightBgTertiary,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: _lightBorder),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: _lightBorder),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: _lightAccentBlue, width: 2),
      ),
      hintStyle: const TextStyle(color: _lightTextSecondary),
    ),
    textTheme: const TextTheme(
      headlineLarge: TextStyle(color: _lightTextPrimary, fontWeight: FontWeight.bold),
      headlineMedium: TextStyle(color: _lightTextPrimary, fontWeight: FontWeight.w600),
      bodyLarge: TextStyle(color: _lightTextPrimary),
      bodyMedium: TextStyle(color: _lightTextSecondary),
      labelSmall: TextStyle(color: _lightTextSecondary, fontSize: 11),
    ),
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        backgroundColor: _lightAccentBlue,
        foregroundColor: Colors.white,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
      ),
    ),
    floatingActionButtonTheme: const FloatingActionButtonThemeData(
      backgroundColor: _lightAccentBlue,
      foregroundColor: Colors.white,
    ),
    chipTheme: ChipThemeData(
      backgroundColor: _lightBgTertiary,
      labelStyle: const TextStyle(color: _lightTextPrimary, fontSize: 12),
      side: const BorderSide(color: _lightBorder),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
    ),
    dividerColor: _lightBorder,
    useMaterial3: true,
  );
}

/// Extension to resolve semantic colors from the current theme brightness.
/// Use `context.vibeColors` instead of hardcoded `AppTheme.*` statics.
extension VibeCodyColors on BuildContext {
  VibeColors get vibeColors {
    final brightness = Theme.of(this).brightness;
    return brightness == Brightness.dark ? VibeColors.dark : VibeColors.light;
  }
}

class VibeColors {
  final Color bgPrimary;
  final Color bgSecondary;
  final Color bgTertiary;
  final Color textPrimary;
  final Color textSecondary;
  final Color borderColor;
  final Color accentBlue;
  final Color accentGreen;
  final Color accentRed;
  final Color accentOrange;

  const _VibeColors({
    required this.bgPrimary,
    required this.bgSecondary,
    required this.bgTertiary,
    required this.textPrimary,
    required this.textSecondary,
    required this.borderColor,
    required this.accentBlue,
    required this.accentGreen,
    required this.accentRed,
    required this.accentOrange,
  });

  static const dark = _VibeColors(
    bgPrimary: Color(0xFF1E1E2E),
    bgSecondary: Color(0xFF252536),
    bgTertiary: Color(0xFF2A2A3C),
    textPrimary: Color(0xFFE4E4EF),
    textSecondary: Color(0xFF9999AA),
    borderColor: Color(0xFF3A3A4C),
    accentBlue: Color(0xFF6C8EEF),
    accentGreen: Color(0xFF4EC9B0),
    accentRed: Color(0xFFE06C75),
    accentOrange: Color(0xFFE5C07B),
  );

  static const light = _VibeColors(
    bgPrimary: Color(0xFFF8F8FA),
    bgSecondary: Color(0xFFFFFFFF),
    bgTertiary: Color(0xFFF0F0F4),
    textPrimary: Color(0xFF1E1E2E),
    textSecondary: Color(0xFF6B6B80),
    borderColor: Color(0xFFDDDDE4),
    accentBlue: Color(0xFF4A6FD9),
    accentGreen: Color(0xFF2E9A82),
    accentRed: Color(0xFFD04A54),
    accentOrange: Color(0xFFBD9520),
  );
}
