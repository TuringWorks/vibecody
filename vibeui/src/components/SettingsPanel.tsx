/**
 * SettingsPanel — Comprehensive settings panel opened via the gear icon.
 *
 * Sections:
 *   1. Profile — Display name, avatar, email, bio
 *   2. Appearance — 12 theme pairs (dark/light/high-contrast/color-blind), font size, UI density
 *   3. OAuth Login — Google, GitHub, GitLab, Bitbucket, Microsoft, Apple
 *   4. Saved Customizations — Export/import/reset workspace preferences
 *   5. API Keys — BYOK provider keys (existing functionality preserved)
 */
import React, { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  User, Palette, LogIn, Save, Key, X, Check, Upload, Download, RotateCcw,
  Sun, Moon, Eye, EyeOff, ChevronRight,
} from "lucide-react";

/* ── Types ──────────────────────────────────────────────────────────── */

type SettingsSection = "profile" | "appearance" | "oauth" | "customizations" | "apikeys";

interface UserProfile {
  displayName: string;
  email: string;
  bio: string;
  avatarUrl: string;
}

export interface ThemeDef {
  id: string;
  name: string;
  category: "standard" | "high-contrast" | "color-blind";
  mode: "dark" | "light";
  pairId: string; // links dark/light counterparts
  preview: { bg: string; fg: string; accent: string; secondary: string };
  vars: Record<string, string>;
}

interface OAuthProvider {
  id: string;
  name: string;
  icon: string;
  connected: boolean;
  email?: string;
}

interface SavedCustomization {
  id: string;
  name: string;
  createdAt: string;
  theme: string;
  fontSize: number;
  density: string;
}

interface ApiKeySettings {
  anthropic_api_key: string;
  openai_api_key: string;
  gemini_api_key: string;
  grok_api_key: string;
  claude_model: string;
  openai_model: string;
}

/* ── Theme definitions ─────────────────────────────────────────────── */

// Each theme has a pairId linking its dark/light counterpart
export const THEMES: ThemeDef[] = [
  // ── Pair: Default (Midnight Blue / Clean White) ──
  {
    id: "dark-default", name: "Midnight Blue", category: "standard", mode: "dark", pairId: "default",
    preview: { bg: "#0f1117", fg: "#e2e4ea", accent: "#6c8cff", secondary: "#161821" },
    vars: {
      "--bg-primary": "#0f1117", "--bg-secondary": "#161821", "--bg-tertiary": "#1c1f2b", "--bg-elevated": "#222638",
      "--text-primary": "#e2e4ea", "--text-secondary": "#6e7491", "--accent-blue": "#6c8cff", "--accent-green": "#34d399",
      "--accent-purple": "#a78bfa", "--accent-gold": "#f5c542", "--accent-rose": "#f472b6",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#ef4444",
    },
  },
  {
    id: "light-default", name: "Clean White", category: "standard", mode: "light", pairId: "default",
    preview: { bg: "#fafbfd", fg: "#1a1d2e", accent: "#4f6df5", secondary: "#f0f1f5" },
    vars: {
      "--bg-primary": "#fafbfd", "--bg-secondary": "#f0f1f5", "--bg-tertiary": "#e6e8ef", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1d2e", "--text-secondary": "#6b7089", "--accent-blue": "#4f6df5", "--accent-green": "#10b981",
      "--accent-purple": "#8b5cf6", "--accent-gold": "#d4a017", "--accent-rose": "#ec4899",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Charcoal / Silver ──
  {
    id: "dark-charcoal", name: "Charcoal", category: "standard", mode: "dark", pairId: "charcoal",
    preview: { bg: "#1a1a1a", fg: "#d4d4d4", accent: "#569cd6", secondary: "#252526" },
    vars: {
      "--bg-primary": "#1a1a1a", "--bg-secondary": "#252526", "--bg-tertiary": "#2d2d30", "--bg-elevated": "#333337",
      "--text-primary": "#d4d4d4", "--text-secondary": "#808080", "--accent-blue": "#569cd6", "--accent-green": "#6a9955",
      "--accent-purple": "#c586c0", "--accent-gold": "#dcdcaa", "--accent-rose": "#d7ba7d",
      "--border-color": "rgba(255, 255, 255, 0.05)", "--error-color": "#f14c4c",
    },
  },
  {
    id: "light-charcoal", name: "Silver", category: "standard", mode: "light", pairId: "charcoal",
    preview: { bg: "#f3f3f3", fg: "#1e1e1e", accent: "#005fb8", secondary: "#e8e8e8" },
    vars: {
      "--bg-primary": "#f3f3f3", "--bg-secondary": "#e8e8e8", "--bg-tertiary": "#d6d6d6", "--bg-elevated": "#ffffff",
      "--text-primary": "#1e1e1e", "--text-secondary": "#616161", "--accent-blue": "#005fb8", "--accent-green": "#388a34",
      "--accent-purple": "#8839a1", "--accent-gold": "#bf8803", "--accent-rose": "#c72e49",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#cd3131",
    },
  },
  // ── Pair: Warm (Warm Dusk / Warm Sand) ──
  {
    id: "dark-warm", name: "Warm Dusk", category: "standard", mode: "dark", pairId: "warm",
    preview: { bg: "#1a1410", fg: "#e6ddd0", accent: "#d4a373", secondary: "#2a2118" },
    vars: {
      "--bg-primary": "#1a1410", "--bg-secondary": "#2a2118", "--bg-tertiary": "#3a2e22", "--bg-elevated": "#453828",
      "--text-primary": "#e6ddd0", "--text-secondary": "#a89880", "--accent-blue": "#d4a373", "--accent-green": "#859900",
      "--accent-purple": "#b58db6", "--accent-gold": "#d4a373", "--accent-rose": "#d33682",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#dc322f",
    },
  },
  {
    id: "light-warm", name: "Warm Sand", category: "standard", mode: "light", pairId: "warm",
    preview: { bg: "#fdf6e3", fg: "#073642", accent: "#268bd2", secondary: "#eee8d5" },
    vars: {
      "--bg-primary": "#fdf6e3", "--bg-secondary": "#eee8d5", "--bg-tertiary": "#e0dbc7", "--bg-elevated": "#fffdf5",
      "--text-primary": "#073642", "--text-secondary": "#586e75", "--accent-blue": "#268bd2", "--accent-green": "#859900",
      "--accent-purple": "#6c71c4", "--accent-gold": "#b58900", "--accent-rose": "#d33682",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#dc322f",
    },
  },
  // ── Pair: Ocean (Deep Ocean / Coastal Light) ──
  {
    id: "dark-ocean", name: "Deep Ocean", category: "standard", mode: "dark", pairId: "ocean",
    preview: { bg: "#0d1b2a", fg: "#e0e1dd", accent: "#48cae4", secondary: "#1b2838" },
    vars: {
      "--bg-primary": "#0d1b2a", "--bg-secondary": "#1b2838", "--bg-tertiary": "#233345", "--bg-elevated": "#2b3e50",
      "--text-primary": "#e0e1dd", "--text-secondary": "#778da9", "--accent-blue": "#48cae4", "--accent-green": "#52b788",
      "--accent-purple": "#b392f0", "--accent-gold": "#ffb703", "--accent-rose": "#ff6b6b",
      "--border-color": "rgba(255, 255, 255, 0.05)", "--error-color": "#ff6b6b",
    },
  },
  {
    id: "light-ocean", name: "Coastal Light", category: "standard", mode: "light", pairId: "ocean",
    preview: { bg: "#f0f8ff", fg: "#0d1b2a", accent: "#0077b6", secondary: "#e0f0fa" },
    vars: {
      "--bg-primary": "#f0f8ff", "--bg-secondary": "#e0f0fa", "--bg-tertiary": "#c8e3f5", "--bg-elevated": "#ffffff",
      "--text-primary": "#0d1b2a", "--text-secondary": "#415a77", "--accent-blue": "#0077b6", "--accent-green": "#2d9f6f",
      "--accent-purple": "#7c5cbf", "--accent-gold": "#d4960a", "--accent-rose": "#d94e5c",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#d94e5c",
    },
  },
  // ── Pair: Rose (Rose Night / Rose Garden) ──
  {
    id: "dark-rose", name: "Rose Night", category: "standard", mode: "dark", pairId: "rose",
    preview: { bg: "#1a0f10", fg: "#f0dde0", accent: "#f43f5e", secondary: "#2a1a1c" },
    vars: {
      "--bg-primary": "#1a0f10", "--bg-secondary": "#2a1a1c", "--bg-tertiary": "#3a2528", "--bg-elevated": "#452e32",
      "--text-primary": "#f0dde0", "--text-secondary": "#a88b8e", "--accent-blue": "#f43f5e", "--accent-green": "#059669",
      "--accent-purple": "#a855f7", "--accent-gold": "#ca8a04", "--accent-rose": "#f43f5e",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#ef4444",
    },
  },
  {
    id: "light-rose", name: "Rose Garden", category: "standard", mode: "light", pairId: "rose",
    preview: { bg: "#fff5f5", fg: "#2d1b1b", accent: "#e11d48", secondary: "#ffe4e6" },
    vars: {
      "--bg-primary": "#fff5f5", "--bg-secondary": "#ffe4e6", "--bg-tertiary": "#fecdd3", "--bg-elevated": "#ffffff",
      "--text-primary": "#2d1b1b", "--text-secondary": "#9f6b6b", "--accent-blue": "#e11d48", "--accent-green": "#059669",
      "--accent-purple": "#a855f7", "--accent-gold": "#ca8a04", "--accent-rose": "#e11d48",
      "--border-color": "rgba(0, 0, 0, 0.06)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: High Contrast ──
  {
    id: "hc-dark", name: "High Contrast Dark", category: "high-contrast", mode: "dark", pairId: "hc",
    preview: { bg: "#000000", fg: "#ffffff", accent: "#00e0ff", secondary: "#0a0a0a" },
    vars: {
      "--bg-primary": "#000000", "--bg-secondary": "#0a0a0a", "--bg-tertiary": "#141414", "--bg-elevated": "#1e1e1e",
      "--text-primary": "#ffffff", "--text-secondary": "#cccccc", "--accent-blue": "#00e0ff", "--accent-green": "#00ff88",
      "--accent-purple": "#d0a0ff", "--accent-gold": "#ffdd00", "--accent-rose": "#ff6699",
      "--border-color": "rgba(255, 255, 255, 0.25)", "--error-color": "#ff3333",
    },
  },
  {
    id: "hc-light", name: "High Contrast Light", category: "high-contrast", mode: "light", pairId: "hc",
    preview: { bg: "#ffffff", fg: "#000000", accent: "#0033cc", secondary: "#f0f0f0" },
    vars: {
      "--bg-primary": "#ffffff", "--bg-secondary": "#f0f0f0", "--bg-tertiary": "#e0e0e0", "--bg-elevated": "#ffffff",
      "--text-primary": "#000000", "--text-secondary": "#333333", "--accent-blue": "#0033cc", "--accent-green": "#006633",
      "--accent-purple": "#6600cc", "--accent-gold": "#996600", "--accent-rose": "#cc0044",
      "--border-color": "rgba(0, 0, 0, 0.3)", "--error-color": "#cc0000",
    },
  },
  // ── Pair: Deuteranopia ──
  {
    id: "cb-deuteranopia-dark", name: "Deuteranopia Dark", category: "color-blind", mode: "dark", pairId: "deuteranopia",
    preview: { bg: "#0f1117", fg: "#e2e4ea", accent: "#648fff", secondary: "#161821" },
    vars: {
      "--bg-primary": "#0f1117", "--bg-secondary": "#161821", "--bg-tertiary": "#1c1f2b", "--bg-elevated": "#222638",
      "--text-primary": "#e2e4ea", "--text-secondary": "#6e7491", "--accent-blue": "#648fff", "--accent-green": "#ffb000",
      "--accent-purple": "#dc267f", "--accent-gold": "#ffb000", "--accent-rose": "#dc267f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#fe6100",
      "--success-color": "#ffb000", "--warning-color": "#fe6100",
    },
  },
  {
    id: "cb-deuteranopia-light", name: "Deuteranopia Light", category: "color-blind", mode: "light", pairId: "deuteranopia",
    preview: { bg: "#fafbfd", fg: "#1a1d2e", accent: "#3949ab", secondary: "#f0f1f5" },
    vars: {
      "--bg-primary": "#fafbfd", "--bg-secondary": "#f0f1f5", "--bg-tertiary": "#e6e8ef", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1d2e", "--text-secondary": "#6b7089", "--accent-blue": "#3949ab", "--accent-green": "#e68a00",
      "--accent-purple": "#ad1457", "--accent-gold": "#e68a00", "--accent-rose": "#ad1457",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#d84315",
      "--success-color": "#e68a00", "--warning-color": "#d84315",
    },
  },
  // ── Pair: Protanopia ──
  {
    id: "cb-protanopia-dark", name: "Protanopia Dark", category: "color-blind", mode: "dark", pairId: "protanopia",
    preview: { bg: "#0f1117", fg: "#e2e4ea", accent: "#785ef0", secondary: "#161821" },
    vars: {
      "--bg-primary": "#0f1117", "--bg-secondary": "#161821", "--bg-tertiary": "#1c1f2b", "--bg-elevated": "#222638",
      "--text-primary": "#e2e4ea", "--text-secondary": "#6e7491", "--accent-blue": "#785ef0", "--accent-green": "#ffb000",
      "--accent-purple": "#648fff", "--accent-gold": "#ffb000", "--accent-rose": "#dc267f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#fe6100",
      "--success-color": "#ffb000", "--warning-color": "#fe6100",
    },
  },
  {
    id: "cb-protanopia-light", name: "Protanopia Light", category: "color-blind", mode: "light", pairId: "protanopia",
    preview: { bg: "#fafbfd", fg: "#1a1d2e", accent: "#5c41c9", secondary: "#f0f1f5" },
    vars: {
      "--bg-primary": "#fafbfd", "--bg-secondary": "#f0f1f5", "--bg-tertiary": "#e6e8ef", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1d2e", "--text-secondary": "#6b7089", "--accent-blue": "#5c41c9", "--accent-green": "#e68a00",
      "--accent-purple": "#3d5afe", "--accent-gold": "#e68a00", "--accent-rose": "#ad1457",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#d84315",
      "--success-color": "#e68a00", "--warning-color": "#d84315",
    },
  },
  // ── Pair: Tritanopia ──
  {
    id: "cb-tritanopia-dark", name: "Tritanopia Dark", category: "color-blind", mode: "dark", pairId: "tritanopia",
    preview: { bg: "#0f1117", fg: "#e2e4ea", accent: "#e8384f", secondary: "#161821" },
    vars: {
      "--bg-primary": "#0f1117", "--bg-secondary": "#161821", "--bg-tertiary": "#1c1f2b", "--bg-elevated": "#222638",
      "--text-primary": "#e2e4ea", "--text-secondary": "#6e7491", "--accent-blue": "#e8384f", "--accent-green": "#37a862",
      "--accent-purple": "#e8384f", "--accent-gold": "#37a862", "--accent-rose": "#e8384f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e8384f",
      "--success-color": "#37a862", "--warning-color": "#e8a537",
    },
  },
  {
    id: "cb-tritanopia-light", name: "Tritanopia Light", category: "color-blind", mode: "light", pairId: "tritanopia",
    preview: { bg: "#fafbfd", fg: "#1a1d2e", accent: "#c62038", secondary: "#f0f1f5" },
    vars: {
      "--bg-primary": "#fafbfd", "--bg-secondary": "#f0f1f5", "--bg-tertiary": "#e6e8ef", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1d2e", "--text-secondary": "#6b7089", "--accent-blue": "#c62038", "--accent-green": "#2a8a4e",
      "--accent-purple": "#c62038", "--accent-gold": "#2a8a4e", "--accent-rose": "#c62038",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#c62038",
      "--success-color": "#2a8a4e", "--warning-color": "#c69425",
    },
  },

  // ═══════════════════════════════════════════════════════════════════
  //  Popular Developer & Organization Themes
  // ═══════════════════════════════════════════════════════════════════

  // ── Pair: Monokai ──
  {
    id: "dark-monokai", name: "Monokai", category: "standard", mode: "dark", pairId: "monokai",
    preview: { bg: "#272822", fg: "#f8f8f2", accent: "#a6e22e", secondary: "#3e3d32" },
    vars: {
      "--bg-primary": "#272822", "--bg-secondary": "#3e3d32", "--bg-tertiary": "#49483e", "--bg-elevated": "#555449",
      "--text-primary": "#f8f8f2", "--text-secondary": "#75715e", "--accent-blue": "#66d9ef", "--accent-green": "#a6e22e",
      "--accent-purple": "#ae81ff", "--accent-gold": "#e6db74", "--accent-rose": "#f92672",
      "--border-color": "rgba(255, 255, 255, 0.07)", "--error-color": "#f92672",
    },
  },
  {
    id: "light-monokai", name: "Monokai Light", category: "standard", mode: "light", pairId: "monokai",
    preview: { bg: "#fafafa", fg: "#272822", accent: "#629755", secondary: "#eeeee8" },
    vars: {
      "--bg-primary": "#fafafa", "--bg-secondary": "#eeeee8", "--bg-tertiary": "#e0e0d8", "--bg-elevated": "#ffffff",
      "--text-primary": "#272822", "--text-secondary": "#75715e", "--accent-blue": "#1290bf", "--accent-green": "#629755",
      "--accent-purple": "#7a3ea0", "--accent-gold": "#b58900", "--accent-rose": "#c4265e",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#c4265e",
    },
  },
  // ── Pair: Dracula ──
  {
    id: "dark-dracula", name: "Dracula", category: "standard", mode: "dark", pairId: "dracula",
    preview: { bg: "#282a36", fg: "#f8f8f2", accent: "#bd93f9", secondary: "#44475a" },
    vars: {
      "--bg-primary": "#282a36", "--bg-secondary": "#44475a", "--bg-tertiary": "#4e5166", "--bg-elevated": "#555770",
      "--text-primary": "#f8f8f2", "--text-secondary": "#6272a4", "--accent-blue": "#8be9fd", "--accent-green": "#50fa7b",
      "--accent-purple": "#bd93f9", "--accent-gold": "#f1fa8c", "--accent-rose": "#ff79c6",
      "--border-color": "rgba(255, 255, 255, 0.08)", "--error-color": "#ff5555",
    },
  },
  {
    id: "light-dracula", name: "Dracula Soft", category: "standard", mode: "light", pairId: "dracula",
    preview: { bg: "#f8f8f2", fg: "#282a36", accent: "#7c3aed", secondary: "#ededec" },
    vars: {
      "--bg-primary": "#f8f8f2", "--bg-secondary": "#ededec", "--bg-tertiary": "#e0dfe0", "--bg-elevated": "#ffffff",
      "--text-primary": "#282a36", "--text-secondary": "#6272a4", "--accent-blue": "#0891b2", "--accent-green": "#16a34a",
      "--accent-purple": "#7c3aed", "--accent-gold": "#a16207", "--accent-rose": "#db2777",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#dc2626",
    },
  },
  // ── Pair: Nord ──
  {
    id: "dark-nord", name: "Nord", category: "standard", mode: "dark", pairId: "nord",
    preview: { bg: "#2e3440", fg: "#eceff4", accent: "#88c0d0", secondary: "#3b4252" },
    vars: {
      "--bg-primary": "#2e3440", "--bg-secondary": "#3b4252", "--bg-tertiary": "#434c5e", "--bg-elevated": "#4c566a",
      "--text-primary": "#eceff4", "--text-secondary": "#7b88a1", "--accent-blue": "#88c0d0", "--accent-green": "#a3be8c",
      "--accent-purple": "#b48ead", "--accent-gold": "#ebcb8b", "--accent-rose": "#bf616a",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#bf616a",
    },
  },
  {
    id: "light-nord", name: "Nord Light", category: "standard", mode: "light", pairId: "nord",
    preview: { bg: "#eceff4", fg: "#2e3440", accent: "#5e81ac", secondary: "#e5e9f0" },
    vars: {
      "--bg-primary": "#eceff4", "--bg-secondary": "#e5e9f0", "--bg-tertiary": "#d8dee9", "--bg-elevated": "#f8fafc",
      "--text-primary": "#2e3440", "--text-secondary": "#4c566a", "--accent-blue": "#5e81ac", "--accent-green": "#689d6a",
      "--accent-purple": "#8f6594", "--accent-gold": "#c08b30", "--accent-rose": "#a3373e",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#a3373e",
    },
  },
  // ── Pair: One (Atom) ──
  {
    id: "dark-one", name: "One Dark", category: "standard", mode: "dark", pairId: "one",
    preview: { bg: "#282c34", fg: "#abb2bf", accent: "#61afef", secondary: "#21252b" },
    vars: {
      "--bg-primary": "#282c34", "--bg-secondary": "#21252b", "--bg-tertiary": "#2c313a", "--bg-elevated": "#333842",
      "--text-primary": "#abb2bf", "--text-secondary": "#5c6370", "--accent-blue": "#61afef", "--accent-green": "#98c379",
      "--accent-purple": "#c678dd", "--accent-gold": "#e5c07b", "--accent-rose": "#e06c75",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e06c75",
    },
  },
  {
    id: "light-one", name: "One Light", category: "standard", mode: "light", pairId: "one",
    preview: { bg: "#fafafa", fg: "#383a42", accent: "#4078f2", secondary: "#f0f0f0" },
    vars: {
      "--bg-primary": "#fafafa", "--bg-secondary": "#f0f0f0", "--bg-tertiary": "#e5e5e6", "--bg-elevated": "#ffffff",
      "--text-primary": "#383a42", "--text-secondary": "#a0a1a7", "--accent-blue": "#4078f2", "--accent-green": "#50a14f",
      "--accent-purple": "#a626a4", "--accent-gold": "#c18401", "--accent-rose": "#e45649",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#e45649",
    },
  },
  // ── Pair: GitHub ──
  {
    id: "dark-github", name: "GitHub Dark", category: "standard", mode: "dark", pairId: "github",
    preview: { bg: "#0d1117", fg: "#e6edf3", accent: "#58a6ff", secondary: "#161b22" },
    vars: {
      "--bg-primary": "#0d1117", "--bg-secondary": "#161b22", "--bg-tertiary": "#21262d", "--bg-elevated": "#30363d",
      "--text-primary": "#e6edf3", "--text-secondary": "#7d8590", "--accent-blue": "#58a6ff", "--accent-green": "#3fb950",
      "--accent-purple": "#bc8cff", "--accent-gold": "#d29922", "--accent-rose": "#f85149",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f85149",
    },
  },
  {
    id: "light-github", name: "GitHub Light", category: "standard", mode: "light", pairId: "github",
    preview: { bg: "#ffffff", fg: "#1f2328", accent: "#0969da", secondary: "#f6f8fa" },
    vars: {
      "--bg-primary": "#ffffff", "--bg-secondary": "#f6f8fa", "--bg-tertiary": "#eaeef2", "--bg-elevated": "#ffffff",
      "--text-primary": "#1f2328", "--text-secondary": "#656d76", "--accent-blue": "#0969da", "--accent-green": "#1a7f37",
      "--accent-purple": "#8250df", "--accent-gold": "#9a6700", "--accent-rose": "#cf222e",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#cf222e",
    },
  },
  // ── Pair: Catppuccin (Mocha/Latte) ──
  {
    id: "dark-catppuccin", name: "Catppuccin Mocha", category: "standard", mode: "dark", pairId: "catppuccin",
    preview: { bg: "#1e1e2e", fg: "#cdd6f4", accent: "#89b4fa", secondary: "#313244" },
    vars: {
      "--bg-primary": "#1e1e2e", "--bg-secondary": "#313244", "--bg-tertiary": "#45475a", "--bg-elevated": "#585b70",
      "--text-primary": "#cdd6f4", "--text-secondary": "#6c7086", "--accent-blue": "#89b4fa", "--accent-green": "#a6e3a1",
      "--accent-purple": "#cba6f7", "--accent-gold": "#f9e2af", "--accent-rose": "#f38ba8",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f38ba8",
    },
  },
  {
    id: "light-catppuccin", name: "Catppuccin Latte", category: "standard", mode: "light", pairId: "catppuccin",
    preview: { bg: "#eff1f5", fg: "#4c4f69", accent: "#1e66f5", secondary: "#e6e9ef" },
    vars: {
      "--bg-primary": "#eff1f5", "--bg-secondary": "#e6e9ef", "--bg-tertiary": "#ccd0da", "--bg-elevated": "#ffffff",
      "--text-primary": "#4c4f69", "--text-secondary": "#6c6f85", "--accent-blue": "#1e66f5", "--accent-green": "#40a02b",
      "--accent-purple": "#8839ef", "--accent-gold": "#df8e1d", "--accent-rose": "#d20f39",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#d20f39",
    },
  },
  // ── Pair: Gruvbox ──
  {
    id: "dark-gruvbox", name: "Gruvbox Dark", category: "standard", mode: "dark", pairId: "gruvbox",
    preview: { bg: "#282828", fg: "#ebdbb2", accent: "#fabd2f", secondary: "#3c3836" },
    vars: {
      "--bg-primary": "#282828", "--bg-secondary": "#3c3836", "--bg-tertiary": "#504945", "--bg-elevated": "#665c54",
      "--text-primary": "#ebdbb2", "--text-secondary": "#928374", "--accent-blue": "#83a598", "--accent-green": "#b8bb26",
      "--accent-purple": "#d3869b", "--accent-gold": "#fabd2f", "--accent-rose": "#fb4934",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#fb4934",
    },
  },
  {
    id: "light-gruvbox", name: "Gruvbox Light", category: "standard", mode: "light", pairId: "gruvbox",
    preview: { bg: "#fbf1c7", fg: "#3c3836", accent: "#b57614", secondary: "#ebdbb2" },
    vars: {
      "--bg-primary": "#fbf1c7", "--bg-secondary": "#ebdbb2", "--bg-tertiary": "#d5c4a1", "--bg-elevated": "#fffbef",
      "--text-primary": "#3c3836", "--text-secondary": "#7c6f64", "--accent-blue": "#076678", "--accent-green": "#79740e",
      "--accent-purple": "#8f3f71", "--accent-gold": "#b57614", "--accent-rose": "#9d0006",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#9d0006",
    },
  },
  // ── Pair: Tokyo Night ──
  {
    id: "dark-tokyo", name: "Tokyo Night", category: "standard", mode: "dark", pairId: "tokyo",
    preview: { bg: "#1a1b26", fg: "#c0caf5", accent: "#7aa2f7", secondary: "#24283b" },
    vars: {
      "--bg-primary": "#1a1b26", "--bg-secondary": "#24283b", "--bg-tertiary": "#2f3347", "--bg-elevated": "#3b3f54",
      "--text-primary": "#c0caf5", "--text-secondary": "#565f89", "--accent-blue": "#7aa2f7", "--accent-green": "#9ece6a",
      "--accent-purple": "#bb9af7", "--accent-gold": "#e0af68", "--accent-rose": "#f7768e",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f7768e",
    },
  },
  {
    id: "light-tokyo", name: "Tokyo Day", category: "standard", mode: "light", pairId: "tokyo",
    preview: { bg: "#d5d6db", fg: "#343b58", accent: "#34548a", secondary: "#c8c8ce" },
    vars: {
      "--bg-primary": "#d5d6db", "--bg-secondary": "#c8c8ce", "--bg-tertiary": "#b8b8c0", "--bg-elevated": "#e5e5ea",
      "--text-primary": "#343b58", "--text-secondary": "#6172a6", "--accent-blue": "#34548a", "--accent-green": "#485e30",
      "--accent-purple": "#7847bd", "--accent-gold": "#8f5e15", "--accent-rose": "#8c4351",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#8c4351",
    },
  },
  // ── Pair: Material ──
  {
    id: "dark-material", name: "Material Darker", category: "standard", mode: "dark", pairId: "material",
    preview: { bg: "#212121", fg: "#eeffff", accent: "#82aaff", secondary: "#303030" },
    vars: {
      "--bg-primary": "#212121", "--bg-secondary": "#303030", "--bg-tertiary": "#3a3a3a", "--bg-elevated": "#424242",
      "--text-primary": "#eeffff", "--text-secondary": "#545454", "--accent-blue": "#82aaff", "--accent-green": "#c3e88d",
      "--accent-purple": "#c792ea", "--accent-gold": "#ffcb6b", "--accent-rose": "#f07178",
      "--border-color": "rgba(255, 255, 255, 0.05)", "--error-color": "#f07178",
    },
  },
  {
    id: "light-material", name: "Material Lighter", category: "standard", mode: "light", pairId: "material",
    preview: { bg: "#fafafa", fg: "#546e7a", accent: "#6182b8", secondary: "#eaeaea" },
    vars: {
      "--bg-primary": "#fafafa", "--bg-secondary": "#eaeaea", "--bg-tertiary": "#d4d4d4", "--bg-elevated": "#ffffff",
      "--text-primary": "#546e7a", "--text-secondary": "#90a4ae", "--accent-blue": "#6182b8", "--accent-green": "#91b859",
      "--accent-purple": "#7c4dff", "--accent-gold": "#f6a434", "--accent-rose": "#e53935",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#e53935",
    },
  },
  // ── Pair: Solarized ──
  {
    id: "dark-solarized", name: "Solarized Dark", category: "standard", mode: "dark", pairId: "solarized",
    preview: { bg: "#002b36", fg: "#839496", accent: "#268bd2", secondary: "#073642" },
    vars: {
      "--bg-primary": "#002b36", "--bg-secondary": "#073642", "--bg-tertiary": "#0a4050", "--bg-elevated": "#0d4f5e",
      "--text-primary": "#839496", "--text-secondary": "#586e75", "--accent-blue": "#268bd2", "--accent-green": "#859900",
      "--accent-purple": "#6c71c4", "--accent-gold": "#b58900", "--accent-rose": "#dc322f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#dc322f",
    },
  },
  {
    id: "light-solarized", name: "Solarized Light", category: "standard", mode: "light", pairId: "solarized",
    preview: { bg: "#fdf6e3", fg: "#657b83", accent: "#268bd2", secondary: "#eee8d5" },
    vars: {
      "--bg-primary": "#fdf6e3", "--bg-secondary": "#eee8d5", "--bg-tertiary": "#e0dbc7", "--bg-elevated": "#fffdf5",
      "--text-primary": "#657b83", "--text-secondary": "#93a1a1", "--accent-blue": "#268bd2", "--accent-green": "#859900",
      "--accent-purple": "#6c71c4", "--accent-gold": "#b58900", "--accent-rose": "#dc322f",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#dc322f",
    },
  },
  // ── Pair: Palenight ──
  {
    id: "dark-palenight", name: "Palenight", category: "standard", mode: "dark", pairId: "palenight",
    preview: { bg: "#292d3e", fg: "#a6accd", accent: "#82aaff", secondary: "#34324a" },
    vars: {
      "--bg-primary": "#292d3e", "--bg-secondary": "#34324a", "--bg-tertiary": "#3e3c56", "--bg-elevated": "#484660",
      "--text-primary": "#a6accd", "--text-secondary": "#676e95", "--accent-blue": "#82aaff", "--accent-green": "#c3e88d",
      "--accent-purple": "#c792ea", "--accent-gold": "#ffcb6b", "--accent-rose": "#f07178",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f07178",
    },
  },
  {
    id: "light-palenight", name: "Paleday", category: "standard", mode: "light", pairId: "palenight",
    preview: { bg: "#f0f0f8", fg: "#3b3d55", accent: "#5a6acf", secondary: "#e4e4ef" },
    vars: {
      "--bg-primary": "#f0f0f8", "--bg-secondary": "#e4e4ef", "--bg-tertiary": "#d4d4e2", "--bg-elevated": "#fafaff",
      "--text-primary": "#3b3d55", "--text-secondary": "#676e95", "--accent-blue": "#5a6acf", "--accent-green": "#689d6a",
      "--accent-purple": "#9c5fb5", "--accent-gold": "#c08b30", "--accent-rose": "#c45060",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#c45060",
    },
  },
  // ── Pair: Ayu ──
  {
    id: "dark-ayu", name: "Ayu Dark", category: "standard", mode: "dark", pairId: "ayu",
    preview: { bg: "#0a0e14", fg: "#b3b1ad", accent: "#ffb454", secondary: "#1f2430" },
    vars: {
      "--bg-primary": "#0a0e14", "--bg-secondary": "#1f2430", "--bg-tertiary": "#272d38", "--bg-elevated": "#2e3440",
      "--text-primary": "#b3b1ad", "--text-secondary": "#5c6773", "--accent-blue": "#36a3d9", "--accent-green": "#bae67e",
      "--accent-purple": "#d4bfff", "--accent-gold": "#ffb454", "--accent-rose": "#ff3333",
      "--border-color": "rgba(255, 255, 255, 0.05)", "--error-color": "#ff3333",
    },
  },
  {
    id: "light-ayu", name: "Ayu Light", category: "standard", mode: "light", pairId: "ayu",
    preview: { bg: "#fafafa", fg: "#575f66", accent: "#ff9940", secondary: "#f0f0f0" },
    vars: {
      "--bg-primary": "#fafafa", "--bg-secondary": "#f0f0f0", "--bg-tertiary": "#e1e1e1", "--bg-elevated": "#ffffff",
      "--text-primary": "#575f66", "--text-secondary": "#abb0b6", "--accent-blue": "#399ee6", "--accent-green": "#86b300",
      "--accent-purple": "#a37acc", "--accent-gold": "#ff9940", "--accent-rose": "#f51818",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#f51818",
    },
  },
  // ── Pair: Slack (Organization) ──
  {
    id: "dark-slack", name: "Slack Aubergine", category: "standard", mode: "dark", pairId: "slack",
    preview: { bg: "#1a1d21", fg: "#d1d2d3", accent: "#36c5f0", secondary: "#27242c" },
    vars: {
      "--bg-primary": "#1a1d21", "--bg-secondary": "#27242c", "--bg-tertiary": "#332f3b", "--bg-elevated": "#3d3848",
      "--text-primary": "#d1d2d3", "--text-secondary": "#9a9a9d", "--accent-blue": "#36c5f0", "--accent-green": "#2eb67d",
      "--accent-purple": "#611f69", "--accent-gold": "#ecb22e", "--accent-rose": "#e01e5a",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e01e5a",
    },
  },
  {
    id: "light-slack", name: "Slack Daylight", category: "standard", mode: "light", pairId: "slack",
    preview: { bg: "#ffffff", fg: "#1d1c1d", accent: "#1264a3", secondary: "#f8f8f8" },
    vars: {
      "--bg-primary": "#ffffff", "--bg-secondary": "#f8f8f8", "--bg-tertiary": "#ececec", "--bg-elevated": "#ffffff",
      "--text-primary": "#1d1c1d", "--text-secondary": "#616061", "--accent-blue": "#1264a3", "--accent-green": "#007a5a",
      "--accent-purple": "#611f69", "--accent-gold": "#daa520", "--accent-rose": "#e01e5a",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#e01e5a",
    },
  },
  // ── Pair: Cobalt ──
  {
    id: "dark-cobalt", name: "Cobalt2", category: "standard", mode: "dark", pairId: "cobalt",
    preview: { bg: "#193549", fg: "#e1efff", accent: "#ffc600", secondary: "#1f4662" },
    vars: {
      "--bg-primary": "#193549", "--bg-secondary": "#1f4662", "--bg-tertiary": "#245170", "--bg-elevated": "#2a5c80",
      "--text-primary": "#e1efff", "--text-secondary": "#6fa0c7", "--accent-blue": "#80ffbb", "--accent-green": "#3ad900",
      "--accent-purple": "#fb94ff", "--accent-gold": "#ffc600", "--accent-rose": "#ff628c",
      "--border-color": "rgba(255, 255, 255, 0.08)", "--error-color": "#ff628c",
    },
  },
  {
    id: "light-cobalt", name: "Cobalt Light", category: "standard", mode: "light", pairId: "cobalt",
    preview: { bg: "#f0f5fa", fg: "#193549", accent: "#b8860b", secondary: "#dfe8f0" },
    vars: {
      "--bg-primary": "#f0f5fa", "--bg-secondary": "#dfe8f0", "--bg-tertiary": "#c8d8e4", "--bg-elevated": "#ffffff",
      "--text-primary": "#193549", "--text-secondary": "#4e7a97", "--accent-blue": "#16825d", "--accent-green": "#2e8b00",
      "--accent-purple": "#9b42a0", "--accent-gold": "#b8860b", "--accent-rose": "#c0284a",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#c0284a",
    },
  },
  // ── Pair: Synthwave ──
  {
    id: "dark-synthwave", name: "Synthwave '84", category: "standard", mode: "dark", pairId: "synthwave",
    preview: { bg: "#262335", fg: "#e0def4", accent: "#f97e72", secondary: "#34294f" },
    vars: {
      "--bg-primary": "#262335", "--bg-secondary": "#34294f", "--bg-tertiary": "#3e3461", "--bg-elevated": "#4a3f73",
      "--text-primary": "#e0def4", "--text-secondary": "#7f7a9b", "--accent-blue": "#72f1b8", "--accent-green": "#72f1b8",
      "--accent-purple": "#f97e72", "--accent-gold": "#fede5d", "--accent-rose": "#fe4450",
      "--border-color": "rgba(255, 255, 255, 0.07)", "--error-color": "#fe4450",
    },
  },
  {
    id: "light-synthwave", name: "Synthwave Day", category: "standard", mode: "light", pairId: "synthwave",
    preview: { bg: "#f5f0ff", fg: "#2d2350", accent: "#c44040", secondary: "#e8e0f5" },
    vars: {
      "--bg-primary": "#f5f0ff", "--bg-secondary": "#e8e0f5", "--bg-tertiary": "#d8cee8", "--bg-elevated": "#ffffff",
      "--text-primary": "#2d2350", "--text-secondary": "#685a8b", "--accent-blue": "#2a8a5e", "--accent-green": "#2a8a5e",
      "--accent-purple": "#c44040", "--accent-gold": "#a78000", "--accent-rose": "#c02030",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#c02030",
    },
  },
  // ── Pair: Everforest ──
  {
    id: "dark-everforest", name: "Everforest Dark", category: "standard", mode: "dark", pairId: "everforest",
    preview: { bg: "#2d353b", fg: "#d3c6aa", accent: "#a7c080", secondary: "#343f44" },
    vars: {
      "--bg-primary": "#2d353b", "--bg-secondary": "#343f44", "--bg-tertiary": "#3d484d", "--bg-elevated": "#475258",
      "--text-primary": "#d3c6aa", "--text-secondary": "#859289", "--accent-blue": "#7fbbb3", "--accent-green": "#a7c080",
      "--accent-purple": "#d699b6", "--accent-gold": "#dbbc7f", "--accent-rose": "#e67e80",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e67e80",
    },
  },
  {
    id: "light-everforest", name: "Everforest Light", category: "standard", mode: "light", pairId: "everforest",
    preview: { bg: "#fdf6e3", fg: "#5c6a72", accent: "#8da101", secondary: "#f0ead2" },
    vars: {
      "--bg-primary": "#fdf6e3", "--bg-secondary": "#f0ead2", "--bg-tertiary": "#e0dab8", "--bg-elevated": "#fffbf0",
      "--text-primary": "#5c6a72", "--text-secondary": "#829181", "--accent-blue": "#3a94c5", "--accent-green": "#8da101",
      "--accent-purple": "#df69ba", "--accent-gold": "#dfa000", "--accent-rose": "#f85552",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#f85552",
    },
  },
  // ── Pair: Kanagawa ──
  {
    id: "dark-kanagawa", name: "Kanagawa", category: "standard", mode: "dark", pairId: "kanagawa",
    preview: { bg: "#1f1f28", fg: "#dcd7ba", accent: "#7e9cd8", secondary: "#2a2a37" },
    vars: {
      "--bg-primary": "#1f1f28", "--bg-secondary": "#2a2a37", "--bg-tertiary": "#363646", "--bg-elevated": "#3d3d55",
      "--text-primary": "#dcd7ba", "--text-secondary": "#727169", "--accent-blue": "#7e9cd8", "--accent-green": "#98bb6c",
      "--accent-purple": "#957fb8", "--accent-gold": "#e6c384", "--accent-rose": "#e82424",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e82424",
    },
  },
  {
    id: "light-kanagawa", name: "Kanagawa Lotus", category: "standard", mode: "light", pairId: "kanagawa",
    preview: { bg: "#f2ecbc", fg: "#43436c", accent: "#4d699b", secondary: "#e7dba0" },
    vars: {
      "--bg-primary": "#f2ecbc", "--bg-secondary": "#e7dba0", "--bg-tertiary": "#d8cc88", "--bg-elevated": "#faf5d0",
      "--text-primary": "#43436c", "--text-secondary": "#8a8980", "--accent-blue": "#4d699b", "--accent-green": "#6f894e",
      "--accent-purple": "#624c83", "--accent-gold": "#a96b2c", "--accent-rose": "#c84053",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#c84053",
    },
  },
];

/**
 * For each theme, ensure adequate contrast for buttons:
 * - --btn-primary-fg: text color for primary buttons (on accent-blue bg)
 * - --btn-error-fg: text color for error buttons (on error-color bg)
 * - --text-secondary must have >=3:1 contrast on --bg-tertiary
 *
 * This post-process step auto-corrects any theme missing these.
 */
function hexLum(hex: string): number {
  const h = hex.replace('#', '');
  const r = parseInt(h.substring(0, 2), 16) / 255;
  const g = parseInt(h.substring(2, 4), 16) / 255;
  const b = parseInt(h.substring(4, 6), 16) / 255;
  const srgb = (c: number) => c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  return 0.2126 * srgb(r) + 0.7152 * srgb(g) + 0.0722 * srgb(b);
}
function contrast(h1: string, h2: string): number {
  const l1 = hexLum(h1), l2 = hexLum(h2);
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}
function bestFgForBg(bg: string): string {
  return contrast("#ffffff", bg) >= contrast("#000000", bg) ? "#ffffff" : "#000000";
}

// Post-process: add --btn-primary-fg and --btn-error-fg
for (const t of THEMES) {
  const accent = t.vars["--accent-blue"];
  const err = t.vars["--error-color"];
  if (accent && !accent.startsWith("rgba")) {
    t.vars["--btn-primary-fg"] = bestFgForBg(accent);
  }
  if (err && !err.startsWith("rgba")) {
    t.vars["--btn-error-fg"] = bestFgForBg(err);
  }
  // Fix secondary text if contrast on bg-tertiary is < 3:1
  const sec = t.vars["--text-secondary"];
  const tert = t.vars["--bg-tertiary"];
  if (sec && tert && !sec.startsWith("rgba") && !tert.startsWith("rgba")) {
    if (contrast(sec, tert) < 3.0) {
      // Darken for light themes, lighten for dark themes
      if (t.mode === "dark") {
        // Make secondary lighter
        const lum = hexLum(sec);
        const tertLum = hexLum(tert);
        // Target ~3.5:1 contrast — solve for needed luminance
        const needed = (tertLum + 0.05) * 3.5 - 0.05;
        if (needed <= 1 && needed > lum) {
          const factor = Math.min(1, needed / Math.max(lum, 0.01));
          const h = sec.replace('#', '');
          const r = Math.min(255, Math.round(parseInt(h.substring(0, 2), 16) * factor + (255 - parseInt(h.substring(0, 2), 16) * factor) * 0.3));
          const g = Math.min(255, Math.round(parseInt(h.substring(2, 4), 16) * factor + (255 - parseInt(h.substring(2, 4), 16) * factor) * 0.3));
          const b = Math.min(255, Math.round(parseInt(h.substring(4, 6), 16) * factor + (255 - parseInt(h.substring(4, 6), 16) * factor) * 0.3));
          t.vars["--text-secondary"] = `#${r.toString(16).padStart(2,'0')}${g.toString(16).padStart(2,'0')}${b.toString(16).padStart(2,'0')}`;
        }
      } else {
        // Make secondary darker
        const h = sec.replace('#', '');
        const r = Math.max(0, Math.round(parseInt(h.substring(0, 2), 16) * 0.65));
        const g = Math.max(0, Math.round(parseInt(h.substring(2, 4), 16) * 0.65));
        const b = Math.max(0, Math.round(parseInt(h.substring(4, 6), 16) * 0.65));
        t.vars["--text-secondary"] = `#${r.toString(16).padStart(2,'0')}${g.toString(16).padStart(2,'0')}${b.toString(16).padStart(2,'0')}`;
      }
    }
  }
}

/** Get the paired theme (dark↔light) for the given theme id */
export function getPairedTheme(currentId: string): ThemeDef | undefined {
  const current = THEMES.find(t => t.id === currentId);
  if (!current) return undefined;
  const targetMode = current.mode === "dark" ? "light" : "dark";
  return THEMES.find(t => t.pairId === current.pairId && t.mode === targetMode);
}

/** Apply a theme by id — sets CSS vars and localStorage */
export function applyThemeById(themeId: string): void {
  const theme = THEMES.find(t => t.id === themeId);
  if (!theme) return;
  localStorage.setItem("vibeui-theme-id", theme.id);
  localStorage.setItem("vibeui-theme", theme.mode);
  document.documentElement.setAttribute("data-theme", theme.mode);
  for (const [key, value] of Object.entries(theme.vars)) {
    document.documentElement.style.setProperty(key, value);
  }
}

const OAUTH_PROVIDERS: OAuthProvider[] = [
  { id: "google", name: "Google", icon: "G", connected: false },
  { id: "github", name: "GitHub", icon: "GH", connected: false },
  { id: "gitlab", name: "GitLab", icon: "GL", connected: false },
  { id: "bitbucket", name: "Bitbucket", icon: "BB", connected: false },
  { id: "microsoft", name: "Microsoft", icon: "MS", connected: false },
  { id: "apple", name: "Apple", icon: "A", connected: false },
];

const STORAGE_KEYS = {
  profile: "vibeui-profile",
  theme: "vibeui-theme-id",
  themeMode: "vibeui-theme",
  fontSize: "vibeui-font-size",
  density: "vibeui-density",
  oauth: "vibeui-oauth",
  customizations: "vibeui-customizations",
};

/* ── Shared styles ─────────────────────────────────────────────────── */

const sectionBtnStyle = (active: boolean): React.CSSProperties => ({
  display: "flex", alignItems: "center", gap: 10, width: "100%", padding: "10px 14px",
  background: active ? "var(--accent-bg)" : "transparent", border: "none",
  borderRadius: "var(--radius-sm)", cursor: "pointer", fontSize: 13, fontWeight: active ? 600 : 400,
  color: active ? "var(--accent-blue)" : "var(--text-primary)", textAlign: "left",
  transition: "var(--transition-fast)",
});

const labelStyle: React.CSSProperties = { display: "block", fontSize: 11, color: "var(--text-secondary)", marginBottom: 4, fontWeight: 500 };
const fieldStyle: React.CSSProperties = {
  width: "100%", boxSizing: "border-box" as const, padding: "8px 10px", fontSize: 13,
  background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
  color: "var(--text-primary)", borderRadius: "var(--radius-sm)", transition: "var(--transition-fast)",
};
const btnStyle: React.CSSProperties = {
  padding: "8px 16px", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)",
  background: "var(--bg-elevated)", color: "var(--text-primary)", cursor: "pointer",
  fontSize: 12, fontWeight: 500, transition: "var(--transition-fast)",
};
const btnPrimary: React.CSSProperties = { ...btnStyle, background: "var(--accent-blue)", color: "var(--btn-primary-fg)", borderColor: "var(--accent-blue)" };
const dividerStyle: React.CSSProperties = { height: 1, background: "var(--border-color)", margin: "16px 0" };
const cardBox: React.CSSProperties = {
  padding: 14, borderRadius: "var(--radius-md)", border: "1px solid var(--border-color)",
  background: "var(--bg-secondary)", marginBottom: 10,
};

/* ── Section Components ────────────────────────────────────────────── */

function ProfileSection() {
  const [profile, setProfile] = useState<UserProfile>({ displayName: "", email: "", bio: "", avatarUrl: "" });
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    const stored = localStorage.getItem(STORAGE_KEYS.profile);
    if (stored) setProfile(JSON.parse(stored));
  }, []);

  const save = () => {
    localStorage.setItem(STORAGE_KEYS.profile, JSON.stringify(profile));
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const initials = profile.displayName.split(/\s+/).map(w => w[0]?.toUpperCase() || "").join("").slice(0, 2) || "?";

  return (
    <div>
      <h3 style={{ margin: "0 0 16px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>Profile</h3>

      {/* Avatar */}
      <div style={{ display: "flex", alignItems: "center", gap: 16, marginBottom: 20 }}>
        <div style={{
          width: 64, height: 64, borderRadius: "50%", background: "var(--gradient-accent)",
          display: "flex", alignItems: "center", justifyContent: "center", fontSize: 22, fontWeight: 700, color: "#fff",
          overflow: "hidden", flexShrink: 0,
        }}>
          {profile.avatarUrl ? <img src={profile.avatarUrl} alt="" style={{ width: "100%", height: "100%", objectFit: "cover" }} /> : initials}
        </div>
        <div style={{ flex: 1 }}>
          <div style={{ fontWeight: 600, fontSize: 15, color: "var(--text-primary)" }}>{profile.displayName || "Set your name"}</div>
          <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{profile.email || "No email set"}</div>
        </div>
      </div>

      <div style={{ marginBottom: 12 }}>
        <label style={labelStyle}>Display Name</label>
        <input style={fieldStyle} value={profile.displayName} onChange={e => setProfile({ ...profile, displayName: e.target.value })} placeholder="Your name" />
      </div>
      <div style={{ marginBottom: 12 }}>
        <label style={labelStyle}>Email</label>
        <input style={fieldStyle} type="email" value={profile.email} onChange={e => setProfile({ ...profile, email: e.target.value })} placeholder="you@example.com" />
      </div>
      <div style={{ marginBottom: 12 }}>
        <label style={labelStyle}>Bio</label>
        <textarea style={{ ...fieldStyle, minHeight: 60, resize: "vertical" }} value={profile.bio} onChange={e => setProfile({ ...profile, bio: e.target.value })} placeholder="A short bio..." />
      </div>
      <div style={{ marginBottom: 12 }}>
        <label style={labelStyle}>Avatar URL</label>
        <input style={fieldStyle} value={profile.avatarUrl} onChange={e => setProfile({ ...profile, avatarUrl: e.target.value })} placeholder="https://..." />
      </div>

      <button style={btnPrimary} onClick={save}>
        {saved ? <><Check size={14} /> Saved</> : <><Save size={14} /> Save Profile</>}
      </button>
    </div>
  );
}

function AppearanceSection() {
  const [activeThemeId, setActiveThemeId] = useState("dark-default");
  const [fontSize, setFontSize] = useState(13);
  const [density, setDensity] = useState<"compact" | "normal" | "spacious">("normal");
  const [filterCategory, setFilterCategory] = useState<"all" | "standard" | "high-contrast" | "color-blind">("all");

  useEffect(() => {
    const storedTheme = localStorage.getItem(STORAGE_KEYS.theme) || "dark-default";
    const storedSize = localStorage.getItem(STORAGE_KEYS.fontSize);
    const storedDensity = localStorage.getItem("vibeui-density");
    setActiveThemeId(storedTheme);
    if (storedSize) setFontSize(parseInt(storedSize, 10));
    if (storedDensity) setDensity(storedDensity as typeof density);
  }, []);

  const applyTheme = useCallback((theme: ThemeDef) => {
    setActiveThemeId(theme.id);
    applyThemeById(theme.id);
  }, []);

  const applyFontSize = (size: number) => {
    setFontSize(size);
    localStorage.setItem(STORAGE_KEYS.fontSize, String(size));
    document.documentElement.style.setProperty("--editor-font-size", `${size}px`);
  };

  const applyDensity = (d: typeof density) => {
    setDensity(d);
    localStorage.setItem("vibeui-density", d);
    const spacing = d === "compact" ? "2px" : d === "spacious" ? "6px" : "4px";
    document.documentElement.style.setProperty("--density-spacing", spacing);
  };

  const filtered = filterCategory === "all" ? THEMES : THEMES.filter(t => t.category === filterCategory);

  return (
    <div>
      <h3 style={{ margin: "0 0 16px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>Appearance</h3>

      {/* Category filter */}
      <div style={{ display: "flex", gap: 4, marginBottom: 14, flexWrap: "wrap" }}>
        {(["all", "standard", "high-contrast", "color-blind"] as const).map(cat => (
          <button key={cat} onClick={() => setFilterCategory(cat)} style={{
            ...btnStyle, padding: "4px 12px", fontSize: 11, textTransform: "capitalize",
            background: filterCategory === cat ? "var(--accent-blue)" : "var(--bg-elevated)",
            color: filterCategory === cat ? "var(--btn-primary-fg)" : "var(--text-primary)",
            borderColor: filterCategory === cat ? "var(--accent-blue)" : "var(--border-color)",
          }}>
            {cat === "all" ? "All Themes" : cat.replace("-", " ")}
          </button>
        ))}
      </div>

      {/* Theme pairs grid — grouped by pairId */}
      {(() => {
        const pairIds = [...new Set(filtered.map(t => t.pairId))];
        return pairIds.map(pid => {
          const pair = filtered.filter(t => t.pairId === pid);
          const dark = pair.find(t => t.mode === "dark");
          const light = pair.find(t => t.mode === "light");
          const pairName = dark?.name.replace(/ Dark$/, "") || light?.name.replace(/ Light$/, "") || pid;
          const isActivePair = pair.some(t => t.id === activeThemeId);
          return (
            <div key={pid} style={{
              marginBottom: 14, borderRadius: "var(--radius-md)", overflow: "hidden",
              border: isActivePair ? "2px solid var(--accent-blue)" : "1px solid var(--border-color)",
              outline: isActivePair ? "2px solid rgba(108,140,255,0.2)" : "none", outlineOffset: 2,
            }}>
              <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", padding: "6px 10px", background: "var(--bg-tertiary)", textTransform: "uppercase", letterSpacing: 0.5 }}>
                {pairName}
              </div>
              <div style={{ display: "grid", gridTemplateColumns: pair.length > 1 ? "1fr 1fr" : "1fr" }}>
                {[dark, light].filter(Boolean).map(theme => {
                  if (!theme) return null;
                  const isActive = activeThemeId === theme.id;
                  return (
                    <button key={theme.id} onClick={() => applyTheme(theme)} style={{
                      padding: 0, border: "none", borderRight: theme.mode === "dark" && light ? "1px solid var(--border-color)" : "none",
                      cursor: "pointer", background: "none", transition: "var(--transition-fast)",
                      outline: isActive ? "2px solid var(--accent-blue) inset" : "none",
                    }}>
                      <div style={{ display: "flex", height: 36 }}>
                        <div style={{ flex: 3, background: theme.preview.bg }} />
                        <div style={{ flex: 2, background: theme.preview.secondary }} />
                        <div style={{ flex: 1, background: theme.preview.accent }} />
                      </div>
                      <div style={{ padding: "5px 8px", background: theme.preview.bg, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                        <span style={{ fontSize: 11, fontWeight: 500, color: theme.preview.fg }}>{theme.name}</span>
                        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                          {theme.mode === "dark" ? <Moon size={10} color={theme.preview.fg} /> : <Sun size={10} color={theme.preview.fg} />}
                          {isActive && <Check size={12} color={theme.preview.accent} />}
                        </div>
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>
          );
        });
      })()}

      <div style={dividerStyle} />

      {/* Font size */}
      <div style={{ marginBottom: 16 }}>
        <label style={labelStyle}>Editor Font Size: {fontSize}px</label>
        <input type="range" min={10} max={22} value={fontSize} onChange={e => applyFontSize(+e.target.value)} style={{ width: "100%", accentColor: "var(--accent-blue)" }} />
      </div>

      {/* UI Density */}
      <div>
        <label style={labelStyle}>UI Density</label>
        <div style={{ display: "flex", gap: 8 }}>
          {(["compact", "normal", "spacious"] as const).map(d => (
            <button key={d} onClick={() => applyDensity(d)} style={{
              ...btnStyle, flex: 1, textTransform: "capitalize",
              background: density === d ? "var(--accent-blue)" : "var(--bg-elevated)",
              color: density === d ? "var(--btn-primary-fg)" : "var(--text-primary)",
              borderColor: density === d ? "var(--accent-blue)" : "var(--border-color)",
            }}>{d}</button>
          ))}
        </div>
      </div>
    </div>
  );
}

function OAuthSection() {
  const [providers, setProviders] = useState<OAuthProvider[]>(() => {
    const stored = localStorage.getItem(STORAGE_KEYS.oauth);
    return stored ? JSON.parse(stored) : OAUTH_PROVIDERS;
  });

  const save = (updated: OAuthProvider[]) => {
    setProviders(updated);
    localStorage.setItem(STORAGE_KEYS.oauth, JSON.stringify(updated));
  };

  const handleConnect = (id: string) => {
    // In a real app this would redirect to OAuth flow. For now, simulate connection.
    const email = prompt(`Enter your ${providers.find(p => p.id === id)?.name} email to simulate OAuth login:`);
    if (!email) return;
    save(providers.map(p => p.id === id ? { ...p, connected: true, email } : p));
  };

  const handleDisconnect = (id: string) => {
    save(providers.map(p => p.id === id ? { ...p, connected: false, email: undefined } : p));
  };

  const providerColors: Record<string, string> = {
    google: "#4285f4", github: "#333", gitlab: "#fc6d26",
    bitbucket: "#0052cc", microsoft: "#00a4ef", apple: "#000",
  };

  return (
    <div>
      <h3 style={{ margin: "0 0 6px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>OAuth Login</h3>
      <p style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.5 }}>
        Connect your accounts for seamless authentication and source control integration.
      </p>

      {providers.map(p => (
        <div key={p.id} style={{ ...cardBox, display: "flex", alignItems: "center", gap: 12 }}>
          <div style={{
            width: 36, height: 36, borderRadius: "var(--radius-sm)", background: providerColors[p.id] || "var(--bg-tertiary)",
            display: "flex", alignItems: "center", justifyContent: "center", color: "#fff", fontSize: 12, fontWeight: 700, flexShrink: 0,
          }}>
            {p.icon}
          </div>
          <div style={{ flex: 1 }}>
            <div style={{ fontWeight: 600, fontSize: 13, color: "var(--text-primary)" }}>{p.name}</div>
            {p.connected
              ? <div style={{ fontSize: 11, color: "var(--success-color)" }}>Connected as {p.email}</div>
              : <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Not connected</div>
            }
          </div>
          {p.connected ? (
            <button style={{ ...btnStyle, padding: "5px 12px", fontSize: 11, color: "var(--error-color)" }} onClick={() => handleDisconnect(p.id)}>
              Disconnect
            </button>
          ) : (
            <button style={{ ...btnPrimary, padding: "5px 12px", fontSize: 11 }} onClick={() => handleConnect(p.id)}>
              Connect
            </button>
          )}
        </div>
      ))}
    </div>
  );
}

function CustomizationsSection() {
  const [customs, setCustoms] = useState<SavedCustomization[]>(() => {
    const stored = localStorage.getItem(STORAGE_KEYS.customizations);
    return stored ? JSON.parse(stored) : [];
  });
  const [name, setName] = useState("");
  const [message, setMessage] = useState("");

  const saveList = (list: SavedCustomization[]) => {
    setCustoms(list);
    localStorage.setItem(STORAGE_KEYS.customizations, JSON.stringify(list));
  };

  const saveCurrentPrefs = () => {
    if (!name.trim()) return;
    const custom: SavedCustomization = {
      id: Date.now().toString(36),
      name: name.trim(),
      createdAt: new Date().toISOString(),
      theme: localStorage.getItem(STORAGE_KEYS.theme) || "dark-default",
      fontSize: parseInt(localStorage.getItem(STORAGE_KEYS.fontSize) || "13", 10),
      density: localStorage.getItem("vibeui-density") || "normal",
    };
    saveList([...customs, custom]);
    setName("");
    setMessage("Customization saved!");
    setTimeout(() => setMessage(""), 2000);
  };

  const loadCustom = (c: SavedCustomization) => {
    const theme = THEMES.find(t => t.id === c.theme);
    if (theme) {
      localStorage.setItem(STORAGE_KEYS.theme, theme.id);
      localStorage.setItem(STORAGE_KEYS.themeMode, theme.mode);
      document.documentElement.setAttribute("data-theme", theme.mode);
      for (const [key, value] of Object.entries(theme.vars)) {
        document.documentElement.style.setProperty(key, value);
      }
    }
    localStorage.setItem(STORAGE_KEYS.fontSize, String(c.fontSize));
    document.documentElement.style.setProperty("--editor-font-size", `${c.fontSize}px`);
    localStorage.setItem("vibeui-density", c.density);
    setMessage(`Loaded "${c.name}"`);
    setTimeout(() => setMessage(""), 2000);
  };

  const deleteCustom = (id: string) => saveList(customs.filter(c => c.id !== id));

  const exportAll = () => {
    const blob = new Blob([JSON.stringify({ profile: localStorage.getItem(STORAGE_KEYS.profile), customs, oauth: localStorage.getItem(STORAGE_KEYS.oauth) }, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url; a.download = "vibeui-settings.json"; a.click();
    URL.revokeObjectURL(url);
  };

  const importSettings = () => {
    const input = document.createElement("input");
    input.type = "file"; input.accept = ".json";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      const text = await file.text();
      try {
        const data = JSON.parse(text);
        if (data.profile) localStorage.setItem(STORAGE_KEYS.profile, data.profile);
        if (data.oauth) localStorage.setItem(STORAGE_KEYS.oauth, data.oauth);
        if (data.customs) saveList(data.customs);
        setMessage("Settings imported!");
        setTimeout(() => setMessage(""), 2000);
      } catch {
        setMessage("Invalid file format");
        setTimeout(() => setMessage(""), 2000);
      }
    };
    input.click();
  };

  const resetAll = () => {
    if (!confirm("Reset all customizations and preferences to defaults?")) return;
    Object.values(STORAGE_KEYS).forEach(k => localStorage.removeItem(k));
    localStorage.removeItem("vibeui-density");
    document.documentElement.setAttribute("data-theme", "dark");
    // Clear inline styles
    const root = document.documentElement;
    THEMES[0].vars && Object.keys(THEMES[0].vars).forEach(k => root.style.removeProperty(k));
    root.style.removeProperty("--editor-font-size");
    root.style.removeProperty("--density-spacing");
    setCustoms([]);
    setMessage("All settings reset to defaults");
    setTimeout(() => setMessage(""), 2000);
  };

  return (
    <div>
      <h3 style={{ margin: "0 0 16px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>Saved Customizations</h3>

      {/* Save current */}
      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <input style={{ ...fieldStyle, flex: 1 }} placeholder="Customization name..." value={name} onChange={e => setName(e.target.value)} onKeyDown={e => e.key === "Enter" && saveCurrentPrefs()} />
        <button style={btnPrimary} onClick={saveCurrentPrefs}><Save size={14} /> Save Current</button>
      </div>

      {message && <div style={{ padding: "6px 10px", borderRadius: "var(--radius-sm)", background: "var(--success-bg)", color: "var(--success-color)", fontSize: 12, marginBottom: 12 }}>{message}</div>}

      {/* Saved list */}
      {customs.length === 0 ? (
        <div style={{ padding: 20, textAlign: "center", color: "var(--text-secondary)", fontSize: 13 }}>No saved customizations yet. Save your current setup above.</div>
      ) : (
        customs.map(c => (
          <div key={c.id} style={{ ...cardBox, display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <div>
              <div style={{ fontWeight: 600, fontSize: 13, color: "var(--text-primary)" }}>{c.name}</div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                {THEMES.find(t => t.id === c.theme)?.name || c.theme} · {c.fontSize}px · {c.density} · {new Date(c.createdAt).toLocaleDateString()}
              </div>
            </div>
            <div style={{ display: "flex", gap: 4 }}>
              <button style={{ ...btnStyle, padding: "4px 10px", fontSize: 11 }} onClick={() => loadCustom(c)}>Load</button>
              <button style={{ ...btnStyle, padding: "4px 10px", fontSize: 11, color: "var(--error-color)" }} onClick={() => deleteCustom(c.id)}>Delete</button>
            </div>
          </div>
        ))
      )}

      <div style={dividerStyle} />

      {/* Import / Export / Reset */}
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        <button style={btnStyle} onClick={exportAll}><Download size={13} /> Export All</button>
        <button style={btnStyle} onClick={importSettings}><Upload size={13} /> Import</button>
        <button style={{ ...btnStyle, color: "var(--error-color)" }} onClick={resetAll}><RotateCcw size={13} /> Reset All</button>
      </div>
    </div>
  );
}

function ApiKeysSection() {
  const [settings, setSettings] = useState<ApiKeySettings>({
    anthropic_api_key: "", openai_api_key: "", gemini_api_key: "", grok_api_key: "",
    claude_model: "claude-3-5-sonnet-latest", openai_model: "gpt-4o",
  });
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const [showKey, setShowKey] = useState<Record<string, boolean>>({});

  useEffect(() => {
    let cancelled = false;
    invoke<ApiKeySettings>("get_provider_api_keys")
      .then(s => { if (!cancelled) setSettings(s); })
      .catch(() => {});
    return () => { cancelled = true; };
  }, []);

  const handleSave = async () => {
    setSaving(true); setMessage(null);
    try {
      await invoke("save_provider_api_keys", { settings });
      setMessage({ type: "success", text: "Settings saved. Providers re-registered." });
    } catch (e) {
      setMessage({ type: "error", text: String(e) });
    } finally { setSaving(false); }
  };

  const SecretField = ({ label, fieldKey, placeholder }: { label: string; fieldKey: keyof ApiKeySettings; placeholder: string }) => (
    <div style={{ marginBottom: 12 }}>
      <label style={labelStyle}>{label}</label>
      <div style={{ display: "flex", gap: 6 }}>
        <input
          type={showKey[fieldKey] ? "text" : "password"}
          value={settings[fieldKey]}
          onChange={e => setSettings({ ...settings, [fieldKey]: e.target.value })}
          placeholder={placeholder}
          style={{ ...fieldStyle, flex: 1, fontFamily: "var(--font-mono)" }}
        />
        <button onClick={() => setShowKey({ ...showKey, [fieldKey]: !showKey[fieldKey] })} style={{ ...btnStyle, padding: "4px 8px", display: "flex", alignItems: "center" }}>
          {showKey[fieldKey] ? <EyeOff size={14} /> : <Eye size={14} />}
        </button>
      </div>
    </div>
  );

  const SectionHeader = ({ title }: { title: string }) => (
    <div style={{ fontSize: 10, fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.07em", marginBottom: 10, borderBottom: "1px solid var(--border-color)", paddingBottom: 4 }}>
      {title}
    </div>
  );

  return (
    <div>
      <h3 style={{ margin: "0 0 6px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>API Keys (BYOK)</h3>
      <p style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 18, lineHeight: 1.5 }}>
        Keys stored at <code style={{ background: "var(--bg-tertiary)", padding: "1px 4px", borderRadius: 3 }}>~/.vibeui/api_keys.json</code>. Leave empty to disable.
      </p>

      <div style={{ marginBottom: 20 }}>
        <SectionHeader title="Anthropic (Claude)" />
        <SecretField label="API Key" fieldKey="anthropic_api_key" placeholder="sk-ant-api03-..." />
        <div style={{ marginBottom: 12 }}>
          <label style={labelStyle}>Model</label>
          <input style={fieldStyle} value={settings.claude_model} onChange={e => setSettings({ ...settings, claude_model: e.target.value })} placeholder="claude-3-5-sonnet-latest" />
        </div>
      </div>

      <div style={{ marginBottom: 20 }}>
        <SectionHeader title="OpenAI" />
        <SecretField label="API Key" fieldKey="openai_api_key" placeholder="sk-proj-..." />
        <div style={{ marginBottom: 12 }}>
          <label style={labelStyle}>Model</label>
          <input style={fieldStyle} value={settings.openai_model} onChange={e => setSettings({ ...settings, openai_model: e.target.value })} placeholder="gpt-4o" />
        </div>
      </div>

      <div style={{ marginBottom: 20 }}>
        <SectionHeader title="Google (Gemini)" />
        <SecretField label="API Key" fieldKey="gemini_api_key" placeholder="AIzaSy..." />
      </div>

      <div style={{ marginBottom: 20 }}>
        <SectionHeader title="xAI (Grok)" />
        <SecretField label="API Key" fieldKey="grok_api_key" placeholder="xai-..." />
      </div>

      <button style={{ ...btnPrimary, width: "100%" }} onClick={handleSave} disabled={saving}>
        {saving ? "Saving..." : "Save & Apply"}
      </button>

      {message && (
        <div style={{
          marginTop: 12, padding: "8px 10px", borderRadius: "var(--radius-sm)", fontSize: 12,
          background: message.type === "success" ? "var(--success-bg)" : "var(--error-bg)",
          color: message.type === "success" ? "var(--success-color)" : "var(--error-color)",
          border: `1px solid ${message.type === "success" ? "var(--success-color)" : "var(--error-color)"}`,
        }}>
          {message.type === "success" ? "OK " : "ERR "}{message.text}
        </div>
      )}

      <div style={{ marginTop: 24, borderTop: "1px solid var(--border-color)", paddingTop: 16 }}>
        <SectionHeader title="Local Models (Ollama)" />
        <p style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.5 }}>
          Ollama auto-detected at <code style={{ background: "var(--bg-tertiary)", padding: "1px 4px", borderRadius: 3 }}>http://localhost:11434</code>. Start Ollama and models appear automatically.
        </p>
      </div>
    </div>
  );
}

/* ── Main Settings Panel ───────────────────────────────────────────── */

const SECTIONS: { key: SettingsSection; label: string; icon: React.ReactNode }[] = [
  { key: "profile", label: "Profile", icon: <User size={16} /> },
  { key: "appearance", label: "Appearance", icon: <Palette size={16} /> },
  { key: "oauth", label: "OAuth Login", icon: <LogIn size={16} /> },
  { key: "customizations", label: "Customizations", icon: <Save size={16} /> },
  { key: "apikeys", label: "API Keys", icon: <Key size={16} /> },
];

export function SettingsPanel({ onClose }: { onClose?: () => void }) {
  const [section, setSection] = useState<SettingsSection>("profile");

  return (
    <div style={{
      display: "flex", height: "100%", background: "var(--bg-primary)", color: "var(--text-primary)",
      borderRadius: "var(--radius-lg)", overflow: "hidden", border: "1px solid var(--border-color)",
      boxShadow: "var(--elevation-3)",
    }}>
      {/* Sidebar nav */}
      <div style={{
        width: 200, background: "var(--bg-secondary)", borderRight: "1px solid var(--border-color)",
        display: "flex", flexDirection: "column", padding: "12px 8px",
      }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "4px 8px", marginBottom: 12 }}>
          <span style={{ fontWeight: 700, fontSize: 14, background: "var(--gradient-accent)", WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent" }}>Settings</span>
          {onClose && <button onClick={onClose} style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer" }}><X size={16} /></button>}
        </div>
        {SECTIONS.map(s => (
          <button key={s.key} style={sectionBtnStyle(section === s.key)} onClick={() => setSection(s.key)}>
            {s.icon}
            <span>{s.label}</span>
            {section === s.key && <ChevronRight size={14} style={{ marginLeft: "auto", opacity: 0.5 }} />}
          </button>
        ))}
      </div>

      {/* Content */}
      <div style={{ flex: 1, padding: 24, overflowY: "auto" }}>
        {section === "profile" && <ProfileSection />}
        {section === "appearance" && <AppearanceSection />}
        {section === "oauth" && <OAuthSection />}
        {section === "customizations" && <CustomizationsSection />}
        {section === "apikeys" && <ApiKeysSection />}
      </div>
    </div>
  );
}

export default SettingsPanel;
