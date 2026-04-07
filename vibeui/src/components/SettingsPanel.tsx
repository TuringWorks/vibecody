/**
 * SettingsPanel — Comprehensive settings panel opened via the gear icon.
 *
 * Sections:
 *   1. Profile — Display name, avatar, email, bio
 *   2. Appearance — 19 theme pairs (dark/light/high-contrast/color-blind/supercar), font size, UI density
 *   3. OAuth Login — Google, GitHub, GitLab, Bitbucket, Microsoft, Apple
 *   4. Saved Customizations — Export/import/reset workspace preferences
 *   5. API Keys — BYOK provider keys (existing functionality preserved)
 */
import React, { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  User, Palette, LogIn, Save, Key, X, Check, Upload, Download, RotateCcw,
  Sun, Moon, Eye, EyeOff, ChevronRight, CheckCircle, MinusCircle, AlertCircle,
  Loader2, Zap,
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
  category: "standard" | "high-contrast" | "color-blind" | "supercar";
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
  displayName?: string;
  expired?: boolean;
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
  groq_api_key: string;
  openrouter_api_key: string;
  azure_openai_api_key: string;
  azure_openai_api_url: string;
  mistral_api_key: string;
  cerebras_api_key: string;
  deepseek_api_key: string;
  zhipu_api_key: string;
  vercel_ai_api_key: string;
  vercel_ai_api_url: string;
  minimax_api_key: string;
  perplexity_api_key: string;
  together_api_key: string;
  fireworks_api_key: string;
  sambanova_api_key: string;
  ollama_api_key: string;
  ollama_api_url: string;
  claude_model: string;
  openai_model: string;
  openrouter_model: string;
}

/* ── Theme definitions ─────────────────────────────────────────────── */

// Each theme has a pairId linking its dark/light counterpart
export const THEMES: ThemeDef[] = [
  // ── Pair: Default (Midnight Blue / Clean White) ──
  {
    id: "dark-default", name: "Default", category: "standard", mode: "dark", pairId: "default",
    preview: { bg: "#0f1117", fg: "#e2e4ea", accent: "#6c8cff", secondary: "#161821" },
    vars: {
      "--bg-primary": "#0f1117", "--bg-secondary": "#161821", "--bg-tertiary": "#1c1f2b", "--bg-elevated": "#222638",
      "--text-primary": "#e2e4ea", "--text-secondary": "#6e7491", "--accent-blue": "#6c8cff", "--accent-green": "#34d399",
      "--accent-purple": "#a78bfa", "--accent-gold": "#f5c542", "--accent-rose": "#f472b6",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "var(--error-color)",
    },
  },
  {
    id: "light-default", name: "Default", category: "standard", mode: "light", pairId: "default",
    preview: { bg: "#fafbfd", fg: "#1a1d2e", accent: "#4f6df5", secondary: "#f0f1f5" },
    vars: {
      "--bg-primary": "#fafbfd", "--bg-secondary": "#f0f1f5", "--bg-tertiary": "#e6e8ef", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1d2e", "--text-secondary": "#6b7089", "--accent-blue": "#4f6df5", "--accent-green": "var(--success-color)",
      "--accent-purple": "var(--accent-purple)", "--accent-gold": "#d4a017", "--accent-rose": "var(--error-color)",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "var(--error-color)",
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
    id: "light-charcoal", name: "Charcoal", category: "standard", mode: "light", pairId: "charcoal",
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
    id: "dark-warm", name: "Warm", category: "standard", mode: "dark", pairId: "warm",
    preview: { bg: "#1a1410", fg: "#e6ddd0", accent: "#d4a373", secondary: "#2a2118" },
    vars: {
      "--bg-primary": "#1a1410", "--bg-secondary": "#2a2118", "--bg-tertiary": "#3a2e22", "--bg-elevated": "#453828",
      "--text-primary": "#e6ddd0", "--text-secondary": "#a89880", "--accent-blue": "#d4a373", "--accent-green": "#859900",
      "--accent-purple": "#b58db6", "--accent-gold": "#d4a373", "--accent-rose": "#d33682",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#dc322f",
    },
  },
  {
    id: "light-warm", name: "Warm", category: "standard", mode: "light", pairId: "warm",
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
    id: "dark-ocean", name: "Ocean", category: "standard", mode: "dark", pairId: "ocean",
    preview: { bg: "#0d1b2a", fg: "#e0e1dd", accent: "#48cae4", secondary: "#1b2838" },
    vars: {
      "--bg-primary": "#0d1b2a", "--bg-secondary": "#1b2838", "--bg-tertiary": "#233345", "--bg-elevated": "#2b3e50",
      "--text-primary": "#e0e1dd", "--text-secondary": "#778da9", "--accent-blue": "#48cae4", "--accent-green": "#52b788",
      "--accent-purple": "#b392f0", "--accent-gold": "#ffb703", "--accent-rose": "#ff6b6b",
      "--border-color": "rgba(255, 255, 255, 0.05)", "--error-color": "#ff6b6b",
    },
  },
  {
    id: "light-ocean", name: "Ocean", category: "standard", mode: "light", pairId: "ocean",
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
    id: "dark-rose", name: "Rose", category: "standard", mode: "dark", pairId: "rose",
    preview: { bg: "#1a0f10", fg: "#f0dde0", accent: "#f43f5e", secondary: "#2a1a1c" },
    vars: {
      "--bg-primary": "#1a0f10", "--bg-secondary": "#2a1a1c", "--bg-tertiary": "#3a2528", "--bg-elevated": "#452e32",
      "--text-primary": "#f0dde0", "--text-secondary": "#a88b8e", "--accent-blue": "#f43f5e", "--accent-green": "#059669",
      "--accent-purple": "var(--accent-purple)", "--accent-gold": "#ca8a04", "--accent-rose": "#f43f5e",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "var(--error-color)",
    },
  },
  {
    id: "light-rose", name: "Rose", category: "standard", mode: "light", pairId: "rose",
    preview: { bg: "#fff5f5", fg: "#2d1b1b", accent: "#e11d48", secondary: "#ffe4e6" },
    vars: {
      "--bg-primary": "#fff5f5", "--bg-secondary": "#ffe4e6", "--bg-tertiary": "#fecdd3", "--bg-elevated": "#ffffff",
      "--text-primary": "#2d1b1b", "--text-secondary": "#9f6b6b", "--accent-blue": "#e11d48", "--accent-green": "#059669",
      "--accent-purple": "var(--accent-purple)", "--accent-gold": "#ca8a04", "--accent-rose": "#e11d48",
      "--border-color": "rgba(0, 0, 0, 0.06)", "--error-color": "var(--error-color)",
    },
  },
  // ── Pair: High Contrast ──
  {
    id: "hc-dark", name: "High Contrast", category: "high-contrast", mode: "dark", pairId: "hc",
    preview: { bg: "#000000", fg: "#ffffff", accent: "#00e0ff", secondary: "#0a0a0a" },
    vars: {
      "--bg-primary": "#000000", "--bg-secondary": "#0a0a0a", "--bg-tertiary": "#141414", "--bg-elevated": "#1e1e1e",
      "--text-primary": "#ffffff", "--text-secondary": "#cccccc", "--accent-blue": "#00e0ff", "--accent-green": "#00ff88",
      "--accent-purple": "#d0a0ff", "--accent-gold": "#ffdd00", "--accent-rose": "#ff6699",
      "--border-color": "rgba(255, 255, 255, 0.25)", "--error-color": "#ff3333",
    },
  },
  {
    id: "hc-light", name: "High Contrast", category: "high-contrast", mode: "light", pairId: "hc",
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
    id: "cb-deuteranopia-dark", name: "Deuteranopia", category: "color-blind", mode: "dark", pairId: "deuteranopia",
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
    id: "cb-deuteranopia-light", name: "Deuteranopia", category: "color-blind", mode: "light", pairId: "deuteranopia",
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
    id: "cb-protanopia-dark", name: "Protanopia", category: "color-blind", mode: "dark", pairId: "protanopia",
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
    id: "cb-protanopia-light", name: "Protanopia", category: "color-blind", mode: "light", pairId: "protanopia",
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
    id: "cb-tritanopia-dark", name: "Tritanopia", category: "color-blind", mode: "dark", pairId: "tritanopia",
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
    id: "cb-tritanopia-light", name: "Tritanopia", category: "color-blind", mode: "light", pairId: "tritanopia",
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
      "--text-primary": "#f8f8f2", "--text-secondary": "#a5a08a", "--accent-blue": "#66d9ef", "--accent-green": "#a6e22e",
      "--accent-purple": "#ae81ff", "--accent-gold": "#e6db74", "--accent-rose": "#f92672",
      "--border-color": "rgba(255, 255, 255, 0.07)", "--error-color": "#f92672",
    },
  },
  {
    id: "light-monokai", name: "Monokai", category: "standard", mode: "light", pairId: "monokai",
    preview: { bg: "#fafafa", fg: "#272822", accent: "#629755", secondary: "#eeeee8" },
    vars: {
      "--bg-primary": "#fafafa", "--bg-secondary": "#eeeee8", "--bg-tertiary": "#e0e0d8", "--bg-elevated": "#ffffff",
      "--text-primary": "#272822", "--text-secondary": "#605c46", "--accent-blue": "#1290bf", "--accent-green": "#629755",
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
      "--text-primary": "#f8f8f2", "--text-secondary": "#8a96c0", "--accent-blue": "#8be9fd", "--accent-green": "#50fa7b",
      "--accent-purple": "#bd93f9", "--accent-gold": "#f1fa8c", "--accent-rose": "#ff79c6",
      "--border-color": "rgba(255, 255, 255, 0.08)", "--error-color": "#ff5555",
    },
  },
  {
    id: "light-dracula", name: "Dracula", category: "standard", mode: "light", pairId: "dracula",
    preview: { bg: "#f8f8f2", fg: "#282a36", accent: "var(--accent-purple)", secondary: "#ededec" },
    vars: {
      "--bg-primary": "#f8f8f2", "--bg-secondary": "#ededec", "--bg-tertiary": "#e0dfe0", "--bg-elevated": "#ffffff",
      "--text-primary": "#282a36", "--text-secondary": "#4e5a7e", "--accent-blue": "#0891b2", "--accent-green": "#16a34a",
      "--accent-purple": "var(--accent-purple)", "--accent-gold": "#a16207", "--accent-rose": "#db2777",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "var(--error-color)",
    },
  },
  // ── Pair: Nord ──
  {
    id: "dark-nord", name: "Nord", category: "standard", mode: "dark", pairId: "nord",
    preview: { bg: "#2e3440", fg: "#eceff4", accent: "#88c0d0", secondary: "#3b4252" },
    vars: {
      "--bg-primary": "#2e3440", "--bg-secondary": "#3b4252", "--bg-tertiary": "#434c5e", "--bg-elevated": "#4c566a",
      "--text-primary": "#eceff4", "--text-secondary": "#9aa4b8", "--accent-blue": "#88c0d0", "--accent-green": "#a3be8c",
      "--accent-purple": "#b48ead", "--accent-gold": "#ebcb8b", "--accent-rose": "#bf616a",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#bf616a",
    },
  },
  {
    id: "light-nord", name: "Nord", category: "standard", mode: "light", pairId: "nord",
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
    id: "dark-one", name: "One", category: "standard", mode: "dark", pairId: "one",
    preview: { bg: "#282c34", fg: "#abb2bf", accent: "#61afef", secondary: "#21252b" },
    vars: {
      "--bg-primary": "#282c34", "--bg-secondary": "#21252b", "--bg-tertiary": "#2c313a", "--bg-elevated": "#333842",
      "--text-primary": "#abb2bf", "--text-secondary": "#838994", "--accent-blue": "#61afef", "--accent-green": "#98c379",
      "--accent-purple": "#c678dd", "--accent-gold": "#e5c07b", "--accent-rose": "#e06c75",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e06c75",
    },
  },
  {
    id: "light-one", name: "One", category: "standard", mode: "light", pairId: "one",
    preview: { bg: "#fafafa", fg: "#383a42", accent: "#4078f2", secondary: "#f0f0f0" },
    vars: {
      "--bg-primary": "#fafafa", "--bg-secondary": "#f0f0f0", "--bg-tertiary": "#e5e5e6", "--bg-elevated": "#ffffff",
      "--text-primary": "#383a42", "--text-secondary": "#696a70", "--accent-blue": "#4078f2", "--accent-green": "#50a14f",
      "--accent-purple": "#a626a4", "--accent-gold": "#c18401", "--accent-rose": "#e45649",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#e45649",
    },
  },
  // ── Pair: GitHub ──
  {
    id: "dark-github", name: "GitHub", category: "standard", mode: "dark", pairId: "github",
    preview: { bg: "#0d1117", fg: "#e6edf3", accent: "#58a6ff", secondary: "#161b22" },
    vars: {
      "--bg-primary": "#0d1117", "--bg-secondary": "#161b22", "--bg-tertiary": "#21262d", "--bg-elevated": "#30363d",
      "--text-primary": "#e6edf3", "--text-secondary": "#8b929a", "--accent-blue": "#58a6ff", "--accent-green": "#3fb950",
      "--accent-purple": "#bc8cff", "--accent-gold": "#d29922", "--accent-rose": "#f85149",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f85149",
    },
  },
  {
    id: "light-github", name: "GitHub", category: "standard", mode: "light", pairId: "github",
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
    id: "dark-catppuccin", name: "Catppuccin", category: "standard", mode: "dark", pairId: "catppuccin",
    preview: { bg: "#1e1e2e", fg: "#cdd6f4", accent: "#89b4fa", secondary: "#313244" },
    vars: {
      "--bg-primary": "#1e1e2e", "--bg-secondary": "#313244", "--bg-tertiary": "#45475a", "--bg-elevated": "#585b70",
      "--text-primary": "#cdd6f4", "--text-secondary": "#9399b2", "--accent-blue": "#89b4fa", "--accent-green": "#a6e3a1",
      "--accent-purple": "#cba6f7", "--accent-gold": "#f9e2af", "--accent-rose": "#f38ba8",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f38ba8",
    },
  },
  {
    id: "light-catppuccin", name: "Catppuccin", category: "standard", mode: "light", pairId: "catppuccin",
    preview: { bg: "#eff1f5", fg: "#4c4f69", accent: "#1e66f5", secondary: "#e6e9ef" },
    vars: {
      "--bg-primary": "#eff1f5", "--bg-secondary": "#e6e9ef", "--bg-tertiary": "#ccd0da", "--bg-elevated": "#ffffff",
      "--text-primary": "#4c4f69", "--text-secondary": "#5c5f73", "--accent-blue": "#1e66f5", "--accent-green": "#40a02b",
      "--accent-purple": "#8839ef", "--accent-gold": "#df8e1d", "--accent-rose": "#d20f39",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#d20f39",
    },
  },
  // ── Pair: Gruvbox ──
  {
    id: "dark-gruvbox", name: "Gruvbox", category: "standard", mode: "dark", pairId: "gruvbox",
    preview: { bg: "#282828", fg: "#ebdbb2", accent: "#fabd2f", secondary: "#3c3836" },
    vars: {
      "--bg-primary": "#282828", "--bg-secondary": "#3c3836", "--bg-tertiary": "#504945", "--bg-elevated": "#665c54",
      "--text-primary": "#ebdbb2", "--text-secondary": "#a89b8c", "--accent-blue": "#83a598", "--accent-green": "#b8bb26",
      "--accent-purple": "#d3869b", "--accent-gold": "#fabd2f", "--accent-rose": "#fb4934",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#fb4934",
    },
  },
  {
    id: "light-gruvbox", name: "Gruvbox", category: "standard", mode: "light", pairId: "gruvbox",
    preview: { bg: "#fbf1c7", fg: "#3c3836", accent: "#b57614", secondary: "#ebdbb2" },
    vars: {
      "--bg-primary": "#fbf1c7", "--bg-secondary": "#ebdbb2", "--bg-tertiary": "#d5c4a1", "--bg-elevated": "#fffbef",
      "--text-primary": "#3c3836", "--text-secondary": "#5e5448", "--accent-blue": "#076678", "--accent-green": "#79740e",
      "--accent-purple": "#8f3f71", "--accent-gold": "#b57614", "--accent-rose": "#9d0006",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#9d0006",
    },
  },
  // ── Pair: Tokyo Night ──
  {
    id: "dark-tokyo", name: "Tokyo", category: "standard", mode: "dark", pairId: "tokyo",
    preview: { bg: "#1a1b26", fg: "#c0caf5", accent: "#7aa2f7", secondary: "#24283b" },
    vars: {
      "--bg-primary": "#1a1b26", "--bg-secondary": "#24283b", "--bg-tertiary": "#2f3347", "--bg-elevated": "#3b3f54",
      "--text-primary": "#c0caf5", "--text-secondary": "#7a82a8", "--accent-blue": "#7aa2f7", "--accent-green": "#9ece6a",
      "--accent-purple": "#bb9af7", "--accent-gold": "#e0af68", "--accent-rose": "#f7768e",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f7768e",
    },
  },
  {
    id: "light-tokyo", name: "Tokyo", category: "standard", mode: "light", pairId: "tokyo",
    preview: { bg: "#d5d6db", fg: "#343b58", accent: "#34548a", secondary: "#c8c8ce" },
    vars: {
      "--bg-primary": "#d5d6db", "--bg-secondary": "#c8c8ce", "--bg-tertiary": "#b8b8c0", "--bg-elevated": "#e5e5ea",
      "--text-primary": "#343b58", "--text-secondary": "#4a5880", "--accent-blue": "#34548a", "--accent-green": "#485e30",
      "--accent-purple": "#7847bd", "--accent-gold": "#8f5e15", "--accent-rose": "#8c4351",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#8c4351",
    },
  },
  // ── Pair: Material ──
  {
    id: "dark-material", name: "Material", category: "standard", mode: "dark", pairId: "material",
    preview: { bg: "#212121", fg: "#eeffff", accent: "#82aaff", secondary: "#303030" },
    vars: {
      "--bg-primary": "#212121", "--bg-secondary": "#303030", "--bg-tertiary": "#3a3a3a", "--bg-elevated": "#424242",
      "--text-primary": "#eeffff", "--text-secondary": "#8a8a8a", "--accent-blue": "#82aaff", "--accent-green": "#c3e88d",
      "--accent-purple": "#c792ea", "--accent-gold": "#ffcb6b", "--accent-rose": "#f07178",
      "--border-color": "rgba(255, 255, 255, 0.05)", "--error-color": "#f07178",
    },
  },
  {
    id: "light-material", name: "Material", category: "standard", mode: "light", pairId: "material",
    preview: { bg: "#fafafa", fg: "#546e7a", accent: "#6182b8", secondary: "#eaeaea" },
    vars: {
      "--bg-primary": "#fafafa", "--bg-secondary": "#eaeaea", "--bg-tertiary": "#d4d4d4", "--bg-elevated": "#ffffff",
      "--text-primary": "#546e7a", "--text-secondary": "#5e7680", "--accent-blue": "#6182b8", "--accent-green": "#91b859",
      "--accent-purple": "#7c4dff", "--accent-gold": "#f6a434", "--accent-rose": "#e53935",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#e53935",
    },
  },
  // ── Pair: Solarized ──
  {
    id: "dark-solarized", name: "Solarized", category: "standard", mode: "dark", pairId: "solarized",
    preview: { bg: "#002b36", fg: "#839496", accent: "#268bd2", secondary: "#073642" },
    vars: {
      "--bg-primary": "#002b36", "--bg-secondary": "#073642", "--bg-tertiary": "#0a4050", "--bg-elevated": "#0d4f5e",
      "--text-primary": "#93a1a1", "--text-secondary": "#6d8388", "--accent-blue": "#268bd2", "--accent-green": "#859900",
      "--accent-purple": "#6c71c4", "--accent-gold": "#b58900", "--accent-rose": "#dc322f",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#dc322f",
    },
  },
  {
    id: "light-solarized", name: "Solarized", category: "standard", mode: "light", pairId: "solarized",
    preview: { bg: "#fdf6e3", fg: "#4a5a60", accent: "#268bd2", secondary: "#eee8d5" },
    vars: {
      "--bg-primary": "#fdf6e3", "--bg-secondary": "#eee8d5", "--bg-tertiary": "#e0dbc7", "--bg-elevated": "#fffdf5",
      "--text-primary": "#4a5a60", "--text-secondary": "#6b7c80", "--accent-blue": "#268bd2", "--accent-green": "#859900",
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
      "--text-primary": "#bfc5e0", "--text-secondary": "#8088b0", "--accent-blue": "#82aaff", "--accent-green": "#c3e88d",
      "--accent-purple": "#c792ea", "--accent-gold": "#ffcb6b", "--accent-rose": "#f07178",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#f07178",
    },
  },
  {
    id: "light-palenight", name: "Palenight", category: "standard", mode: "light", pairId: "palenight",
    preview: { bg: "#f0f0f8", fg: "#3b3d55", accent: "#5a6acf", secondary: "#e4e4ef" },
    vars: {
      "--bg-primary": "#f0f0f8", "--bg-secondary": "#e4e4ef", "--bg-tertiary": "#d4d4e2", "--bg-elevated": "#fafaff",
      "--text-primary": "#3b3d55", "--text-secondary": "#555b7a", "--accent-blue": "#5a6acf", "--accent-green": "#689d6a",
      "--accent-purple": "#9c5fb5", "--accent-gold": "#c08b30", "--accent-rose": "#c45060",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#c45060",
    },
  },
  // ── Pair: Ayu ──
  {
    id: "dark-ayu", name: "Ayu", category: "standard", mode: "dark", pairId: "ayu",
    preview: { bg: "#0a0e14", fg: "#b3b1ad", accent: "#ffb454", secondary: "#1f2430" },
    vars: {
      "--bg-primary": "#0a0e14", "--bg-secondary": "#1f2430", "--bg-tertiary": "#272d38", "--bg-elevated": "#2e3440",
      "--text-primary": "#b3b1ad", "--text-secondary": "#7e8894", "--accent-blue": "#36a3d9", "--accent-green": "#bae67e",
      "--accent-purple": "#d4bfff", "--accent-gold": "#ffb454", "--accent-rose": "#ff3333",
      "--border-color": "rgba(255, 255, 255, 0.05)", "--error-color": "#ff3333",
    },
  },
  {
    id: "light-ayu", name: "Ayu", category: "standard", mode: "light", pairId: "ayu",
    preview: { bg: "#fafafa", fg: "#575f66", accent: "#ff9940", secondary: "#f0f0f0" },
    vars: {
      "--bg-primary": "#fafafa", "--bg-secondary": "#f0f0f0", "--bg-tertiary": "#e1e1e1", "--bg-elevated": "#ffffff",
      "--text-primary": "#575f66", "--text-secondary": "#6e7478", "--accent-blue": "#399ee6", "--accent-green": "#86b300",
      "--accent-purple": "#a37acc", "--accent-gold": "#ff9940", "--accent-rose": "#f51818",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#f51818",
    },
  },
  // ── Pair: Slack (Organization) ──
  {
    id: "dark-slack", name: "Slack", category: "standard", mode: "dark", pairId: "slack",
    preview: { bg: "#1a1d21", fg: "#d1d2d3", accent: "#36c5f0", secondary: "#27242c" },
    vars: {
      "--bg-primary": "#1a1d21", "--bg-secondary": "#27242c", "--bg-tertiary": "#332f3b", "--bg-elevated": "#3d3848",
      "--text-primary": "#d1d2d3", "--text-secondary": "#9a9a9d", "--accent-blue": "#36c5f0", "--accent-green": "#2eb67d",
      "--accent-purple": "#611f69", "--accent-gold": "#ecb22e", "--accent-rose": "#e01e5a",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e01e5a",
    },
  },
  {
    id: "light-slack", name: "Slack", category: "standard", mode: "light", pairId: "slack",
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
    id: "dark-cobalt", name: "Cobalt", category: "standard", mode: "dark", pairId: "cobalt",
    preview: { bg: "#193549", fg: "#e1efff", accent: "#ffc600", secondary: "#1f4662" },
    vars: {
      "--bg-primary": "#193549", "--bg-secondary": "#1f4662", "--bg-tertiary": "#245170", "--bg-elevated": "#2a5c80",
      "--text-primary": "#e1efff", "--text-secondary": "#6fa0c7", "--accent-blue": "#80ffbb", "--accent-green": "#3ad900",
      "--accent-purple": "#fb94ff", "--accent-gold": "#ffc600", "--accent-rose": "#ff628c",
      "--border-color": "rgba(255, 255, 255, 0.08)", "--error-color": "#ff628c",
    },
  },
  {
    id: "light-cobalt", name: "Cobalt", category: "standard", mode: "light", pairId: "cobalt",
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
      "--text-primary": "#e0def4", "--text-secondary": "#9d98b8", "--accent-blue": "#72f1b8", "--accent-green": "#72f1b8",
      "--accent-purple": "#f97e72", "--accent-gold": "#fede5d", "--accent-rose": "#fe4450",
      "--border-color": "rgba(255, 255, 255, 0.07)", "--error-color": "#fe4450",
    },
  },
  {
    id: "light-synthwave", name: "Synthwave '84", category: "standard", mode: "light", pairId: "synthwave",
    preview: { bg: "#f5f0ff", fg: "#2d2350", accent: "#c44040", secondary: "#e8e0f5" },
    vars: {
      "--bg-primary": "#f5f0ff", "--bg-secondary": "#e8e0f5", "--bg-tertiary": "#d8cee8", "--bg-elevated": "#ffffff",
      "--text-primary": "#2d2350", "--text-secondary": "#504068", "--accent-blue": "#2a8a5e", "--accent-green": "#2a8a5e",
      "--accent-purple": "#c44040", "--accent-gold": "#a78000", "--accent-rose": "#c02030",
      "--border-color": "rgba(0, 0, 0, 0.07)", "--error-color": "#c02030",
    },
  },
  // ── Pair: Everforest ──
  {
    id: "dark-everforest", name: "Everforest", category: "standard", mode: "dark", pairId: "everforest",
    preview: { bg: "#2d353b", fg: "#d3c6aa", accent: "#a7c080", secondary: "#343f44" },
    vars: {
      "--bg-primary": "#2d353b", "--bg-secondary": "#343f44", "--bg-tertiary": "#3d484d", "--bg-elevated": "#475258",
      "--text-primary": "#d3c6aa", "--text-secondary": "#859289", "--accent-blue": "#7fbbb3", "--accent-green": "#a7c080",
      "--accent-purple": "#d699b6", "--accent-gold": "#dbbc7f", "--accent-rose": "#e67e80",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e67e80",
    },
  },
  {
    id: "light-everforest", name: "Everforest", category: "standard", mode: "light", pairId: "everforest",
    preview: { bg: "#fdf6e3", fg: "#5c6a72", accent: "#8da101", secondary: "#f0ead2" },
    vars: {
      "--bg-primary": "#fdf6e3", "--bg-secondary": "#f0ead2", "--bg-tertiary": "#e0dab8", "--bg-elevated": "#fffbf0",
      "--text-primary": "#5c6a72", "--text-secondary": "#5c6860", "--accent-blue": "#3a94c5", "--accent-green": "#8da101",
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
      "--text-primary": "#dcd7ba", "--text-secondary": "#908f85", "--accent-blue": "#7e9cd8", "--accent-green": "#98bb6c",
      "--accent-purple": "#957fb8", "--accent-gold": "#e6c384", "--accent-rose": "#e82424",
      "--border-color": "rgba(255, 255, 255, 0.06)", "--error-color": "#e82424",
    },
  },
  {
    id: "light-kanagawa", name: "Kanagawa", category: "standard", mode: "light", pairId: "kanagawa",
    preview: { bg: "#f2ecbc", fg: "#43436c", accent: "#4d699b", secondary: "#e7dba0" },
    vars: {
      "--bg-primary": "#f2ecbc", "--bg-secondary": "#e7dba0", "--bg-tertiary": "#d8cc88", "--bg-elevated": "#faf5d0",
      "--text-primary": "#43436c", "--text-secondary": "#5e5e50", "--accent-blue": "#4d699b", "--accent-green": "#6f894e",
      "--accent-purple": "#624c83", "--accent-gold": "#a96b2c", "--accent-rose": "#c84053",
      "--border-color": "rgba(0, 0, 0, 0.08)", "--error-color": "#c84053",
    },
  },
  // ═══════════════════════════════════════════════════════════════════
  //  Rivian R1 & R2 — Exterior & Interior Inspired Themes
  // ═══════════════════════════════════════════════════════════════════

  // ── Pair: Rivian Blue (R1 flagship exterior) ──
  {
    id: "dark-rivian-blue", name: "Rivian Blue", category: "standard", mode: "dark", pairId: "rivian-blue",
    preview: { bg: "#0b1628", fg: "#d4dce8", accent: "#3d7bce", secondary: "#122040" },
    vars: {
      "--bg-primary": "#0b1628", "--bg-secondary": "#122040", "--bg-tertiary": "#1a2d52", "--bg-elevated": "#233a64",
      "--text-primary": "#d4dce8", "--text-secondary": "#7a90ad", "--accent-blue": "#3d7bce", "--accent-green": "#4caf82",
      "--accent-purple": "#9b8ec7", "--accent-gold": "#e5b84c", "--accent-rose": "#e06070",
      "--border-color": "rgba(61, 123, 206, 0.12)", "--error-color": "#e06070",
    },
  },
  {
    id: "light-rivian-blue", name: "Rivian Blue", category: "standard", mode: "light", pairId: "rivian-blue",
    preview: { bg: "#f4f6f9", fg: "#1a2a42", accent: "#2a5fa0", secondary: "#e6ebf2" },
    vars: {
      "--bg-primary": "#f4f6f9", "--bg-secondary": "#e6ebf2", "--bg-tertiary": "#d4dce8", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a2a42", "--text-secondary": "#5a6e85", "--accent-blue": "#2a5fa0", "--accent-green": "#2d8a5e",
      "--accent-purple": "#6b5ea0", "--accent-gold": "#b08a20", "--accent-rose": "#c44858",
      "--border-color": "rgba(42, 95, 160, 0.10)", "--error-color": "#c44858",
    },
  },
  // ── Pair: Forest Green (R1 exterior) / Limestone (R1 warm beige) ──
  {
    id: "dark-rivian-forest", name: "Rivian Forest", category: "standard", mode: "dark", pairId: "rivian-forest",
    preview: { bg: "#0e1a15", fg: "#d0ddd4", accent: "#4a8c6a", secondary: "#162820" },
    vars: {
      "--bg-primary": "#0e1a15", "--bg-secondary": "#162820", "--bg-tertiary": "#1e352b", "--bg-elevated": "#274236",
      "--text-primary": "#d0ddd4", "--text-secondary": "#7a9988", "--accent-blue": "#4a8c6a", "--accent-green": "#5ebd88",
      "--accent-purple": "#a28db5", "--accent-gold": "#d4a855", "--accent-rose": "#d46a5a",
      "--border-color": "rgba(74, 140, 106, 0.12)", "--error-color": "#d46a5a",
    },
  },
  {
    id: "light-rivian-forest", name: "Rivian Forest", category: "standard", mode: "light", pairId: "rivian-forest",
    preview: { bg: "#f5f0e8", fg: "#2a2820", accent: "#3d7a58", secondary: "#e8e0d2" },
    vars: {
      "--bg-primary": "#f5f0e8", "--bg-secondary": "#e8e0d2", "--bg-tertiary": "#d8cebc", "--bg-elevated": "#fdf8f0",
      "--text-primary": "#2a2820", "--text-secondary": "#6a6456", "--accent-blue": "#3d7a58", "--accent-green": "#4a9060",
      "--accent-purple": "#7a6690", "--accent-gold": "#a08228", "--accent-rose": "#b84a40",
      "--border-color": "rgba(61, 122, 88, 0.10)", "--error-color": "#b84a40",
    },
  },
  // ── Pair: El Cap Granite (R1 exterior) / LA Silver (R1 exterior) ──
  {
    id: "dark-rivian-granite", name: "Rivian Granite", category: "standard", mode: "dark", pairId: "rivian-granite",
    preview: { bg: "#161514", fg: "#d5d0ca", accent: "#a09080", secondary: "#221f1d" },
    vars: {
      "--bg-primary": "#161514", "--bg-secondary": "#221f1d", "--bg-tertiary": "#2e2a27", "--bg-elevated": "#3a3532",
      "--text-primary": "#d5d0ca", "--text-secondary": "#8a8278", "--accent-blue": "#a09080", "--accent-green": "#7aaa6c",
      "--accent-purple": "#b098b8", "--accent-gold": "#d4a855", "--accent-rose": "#cc6a5a",
      "--border-color": "rgba(160, 144, 128, 0.12)", "--error-color": "#cc6a5a",
    },
  },
  {
    id: "light-rivian-granite", name: "Rivian Granite", category: "standard", mode: "light", pairId: "rivian-granite",
    preview: { bg: "#f0eeec", fg: "#2a2624", accent: "#6a6058", secondary: "#e2dedb" },
    vars: {
      "--bg-primary": "#f0eeec", "--bg-secondary": "#e2dedb", "--bg-tertiary": "#d0ccc7", "--bg-elevated": "#faf8f6",
      "--text-primary": "#2a2624", "--text-secondary": "#6e665e", "--accent-blue": "#6a6058", "--accent-green": "#508a48",
      "--accent-purple": "#7a6880", "--accent-gold": "#9a7a28", "--accent-rose": "#aa4a40",
      "--border-color": "rgba(106, 96, 88, 0.10)", "--error-color": "#aa4a40",
    },
  },
  // ── Pair: Midnight (R1 exterior) / Ocean Coast (R1 interior) ──
  {
    id: "dark-rivian-midnight", name: "Rivian Midnight", category: "standard", mode: "dark", pairId: "rivian-midnight",
    preview: { bg: "#08090e", fg: "#cdd2dc", accent: "#5a8aaa", secondary: "#10121a" },
    vars: {
      "--bg-primary": "#08090e", "--bg-secondary": "#10121a", "--bg-tertiary": "#181c28", "--bg-elevated": "#222838",
      "--text-primary": "#cdd2dc", "--text-secondary": "#6a7488", "--accent-blue": "#5a8aaa", "--accent-green": "#4aaa80",
      "--accent-purple": "#8a80b8", "--accent-gold": "#ccaa44", "--accent-rose": "#d85860",
      "--border-color": "rgba(90, 138, 170, 0.10)", "--error-color": "#d85860",
    },
  },
  {
    id: "light-rivian-midnight", name: "Rivian Midnight", category: "standard", mode: "light", pairId: "rivian-midnight",
    preview: { bg: "#f0f6f8", fg: "#1a2830", accent: "#3a7a94", secondary: "#deeef4" },
    vars: {
      "--bg-primary": "#f0f6f8", "--bg-secondary": "#deeef4", "--bg-tertiary": "#c8dee8", "--bg-elevated": "#fafcfd",
      "--text-primary": "#1a2830", "--text-secondary": "#4a6878", "--accent-blue": "#3a7a94", "--accent-green": "#2a8a60",
      "--accent-purple": "#6a6090", "--accent-gold": "#a08828", "--accent-rose": "#c04850",
      "--border-color": "rgba(58, 122, 148, 0.10)", "--error-color": "#c04850",
    },
  },
  // ── Pair: Red Canyon (R1 exterior) ──
  {
    id: "dark-rivian-canyon", name: "Rivian Canyon", category: "standard", mode: "dark", pairId: "rivian-canyon",
    preview: { bg: "#140c0a", fg: "#e0d0c8", accent: "#b85a42", secondary: "#221410" },
    vars: {
      "--bg-primary": "#140c0a", "--bg-secondary": "#221410", "--bg-tertiary": "#30201a", "--bg-elevated": "#3e2c24",
      "--text-primary": "#e0d0c8", "--text-secondary": "#a08878", "--accent-blue": "#b85a42", "--accent-green": "#6aaa58",
      "--accent-purple": "#a87898", "--accent-gold": "#d4a040", "--accent-rose": "#d85040",
      "--border-color": "rgba(184, 90, 66, 0.14)", "--error-color": "#d85040",
    },
  },
  {
    id: "light-rivian-canyon", name: "Rivian Canyon", category: "standard", mode: "light", pairId: "rivian-canyon",
    preview: { bg: "#f8f2ec", fg: "#2e1e18", accent: "#984838", secondary: "#ecddd0" },
    vars: {
      "--bg-primary": "#f8f2ec", "--bg-secondary": "#ecddd0", "--bg-tertiary": "#dccabc", "--bg-elevated": "#fffaf5",
      "--text-primary": "#2e1e18", "--text-secondary": "#7a5e50", "--accent-blue": "#984838", "--accent-green": "#4a8a3e",
      "--accent-purple": "#804a6a", "--accent-gold": "#a07820", "--accent-rose": "#b83830",
      "--border-color": "rgba(152, 72, 56, 0.10)", "--error-color": "#b83830",
    },
  },
  // ── Pair: Launch Green (R1 Quad-Motor exclusive) ──
  {
    id: "dark-rivian-launch", name: "Rivian Launch", category: "standard", mode: "dark", pairId: "rivian-launch",
    preview: { bg: "#0a140e", fg: "#d0e0d4", accent: "var(--success-color)", secondary: "#142218" },
    vars: {
      "--bg-primary": "#0a140e", "--bg-secondary": "#142218", "--bg-tertiary": "#1e3024", "--bg-elevated": "#283e30",
      "--text-primary": "#d0e0d4", "--text-secondary": "#7aa088", "--accent-blue": "var(--success-color)", "--accent-green": "#66cc6a",
      "--accent-purple": "#a090c0", "--accent-gold": "#ccb040", "--accent-rose": "#e05858",
      "--border-color": "rgba(76, 175, 80, 0.14)", "--error-color": "#e05858",
    },
  },
  {
    id: "light-rivian-launch", name: "Rivian Launch", category: "standard", mode: "light", pairId: "rivian-launch",
    preview: { bg: "#f2f6f2", fg: "#1a2420", accent: "#2e8a38", secondary: "#e0eae2" },
    vars: {
      "--bg-primary": "#f2f6f2", "--bg-secondary": "#e0eae2", "--bg-tertiary": "#cddcd0", "--bg-elevated": "#fafcfa",
      "--text-primary": "#1a2420", "--text-secondary": "#4a6a52", "--accent-blue": "#2e8a38", "--accent-green": "#3aa040",
      "--accent-purple": "#6a5a8a", "--accent-gold": "#8a7a18", "--accent-rose": "#b84040",
      "--border-color": "rgba(46, 138, 56, 0.10)", "--error-color": "#b84040",
    },
  },
  // ── Pair: Catalina Cove (R2 exclusive) / Coastal Cloud (R2 interior) ──
  {
    id: "dark-rivian-catalina", name: "Rivian Catalina", category: "standard", mode: "dark", pairId: "rivian-catalina",
    preview: { bg: "#0a1418", fg: "#ccdce0", accent: "#3a9aaa", secondary: "#122028" },
    vars: {
      "--bg-primary": "#0a1418", "--bg-secondary": "#122028", "--bg-tertiary": "#1a2e38", "--bg-elevated": "#223c48",
      "--text-primary": "#ccdce0", "--text-secondary": "#6a8a94", "--accent-blue": "#3a9aaa", "--accent-green": "#4ab888",
      "--accent-purple": "#8a88c0", "--accent-gold": "#d0a848", "--accent-rose": "#d86068",
      "--border-color": "rgba(58, 154, 170, 0.12)", "--error-color": "#d86068",
    },
  },
  {
    id: "light-rivian-catalina", name: "Rivian Catalina", category: "standard", mode: "light", pairId: "rivian-catalina",
    preview: { bg: "#f2f8f8", fg: "#182828", accent: "#2a808e", secondary: "#dceef0" },
    vars: {
      "--bg-primary": "#f2f8f8", "--bg-secondary": "#dceef0", "--bg-tertiary": "#c6e0e4", "--bg-elevated": "#fafefe",
      "--text-primary": "#182828", "--text-secondary": "#486a70", "--accent-blue": "#2a808e", "--accent-green": "#2a9068",
      "--accent-purple": "#5a6090", "--accent-gold": "#98841e", "--accent-rose": "#b84850",
      "--border-color": "rgba(42, 128, 142, 0.10)", "--error-color": "#b84850",
    },
  },
  // ── Pair: Storm Blue (R1 Tri/Quad) / Esker Silver (R2 default) ──
  {
    id: "dark-rivian-storm", name: "Rivian Storm", category: "standard", mode: "dark", pairId: "rivian-storm",
    preview: { bg: "#0c1218", fg: "#ccd4dc", accent: "#4a6a88", secondary: "#141e2a" },
    vars: {
      "--bg-primary": "#0c1218", "--bg-secondary": "#141e2a", "--bg-tertiary": "#1e2c3c", "--bg-elevated": "#283a4e",
      "--text-primary": "#ccd4dc", "--text-secondary": "#6e8098", "--accent-blue": "#4a6a88", "--accent-green": "#58a878",
      "--accent-purple": "#8878a8", "--accent-gold": "#c8a44a", "--accent-rose": "#cc5860",
      "--border-color": "rgba(74, 106, 136, 0.12)", "--error-color": "#cc5860",
    },
  },
  {
    id: "light-rivian-storm", name: "Rivian Storm", category: "standard", mode: "light", pairId: "rivian-storm",
    preview: { bg: "#f0f2f4", fg: "#1e2830", accent: "#3a5a72", secondary: "#e0e4e8" },
    vars: {
      "--bg-primary": "#f0f2f4", "--bg-secondary": "#e0e4e8", "--bg-tertiary": "#ccd2d8", "--bg-elevated": "#fafbfc",
      "--text-primary": "#1e2830", "--text-secondary": "#546878", "--accent-blue": "#3a5a72", "--accent-green": "#388a58",
      "--accent-purple": "#605880", "--accent-gold": "#8a7a1e", "--accent-rose": "#a84448",
      "--border-color": "rgba(58, 90, 114, 0.10)", "--error-color": "#a84448",
    },
  },
  // ── Pair: Half Moon Grey (R2 exterior) / Slate Sky (R1 interior) ──
  {
    id: "dark-rivian-halfmoon", name: "Rivian Halfmoon", category: "standard", mode: "dark", pairId: "rivian-halfmoon",
    preview: { bg: "#121210", fg: "#d2d0cc", accent: "#8a8478", secondary: "#1e1c1a" },
    vars: {
      "--bg-primary": "#121210", "--bg-secondary": "#1e1c1a", "--bg-tertiary": "#2a2826", "--bg-elevated": "#363432",
      "--text-primary": "#d2d0cc", "--text-secondary": "#8a8680", "--accent-blue": "#8a8478", "--accent-green": "#6ea868",
      "--accent-purple": "#a890b0", "--accent-gold": "#c8a448", "--accent-rose": "#c86058",
      "--border-color": "rgba(138, 132, 120, 0.12)", "--error-color": "#c86058",
    },
  },
  {
    id: "light-rivian-halfmoon", name: "Rivian Halfmoon", category: "standard", mode: "light", pairId: "rivian-halfmoon",
    preview: { bg: "#f0eeec", fg: "#242220", accent: "#6a6460", secondary: "#e0dcda" },
    vars: {
      "--bg-primary": "#f0eeec", "--bg-secondary": "#e0dcda", "--bg-tertiary": "#cec8c4", "--bg-elevated": "#faf8f6",
      "--text-primary": "#242220", "--text-secondary": "#645e58", "--accent-blue": "#6a6460", "--accent-green": "#488a40",
      "--accent-purple": "#6a5878", "--accent-gold": "#8a7a20", "--accent-rose": "#a84038",
      "--border-color": "rgba(106, 100, 96, 0.10)", "--error-color": "#a84038",
    },
  },
  // ── Pair: Borealis (R2 Performance exclusive) ──
  {
    id: "dark-rivian-borealis", name: "Rivian Borealis", category: "standard", mode: "dark", pairId: "rivian-borealis",
    preview: { bg: "#0a1210", fg: "#d0e0d8", accent: "#38a088", secondary: "#142220" },
    vars: {
      "--bg-primary": "#0a1210", "--bg-secondary": "#142220", "--bg-tertiary": "#1c302c", "--bg-elevated": "#243e38",
      "--text-primary": "#d0e0d8", "--text-secondary": "#6a9a8c", "--accent-blue": "#38a088", "--accent-green": "#50c898",
      "--accent-purple": "#8888c0", "--accent-gold": "#c8b048", "--accent-rose": "#d06060",
      "--border-color": "rgba(56, 160, 136, 0.14)", "--error-color": "#d06060",
    },
  },
  {
    id: "light-rivian-borealis", name: "Rivian Borealis", category: "standard", mode: "light", pairId: "rivian-borealis",
    preview: { bg: "#f4f8f6", fg: "#1a2822", accent: "#2a8070", secondary: "#e0ece8" },
    vars: {
      "--bg-primary": "#f4f8f6", "--bg-secondary": "#e0ece8", "--bg-tertiary": "#ccdcd6", "--bg-elevated": "#fafefc",
      "--text-primary": "#1a2822", "--text-secondary": "#466a60", "--accent-blue": "#2a8070", "--accent-green": "#38a06a",
      "--accent-purple": "#5a5a88", "--accent-gold": "#8a8020", "--accent-rose": "#b04040",
      "--border-color": "rgba(42, 128, 112, 0.10)", "--error-color": "#b04040",
    },
  },

  // ═══════════════════════════════════════════════════════════════════
  //  Tesla — Vehicle Color Inspired Themes
  // ═══════════════════════════════════════════════════════════════════

  // ── Pair: Midnight Cherry (Model S/X refresh) / Pearl White (most popular) ──
  {
    id: "dark-tesla-cherry", name: "Tesla Cherry", category: "standard", mode: "dark", pairId: "tesla-cherry",
    preview: { bg: "#140a0e", fg: "#e4d0d6", accent: "#c0485a", secondary: "#221218" },
    vars: {
      "--bg-primary": "#140a0e", "--bg-secondary": "#221218", "--bg-tertiary": "#301c22", "--bg-elevated": "#3e262e",
      "--text-primary": "#e4d0d6", "--text-secondary": "#a07a84", "--accent-blue": "#c0485a", "--accent-green": "#5aaa6a",
      "--accent-purple": "#9a6aaa", "--accent-gold": "#cc9a30", "--accent-rose": "#e04458",
      "--border-color": "rgba(192, 72, 90, 0.12)", "--error-color": "#e04458",
    },
  },
  {
    id: "light-tesla-cherry", name: "Tesla Cherry", category: "standard", mode: "light", pairId: "tesla-cherry",
    preview: { bg: "#f8f6f6", fg: "#2a1a1e", accent: "#9a2840", secondary: "#ece4e6" },
    vars: {
      "--bg-primary": "#f8f6f6", "--bg-secondary": "#ece4e6", "--bg-tertiary": "#dcd0d4", "--bg-elevated": "#ffffff",
      "--text-primary": "#2a1a1e", "--text-secondary": "#6a4a52", "--accent-blue": "#9a2840", "--accent-green": "#388a48",
      "--accent-purple": "#7a4a8a", "--accent-gold": "#9a7a18", "--accent-rose": "#c02040",
      "--border-color": "rgba(154, 40, 64, 0.10)", "--error-color": "#c02040",
    },
  },
  // ── Pair: Ultra Red (Model 3/Y) / Quicksilver (Model S/X/3/Y) ──
  {
    id: "dark-tesla-red", name: "Tesla Red", category: "standard", mode: "dark", pairId: "tesla-red",
    preview: { bg: "#180808", fg: "#e8d0d0", accent: "#e03030", secondary: "#2a1010" },
    vars: {
      "--bg-primary": "#180808", "--bg-secondary": "#2a1010", "--bg-tertiary": "#3a1a1a", "--bg-elevated": "#4a2424",
      "--text-primary": "#e8d0d0", "--text-secondary": "#a88080", "--accent-blue": "#e03030", "--accent-green": "#4aaa5a",
      "--accent-purple": "#aa68aa", "--accent-gold": "#d0a030", "--accent-rose": "#e03030",
      "--border-color": "rgba(224, 48, 48, 0.14)", "--error-color": "#e84040",
    },
  },
  {
    id: "light-tesla-red", name: "Tesla Red", category: "standard", mode: "light", pairId: "tesla-red",
    preview: { bg: "#f2f2f0", fg: "#222220", accent: "#6a6a68", secondary: "#e4e4e0" },
    vars: {
      "--bg-primary": "#f2f2f0", "--bg-secondary": "#e4e4e0", "--bg-tertiary": "#d2d2ce", "--bg-elevated": "#fafaf8",
      "--text-primary": "#222220", "--text-secondary": "#5a5a56", "--accent-blue": "#6a6a68", "--accent-green": "#488a3a",
      "--accent-purple": "#6a5a7a", "--accent-gold": "#8a7a20", "--accent-rose": "#a03030",
      "--border-color": "rgba(106, 106, 104, 0.10)", "--error-color": "#a03030",
    },
  },
  // ── Pair: Deep Blue Metallic (Model S/X) / Solid Black (base color) ──
  {
    id: "dark-tesla-blue", name: "Tesla Blue", category: "standard", mode: "dark", pairId: "tesla-blue",
    preview: { bg: "#080e18", fg: "#d0d8e6", accent: "#4070b0", secondary: "#101828" },
    vars: {
      "--bg-primary": "#080e18", "--bg-secondary": "#101828", "--bg-tertiary": "#1a2438", "--bg-elevated": "#243048",
      "--text-primary": "#d0d8e6", "--text-secondary": "#7088a8", "--accent-blue": "#4070b0", "--accent-green": "#48aa68",
      "--accent-purple": "#8870b0", "--accent-gold": "#c8a840", "--accent-rose": "#d04858",
      "--border-color": "rgba(64, 112, 176, 0.12)", "--error-color": "#d04858",
    },
  },
  {
    id: "light-tesla-blue", name: "Tesla Blue", category: "standard", mode: "light", pairId: "tesla-blue",
    preview: { bg: "#f0f2f4", fg: "#141618", accent: "#2a4a78", secondary: "#e0e4e8" },
    vars: {
      "--bg-primary": "#f0f2f4", "--bg-secondary": "#e0e4e8", "--bg-tertiary": "#ccd2d8", "--bg-elevated": "#fafbfc",
      "--text-primary": "#141618", "--text-secondary": "#4a5a6a", "--accent-blue": "#2a4a78", "--accent-green": "#2a7a40",
      "--accent-purple": "#5a4a7a", "--accent-gold": "#7a6a18", "--accent-rose": "#a82838",
      "--border-color": "rgba(42, 74, 120, 0.10)", "--error-color": "#a82838",
    },
  },
  // ── Pair: Midnight Silver (classic Model 3/Y) / Ultra White Interior ──
  {
    id: "dark-tesla-silver", name: "Tesla Silver", category: "standard", mode: "dark", pairId: "tesla-silver",
    preview: { bg: "#101214", fg: "#d2d4d8", accent: "#7a8898", secondary: "#1a1e22" },
    vars: {
      "--bg-primary": "#101214", "--bg-secondary": "#1a1e22", "--bg-tertiary": "#262a30", "--bg-elevated": "#32383e",
      "--text-primary": "#d2d4d8", "--text-secondary": "#808890", "--accent-blue": "#7a8898", "--accent-green": "#58a068",
      "--accent-purple": "#9080a8", "--accent-gold": "#baa840", "--accent-rose": "#c85060",
      "--border-color": "rgba(122, 136, 152, 0.10)", "--error-color": "#c85060",
    },
  },
  {
    id: "light-tesla-silver", name: "Tesla Silver", category: "standard", mode: "light", pairId: "tesla-silver",
    preview: { bg: "#fafafa", fg: "#1e2024", accent: "#4a5468", secondary: "#eeeeee" },
    vars: {
      "--bg-primary": "#fafafa", "--bg-secondary": "#eeeeee", "--bg-tertiary": "#dcdcdc", "--bg-elevated": "#ffffff",
      "--text-primary": "#1e2024", "--text-secondary": "#555a64", "--accent-blue": "#4a5468", "--accent-green": "#388a48",
      "--accent-purple": "#5a4a72", "--accent-gold": "#807020", "--accent-rose": "#a03040",
      "--border-color": "rgba(74, 84, 104, 0.10)", "--error-color": "#a03040",
    },
  },
  // ── Pair: Stealth Grey (Cybertruck) / Stainless Steel (Cybertruck raw) ──
  {
    id: "dark-tesla-stealth", name: "Tesla Stealth", category: "standard", mode: "dark", pairId: "tesla-stealth",
    preview: { bg: "#0e0e10", fg: "#c8c8cc", accent: "#5a5a62", secondary: "#1a1a1e" },
    vars: {
      "--bg-primary": "#0e0e10", "--bg-secondary": "#1a1a1e", "--bg-tertiary": "#26262c", "--bg-elevated": "#32323a",
      "--text-primary": "#c8c8cc", "--text-secondary": "#7e7e86", "--accent-blue": "#5a5a62", "--accent-green": "#50a060",
      "--accent-purple": "#8878a0", "--accent-gold": "#b8a838", "--accent-rose": "#c04858",
      "--border-color": "rgba(90, 90, 98, 0.12)", "--error-color": "#c04858",
    },
  },
  {
    id: "light-tesla-stealth", name: "Tesla Stealth", category: "standard", mode: "light", pairId: "tesla-stealth",
    preview: { bg: "#f0f0ee", fg: "#202024", accent: "#707078", secondary: "#e2e2de" },
    vars: {
      "--bg-primary": "#f0f0ee", "--bg-secondary": "#e2e2de", "--bg-tertiary": "#d0d0cc", "--bg-elevated": "#fafaf8",
      "--text-primary": "#202024", "--text-secondary": "#555558", "--accent-blue": "#707078", "--accent-green": "#408a40",
      "--accent-purple": "#5a5078", "--accent-gold": "#7a7018", "--accent-rose": "#a03040",
      "--border-color": "rgba(112, 112, 120, 0.10)", "--error-color": "#a03040",
    },
  },
  // ── Pair: Glacier Blue (Model Y Juniper) / Cream Interior (Juniper tan) ──
  {
    id: "dark-tesla-glacier", name: "Tesla Glacier", category: "standard", mode: "dark", pairId: "tesla-glacier",
    preview: { bg: "#0a1218", fg: "#d0dce4", accent: "#5a9abc", secondary: "#142028" },
    vars: {
      "--bg-primary": "#0a1218", "--bg-secondary": "#142028", "--bg-tertiary": "#1e2e38", "--bg-elevated": "#283a48",
      "--text-primary": "#d0dce4", "--text-secondary": "#7898ac", "--accent-blue": "#5a9abc", "--accent-green": "#50aa6a",
      "--accent-purple": "#8a7ab0", "--accent-gold": "#c8a840", "--accent-rose": "#d05060",
      "--border-color": "rgba(90, 154, 188, 0.12)", "--error-color": "#d05060",
    },
  },
  {
    id: "light-tesla-glacier", name: "Tesla Glacier", category: "standard", mode: "light", pairId: "tesla-glacier",
    preview: { bg: "#f8f4ee", fg: "#2a2418", accent: "#3a7a9a", secondary: "#ece4d8" },
    vars: {
      "--bg-primary": "#f8f4ee", "--bg-secondary": "#ece4d8", "--bg-tertiary": "#dcd2c4", "--bg-elevated": "#fffcf6",
      "--text-primary": "#2a2418", "--text-secondary": "#60584a", "--accent-blue": "#3a7a9a", "--accent-green": "#3a8a4a",
      "--accent-purple": "#6a5a80", "--accent-gold": "#8a7820", "--accent-rose": "#a83040",
      "--border-color": "rgba(58, 122, 154, 0.10)", "--error-color": "#a83040",
    },
  },
  // ── Pair: Lunar Silver (Model S Plaid) / Titanium Copper (Model Y Highland) ──
  {
    id: "dark-tesla-lunar", name: "Tesla Lunar", category: "standard", mode: "dark", pairId: "tesla-lunar",
    preview: { bg: "#0e1012", fg: "#d4d6d8", accent: "#8a9098", secondary: "#1a1e20" },
    vars: {
      "--bg-primary": "#0e1012", "--bg-secondary": "#1a1e20", "--bg-tertiary": "#282c30", "--bg-elevated": "#343a3e",
      "--text-primary": "#d4d6d8", "--text-secondary": "#848a90", "--accent-blue": "#8a9098", "--accent-green": "#58a868",
      "--accent-purple": "#9888a8", "--accent-gold": "#c0a840", "--accent-rose": "#c85060",
      "--border-color": "rgba(138, 144, 152, 0.10)", "--error-color": "#c85060",
    },
  },
  {
    id: "light-tesla-lunar", name: "Tesla Lunar", category: "standard", mode: "light", pairId: "tesla-lunar",
    preview: { bg: "#f6f0ea", fg: "#28201a", accent: "#8a6840", secondary: "#e8dcd0" },
    vars: {
      "--bg-primary": "#f6f0ea", "--bg-secondary": "#e8dcd0", "--bg-tertiary": "#d8cab8", "--bg-elevated": "#fefaf4",
      "--text-primary": "#28201a", "--text-secondary": "#605040", "--accent-blue": "#8a6840", "--accent-green": "#4a8a3a",
      "--accent-purple": "#6a5470", "--accent-gold": "#8a7420", "--accent-rose": "#a83030",
      "--border-color": "rgba(138, 104, 64, 0.10)", "--error-color": "#a83030",
    },
  },
  // ── Pair: Ludicrous Red (Performance) / Plaid Neon (Easter egg green) ──
  {
    id: "dark-tesla-ludicrous", name: "Tesla Ludicrous", category: "standard", mode: "dark", pairId: "tesla-ludicrous",
    preview: { bg: "#1a0808", fg: "#e8cccc", accent: "#ff2020", secondary: "#2e1010" },
    vars: {
      "--bg-primary": "#1a0808", "--bg-secondary": "#2e1010", "--bg-tertiary": "#401818", "--bg-elevated": "#502222",
      "--text-primary": "#e8cccc", "--text-secondary": "#a88080", "--accent-blue": "#ff2020", "--accent-green": "#40cc60",
      "--accent-purple": "#cc60cc", "--accent-gold": "#e0a020", "--accent-rose": "#ff2020",
      "--border-color": "rgba(255, 32, 32, 0.14)", "--error-color": "#ff3838",
    },
  },
  {
    id: "light-tesla-ludicrous", name: "Tesla Ludicrous", category: "standard", mode: "light", pairId: "tesla-ludicrous",
    preview: { bg: "#f0faf2", fg: "#0e2818", accent: "#18aa40", secondary: "#d8f0de" },
    vars: {
      "--bg-primary": "#f0faf2", "--bg-secondary": "#d8f0de", "--bg-tertiary": "#c4e2cc", "--bg-elevated": "#fafef8",
      "--text-primary": "#0e2818", "--text-secondary": "#3a6a48", "--accent-blue": "#18aa40", "--accent-green": "#18aa40",
      "--accent-purple": "#5a6a90", "--accent-gold": "#7a8020", "--accent-rose": "#c02020",
      "--border-color": "rgba(24, 170, 64, 0.10)", "--error-color": "#c02020",
    },
  },
  // ── Pair: Cyberbeast Matte Black / Foundation Series Satin ──
  {
    id: "dark-tesla-cyberbeast", name: "Tesla Cyberbeast", category: "standard", mode: "dark", pairId: "tesla-cyberbeast",
    preview: { bg: "#080808", fg: "#c0c0c0", accent: "#e0e0e0", secondary: "#141414" },
    vars: {
      "--bg-primary": "#080808", "--bg-secondary": "#141414", "--bg-tertiary": "#1e1e1e", "--bg-elevated": "#2a2a2a",
      "--text-primary": "#c0c0c0", "--text-secondary": "#7a7a7a", "--accent-blue": "#e0e0e0", "--accent-green": "#48c860",
      "--accent-purple": "#a090c0", "--accent-gold": "#d0b040", "--accent-rose": "#e04050",
      "--border-color": "rgba(224, 224, 224, 0.08)", "--error-color": "#e04050",
    },
  },
  {
    id: "light-tesla-cyberbeast", name: "Tesla Cyberbeast", category: "standard", mode: "light", pairId: "tesla-cyberbeast",
    preview: { bg: "#f4f4f2", fg: "#1a1a1a", accent: "#404040", secondary: "#e6e6e2" },
    vars: {
      "--bg-primary": "#f4f4f2", "--bg-secondary": "#e6e6e2", "--bg-tertiary": "#d4d4d0", "--bg-elevated": "#fcfcfa",
      "--text-primary": "#1a1a1a", "--text-secondary": "#505050", "--accent-blue": "#404040", "--accent-green": "#2a8a3a",
      "--accent-purple": "#5a4a70", "--accent-gold": "#7a6a18", "--accent-rose": "#a02030",
      "--border-color": "rgba(64, 64, 64, 0.10)", "--error-color": "#a02030",
    },
  },
  // ── Pair: Autopilot Blue (UI accent) / Supercharger White ──
  {
    id: "dark-tesla-autopilot", name: "Tesla Autopilot", category: "standard", mode: "dark", pairId: "tesla-autopilot",
    preview: { bg: "#0a0e1a", fg: "#d0d8ea", accent: "#3880e0", secondary: "#121828" },
    vars: {
      "--bg-primary": "#0a0e1a", "--bg-secondary": "#121828", "--bg-tertiary": "#1c2638", "--bg-elevated": "#263248",
      "--text-primary": "#d0d8ea", "--text-secondary": "#7088b0", "--accent-blue": "#3880e0", "--accent-green": "#40b060",
      "--accent-purple": "#8068c0", "--accent-gold": "#d0a830", "--accent-rose": "#e04858",
      "--border-color": "rgba(56, 128, 224, 0.12)", "--error-color": "#e04858",
    },
  },
  {
    id: "light-tesla-autopilot", name: "Tesla Autopilot", category: "standard", mode: "light", pairId: "tesla-autopilot",
    preview: { bg: "#fafcfe", fg: "#141820", accent: "#2060c0", secondary: "#eaf0f8" },
    vars: {
      "--bg-primary": "#fafcfe", "--bg-secondary": "#eaf0f8", "--bg-tertiary": "#d4dfe8", "--bg-elevated": "#ffffff",
      "--text-primary": "#141820", "--text-secondary": "#445068", "--accent-blue": "#2060c0", "--accent-green": "#288a40",
      "--accent-purple": "#5848a0", "--accent-gold": "#807018", "--accent-rose": "#b02838",
      "--border-color": "rgba(32, 96, 192, 0.10)", "--error-color": "#b02838",
    },
  },

  // ═══════════════════════════════════════════════════════════════════
  //  Apple MacBook & iPhone — Product Color Inspired Themes
  // ═══════════════════════════════════════════════════════════════════

  // ── Pair: Space Black (MacBook Pro M3 Pro/Max) / Silver (MacBook Pro classic) ──
  {
    id: "dark-mac-spaceblack", name: "Space Black", category: "standard", mode: "dark", pairId: "mac-spaceblack",
    preview: { bg: "#0c0c0e", fg: "#d8d8dc", accent: "#6eaadc", secondary: "#18181c" },
    vars: {
      "--bg-primary": "#0c0c0e", "--bg-secondary": "#18181c", "--bg-tertiary": "#222228", "--bg-elevated": "#2c2c34",
      "--text-primary": "#d8d8dc", "--text-secondary": "#7a7a86", "--accent-blue": "#6eaadc", "--accent-green": "#5ec27a",
      "--accent-purple": "#b48cda", "--accent-gold": "#e2b84a", "--accent-rose": "#e86070",
      "--border-color": "rgba(110, 170, 220, 0.08)", "--error-color": "#e86070",
    },
  },
  {
    id: "light-mac-spaceblack", name: "Space Black", category: "standard", mode: "light", pairId: "mac-spaceblack",
    preview: { bg: "#f4f4f6", fg: "#1c1c22", accent: "#3478f6", secondary: "#e8e8ec" },
    vars: {
      "--bg-primary": "#f4f4f6", "--bg-secondary": "#e8e8ec", "--bg-tertiary": "#d8d8de", "--bg-elevated": "#ffffff",
      "--text-primary": "#1c1c22", "--text-secondary": "#636370", "--accent-blue": "#3478f6", "--accent-green": "#30a856",
      "--accent-purple": "#8944da", "--accent-gold": "#c08a10", "--accent-rose": "#d63852",
      "--border-color": "rgba(52, 120, 246, 0.08)", "--error-color": "#d63852",
    },
  },
  // ── Pair: Midnight (MacBook Air M2/M3 — dark navy) / Starlight (MacBook Air warm champagne) ──
  {
    id: "dark-mac-midnight", name: "Starlight", category: "standard", mode: "dark", pairId: "mac-midnight",
    preview: { bg: "#0a0c14", fg: "#d0d4e0", accent: "#4a78c0", secondary: "#141828" },
    vars: {
      "--bg-primary": "#0a0c14", "--bg-secondary": "#141828", "--bg-tertiary": "#1c2238", "--bg-elevated": "#262e48",
      "--text-primary": "#d0d4e0", "--text-secondary": "#6a72a8", "--accent-blue": "#4a78c0", "--accent-green": "#48a870",
      "--accent-purple": "#9a7cc8", "--accent-gold": "#d4aa3a", "--accent-rose": "#d85868",
      "--border-color": "rgba(74, 120, 192, 0.10)", "--error-color": "#d85868",
    },
  },
  {
    id: "light-mac-midnight", name: "Starlight", category: "standard", mode: "light", pairId: "mac-midnight",
    preview: { bg: "#f8f4ee", fg: "#2a2620", accent: "#a0782a", secondary: "#eee8de" },
    vars: {
      "--bg-primary": "#f8f4ee", "--bg-secondary": "#eee8de", "--bg-tertiary": "#e0d8ca", "--bg-elevated": "#fefaf4",
      "--text-primary": "#2a2620", "--text-secondary": "#706452", "--accent-blue": "#a0782a", "--accent-green": "#5a8a40",
      "--accent-purple": "#8a6a98", "--accent-gold": "#b08a18", "--accent-rose": "#b84838",
      "--border-color": "rgba(160, 120, 42, 0.10)", "--error-color": "#b84838",
    },
  },
  // ── Pair: Space Gray (classic MacBook) ──
  {
    id: "dark-mac-spacegray", name: "Space Gray", category: "standard", mode: "dark", pairId: "mac-spacegray",
    preview: { bg: "#111113", fg: "#d4d4d8", accent: "#8c8ca0", secondary: "#1c1c20" },
    vars: {
      "--bg-primary": "#111113", "--bg-secondary": "#1c1c20", "--bg-tertiary": "#26262c", "--bg-elevated": "#303038",
      "--text-primary": "#d4d4d8", "--text-secondary": "#78788a", "--accent-blue": "#8c8ca0", "--accent-green": "#5ab872",
      "--accent-purple": "#a888c0", "--accent-gold": "#d0a840", "--accent-rose": "#d86068",
      "--border-color": "rgba(140, 140, 160, 0.10)", "--error-color": "#d86068",
    },
  },
  {
    id: "light-mac-spacegray", name: "Space Gray", category: "standard", mode: "light", pairId: "mac-spacegray",
    preview: { bg: "#f6f6f8", fg: "#1e1e24", accent: "#5a5a72", secondary: "#eaeaee" },
    vars: {
      "--bg-primary": "#f6f6f8", "--bg-secondary": "#eaeaee", "--bg-tertiary": "#dcdce2", "--bg-elevated": "#ffffff",
      "--text-primary": "#1e1e24", "--text-secondary": "#5e5e70", "--accent-blue": "#5a5a72", "--accent-green": "#3a8a4e",
      "--accent-purple": "#6e5a88", "--accent-gold": "#9a8018", "--accent-rose": "#b84048",
      "--border-color": "rgba(90, 90, 114, 0.08)", "--error-color": "#b84048",
    },
  },
  // ── Pair: Black Titanium (iPhone 15/16 Pro) / White Titanium ──
  {
    id: "dark-iphone-blackti", name: "Black Titanium", category: "standard", mode: "dark", pairId: "iphone-blackti",
    preview: { bg: "#0e0e10", fg: "#d6d4d0", accent: "#8a8680", secondary: "#1a1a1e" },
    vars: {
      "--bg-primary": "#0e0e10", "--bg-secondary": "#1a1a1e", "--bg-tertiary": "#24242a", "--bg-elevated": "#2e2e36",
      "--text-primary": "#d6d4d0", "--text-secondary": "#807c76", "--accent-blue": "#8a8680", "--accent-green": "#68b070",
      "--accent-purple": "#a090b8", "--accent-gold": "#d0a848", "--accent-rose": "#d46058",
      "--border-color": "rgba(138, 134, 128, 0.10)", "--error-color": "#d46058",
    },
  },
  {
    id: "light-iphone-blackti", name: "Black Titanium", category: "standard", mode: "light", pairId: "iphone-blackti",
    preview: { bg: "#f6f4f2", fg: "#22201e", accent: "#706c68", secondary: "#eae8e4" },
    vars: {
      "--bg-primary": "#f6f4f2", "--bg-secondary": "#eae8e4", "--bg-tertiary": "#d8d4d0", "--bg-elevated": "#fcfaf8",
      "--text-primary": "#22201e", "--text-secondary": "#5e5a56", "--accent-blue": "#706c68", "--accent-green": "#488a42",
      "--accent-purple": "#685c78", "--accent-gold": "#8a7a1e", "--accent-rose": "#a84038",
      "--border-color": "rgba(112, 108, 104, 0.08)", "--error-color": "#a84038",
    },
  },
  // ── Pair: Blue Titanium (iPhone 15 Pro) / Natural Titanium ──
  {
    id: "dark-iphone-blueti", name: "Blue Titanium", category: "standard", mode: "dark", pairId: "iphone-blueti",
    preview: { bg: "#0c1018", fg: "#d0d4dc", accent: "#5a7898", secondary: "#141c28" },
    vars: {
      "--bg-primary": "#0c1018", "--bg-secondary": "#141c28", "--bg-tertiary": "#1c2838", "--bg-elevated": "#263448",
      "--text-primary": "#d0d4dc", "--text-secondary": "#6a7a94", "--accent-blue": "#5a7898", "--accent-green": "#50a872",
      "--accent-purple": "#8a80b0", "--accent-gold": "#c8a040", "--accent-rose": "#cc5860",
      "--border-color": "rgba(90, 120, 152, 0.10)", "--error-color": "#cc5860",
    },
  },
  {
    id: "light-iphone-blueti", name: "Blue Titanium", category: "standard", mode: "light", pairId: "iphone-blueti",
    preview: { bg: "#f2f0ee", fg: "#222020", accent: "#5a6878", secondary: "#e4e2de" },
    vars: {
      "--bg-primary": "#f2f0ee", "--bg-secondary": "#e4e2de", "--bg-tertiary": "#d2cec8", "--bg-elevated": "#faf8f6",
      "--text-primary": "#222020", "--text-secondary": "#5c5854", "--accent-blue": "#5a6878", "--accent-green": "#3e8a48",
      "--accent-purple": "#645a78", "--accent-gold": "#8a7820", "--accent-rose": "#a84040",
      "--border-color": "rgba(90, 104, 120, 0.08)", "--error-color": "#a84040",
    },
  },
  // ── Pair: Desert Titanium (iPhone 16 Pro) ──
  {
    id: "dark-iphone-desertti", name: "Desert Titanium", category: "standard", mode: "dark", pairId: "iphone-desertti",
    preview: { bg: "#12100e", fg: "#d8d0c8", accent: "#a89070", secondary: "#1e1a16" },
    vars: {
      "--bg-primary": "#12100e", "--bg-secondary": "#1e1a16", "--bg-tertiary": "#2a2620", "--bg-elevated": "#36302a",
      "--text-primary": "#d8d0c8", "--text-secondary": "#8a8274", "--accent-blue": "#a89070", "--accent-green": "#6aa860",
      "--accent-purple": "#a890a8", "--accent-gold": "#d0a44a", "--accent-rose": "#cc6050",
      "--border-color": "rgba(168, 144, 112, 0.12)", "--error-color": "#cc6050",
    },
  },
  {
    id: "light-iphone-desertti", name: "Desert Titanium", category: "standard", mode: "light", pairId: "iphone-desertti",
    preview: { bg: "#f6f0ea", fg: "#282218", accent: "#886838", secondary: "#ece2d6" },
    vars: {
      "--bg-primary": "#f6f0ea", "--bg-secondary": "#ece2d6", "--bg-tertiary": "#dcd0c0", "--bg-elevated": "#fef8f0",
      "--text-primary": "#282218", "--text-secondary": "#6e6050", "--accent-blue": "#886838", "--accent-green": "#508838",
      "--accent-purple": "#7a6680", "--accent-gold": "#9a7a18", "--accent-rose": "#aa4438",
      "--border-color": "rgba(136, 104, 56, 0.10)", "--error-color": "#aa4438",
    },
  },
  // ── Pair: Ultramarine (iPhone 16) ──
  {
    id: "dark-iphone-ultramarine", name: "Ultramarine", category: "standard", mode: "dark", pairId: "iphone-ultramarine",
    preview: { bg: "#0a0c1a", fg: "#d0d4e8", accent: "#4860d0", secondary: "#141840" },
    vars: {
      "--bg-primary": "#0a0c1a", "--bg-secondary": "#141840", "--bg-tertiary": "#1c2258", "--bg-elevated": "#262e68",
      "--text-primary": "#d0d4e8", "--text-secondary": "#6a70b0", "--accent-blue": "#4860d0", "--accent-green": "#48b878",
      "--accent-purple": "#8a6ae0", "--accent-gold": "#dab040", "--accent-rose": "#e05868",
      "--border-color": "rgba(72, 96, 208, 0.12)", "--error-color": "#e05868",
    },
  },
  {
    id: "light-iphone-ultramarine", name: "Ultramarine", category: "standard", mode: "light", pairId: "iphone-ultramarine",
    preview: { bg: "#f2f2fa", fg: "#181830", accent: "#3040b0", secondary: "#e2e2f0" },
    vars: {
      "--bg-primary": "#f2f2fa", "--bg-secondary": "#e2e2f0", "--bg-tertiary": "#d0d0e2", "--bg-elevated": "#fafaff",
      "--text-primary": "#181830", "--text-secondary": "#4a4a78", "--accent-blue": "#3040b0", "--accent-green": "#2a8a50",
      "--accent-purple": "#5a3aaa", "--accent-gold": "#9a8018", "--accent-rose": "#b83848",
      "--border-color": "rgba(48, 64, 176, 0.10)", "--error-color": "#b83848",
    },
  },
  // ── Pair: Teal (iPhone 16) ──
  {
    id: "dark-iphone-teal", name: "iPhone Teal", category: "standard", mode: "dark", pairId: "iphone-teal",
    preview: { bg: "#0a1214", fg: "#d0dce0", accent: "#3a9aa0", secondary: "#121e22" },
    vars: {
      "--bg-primary": "#0a1214", "--bg-secondary": "#121e22", "--bg-tertiary": "#1a2c32", "--bg-elevated": "#223a42",
      "--text-primary": "#d0dce0", "--text-secondary": "#6a8a90", "--accent-blue": "#3a9aa0", "--accent-green": "#48c088",
      "--accent-purple": "#8088c0", "--accent-gold": "#c8aa40", "--accent-rose": "#d06060",
      "--border-color": "rgba(58, 154, 160, 0.12)", "--error-color": "#d06060",
    },
  },
  {
    id: "light-iphone-teal", name: "iPhone Teal", category: "standard", mode: "light", pairId: "iphone-teal",
    preview: { bg: "#f0f8f8", fg: "#182828", accent: "#288088", secondary: "#dceef0" },
    vars: {
      "--bg-primary": "#f0f8f8", "--bg-secondary": "#dceef0", "--bg-tertiary": "#c8e0e2", "--bg-elevated": "#fafefe",
      "--text-primary": "#182828", "--text-secondary": "#466a6e", "--accent-blue": "#288088", "--accent-green": "#2a8a5e",
      "--accent-purple": "#5a6088", "--accent-gold": "#8a8018", "--accent-rose": "#b04848",
      "--border-color": "rgba(40, 128, 136, 0.10)", "--error-color": "#b04848",
    },
  },
  // ── Pair: iPhone Pink (iPhone 15/16) ──
  {
    id: "dark-iphone-pink", name: "iPhone Pink", category: "standard", mode: "dark", pairId: "iphone-pink",
    preview: { bg: "#140c10", fg: "#e0d4d8", accent: "#c06888", secondary: "#221420" },
    vars: {
      "--bg-primary": "#140c10", "--bg-secondary": "#221420", "--bg-tertiary": "#301e2c", "--bg-elevated": "#3e283a",
      "--text-primary": "#e0d4d8", "--text-secondary": "#9a7888", "--accent-blue": "#c06888", "--accent-green": "#5ab870",
      "--accent-purple": "#b080c8", "--accent-gold": "#d0a448", "--accent-rose": "#e05070",
      "--border-color": "rgba(192, 104, 136, 0.12)", "--error-color": "#e05070",
    },
  },
  {
    id: "light-iphone-pink", name: "iPhone Pink", category: "standard", mode: "light", pairId: "iphone-pink",
    preview: { bg: "#f8f0f2", fg: "#2a1e22", accent: "#a04868", secondary: "#f0e0e4" },
    vars: {
      "--bg-primary": "#f8f0f2", "--bg-secondary": "#f0e0e4", "--bg-tertiary": "#e0ccd2", "--bg-elevated": "#fef8fa",
      "--text-primary": "#2a1e22", "--text-secondary": "#785060", "--accent-blue": "#a04868", "--accent-green": "#3e8a48",
      "--accent-purple": "#884880", "--accent-gold": "#9a7a20", "--accent-rose": "#c03050",
      "--border-color": "rgba(160, 72, 104, 0.10)", "--error-color": "#c03050",
    },
  },
  // ── Pair: iPhone Green (iPhone 15) / iPhone Yellow (iPhone 15) ──
  {
    id: "dark-iphone-green", name: "iPhone Green", category: "standard", mode: "dark", pairId: "iphone-green",
    preview: { bg: "#0c120e", fg: "#d0dcd4", accent: "#58a868", secondary: "#142018" },
    vars: {
      "--bg-primary": "#0c120e", "--bg-secondary": "#142018", "--bg-tertiary": "#1e2e22", "--bg-elevated": "#283c2e",
      "--text-primary": "#d0dcd4", "--text-secondary": "#6e9878", "--accent-blue": "#58a868", "--accent-green": "#68cc78",
      "--accent-purple": "#9888b8", "--accent-gold": "#c8aa38", "--accent-rose": "#d06058",
      "--border-color": "rgba(88, 168, 104, 0.12)", "--error-color": "#d06058",
    },
  },
  {
    id: "light-iphone-green", name: "iPhone Green", category: "standard", mode: "light", pairId: "iphone-green",
    preview: { bg: "#f8f6ee", fg: "#22201a", accent: "#a09020", secondary: "#eeeadc" },
    vars: {
      "--bg-primary": "#f8f6ee", "--bg-secondary": "#eeeadc", "--bg-tertiary": "#dcd6c4", "--bg-elevated": "#fefcf4",
      "--text-primary": "#22201a", "--text-secondary": "#6a6450", "--accent-blue": "#a09020", "--accent-green": "#4a8a38",
      "--accent-purple": "#6e5e88", "--accent-gold": "#a08a18", "--accent-rose": "#b04038",
      "--border-color": "rgba(160, 144, 32, 0.10)", "--error-color": "#b04038",
    },
  },

  // ═══════════════════════════════════════════════════════════════════
  //  Supercar-Inspired Themes
  // ═══════════════════════════════════════════════════════════════════

  // ── Pair: Pagani (Huayra Silver & Carbon) ──
  {
    id: "dark-pagani", name: "Pagani", category: "supercar", mode: "dark", pairId: "pagani",
    preview: { bg: "#0a0c10", fg: "#c8cdd8", accent: "#7eb8da", secondary: "#141820" },
    vars: {
      "--bg-primary": "#0a0c10", "--bg-secondary": "#141820", "--bg-tertiary": "#1c222e", "--bg-elevated": "#242c3a",
      "--text-primary": "#c8cdd8", "--text-secondary": "#6a7488", "--accent-blue": "#7eb8da", "--accent-green": "#6ecfb0",
      "--accent-purple": "#9ca8c0", "--accent-gold": "#b8c4d8", "--accent-rose": "#d47888",
      "--border-color": "rgba(126, 184, 218, 0.08)", "--error-color": "#d47888",
    },
  },
  {
    id: "light-pagani", name: "Pagani", category: "supercar", mode: "light", pairId: "pagani",
    preview: { bg: "#f4f6f9", fg: "#0e1218", accent: "#3a7ca5", secondary: "#e4e8ee" },
    vars: {
      "--bg-primary": "#f4f6f9", "--bg-secondary": "#e4e8ee", "--bg-tertiary": "#d0d6e0", "--bg-elevated": "#ffffff",
      "--text-primary": "#0e1218", "--text-secondary": "#5a6478", "--accent-blue": "#3a7ca5", "--accent-green": "#3a9a7c",
      "--accent-purple": "#6878a0", "--accent-gold": "#5a7a98", "--accent-rose": "#a84858",
      "--border-color": "rgba(58, 124, 165, 0.10)", "--error-color": "#a84858",
    },
  },

  // ── Pair: Lamborghini (Giallo Orion / Verde Mantis) ──
  {
    id: "dark-lamborghini", name: "Lamborghini", category: "supercar", mode: "dark", pairId: "lamborghini",
    preview: { bg: "#0c0a00", fg: "#f0e8c8", accent: "#e8c820", secondary: "#1c1800" },
    vars: {
      "--bg-primary": "#0c0a00", "--bg-secondary": "#1c1800", "--bg-tertiary": "#282200", "--bg-elevated": "#342c08",
      "--text-primary": "#f0e8c8", "--text-secondary": "#8a8260", "--accent-blue": "#e8c820", "--accent-green": "#88c828",
      "--accent-purple": "#c8a830", "--accent-gold": "#e8c820", "--accent-rose": "#e84830",
      "--border-color": "rgba(232, 200, 32, 0.10)", "--error-color": "#e84830",
    },
  },
  {
    id: "light-lamborghini", name: "Lamborghini", category: "supercar", mode: "light", pairId: "lamborghini",
    preview: { bg: "#fefbe8", fg: "#1a1800", accent: "#b89b00", secondary: "#f5f0c8" },
    vars: {
      "--bg-primary": "#fefbe8", "--bg-secondary": "#f5f0c8", "--bg-tertiary": "#e8e2a8", "--bg-elevated": "#fffef0",
      "--text-primary": "#1a1800", "--text-secondary": "#6a6230", "--accent-blue": "#b89b00", "--accent-green": "#5a8a10",
      "--accent-purple": "#8a7a18", "--accent-gold": "#b89b00", "--accent-rose": "#c03020",
      "--border-color": "rgba(184, 155, 0, 0.12)", "--error-color": "#c03020",
    },
  },

  // ── Pair: Ferrari (Rosso Corsa / Bianco Avus) ──
  {
    id: "dark-ferrari", name: "Ferrari", category: "supercar", mode: "dark", pairId: "ferrari",
    preview: { bg: "#0e0a0a", fg: "#f2e8e6", accent: "#e12726", secondary: "#1c1414" },
    vars: {
      "--bg-primary": "#0e0a0a", "--bg-secondary": "#1c1414", "--bg-tertiary": "#2a1e1e", "--bg-elevated": "#362828",
      "--text-primary": "#f2e8e6", "--text-secondary": "#9a7e7a", "--accent-blue": "#e12726", "--accent-green": "#f0c808",
      "--accent-purple": "#cc3838", "--accent-gold": "#f0c808", "--accent-rose": "#ff4040",
      "--border-color": "rgba(225, 39, 38, 0.14)", "--error-color": "#ff5050",
      "--success-color": "#f0c808", "--warning-color": "#e89020", "--info-color": "#e12726",
      "--accent-color": "#e12726", "--glow-accent": "0 0 20px rgba(225, 39, 38, 0.25)",
    },
  },
  {
    id: "light-ferrari", name: "Ferrari", category: "supercar", mode: "light", pairId: "ferrari",
    preview: { bg: "#fdf6f5", fg: "#1a0c0a", accent: "#e12726", secondary: "#f8e6e4" },
    vars: {
      "--bg-primary": "#fdf6f5", "--bg-secondary": "#f8e6e4", "--bg-tertiary": "#f0d4d0", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a0c0a", "--text-secondary": "#7a504c", "--accent-blue": "#c82020", "--accent-green": "#b89b00",
      "--accent-purple": "#a82828", "--accent-gold": "#b89b00", "--accent-rose": "#e12726",
      "--border-color": "rgba(200, 32, 32, 0.12)", "--error-color": "#b91c1c",
      "--success-color": "#b89b00", "--warning-color": "#c07818", "--info-color": "#c82020",
      "--accent-color": "#c82020", "--glow-accent": "0 0 20px rgba(200, 32, 32, 0.15)",
    },
  },

  // ── Pair: Porsche (GT Silver / Racing Green) ──
  {
    id: "dark-porsche", name: "Porsche", category: "supercar", mode: "dark", pairId: "porsche",
    preview: { bg: "#08100c", fg: "#d0e0d4", accent: "#2e8b57", secondary: "#142018" },
    vars: {
      "--bg-primary": "#08100c", "--bg-secondary": "#142018", "--bg-tertiary": "#1c2e24", "--bg-elevated": "#243a2e",
      "--text-primary": "#d0e0d4", "--text-secondary": "#6a8a74", "--accent-blue": "#2e8b57", "--accent-green": "#4ade80",
      "--accent-purple": "#58a878", "--accent-gold": "#c8b830", "--accent-rose": "#d06858",
      "--border-color": "rgba(46, 139, 87, 0.10)", "--error-color": "#d06858",
    },
  },
  {
    id: "light-porsche", name: "Porsche", category: "supercar", mode: "light", pairId: "porsche",
    preview: { bg: "#f2f7f4", fg: "#0a1a10", accent: "#1a6b3c", secondary: "#dceee4" },
    vars: {
      "--bg-primary": "#f2f7f4", "--bg-secondary": "#dceee4", "--bg-tertiary": "#c4e0cc", "--bg-elevated": "#ffffff",
      "--text-primary": "#0a1a10", "--text-secondary": "#4a6a54", "--accent-blue": "#1a6b3c", "--accent-green": "#2a9a58",
      "--accent-purple": "#3a7a50", "--accent-gold": "#9a8a10", "--accent-rose": "#a85040",
      "--border-color": "rgba(26, 107, 60, 0.10)", "--error-color": "#a85040",
    },
  },

  // ── Pair: Bugatti (Atlantic Blue / Chiron White) ──
  {
    id: "dark-bugatti", name: "Bugatti", category: "supercar", mode: "dark", pairId: "bugatti",
    preview: { bg: "#040810", fg: "#c0c8e0", accent: "#1e3a8a", secondary: "#0c1428" },
    vars: {
      "--bg-primary": "#040810", "--bg-secondary": "#0c1428", "--bg-tertiary": "#142040", "--bg-elevated": "#1c2850",
      "--text-primary": "#c0c8e0", "--text-secondary": "#5868a0", "--accent-blue": "var(--info-color)", "--accent-green": "#38b2ac",
      "--accent-purple": "#5b6abf", "--accent-gold": "#c0a030", "--accent-rose": "#c84858",
      "--border-color": "rgba(59, 130, 246, 0.10)", "--error-color": "#c84858",
    },
  },
  {
    id: "light-bugatti", name: "Bugatti", category: "supercar", mode: "light", pairId: "bugatti",
    preview: { bg: "#f0f4fc", fg: "#0a1028", accent: "#1e40af", secondary: "#dce4f8" },
    vars: {
      "--bg-primary": "#f0f4fc", "--bg-secondary": "#dce4f8", "--bg-tertiary": "#c4d0f0", "--bg-elevated": "#ffffff",
      "--text-primary": "#0a1028", "--text-secondary": "#4a5888", "--accent-blue": "#1e40af", "--accent-green": "#1a8a80",
      "--accent-purple": "#3a4a98", "--accent-gold": "#987a10", "--accent-rose": "#a03848",
      "--border-color": "rgba(30, 64, 175, 0.10)", "--error-color": "#a03848",
    },
  },

  // ── Pair: Maserati (Blu Sofisticato / Bianco Eldorado) ──
  {
    id: "dark-maserati", name: "Maserati", category: "supercar", mode: "dark", pairId: "maserati",
    preview: { bg: "#080c18", fg: "#d0d4e8", accent: "#4a6fa5", secondary: "#101828" },
    vars: {
      "--bg-primary": "#080c18", "--bg-secondary": "#101828", "--bg-tertiary": "#182438", "--bg-elevated": "#203048",
      "--text-primary": "#d0d4e8", "--text-secondary": "#6878a0", "--accent-blue": "#4a6fa5", "--accent-green": "#50a878",
      "--accent-purple": "#7888b8", "--accent-gold": "#b8a050", "--accent-rose": "#b86068",
      "--border-color": "rgba(74, 111, 165, 0.10)", "--error-color": "#b86068",
    },
  },
  {
    id: "light-maserati", name: "Maserati", category: "supercar", mode: "light", pairId: "maserati",
    preview: { bg: "#f4f6fb", fg: "#0c1020", accent: "#2c5282", secondary: "#e0e6f2" },
    vars: {
      "--bg-primary": "#f4f6fb", "--bg-secondary": "#e0e6f2", "--bg-tertiary": "#ccd4e6", "--bg-elevated": "#ffffff",
      "--text-primary": "#0c1020", "--text-secondary": "#4a5a80", "--accent-blue": "#2c5282", "--accent-green": "#2a8060",
      "--accent-purple": "#4a6098", "--accent-gold": "#8a7820", "--accent-rose": "#984050",
      "--border-color": "rgba(44, 82, 130, 0.10)", "--error-color": "#984050",
    },
  },

  // ── Pair: Robinhood (Fintech Green) ──
  {
    id: "dark-robinhood", name: "Robinhood", category: "standard", mode: "dark", pairId: "robinhood",
    preview: { bg: "#0a0a0a", fg: "#f0f0f0", accent: "#00c805", secondary: "#141414" },
    vars: {
      "--bg-primary": "#0a0a0a", "--bg-secondary": "#141414", "--bg-tertiary": "#1c1c1c", "--bg-elevated": "#242424",
      "--text-primary": "#f0f0f0", "--text-secondary": "#8a8a8a", "--accent-blue": "#00c805", "--accent-green": "#00c805",
      "--accent-purple": "#9c88ff", "--accent-gold": "#f0c040", "--accent-rose": "#ff4d4d",
      "--border-color": "rgba(255, 255, 255, 0.07)", "--error-color": "#ff4d4d",
      "--success-color": "#00c805", "--warning-color": "#f0c040",
    },
  },
  {
    id: "light-robinhood", name: "Robinhood", category: "standard", mode: "light", pairId: "robinhood",
    preview: { bg: "#ffffff", fg: "#1a1a1a", accent: "#00c805", secondary: "#f5f5f5" },
    vars: {
      "--bg-primary": "#ffffff", "--bg-secondary": "#f5f5f5", "--bg-tertiary": "#ebebeb", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a1a1a", "--text-secondary": "#6e6e6e", "--accent-blue": "#00a804", "--accent-green": "#00a804",
      "--accent-purple": "#6c5ce7", "--accent-gold": "#c8960a", "--accent-rose": "#e03030",
      "--border-color": "rgba(0, 0, 0, 0.09)", "--error-color": "#e03030",
      "--success-color": "#00a804", "--warning-color": "#c8960a",
    },
  },

  // ── Pair: Blaze (Red · Black · Gold) ──
  {
    id: "dark-blaze", name: "Blaze", category: "supercar", mode: "dark", pairId: "blaze",
    preview: { bg: "#0c0806", fg: "#f5e8e0", accent: "#e84428", secondary: "#1c1210" },
    vars: {
      "--bg-primary": "#0c0806", "--bg-secondary": "#1c1210", "--bg-tertiary": "#2a1c18", "--bg-elevated": "#362420",
      "--text-primary": "#f5e8e0", "--text-secondary": "#a08878", "--accent-blue": "#e84428", "--accent-green": "#ffe44d",
      "--accent-purple": "#d44030", "--accent-gold": "#ffe44d", "--accent-rose": "#e84428",
      "--border-color": "rgba(232, 68, 40, 0.14)", "--error-color": "#ff5040",
      "--success-color": "#ffe44d", "--warning-color": "#f0a020", "--info-color": "#e84428",
      "--accent-color": "#e84428", "--glow-accent": "0 0 20px rgba(232, 68, 40, 0.25)",
    },
  },
  {
    id: "light-blaze", name: "Blaze", category: "supercar", mode: "light", pairId: "blaze",
    preview: { bg: "#fef8f5", fg: "#1a0c08", accent: "#d44030", secondary: "#f8e8e2" },
    vars: {
      "--bg-primary": "#fef8f5", "--bg-secondary": "#f8e8e2", "--bg-tertiary": "#f0d8cc", "--bg-elevated": "#ffffff",
      "--text-primary": "#1a0c08", "--text-secondary": "#7a5848", "--accent-blue": "#d44030", "--accent-green": "#c8a010",
      "--accent-purple": "#b83020", "--accent-gold": "#c8a010", "--accent-rose": "#d44030",
      "--border-color": "rgba(212, 64, 48, 0.12)", "--error-color": "#b82020",
      "--success-color": "#c8a010", "--warning-color": "#d08818", "--info-color": "#d44030",
      "--accent-color": "#d44030", "--glow-accent": "0 0 20px rgba(212, 64, 48, 0.15)",
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
      // Iteratively adjust until ≥3.1:1 contrast
      const h = sec.replace('#', '');
      let r = parseInt(h.substring(0, 2), 16);
      let g = parseInt(h.substring(2, 4), 16);
      let b = parseInt(h.substring(4, 6), 16);
      for (let i = 0; i < 60; i++) {
        if (t.mode === "dark") {
          r = Math.min(255, r + 3); g = Math.min(255, g + 3); b = Math.min(255, b + 3);
        } else {
          r = Math.max(0, r - 3); g = Math.max(0, g - 3); b = Math.max(0, b - 3);
        }
        const adj = `#${r.toString(16).padStart(2,'0')}${g.toString(16).padStart(2,'0')}${b.toString(16).padStart(2,'0')}`;
        if (contrast(adj, tert) >= 3.1) { t.vars["--text-secondary"] = adj; break; }
      }
    }
  }
  // Fix text-primary on bg-elevated if < 4.5:1
  const pri = t.vars["--text-primary"];
  const elev = t.vars["--bg-elevated"];
  if (pri && elev && !pri.startsWith("rgba") && !elev.startsWith("rgba")) {
    if (contrast(pri, elev) < 4.5) {
      const hp = pri.replace('#', '');
      let r = parseInt(hp.substring(0, 2), 16);
      let g = parseInt(hp.substring(2, 4), 16);
      let b = parseInt(hp.substring(4, 6), 16);
      for (let i = 0; i < 80; i++) {
        if (t.mode === "dark") {
          r = Math.min(255, r + 2); g = Math.min(255, g + 2); b = Math.min(255, b + 2);
        } else {
          r = Math.max(0, r - 2); g = Math.max(0, g - 2); b = Math.max(0, b - 2);
        }
        const adj = `#${r.toString(16).padStart(2,'0')}${g.toString(16).padStart(2,'0')}${b.toString(16).padStart(2,'0')}`;
        if (contrast(adj, elev) >= 4.5) { t.vars["--text-primary"] = adj; break; }
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

/** Union of every CSS var name any theme can set — used to clear stale inline vars on switch */
const ALL_THEME_VAR_KEYS: string[] = Array.from(
  new Set(THEMES.flatMap(t => Object.keys(t.vars)))
);

/** Apply a theme by id — sets CSS vars, localStorage, and notifies Monaco editor */
export function applyThemeById(themeId: string): void {
  const theme = THEMES.find(t => t.id === themeId);
  if (!theme) return;
  const root = document.documentElement;
  // Clear every var any theme could have set so nothing from a previous theme bleeds through
  ALL_THEME_VAR_KEYS.forEach(key => root.style.removeProperty(key));
  localStorage.setItem("vibeui-theme-id", theme.id);
  localStorage.setItem("vibeui-theme", theme.mode);
  root.setAttribute("data-theme", theme.mode);
  for (const [key, value] of Object.entries(theme.vars)) {
    root.style.setProperty(key, value);
  }
  // Notify Monaco editor to update its theme
  window.dispatchEvent(new CustomEvent("vibeui-theme-change", { detail: { themeId: theme.id, mode: theme.mode } }));
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

// labelStyle is now handled via className="panel-label" where simple; kept here for complex overrides
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
const modelsHintStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", margin: "4px 0 0", lineHeight: 1.4, opacity: 0.8 };
const dividerStyle: React.CSSProperties = { height: 1, background: "var(--border-color)", margin: "16px 0" };
const inputStyle: React.CSSProperties = {
  padding: "6px 10px", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)",
  background: "var(--bg-primary)", color: "var(--text-primary)", width: "100%", boxSizing: "border-box",
};

/* ── Section Components ────────────────────────────────────────────── */

function ProfileSection() {
  const [profile, setProfile] = useState<UserProfile>({ displayName: "", email: "", bio: "", avatarUrl: "" });
  const [saved, setSaved] = useState(false);
  const [googleLoading, setGoogleLoading] = useState(false);
  const [googleConnected, setGoogleConnected] = useState(false);
  const [googleError, setGoogleError] = useState<string | null>(null);
  const [googleConfiguring, setGoogleConfiguring] = useState(false);
  const [gClientId, setGClientId] = useState("");
  const [gClientSecret, setGClientSecret] = useState("");
  const [gAuthCode, setGAuthCode] = useState("");
  const [gAwaitingCode, setGAwaitingCode] = useState(false);

  useEffect(() => {
    const stored = localStorage.getItem(STORAGE_KEYS.profile);
    if (stored) setProfile(JSON.parse(stored));
    checkGoogleStatus();
  }, []);

  // Listen for OAuth callback from the temporary local server
  useEffect(() => {
    const unlisten = listen<{ provider: string; code?: string; error?: string }>("oauth-callback", (event) => {
      if (event.payload.provider !== "google") return;
      if (event.payload.code) {
        setGAuthCode(event.payload.code);
        // Auto-complete the OAuth flow
        (async () => {
          setGoogleLoading(true);
          setGoogleError(null);
          try {
            const config = await invoke<{ client_id: string; client_secret: string }>("cloud_oauth_get_client_config", { provider: "google" });
            await invoke<{ provider: string; email: string; display_name: string; connected: boolean }>("cloud_oauth_complete", {
              provider: "google",
              code: event.payload.code,
              clientId: config.client_id,
              clientSecret: config.client_secret || "",
              redirectUri: "http://localhost:7878/oauth/callback",
            });
            setGAwaitingCode(false);
            setGAuthCode("");
            const gProfile = await invoke<{ displayName: string; email: string; avatarUrl: string }>("cloud_oauth_google_profile");
            const updated = {
              ...profile,
              displayName: gProfile.displayName || profile.displayName,
              email: gProfile.email || profile.email,
              avatarUrl: gProfile.avatarUrl || profile.avatarUrl,
            };
            setProfile(updated);
            localStorage.setItem(STORAGE_KEYS.profile, JSON.stringify(updated));
            setGoogleConnected(true);
            setSaved(true);
            setTimeout(() => setSaved(false), 2000);
          } catch (e) {
            setGoogleError(String(e));
          } finally {
            setGoogleLoading(false);
          }
        })();
      } else if (event.payload.error) {
        setGoogleError(`OAuth error: ${event.payload.error}`);
        setGAwaitingCode(false);
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, [profile]);

  const checkGoogleStatus = async () => {
    try {
      const status = await invoke<{ connected: boolean; expired: boolean; email: string }>("cloud_oauth_status", { provider: "google" });
      setGoogleConnected(status.connected && !status.expired);
    } catch { /* not connected */ }
  };

  const save = () => {
    localStorage.setItem(STORAGE_KEYS.profile, JSON.stringify(profile));
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const handleGoogleLogin = async () => {
    setGoogleError(null);
    // First check if already connected — just fetch profile
    try {
      const gProfile = await invoke<{ displayName: string; email: string; avatarUrl: string }>("cloud_oauth_google_profile");
      if (gProfile.email) {
        const updated = {
          ...profile,
          displayName: gProfile.displayName || profile.displayName,
          email: gProfile.email || profile.email,
          avatarUrl: gProfile.avatarUrl || profile.avatarUrl,
        };
        setProfile(updated);
        localStorage.setItem(STORAGE_KEYS.profile, JSON.stringify(updated));
        setGoogleConnected(true);
        setSaved(true);
        setTimeout(() => setSaved(false), 2000);
        return;
      }
    } catch { /* not connected yet, start flow */ }

    // Check if client config exists
    try {
      const config = await invoke<{ client_id: string; client_secret: string }>("cloud_oauth_get_client_config", { provider: "google" });
      if (config.client_id) {
        await startGoogleOAuth(config.client_id);
        return;
      }
    } catch { /* no config */ }

    // Need to configure client credentials first
    setGoogleConfiguring(true);
    setGClientId("");
    setGClientSecret("");
  };

  const startGoogleOAuth = async (clientId: string) => {
    setGoogleLoading(true);
    setGoogleError(null);
    try {
      const redirectUri = "http://localhost:7878/oauth/callback";
      await invoke<string>("cloud_oauth_initiate", {
        provider: "google",
        clientId,
        redirectUri,
      });
      setGAwaitingCode(true);
      setGAuthCode("");
    } catch (e) {
      setGoogleError(String(e));
    } finally {
      setGoogleLoading(false);
    }
  };

  const saveGoogleConfig = async () => {
    if (!gClientId.trim()) { setGoogleError("Client ID is required"); return; }
    setGoogleLoading(true);
    try {
      await invoke("cloud_oauth_save_client_config", {
        provider: "google",
        clientId: gClientId.trim(),
        clientSecret: gClientSecret.trim(),
      });
      setGoogleConfiguring(false);
      await startGoogleOAuth(gClientId.trim());
    } catch (e) {
      setGoogleError(String(e));
    } finally {
      setGoogleLoading(false);
    }
  };

  const completeGoogleOAuth = async () => {
    if (!gAuthCode.trim()) { setGoogleError("Authorization code is required"); return; }
    setGoogleLoading(true);
    setGoogleError(null);
    try {
      const config = await invoke<{ client_id: string; client_secret: string }>("cloud_oauth_get_client_config", { provider: "google" });
      await invoke<{ provider: string; email: string; display_name: string; connected: boolean }>("cloud_oauth_complete", {
        provider: "google",
        code: gAuthCode.trim(),
        clientId: config.client_id,
        clientSecret: config.client_secret || "",
        redirectUri: "http://localhost:7878/oauth/callback",
      });
      setGAwaitingCode(false);
      setGAuthCode("");

      // Now fetch the full Google profile including avatar
      const gProfile = await invoke<{ displayName: string; email: string; avatarUrl: string }>("cloud_oauth_google_profile");
      const updated = {
        ...profile,
        displayName: gProfile.displayName || profile.displayName,
        email: gProfile.email || profile.email,
        avatarUrl: gProfile.avatarUrl || profile.avatarUrl,
      };
      setProfile(updated);
      localStorage.setItem(STORAGE_KEYS.profile, JSON.stringify(updated));
      setGoogleConnected(true);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setGoogleError(String(e));
    } finally {
      setGoogleLoading(false);
    }
  };

  const handleGoogleDisconnect = async () => {
    try {
      await invoke("cloud_oauth_disconnect", { provider: "google" });
      setGoogleConnected(false);
    } catch (e) {
      setGoogleError(String(e));
    }
  };

  const initials = profile.displayName.split(/\s+/).map(w => w[0]?.toUpperCase() || "").join("").slice(0, 2) || "?";

  return (
    <div>
      <h3 style={{ margin: "0 0 16px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>Profile</h3>

      {/* Google Sign-In card */}
      <div style={{
        padding: 14, borderRadius: "var(--radius-md)", marginBottom: 20,
        border: googleConnected ? "1px solid var(--success-color)" : "1px solid var(--border-color)",
        background: googleConnected ? "var(--success-bg)" : "var(--bg-secondary)",
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          {/* Google "G" logo */}
          <div style={{
            width: 40, height: 40, borderRadius: "var(--radius-sm)", background: "var(--bg-elevated)",
            display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0,
            border: "1px solid var(--border-color)",
          }}>
            <svg width="20" height="20" viewBox="0 0 48 48">
              <path fill="#4285F4" d="M24 9.5c3.54 0 6.71 1.22 9.21 3.6l6.85-6.85C35.9 2.38 30.47 0 24 0 14.62 0 6.51 5.38 2.56 13.22l7.98 6.19C12.43 13.72 17.74 9.5 24 9.5z"/>
              <path fill="#34A853" d="M46.98 24.55c0-1.57-.15-3.09-.38-4.55H24v9.02h12.94c-.58 2.96-2.26 5.48-4.78 7.18l7.73 6c4.51-4.18 7.09-10.36 7.09-17.65z"/>
              <path fill="#FBBC05" d="M10.53 28.59c-.48-1.45-.76-2.99-.76-4.59s.27-3.14.76-4.59l-7.98-6.19C.92 16.46 0 20.12 0 24c0 3.88.92 7.54 2.56 10.78l7.97-6.19z"/>
              <path fill="#EA4335" d="M24 48c6.48 0 11.93-2.13 15.89-5.81l-7.73-6c-2.15 1.45-4.92 2.3-8.16 2.3-6.26 0-11.57-4.22-13.47-9.91l-7.98 6.19C6.51 42.62 14.62 48 24 48z"/>
            </svg>
          </div>
          <div style={{ flex: 1 }}>
            <div style={{ fontWeight: 600, fontSize: 13, color: "var(--text-primary)" }}>Sign in with Google</div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
              {googleConnected
                ? `Connected as ${profile.email || "Google user"}`
                : "Auto-fill your profile with your Google account"}
            </div>
          </div>
          <div>
            {googleConnected ? (
              <div style={{ display: "flex", gap: 6 }}>
                <button style={{ ...btnStyle, padding: "5px 12px", fontSize: 11 }} onClick={handleGoogleLogin}>
                  Refresh
                </button>
                <button style={{ ...btnStyle, padding: "5px 12px", fontSize: 11, color: "var(--error-color)" }} onClick={handleGoogleDisconnect}>
                  Disconnect
                </button>
              </div>
            ) : (
              <button
                style={{
                  padding: "8px 20px", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)",
                  background: "var(--bg-elevated)", color: "var(--text-primary)", cursor: "pointer", fontSize: 13, fontWeight: 500,
                  display: "flex", alignItems: "center", gap: 8,
                }}
                onClick={handleGoogleLogin}
                disabled={googleLoading}
              >
                {googleLoading ? "..." : "Sign in with Google"}
              </button>
            )}
          </div>
        </div>

        {googleError && (
          <div style={{ marginTop: 8, padding: "6px 10px", borderRadius: 4, background: "var(--error-bg)", color: "var(--error-color)", fontSize: 11 }}>
            {googleError}
            <button style={{ float: "right", background: "none", border: "none", cursor: "pointer", color: "inherit" }} onClick={() => setGoogleError(null)}>x</button>
          </div>
        )}

        {/* Client credential configuration */}
        {googleConfiguring && (
          <div style={{ marginTop: 10, padding: 10, background: "var(--bg-tertiary)", borderRadius: "var(--radius-sm)" }}>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>
              Enter your Google OAuth client credentials. Create them at{" "}
              <span style={{ color: "var(--accent-blue)" }}>Google Cloud Console &gt; APIs &amp; Services &gt; Credentials</span>.
              Set the redirect URI to <code style={{ fontSize: 10, background: "var(--bg-primary)", padding: "1px 4px", borderRadius: 3 }}>http://localhost:7878/oauth/callback</code>.
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <input style={{ ...inputStyle, fontSize: 12 }} placeholder="Client ID" value={gClientId} onChange={e => setGClientId(e.target.value)} />
              <input style={{ ...inputStyle, fontSize: 12 }} placeholder="Client Secret" type="password" value={gClientSecret} onChange={e => setGClientSecret(e.target.value)} />
              <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
                <button style={{ ...btnStyle, padding: "4px 10px", fontSize: 11 }} onClick={() => setGoogleConfiguring(false)}>Cancel</button>
                <button style={{ ...btnPrimary, padding: "4px 10px", fontSize: 11 }} onClick={saveGoogleConfig} disabled={googleLoading || !gClientId.trim()}>
                  {googleLoading ? "Saving..." : "Save & Connect"}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Authorization code entry */}
        {gAwaitingCode && (
          <div style={{ marginTop: 10, padding: 10, background: "var(--bg-tertiary)", borderRadius: "var(--radius-sm)" }}>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>
              A browser window has opened. After authorizing with Google, paste the authorization code below:
            </div>
            <div style={{ display: "flex", gap: 6 }}>
              <input style={{ ...inputStyle, fontSize: 12, flex: 1 }} placeholder="Paste authorization code here"
                value={gAuthCode} onChange={e => setGAuthCode(e.target.value)}
                onKeyDown={e => e.key === "Enter" && completeGoogleOAuth()} />
              <button style={{ ...btnStyle, padding: "4px 10px", fontSize: 11 }} onClick={() => { setGAwaitingCode(false); setGAuthCode(""); }}>Cancel</button>
              <button style={{ ...btnPrimary, padding: "4px 10px", fontSize: 11 }} onClick={completeGoogleOAuth} disabled={googleLoading || !gAuthCode.trim()}>
                {googleLoading ? "Connecting..." : "Complete"}
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Avatar */}
      <div style={{ display: "flex", alignItems: "center", gap: 16, marginBottom: 20 }}>
        <div style={{
          width: 64, height: 64, borderRadius: "50%", background: "var(--accent-color)",
          display: "flex", alignItems: "center", justifyContent: "center", fontSize: 22, fontWeight: 700, color: "var(--btn-primary-fg)",
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
  const [activeThemeId, setActiveThemeId] = useState("dark-robinhood");
  const [fontSize, setFontSize] = useState(13);
  const [density, setDensity] = useState<"compact" | "normal" | "spacious">("normal");
  const [filterCategory, setFilterCategory] = useState<"all" | "standard" | "high-contrast" | "color-blind" | "supercar">("all");

  useEffect(() => {
    const storedTheme = localStorage.getItem(STORAGE_KEYS.theme) || "dark-robinhood";
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
        {(["all", "standard", "supercar", "high-contrast", "color-blind"] as const).map(cat => (
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
          const pairName = dark?.name || light?.name || pid;
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
  const [providers, setProviders] = useState<OAuthProvider[]>(OAUTH_PROVIDERS);
  const [loading, setLoading] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [configuring, setConfiguring] = useState<string | null>(null);
  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const [authCode, setAuthCode] = useState("");
  const [awaitingCode, setAwaitingCode] = useState<string | null>(null);

  // Load connection status on mount
  useEffect(() => {
    loadStatuses();
  }, []);

  const loadStatuses = async () => {
    try {
      const connected = await invoke<Array<{ provider: string; email: string; display_name: string; expired: boolean }>>("cloud_oauth_list_connected");
      setProviders(prev => prev.map(p => {
        const match = connected.find((c: { provider: string }) => c.provider === p.id);
        if (match) {
          return { ...p, connected: true, email: match.email || match.display_name, displayName: match.display_name, expired: match.expired };
        }
        return { ...p, connected: false, email: undefined, displayName: undefined, expired: undefined };
      }));
    } catch { /* ignore — first run */ }
  };

  const handleConnect = async (id: string) => {
    setError(null);
    // Check if client config exists
    try {
      const config = await invoke<{ client_id: string; client_secret: string }>("cloud_oauth_get_client_config", { provider: id });
      if (!config.client_id) {
        setConfiguring(id);
        setClientId("");
        setClientSecret("");
        return;
      }
      // Start OAuth flow
      await startOAuthFlow(id, config.client_id);
    } catch (e) {
      setConfiguring(id);
      setClientId("");
      setClientSecret("");
    }
  };

  const startOAuthFlow = async (id: string, cId: string) => {
    setLoading(id);
    setError(null);
    try {
      const redirectUri = "http://localhost:7878/oauth/callback";
      await invoke<string>("cloud_oauth_initiate", {
        provider: id,
        clientId: cId,
        redirectUri,
      });
      // Browser opened — now wait for auth code
      setAwaitingCode(id);
      setAuthCode("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(null);
    }
  };

  const completeOAuth = async (id: string) => {
    if (!authCode.trim()) { setError("Authorization code is required"); return; }
    setLoading(id);
    setError(null);
    try {
      const config = await invoke<{ client_id: string; client_secret: string }>("cloud_oauth_get_client_config", { provider: id });
      const result = await invoke<{ provider: string; email: string; display_name: string; connected: boolean }>("cloud_oauth_complete", {
        provider: id,
        code: authCode.trim(),
        clientId: config.client_id,
        clientSecret: config.client_secret || "",
        redirectUri: "http://localhost:7878/oauth/callback",
      });
      setProviders(prev => prev.map(p =>
        p.id === id ? { ...p, connected: true, email: result.email || result.display_name, displayName: result.display_name, expired: false } : p
      ));
      setAwaitingCode(null);
      setAuthCode("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(null);
    }
  };

  const saveClientConfig = async (id: string) => {
    if (!clientId.trim()) { setError("Client ID is required"); return; }
    setLoading(id);
    try {
      await invoke("cloud_oauth_save_client_config", {
        provider: id,
        clientId: clientId.trim(),
        clientSecret: clientSecret.trim(),
      });
      setConfiguring(null);
      // Now start the OAuth flow
      await startOAuthFlow(id, clientId.trim());
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(null);
    }
  };

  const handleDisconnect = async (id: string) => {
    setLoading(id);
    try {
      await invoke("cloud_oauth_disconnect", { provider: id });
      setProviders(prev => prev.map(p => p.id === id ? { ...p, connected: false, email: undefined, displayName: undefined, expired: undefined } : p));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(null);
    }
  };

  const handleRefresh = async (id: string) => {
    setLoading(id);
    setError(null);
    try {
      const config = await invoke<{ client_id: string; client_secret: string }>("cloud_oauth_get_client_config", { provider: id });
      await invoke("cloud_oauth_refresh", {
        provider: id,
        clientId: config.client_id,
        clientSecret: config.client_secret || "",
      });
      await loadStatuses();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(null);
    }
  };

  const providerColors: Record<string, string> = {
    google: "#4285f4", github: "var(--text-secondary)", gitlab: "#fc6d26",
    bitbucket: "#0052cc", microsoft: "#00a4ef", apple: "var(--text-primary)",
  };

  return (
    <div>
      <h3 style={{ margin: "0 0 6px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>Cloud OAuth</h3>
      <p style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.5 }}>
        Connect your cloud accounts via OAuth 2.0 for scanning, IAM, IaC generation, and cost analysis.
        You'll need to register an OAuth app with each provider and enter your client credentials.
      </p>

      {error && (
        <div className="panel-error" style={{ marginBottom: 12 }}>
          <span>{error}</span>
          <button aria-label="Dismiss error" style={{ float: "right", background: "none", border: "none", cursor: "pointer", color: "inherit" }} onClick={() => setError(null)}>×</button>
        </div>
      )}

      {providers.map(p => (
        <div key={p.id} className="panel-card" style={{ marginBottom: 8 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
            <div style={{
              width: 36, height: 36, borderRadius: "var(--radius-sm)", background: providerColors[p.id] || "var(--bg-tertiary)",
              display: "flex", alignItems: "center", justifyContent: "center", color: "var(--btn-primary-fg)", fontSize: 12, fontWeight: 700, flexShrink: 0,
            }}>
              {p.icon}
            </div>
            <div style={{ flex: 1 }}>
              <div style={{ fontWeight: 600, fontSize: 13, color: "var(--text-primary)" }}>{p.name}</div>
              {p.connected ? (
                <div style={{ fontSize: 11 }}>
                  <span style={{ color: p.expired ? "var(--warning-color)" : "var(--success-color)" }}>
                    {p.expired ? "Token expired" : "Connected"}
                  </span>
                  {p.email && <span style={{ color: "var(--text-secondary)", marginLeft: 6 }}>{p.email}</span>}
                </div>
              ) : (
                <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Not connected</div>
              )}
            </div>
            <div style={{ display: "flex", gap: 6 }}>
              {p.connected && p.expired && (
                <button style={{ ...btnStyle, padding: "5px 10px", fontSize: 11, color: "var(--warning-color)" }}
                  onClick={() => handleRefresh(p.id)} disabled={loading === p.id}>
                  Refresh
                </button>
              )}
              {p.connected ? (
                <button style={{ ...btnStyle, padding: "5px 12px", fontSize: 11, color: "var(--error-color)" }}
                  onClick={() => handleDisconnect(p.id)} disabled={loading === p.id}>
                  Disconnect
                </button>
              ) : (
                <button style={{ ...btnPrimary, padding: "5px 12px", fontSize: 11 }}
                  onClick={() => handleConnect(p.id)} disabled={loading === p.id}>
                  {loading === p.id ? "..." : "Connect"}
                </button>
              )}
            </div>
          </div>

          {/* Client credential configuration form */}
          {configuring === p.id && (
            <div style={{ marginTop: 10, padding: 10, background: "var(--bg-tertiary)", borderRadius: "var(--radius-sm)" }}>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>
                Enter your OAuth app credentials for {p.name}. Register an app at the provider's developer console.
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                <input style={{ ...inputStyle, fontSize: 12 }} placeholder="Client ID" value={clientId} onChange={e => setClientId(e.target.value)} />
                <input style={{ ...inputStyle, fontSize: 12 }} placeholder="Client Secret (optional for some providers)" type="password"
                  value={clientSecret} onChange={e => setClientSecret(e.target.value)} />
                <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
                  <button style={{ ...btnStyle, padding: "4px 10px", fontSize: 11 }} onClick={() => setConfiguring(null)}>Cancel</button>
                  <button style={{ ...btnPrimary, padding: "4px 10px", fontSize: 11 }}
                    onClick={() => saveClientConfig(p.id)} disabled={loading === p.id || !clientId.trim()}>
                    {loading === p.id ? "Saving..." : "Save & Connect"}
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* Authorization code entry (after browser redirect) */}
          {awaitingCode === p.id && (
            <div style={{ marginTop: 10, padding: 10, background: "var(--bg-tertiary)", borderRadius: "var(--radius-sm)" }}>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>
                A browser window has opened. After authorizing, paste the authorization code below:
              </div>
              <div style={{ display: "flex", gap: 6 }}>
                <input style={{ ...inputStyle, fontSize: 12, flex: 1 }} placeholder="Paste authorization code here"
                  value={authCode} onChange={e => setAuthCode(e.target.value)} />
                <button style={{ ...btnStyle, padding: "4px 10px", fontSize: 11 }} onClick={() => { setAwaitingCode(null); setAuthCode(""); }}>Cancel</button>
                <button style={{ ...btnPrimary, padding: "4px 10px", fontSize: 11 }}
                  onClick={() => completeOAuth(p.id)} disabled={loading === p.id || !authCode.trim()}>
                  {loading === p.id ? "Connecting..." : "Complete"}
                </button>
              </div>
            </div>
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
      theme: localStorage.getItem(STORAGE_KEYS.theme) || "dark-robinhood",
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
      applyThemeById(theme.id);
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
          <div key={c.id} className="panel-card" style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
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

interface ApiKeyValidation {
  provider: string;
  valid: boolean;
  error: string | null;
  latency_ms: number;
}

function ApiKeysSection() {
  const [settings, setSettings] = useState<ApiKeySettings>({
    anthropic_api_key: "", openai_api_key: "", gemini_api_key: "", grok_api_key: "", groq_api_key: "",
    openrouter_api_key: "", azure_openai_api_key: "", azure_openai_api_url: "",
    mistral_api_key: "", cerebras_api_key: "", deepseek_api_key: "", zhipu_api_key: "",
    vercel_ai_api_key: "", vercel_ai_api_url: "", minimax_api_key: "", perplexity_api_key: "",
    together_api_key: "", fireworks_api_key: "", sambanova_api_key: "",
    ollama_api_key: "", ollama_api_url: "",
    claude_model: "claude-3-5-sonnet-latest", openai_model: "gpt-4o", openrouter_model: "",
  });
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const [showKey, setShowKey] = useState<Record<string, boolean>>({});
  const [validations, setValidations] = useState<Record<string, ApiKeyValidation>>({});
  const [validating, setValidating] = useState<Record<string, boolean>>({});
  const loadedRef = useRef(false);
  const autoSaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Load keys on mount
  useEffect(() => {
    let cancelled = false;
    invoke<ApiKeySettings>("get_provider_api_keys")
      .then(s => {
        if (!cancelled) {
          setSettings(s);
          // Mark as loaded after a tick so the auto-save effect skips the initial set
          setTimeout(() => { loadedRef.current = true; }, 0);
        }
      })
      .catch(() => { loadedRef.current = true; });
    return () => { cancelled = true; };
  }, []);

  // Auto-save: debounce 1s after any settings change (skip the initial load)
  useEffect(() => {
    if (!loadedRef.current) return;
    if (autoSaveTimerRef.current) clearTimeout(autoSaveTimerRef.current);
    autoSaveTimerRef.current = setTimeout(async () => {
      try {
        const providers = await invoke<string[]>("save_provider_api_keys", { settings });
        window.dispatchEvent(new CustomEvent("vibeui:providers-updated", { detail: providers }));
        setMessage({ type: "success", text: `Auto-saved. ${providers.length} model(s) available.` });
      } catch (e) {
        setMessage({ type: "error", text: String(e) });
      }
    }, 1000);
    return () => { if (autoSaveTimerRef.current) clearTimeout(autoSaveTimerRef.current); };
  }, [settings]);

  // Listen for app-level validation events from useApiKeyMonitor (runs in App.tsx)
  useEffect(() => {
    const onValidations = (e: Event) => {
      const map = (e as CustomEvent<Record<string, ApiKeyValidation>>).detail;
      setValidations(map);
    };
    window.addEventListener("vibeui:api-key-validations", onValidations);

    // Also trigger an initial validation via Tauri for immediate feedback when panel opens
    invoke<ApiKeyValidation[]>("validate_all_api_keys")
      .then(results => {
        const map: Record<string, ApiKeyValidation> = {};
        results.forEach(r => { map[r.provider] = r; });
        setValidations(map);
      })
      .catch(() => {});

    return () => window.removeEventListener("vibeui:api-key-validations", onValidations);
  }, []);

  const validateSingle = async (provider: string, key: string, url?: string) => {
    setValidating(prev => ({ ...prev, [provider]: true }));
    try {
      const result = await invoke<ApiKeyValidation>("validate_api_key", {
        provider, apiKey: key, apiUrl: url ?? null,
      });
      setValidations(prev => ({ ...prev, [provider]: result }));
    } catch {
      setValidations(prev => ({
        ...prev,
        [provider]: { provider, valid: false, error: "Validation failed", latency_ms: 0 },
      }));
    } finally {
      setValidating(prev => ({ ...prev, [provider]: false }));
    }
  };

  const handleSave = async () => {
    setSaving(true); setMessage(null);
    try {
      const providers = await invoke<string[]>("save_provider_api_keys", { settings });
      // Emit a custom event so App.tsx can refresh its provider dropdown
      window.dispatchEvent(new CustomEvent("vibeui:providers-updated", { detail: providers }));
      setMessage({ type: "success", text: `Saved. ${providers.length} model(s) available.` });
    } catch (e) {
      setMessage({ type: "error", text: String(e) });
    } finally { setSaving(false); }
  };

  const renderSecretField = (label: string, fieldKey: keyof ApiKeySettings, placeholder: string, provider?: string) => {
    const v = provider ? validations[provider] : undefined;
    const isValidating = provider ? validating[provider] : false;
    return (
      <div style={{ marginBottom: 12 }}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <label style={labelStyle}>{label}</label>
          {v && (
            <span style={{
              fontSize: 10, fontWeight: 600, display: "inline-flex", alignItems: "center", gap: 4,
              color: v.valid ? "var(--accent-green)" : (v.error === "No key configured" ? "var(--text-secondary)" : "var(--accent-rose)"),
            }}>
              {v.valid
                ? <><CheckCircle size={10} strokeWidth={2} /> OK ({v.latency_ms}ms)</>
                : v.error === "No key configured" && provider !== "ollama"
                  ? <><MinusCircle size={10} strokeWidth={2} /> Not set</>
                  : v.error === "No key configured" && provider === "ollama"
                    ? <><MinusCircle size={10} strokeWidth={2} /> Using device key</>
                    : <><AlertCircle size={10} strokeWidth={2} /> {v.error}</>
              }
            </span>
          )}
        </div>
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
          {provider && (settings[fieldKey] || provider === "ollama") && (
            <button
              onClick={() => validateSingle(provider, settings[fieldKey] || "", provider === "ollama" ? settings.ollama_api_url : provider === "azure_openai" ? settings.azure_openai_api_url : provider === "vercel_ai" ? settings.vercel_ai_api_url : undefined)}
              disabled={isValidating}
              style={{ ...btnStyle, padding: "4px 8px", display: "flex", alignItems: "center", gap: 4, fontSize: 11, opacity: isValidating ? 0.5 : 1 }}
            >
              {isValidating ? <Loader2 size={12} strokeWidth={2} className="spin" /> : <Zap size={12} strokeWidth={2} />}
              Test
            </button>
          )}
        </div>
      </div>
    );
  };

  const renderSectionHeader = (title: string) => (
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
        {renderSectionHeader("Anthropic (Claude)")}
        {renderSecretField("API Key", "anthropic_api_key", "sk-ant-api03-...", "anthropic")}
        <p style={modelsHintStyle}>
          Models: Opus 4.6, Sonnet 4.6, Haiku 4.5, 3.5 Sonnet, 3.5 Haiku, 3 Opus
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("OpenAI")}
        {renderSecretField("API Key", "openai_api_key", "sk-proj-...", "openai")}
        <p style={modelsHintStyle}>
          Models: GPT-4o, GPT-4o mini, GPT-4 Turbo, GPT-4, GPT-3.5 Turbo, o1, o1-mini, o1-preview, o3-mini
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Google (Gemini)")}
        {renderSecretField("API Key", "gemini_api_key", "AIzaSy...", "gemini")}
        <p style={modelsHintStyle}>
          Models: 2.5 Pro, 2.5 Flash, 2.0 Flash, 2.0 Flash Lite, 1.5 Pro, 1.5 Flash
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("xAI (Grok)")}
        {renderSecretField("API Key", "grok_api_key", "xai-...", "grok")}
        <p style={modelsHintStyle}>
          Models: Grok-3, Grok-3 mini, Grok-2, Grok-2 mini
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Groq")}
        {renderSecretField("API Key", "groq_api_key", "gsk_...", "groq")}
        <p style={modelsHintStyle}>
          Models: Llama 3.3 70B, Llama 3.1 8B, Mixtral 8x7B, Gemma 2 9B
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("OpenRouter")}
        {renderSecretField("API Key", "openrouter_api_key", "sk-or-v1-...", "openrouter")}
        <div style={{ marginBottom: 12 }}>
          <label style={labelStyle}>Model</label>
          <input style={fieldStyle} value={settings.openrouter_model} onChange={e => setSettings({ ...settings, openrouter_model: e.target.value })} placeholder="anthropic/claude-3.5-sonnet" />
        </div>
        <p style={modelsHintStyle}>
          Routes to 200+ models. Enter a model ID or browse at openrouter.ai/models
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Azure OpenAI")}
        {renderSecretField("API Key", "azure_openai_api_key", "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", "azure_openai")}
        <div style={{ marginBottom: 12 }}>
          <label style={labelStyle}>Endpoint URL</label>
          <input style={fieldStyle} value={settings.azure_openai_api_url} onChange={e => setSettings({ ...settings, azure_openai_api_url: e.target.value })} placeholder="https://your-resource.openai.azure.com" />
        </div>
        <p style={modelsHintStyle}>
          Models: GPT-4o, GPT-4 Turbo, GPT-3.5 Turbo (via your Azure deployment)
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Mistral AI")}
        {renderSecretField("API Key", "mistral_api_key", "...", "mistral")}
        <p style={modelsHintStyle}>
          Models: Mistral Large, Mistral Medium, Mistral Small, Codestral, Mistral Nemo
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Cerebras")}
        {renderSecretField("API Key", "cerebras_api_key", "csk-...", "cerebras")}
        <p style={modelsHintStyle}>
          Models: Llama 3.3 70B, Llama 3.1 8B (ultra-fast inference)
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("DeepSeek")}
        {renderSecretField("API Key", "deepseek_api_key", "sk-...", "deepseek")}
        <p style={modelsHintStyle}>
          Models: DeepSeek-V3, DeepSeek-Coder, DeepSeek-R1
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Zhipu (GLM)")}
        {renderSecretField("API Key", "zhipu_api_key", "id.secret", "zhipu")}
        <p style={modelsHintStyle}>
          Models: GLM-4, GLM-4V, GLM-3 Turbo (format: &quot;id.secret&quot;)
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Vercel AI Gateway")}
        {renderSecretField("API Key", "vercel_ai_api_key", "...", "vercel_ai")}
        <div style={{ marginBottom: 12 }}>
          <label style={labelStyle}>Gateway URL</label>
          <input style={fieldStyle} value={settings.vercel_ai_api_url} onChange={e => setSettings({ ...settings, vercel_ai_api_url: e.target.value })} placeholder="https://gateway.vercel.ai/v1" />
        </div>
        <p style={modelsHintStyle}>
          Unified proxy to multiple providers. Requires API key and gateway URL.
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("MiniMax")}
        {renderSecretField("API Key", "minimax_api_key", "...", "minimax")}
        <p style={modelsHintStyle}>
          Models: abab6.5, abab6, abab5.5
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Perplexity")}
        {renderSecretField("API Key", "perplexity_api_key", "pplx-...", "perplexity")}
        <p style={modelsHintStyle}>
          Models: Sonar Pro, Sonar, Sonar Deep Research (search-augmented AI)
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Together AI")}
        {renderSecretField("API Key", "together_api_key", "...", "together")}
        <p style={modelsHintStyle}>
          Models: Llama 3.3 70B, Mixtral 8x22B, Qwen 2.5, CodeLlama (open model hosting)
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("Fireworks AI")}
        {renderSecretField("API Key", "fireworks_api_key", "fw_...", "fireworks")}
        <p style={modelsHintStyle}>
          Models: Llama 3.3 70B, Mixtral MoE, FireFunction v2 (fast inference)
        </p>
      </div>

      <div style={{ marginBottom: 20 }}>
        {renderSectionHeader("SambaNova")}
        {renderSecretField("API Key", "sambanova_api_key", "...", "sambanova")}
        <p style={modelsHintStyle}>
          Models: Llama 3.3 70B, Llama 3.1 405B (fast inference on custom silicon)
        </p>
      </div>

      <button style={{ ...btnPrimary, width: "100%" }} onClick={handleSave} disabled={saving}>
        {saving ? "Saving..." : "Save & Apply"}
      </button>

      {message && (
        message.type === "error"
          ? <div className="panel-error" style={{ marginTop: 12 }}><span>{message.text}</span></div>
          : <div style={{ marginTop: 12, padding: "8px 10px", borderRadius: "var(--radius-sm)", fontSize: 12, background: "var(--success-bg)", color: "var(--success-color)", border: "1px solid var(--success-color)" }}>
              OK {message.text}
            </div>
      )}

      <div style={{ marginTop: 24, borderTop: "1px solid var(--border-color)", paddingTop: 16 }}>
        {renderSectionHeader("Local Models (Ollama)")}
        {renderSecretField("API Key", "ollama_api_key", "Optional — leave empty to use device key", "ollama")}
        <div style={{ marginBottom: 12 }}>
          <label style={labelStyle}>API URL</label>
          <input style={fieldStyle} value={settings.ollama_api_url} onChange={e => setSettings({ ...settings, ollama_api_url: e.target.value })} placeholder="http://localhost:11434" />
        </div>
        <p style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.5 }}>
          If no API key is set, a device key derived from your hostname and username is used automatically. Set a key when connecting to a remote or secured Ollama instance.
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
      display: "flex", flex: 1, minHeight: 0, background: "var(--bg-primary)", color: "var(--text-primary)",
      borderRadius: "var(--radius-lg)", overflow: "hidden", border: "1px solid var(--border-color)",
      boxShadow: "var(--elevation-3)",
    }}>
      {/* Sidebar nav */}
      <div style={{
        width: 200, background: "var(--bg-secondary)", borderRight: "1px solid var(--border-color)",
        display: "flex", flexDirection: "column", padding: "12px 8px",
      }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "4px 8px", marginBottom: 12 }}>
          <span style={{ fontWeight: 700, fontSize: 14, color: "var(--accent-color)" }}>Settings</span>
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
