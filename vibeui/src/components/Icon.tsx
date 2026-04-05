/**
 * Icon — Thin, themeable SVG icon system for VibeUI.
 *
 * All icons are 24×24 viewBox, stroke-only (fill: none), using currentColor so
 * they respond to any CSS `color` or `var(--icon-color)` on the parent element.
 * Stroke width defaults to the CSS variable `--icon-stroke` (set per-theme in
 * App.css), and can be overridden per-instance with the `strokeWidth` prop.
 */

import React from "react";

// ── Types ─────────────────────────────────────────────────────────────────────

export type IconName =
  // Activity bar / sidebar
  | "files" | "search" | "git-branch" | "test-tube" | "clipboard-list"
  | "hammer" | "bot" | "shield" | "terminal" | "settings" | "menu"
  // Toolbar
  | "folder-open" | "folder-plus" | "file-plus" | "save" | "panel-left"
  | "message-square" | "play" | "eye" | "eye-off" | "file-text" | "file-code"
  | "globe" | "image" | "layout-grid" | "graduation-cap" | "sparkles"
  | "rocket" | "plug" | "hand" | "puzzle" | "monitor-play"
  // Source control
  | "git-pull-request" | "git-commit" | "users" | "user"
  // Infrastructure
  | "container" | "refresh-cw" | "cloud-cog" | "workflow" | "cpu"
  | "database" | "radio" | "cog" | "terminal-square" | "wrench"
  // Toolkit
  | "binary" | "regex" | "pen-tool" | "user-cog" | "dollar-sign" | "package"
  // AI / Project
  | "store" | "factory" | "infinity" | "swords" | "users-round" | "brain"
  | "ruler" | "palette" | "trending-up" | "activity" | "cloud-upload"
  | "cpu-chip"
  // File type icons
  | "atom" | "coffee" | "gem" | "braces" | "archive" | "paintbrush"
  | "book-open" | "file" | "image-file" | "code2"
  // Misc UI
  | "x" | "check" | "chevron-right" | "chevron-down" | "chevron-up"
  | "chevron-left" | "plus" | "minus" | "external-link" | "copy"
  | "trash" | "edit" | "info" | "alert-triangle" | "alert-circle"
  | "circle-check" | "loader" | "lock" | "unlock" | "key"
  | "moon" | "sun" | "arrow-up" | "arrow-down" | "arrow-right" | "arrow-left"
  | "panel-right" | "maximize" | "minimize" | "sidebar" | "layers"
  | "folder" | "git-graph" | "sparkle" | "zap" | "send" | "mic"
  | "stop-circle" | "pause" | "skip-forward" | "list" | "grid"
  | "rotate-ccw" | "download" | "upload" | "link" | "unlink"
  | "bell" | "bell-off" | "star" | "heart" | "bookmark"
  | "filter" | "sort-asc" | "sort-desc" | "expand" | "compress"
  | "split" | "merge" | "diff" | "compass" | "map-pin"
  | "network" | "wifi" | "bluetooth" | "usb" | "server"
  | "microscope" | "flask" | "chart-bar" | "chart-line" | "pie-chart";

export interface IconProps {
  name: IconName;
  size?: number;
  strokeWidth?: number | string;
  color?: string;
  className?: string;
  style?: React.CSSProperties;
  title?: string;
  "aria-label"?: string;
  "aria-hidden"?: boolean | "true" | "false";
}

// ── SVG path definitions ──────────────────────────────────────────────────────
// Each entry is either a string (single <path>) or an array of element tuples.
// Tuple format: ["path"|"circle"|"line"|"polyline"|"rect"|"ellipse", attrs]

type PathDef = string | Array<[string, Record<string, string | number>]>;

const PATHS: Record<IconName, PathDef> = {
  // ── Activity bar ──────────────────────────────────────────────────────────

  // Two overlapping document pages
  files: [
    ["path", { d: "M16 4H8a2 2 0 00-2 2v14a2 2 0 002 2h10a2 2 0 002-2V8z" }],
    ["path", { d: "M16 4v4h4" }],
    ["path", { d: "M7 6H5a1 1 0 00-1 1v13a1 1 0 001 1h9" }],
    ["line", { x1: "10", y1: "13", x2: "16", y2: "13" }],
    ["line", { x1: "10", y1: "16", x2: "14", y2: "16" }],
  ],

  // Magnifying glass
  search: [
    ["circle", { cx: "11", cy: "11", r: "7" }],
    ["line", { x1: "20", y1: "20", x2: "16", y2: "16" }],
  ],

  // Git branch: two nodes + connecting line + dot for HEAD
  "git-branch": [
    ["circle", { cx: "6", cy: "6", r: "2" }],
    ["circle", { cx: "18", cy: "6", r: "2" }],
    ["circle", { cx: "6", cy: "18", r: "2" }],
    ["path", { d: "M6 8v8M8 6h6a4 4 0 014 4v0" }],
  ],

  // Test tube tilted
  "test-tube": [
    ["path", { d: "M9 3h6M9 3v11a4 4 0 008 0V3M7 9h10" }],
  ],

  // Clipboard with list lines
  "clipboard-list": [
    ["path", { d: "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2" }],
    ["path", { d: "M9 5a2 2 0 114 0H9zM9 12h6M9 15h4" }],
  ],

  // Hammer
  hammer: [
    ["path", { d: "M15 5l4 4-10 10-4-4z" }],
    ["path", { d: "M15 5l2-2a2 2 0 013 3l-2 2" }],
    ["path", { d: "M5 19l-2 2" }],
  ],

  // Bot / robot face
  bot: [
    ["rect", { x: "3", y: "8", width: "18", height: "12", rx: "2" }],
    ["path", { d: "M12 3v5M8 12h1M15 12h1M9 16s.5 1 3 1 3-1 3-1" }],
    ["path", { d: "M8 3h8M3 14H2M22 14h-1" }],
  ],

  // Shield
  shield: [
    ["path", { d: "M12 3L4 7v5c0 5 3.5 9.5 8 11 4.5-1.5 8-6 8-11V7z" }],
  ],

  // Terminal / prompt
  terminal: [
    ["polyline", { points: "4 17 10 11 4 5" }],
    ["line", { x1: "12", y1: "19", x2: "20", y2: "19" }],
  ],

  // Settings / gear
  settings: [
    ["circle", { cx: "12", cy: "12", r: "3" }],
    ["path", { d: "M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z" }],
  ],

  // Hamburger menu
  menu: [
    ["line", { x1: "3", y1: "6", x2: "21", y2: "6" }],
    ["line", { x1: "3", y1: "12", x2: "21", y2: "12" }],
    ["line", { x1: "3", y1: "18", x2: "21", y2: "18" }],
  ],

  // ── Toolbar ───────────────────────────────────────────────────────────────

  // Folder (open)
  "folder-open": [
    ["path", { d: "M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V7a2 2 0 012-2h5l2 2h9a2 2 0 012 2v1" }],
    ["path", { d: "M2 11h20" }],
  ],

  // Folder + plus
  "folder-plus": [
    ["path", { d: "M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 2h9a2 2 0 012 2z" }],
    ["line", { x1: "12", y1: "11", x2: "12", y2: "17" }],
    ["line", { x1: "9", y1: "14", x2: "15", y2: "14" }],
  ],

  // File + plus
  "file-plus": [
    ["path", { d: "M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" }],
    ["path", { d: "M14 2v6h6" }],
    ["line", { x1: "12", y1: "18", x2: "12", y2: "12" }],
    ["line", { x1: "9", y1: "15", x2: "15", y2: "15" }],
  ],

  // Floppy disk
  save: [
    ["path", { d: "M19 21H5a2 2 0 01-2-2V5a2 2 0 012-2h11l5 5v11a2 2 0 01-2 2z" }],
    ["polyline", { points: "17 21 17 13 7 13 7 21" }],
    ["polyline", { points: "7 3 7 8 15 8" }],
  ],

  // Panel left / sidebar toggle
  "panel-left": [
    ["rect", { x: "3", y: "3", width: "18", height: "18", rx: "2" }],
    ["line", { x1: "9", y1: "3", x2: "9", y2: "21" }],
  ],

  "panel-right": [
    ["rect", { x: "3", y: "3", width: "18", height: "18", rx: "2" }],
    ["line", { x1: "15", y1: "3", x2: "15", y2: "21" }],
  ],

  // Chat bubble
  "message-square": [
    ["path", { d: "M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z" }],
  ],

  // Play triangle
  play: [
    ["polygon", { points: "5 3 19 12 5 21 5 3" }],
  ],

  // Eye
  eye: [
    ["path", { d: "M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" }],
    ["circle", { cx: "12", cy: "12", r: "3" }],
  ],

  "eye-off": [
    ["path", { d: "M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19m-6.72-1.07a3 3 0 11-4.24-4.24" }],
    ["line", { x1: "1", y1: "1", x2: "23", y2: "23" }],
  ],

  // File with text lines
  "file-text": [
    ["path", { d: "M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" }],
    ["path", { d: "M14 2v6h6" }],
    ["line", { x1: "16", y1: "13", x2: "8", y2: "13" }],
    ["line", { x1: "16", y1: "17", x2: "8", y2: "17" }],
    ["line", { x1: "10", y1: "9", x2: "8", y2: "9" }],
  ],

  // File with code brackets
  "file-code": [
    ["path", { d: "M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" }],
    ["path", { d: "M14 2v6h6" }],
    ["path", { d: "M10 13l-2 2 2 2M14 13l2 2-2 2" }],
  ],

  // Globe
  globe: [
    ["circle", { cx: "12", cy: "12", r: "10" }],
    ["line", { x1: "2", y1: "12", x2: "22", y2: "12" }],
    ["path", { d: "M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z" }],
  ],

  // Image / picture frame
  image: [
    ["rect", { x: "3", y: "3", width: "18", height: "18", rx: "2" }],
    ["circle", { cx: "8.5", cy: "8.5", r: "1.5" }],
    ["polyline", { points: "21 15 16 10 5 21" }],
  ],

  "image-file": [
    ["path", { d: "M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" }],
    ["path", { d: "M14 2v6h6" }],
    ["circle", { cx: "9.5", cy: "13.5", r: "1.5" }],
    ["polyline", { points: "18 18 14.5 14 10 18" }],
  ],

  // 2×2 layout grid
  "layout-grid": [
    ["rect", { x: "3", y: "3", width: "7", height: "7", rx: "1" }],
    ["rect", { x: "14", y: "3", width: "7", height: "7", rx: "1" }],
    ["rect", { x: "3", y: "14", width: "7", height: "7", rx: "1" }],
    ["rect", { x: "14", y: "14", width: "7", height: "7", rx: "1" }],
  ],

  // Graduation cap
  "graduation-cap": [
    ["polygon", { points: "12 2 22 8.5 12 15 2 8.5 12 2" }],
    ["line", { x1: "12", y1: "15", x2: "12", y2: "22" }],
    ["path", { d: "M20 11v7" }],
    ["path", { d: "M7 13.5V18a5 5 0 005 4 5 5 0 005-4v-4.5" }],
  ],

  // Sparkles (three stars)
  sparkles: [
    ["path", { d: "M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3z" }],
    ["path", { d: "M19 3l.8 2.2 2.2.8-2.2.8L19 9l-.8-2.2L16 6l2.2-.8L19 3z" }],
    ["path", { d: "M5 17l.6 1.6 1.6.6-1.6.6L5 21l-.6-1.6L2.8 18.8l1.6-.6L5 17z" }],
  ],

  sparkle: "M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3z",

  // Rocket
  rocket: [
    ["path", { d: "M13 2L3 14h9l-1 8 10-12h-9l1-8z" }],
  ],

  // Electrical plug
  plug: [
    ["path", { d: "M12 22v-5M9 8V2M15 8V2M7 8h10a4 4 0 010 8H7a4 4 0 010-8z" }],
  ],

  // Hand / cursor
  hand: [
    ["path", { d: "M18 11V7a2 2 0 00-4 0M14 11V5a2 2 0 00-4 0M10 11V7a2 2 0 00-4 0v5" }],
    ["path", { d: "M6 12v4a6 6 0 006 6h2a6 6 0 006-6v-5a2 2 0 00-4 0v2" }],
  ],

  // Puzzle piece
  puzzle: [
    ["path", { d: "M20.29 8.29L16 12.58V8a2 2 0 00-2-2h-4.58l4.29-4.29a1 1 0 011.41 0l5.17 5.17a1 1 0 010 1.41z" }],
    ["path", { d: "M15.71 15.71L12 12l-3.71 3.71A1 1 0 009 17h5l-4 5a1 1 0 001.41 1.41l4-4 4 4A1 1 0 0021 22l-4-5h2a1 1 0 00.71-1.29z" }],
  ],

  // Monitor with play button
  "monitor-play": [
    ["rect", { x: "2", y: "3", width: "20", height: "14", rx: "2" }],
    ["line", { x1: "8", y1: "21", x2: "16", y2: "21" }],
    ["line", { x1: "12", y1: "17", x2: "12", y2: "21" }],
    ["polygon", { points: "9 8 15 11 9 14 9 8" }],
  ],

  // ── Source control ────────────────────────────────────────────────────────

  "git-pull-request": [
    ["circle", { cx: "6", cy: "18", r: "2" }],
    ["circle", { cx: "18", cy: "18", r: "2" }],
    ["circle", { cx: "6", cy: "6", r: "2" }],
    ["path", { d: "M6 8v8M18 16V8a4 4 0 00-4-4h-2" }],
    ["polyline", { points: "10 4 8 6 10 8" }],
  ],

  "git-commit": [
    ["circle", { cx: "12", cy: "12", r: "3" }],
    ["line", { x1: "3", y1: "12", x2: "9", y2: "12" }],
    ["line", { x1: "15", y1: "12", x2: "21", y2: "12" }],
  ],

  "git-graph": [
    ["circle", { cx: "5", cy: "6", r: "2" }],
    ["circle", { cx: "19", cy: "6", r: "2" }],
    ["circle", { cx: "5", cy: "18", r: "2" }],
    ["path", { d: "M5 8v8M7 6h8a4 4 0 014 4v0" }],
  ],

  users: [
    ["path", { d: "M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2" }],
    ["circle", { cx: "9", cy: "7", r: "4" }],
    ["path", { d: "M23 21v-2a4 4 0 00-3-3.87M16 3.13a4 4 0 010 7.75" }],
  ],

  user: [
    ["path", { d: "M20 21v-2a4 4 0 00-4-4H8a4 4 0 00-4 4v2" }],
    ["circle", { cx: "12", cy: "7", r: "4" }],
  ],

  "users-round": [
    ["path", { d: "M18 21a8 8 0 00-16 0M10 13a4 4 0 100-8 4 4 0 000 8zM22 21a8 8 0 00-6-7.7" }],
    ["path", { d: "M16 9a4 4 0 000 4" }],
  ],

  // ── Infrastructure ────────────────────────────────────────────────────────

  container: [
    ["path", { d: "M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z" }],
    ["polyline", { points: "3.27 6.96 12 12.01 20.73 6.96" }],
    ["line", { x1: "12", y1: "22.08", x2: "12", y2: "12" }],
  ],

  "refresh-cw": [
    ["polyline", { points: "23 4 23 10 17 10" }],
    ["polyline", { points: "1 20 1 14 7 14" }],
    ["path", { d: "M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15" }],
  ],

  "cloud-cog": [
    ["path", { d: "M12 7H6a4 4 0 000 8h3M16 7h2a4 4 0 014 4v0a4 4 0 01-4 4h-2" }],
    ["circle", { cx: "13.5", cy: "15.5", r: "2.5" }],
    ["path", { d: "M13.5 13v-.5M13.5 18v.5M11.5 14.5l-.5-.25M15.5 16.5l.5.25M11.5 16.5l-.5.25M15.5 14.5l.5-.25" }],
  ],

  workflow: [
    ["rect", { x: "2", y: "8", width: "6", height: "6", rx: "1" }],
    ["rect", { x: "16", y: "3", width: "6", height: "6", rx: "1" }],
    ["rect", { x: "16", y: "15", width: "6", height: "6", rx: "1" }],
    ["path", { d: "M8 11h4a4 4 0 004-4v-.5M8 13h4a4 4 0 014 4v.5" }],
  ],

  cpu: [
    ["rect", { x: "4", y: "4", width: "16", height: "16", rx: "2" }],
    ["rect", { x: "8", y: "8", width: "8", height: "8" }],
    ["line", { x1: "9", y1: "1", x2: "9", y2: "4" }],
    ["line", { x1: "15", y1: "1", x2: "15", y2: "4" }],
    ["line", { x1: "9", y1: "20", x2: "9", y2: "23" }],
    ["line", { x1: "15", y1: "20", x2: "15", y2: "23" }],
    ["line", { x1: "20", y1: "9", x2: "23", y2: "9" }],
    ["line", { x1: "20", y1: "14", x2: "23", y2: "14" }],
    ["line", { x1: "1", y1: "9", x2: "4", y2: "9" }],
    ["line", { x1: "1", y1: "14", x2: "4", y2: "14" }],
  ],

  "cpu-chip": [
    ["rect", { x: "7", y: "7", width: "10", height: "10", rx: "1" }],
    ["line", { x1: "10", y1: "3", x2: "10", y2: "7" }],
    ["line", { x1: "14", y1: "3", x2: "14", y2: "7" }],
    ["line", { x1: "10", y1: "17", x2: "10", y2: "21" }],
    ["line", { x1: "14", y1: "17", x2: "14", y2: "21" }],
    ["line", { x1: "3", y1: "10", x2: "7", y2: "10" }],
    ["line", { x1: "3", y1: "14", x2: "7", y2: "14" }],
    ["line", { x1: "17", y1: "10", x2: "21", y2: "10" }],
    ["line", { x1: "17", y1: "14", x2: "21", y2: "14" }],
  ],

  database: [
    ["ellipse", { cx: "12", cy: "5", rx: "9", ry: "3" }],
    ["path", { d: "M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" }],
    ["path", { d: "M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" }],
  ],

  radio: [
    ["circle", { cx: "12", cy: "12", r: "2" }],
    ["path", { d: "M16.24 7.76a6 6 0 010 8.49M7.76 16.24a6 6 0 010-8.49M20.49 3.51a12 12 0 010 16.97M3.51 20.49a12 12 0 010-16.97" }],
  ],

  cog: [
    ["circle", { cx: "12", cy: "12", r: "3" }],
    ["path", { d: "M12 2v2M12 20v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M2 12h2M20 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" }],
  ],

  "terminal-square": [
    ["rect", { x: "2", y: "3", width: "20", height: "18", rx: "2" }],
    ["polyline", { points: "8 17 12 13 8 9" }],
    ["line", { x1: "14", y1: "17", x2: "20", y2: "17" }],
  ],

  wrench: [
    ["path", { d: "M14.7 6.3a1 1 0 000 1.4l1.6 1.6a1 1 0 001.4 0l3.77-3.77a6 6 0 01-7.94 7.94l-6.91 6.91a2.12 2.12 0 01-3-3l6.91-6.91a6 6 0 017.94-7.94l-3.76 3.76z" }],
  ],

  // ── Toolkit ───────────────────────────────────────────────────────────────

  binary: [
    ["path", { d: "M6 10V8a2 2 0 012-2M10 10V6M6 14v2a2 2 0 002 2M10 14v4M14 10V6a2 2 0 014 0v4a2 2 0 01-4 0zM14 18a2 2 0 014 0" }],
  ],

  regex: [
    ["path", { d: "M3 3l18 18M9.5 4.5l5 15M14.5 4.5l-5 15M5 9h14M5 15h14" }],
  ],

  "pen-tool": [
    ["path", { d: "M12 19l7-7 3 3-7 7-3-3zM18 13l-1.5-7.5L2 2l3.5 14.5L13 18z" }],
    ["path", { d: "M2 2l7.586 7.586" }],
    ["circle", { cx: "11", cy: "11", r: "2" }],
  ],

  "user-cog": [
    ["circle", { cx: "10", cy: "7", r: "4" }],
    ["path", { d: "M10.3 15H6a4 4 0 00-4 4v2" }],
    ["circle", { cx: "18", cy: "18", r: "2" }],
    ["path", { d: "M18 16v-1.5M18 20v1.5M14.5 18h1.5M21.5 18h-1.5M15.7 15.7l1 1M20.3 20.3l1 1M15.7 20.3l1-1M20.3 15.7l1-1" }],
  ],

  "dollar-sign": "M12 2v20M17 5H9.5a3.5 3.5 0 000 7h5a3.5 3.5 0 010 7H6",

  package: [
    ["line", { x1: "16.5", y1: "9.4", x2: "7.5", y2: "4.21" }],
    ["path", { d: "M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z" }],
    ["polyline", { points: "3.27 6.96 12 12.01 20.73 6.96" }],
    ["line", { x1: "12", y1: "22.08", x2: "12", y2: "12" }],
  ],

  // ── AI / Project ─────────────────────────────────────────────────────────

  store: [
    ["path", { d: "M3 9l1-6h16l1 6M3 9h18M3 9a9 9 0 009 12 9 9 0 009-12" }],
    ["line", { x1: "12", y1: "12", x2: "12", y2: "21" }],
  ],

  factory: [
    ["path", { d: "M2 20V8l6 4V8l6 4V4l8 4v12H2z" }],
    ["line", { x1: "6", y1: "20", x2: "6", y2: "16" }],
    ["line", { x1: "10", y1: "20", x2: "10", y2: "16" }],
    ["line", { x1: "14", y1: "20", x2: "14", y2: "16" }],
  ],

  infinity: "M12 12c-2-2.5-4-4-6-4a4 4 0 000 8c2 0 4-1.5 6-4zm0 0c2 2.5 4 4 6 4a4 4 0 000-8c-2 0-4 1.5-6 4z",

  swords: [
    ["polyline", { points: "14.5 17.5 3 6 3 3 6 3 17.5 14.5" }],
    ["line", { x1: "13", y1: "19", x2: "19", y2: "13" }],
    ["line", { x1: "16", y1: "16", x2: "20", y2: "20" }],
    ["line", { x1: "19", y1: "21", x2: "21", y2: "19" }],
    ["polyline", { points: "14.5 6.5 18 3 21 3 21 6 17.5 9.5" }],
    ["line", { x1: "5", y1: "14", x2: "8.5", y2: "17.5" }],
    ["line", { x1: "3", y1: "20", x2: "5", y2: "18" }],
  ],

  brain: [
    ["path", { d: "M12 5a3 3 0 10-5.99.14A3 3 0 006 8a3 3 0 001 5.83V17a2 2 0 002 2 2 2 0 002-2v-.17A3 3 0 0012 19v0" }],
    ["path", { d: "M12 5a3 3 0 115.99.14A3 3 0 0118 8a3 3 0 01-1 5.83V17a2 2 0 01-2 2 2 2 0 01-2-2v-.17A3 3 0 0112 19v0" }],
    ["path", { d: "M9 13a3 3 0 006 0M6 8a3 3 0 000 5M18 8a3 3 0 010 5" }],
  ],

  ruler: [
    ["rect", { x: "2", y: "6", width: "20", height: "12", rx: "2" }],
    ["line", { x1: "6", y1: "6", x2: "6", y2: "10" }],
    ["line", { x1: "10", y1: "6", x2: "10", y2: "8" }],
    ["line", { x1: "14", y1: "6", x2: "14", y2: "10" }],
    ["line", { x1: "18", y1: "6", x2: "18", y2: "8" }],
  ],

  palette: [
    ["circle", { cx: "12", cy: "12", r: "10" }],
    ["circle", { cx: "8", cy: "12", r: "1.5", fill: "currentColor" }],
    ["circle", { cx: "12", cy: "8", r: "1.5", fill: "currentColor" }],
    ["circle", { cx: "16", cy: "12", r: "1.5", fill: "currentColor" }],
    ["circle", { cx: "12", cy: "16", r: "1.5", fill: "currentColor" }],
  ],

  "trending-up": [
    ["polyline", { points: "23 6 13.5 15.5 8.5 10.5 1 18" }],
    ["polyline", { points: "17 6 23 6 23 12" }],
  ],

  activity: "M22 12h-4l-3 9L9 3l-3 9H2",

  "cloud-upload": [
    ["polyline", { points: "16 16 12 12 8 16" }],
    ["line", { x1: "12", y1: "12", x2: "12", y2: "21" }],
    ["path", { d: "M20.39 18.39A5 5 0 0018 9h-1.26A8 8 0 103 16.3" }],
  ],

  // ── File type icons ───────────────────────────────────────────────────────

  // React / Atom
  atom: [
    ["circle", { cx: "12", cy: "12", r: "2" }],
    ["path", { d: "M12 2C6.48 2 2 6.48 2 12" }],
    ["ellipse", { cx: "12", cy: "12", rx: "10", ry: "4.5" }],
    ["ellipse", { cx: "12", cy: "12", rx: "10", ry: "4.5", transform: "rotate(60 12 12)" }],
    ["ellipse", { cx: "12", cy: "12", rx: "10", ry: "4.5", transform: "rotate(120 12 12)" }],
  ],

  // Coffee / Java
  coffee: [
    ["path", { d: "M18 8h1a4 4 0 010 8h-1" }],
    ["path", { d: "M2 8h16v9a4 4 0 01-4 4H6a4 4 0 01-4-4V8z" }],
    ["line", { x1: "6", y1: "1", x2: "6", y2: "4" }],
    ["line", { x1: "10", y1: "1", x2: "10", y2: "4" }],
    ["line", { x1: "14", y1: "1", x2: "14", y2: "4" }],
  ],

  // Gem / Ruby
  gem: [
    ["polygon", { points: "12 2 22 8.5 22 15.5 12 22 2 15.5 2 8.5 12 2" }],
    ["line", { x1: "12", y1: "2", x2: "12", y2: "22" }],
    ["path", { d: "M2 8.5h20M2 15.5h20" }],
  ],

  // JSON / Braces
  braces: "M8 3H7a2 2 0 00-2 2v5a2 2 0 01-2 2 2 2 0 012 2v5c0 1.1.9 2 2 2h1M16 3h1a2 2 0 012 2v5a2 2 0 002 2 2 2 0 00-2 2v5a2 2 0 01-2 2h-1",

  // Archive / zip
  archive: [
    ["polyline", { points: "21 8 21 21 3 21 3 8" }],
    ["rect", { x: "1", y: "3", width: "22", height: "5", rx: "1" }],
    ["line", { x1: "10", y1: "12", x2: "14", y2: "12" }],
  ],

  // Paintbrush / SVG
  paintbrush: [
    ["path", { d: "M18.37 2.63L14 7l-1.59-1.59a2 2 0 00-2.82 0L8 7l9 9 1.59-1.59a2 2 0 000-2.82L17 10l4.37-4.37a2.12 2.12 0 00-3-3z" }],
    ["path", { d: "M9 8c-2 3-4 3.5-7 4l8 10c2-1 6-5 6-7M7 8l1.5 1.5" }],
  ],

  // Book open
  "book-open": [
    ["path", { d: "M2 3h6a4 4 0 014 4v14a3 3 0 00-3-3H2z" }],
    ["path", { d: "M22 3h-6a4 4 0 00-4 4v14a3 3 0 013-3h7z" }],
  ],

  // Generic file
  file: [
    ["path", { d: "M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" }],
    ["path", { d: "M14 2v6h6" }],
  ],

  code2: "M16 18l6-6-6-6M8 6l-6 6 6 6",

  // ── Misc UI ───────────────────────────────────────────────────────────────

  x: [
    ["line", { x1: "18", y1: "6", x2: "6", y2: "18" }],
    ["line", { x1: "6", y1: "6", x2: "18", y2: "18" }],
  ],

  check: "M20 6L9 17l-5-5",
  "circle-check": [
    ["circle", { cx: "12", cy: "12", r: "10" }],
    ["path", { d: "M9 12l2 2 4-4" }],
  ],

  "chevron-right": "M9 18l6-6-6-6",
  "chevron-left": "M15 18l-6-6 6-6",
  "chevron-down": "M6 9l6 6 6-6",
  "chevron-up": "M18 15l-6-6-6 6",

  plus: [
    ["line", { x1: "12", y1: "5", x2: "12", y2: "19" }],
    ["line", { x1: "5", y1: "12", x2: "19", y2: "12" }],
  ],

  minus: "M5 12h14",

  "external-link": [
    ["path", { d: "M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6" }],
    ["polyline", { points: "15 3 21 3 21 9" }],
    ["line", { x1: "10", y1: "14", x2: "21", y2: "3" }],
  ],

  copy: [
    ["rect", { x: "9", y: "9", width: "13", height: "13", rx: "2" }],
    ["path", { d: "M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" }],
  ],

  trash: [
    ["polyline", { points: "3 6 5 6 21 6" }],
    ["path", { d: "M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6M10 11v6M14 11v6" }],
    ["path", { d: "M9 6V4a1 1 0 011-1h4a1 1 0 011 1v2" }],
  ],

  edit: [
    ["path", { d: "M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7" }],
    ["path", { d: "M18.5 2.5a2.12 2.12 0 013 3L12 15l-4 1 1-4 9.5-9.5z" }],
  ],

  info: [
    ["circle", { cx: "12", cy: "12", r: "10" }],
    ["line", { x1: "12", y1: "16", x2: "12", y2: "12" }],
    ["line", { x1: "12", y1: "8", x2: "12.01", y2: "8" }],
  ],

  "alert-triangle": [
    ["path", { d: "M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" }],
    ["line", { x1: "12", y1: "9", x2: "12", y2: "13" }],
    ["line", { x1: "12", y1: "17", x2: "12.01", y2: "17" }],
  ],

  "alert-circle": [
    ["circle", { cx: "12", cy: "12", r: "10" }],
    ["line", { x1: "12", y1: "8", x2: "12", y2: "12" }],
    ["line", { x1: "12", y1: "16", x2: "12.01", y2: "16" }],
  ],

  loader: [
    ["line", { x1: "12", y1: "2", x2: "12", y2: "6" }],
    ["line", { x1: "12", y1: "18", x2: "12", y2: "22" }],
    ["line", { x1: "4.93", y1: "4.93", x2: "7.76", y2: "7.76" }],
    ["line", { x1: "16.24", y1: "16.24", x2: "19.07", y2: "19.07" }],
    ["line", { x1: "2", y1: "12", x2: "6", y2: "12" }],
    ["line", { x1: "18", y1: "12", x2: "22", y2: "12" }],
    ["line", { x1: "4.93", y1: "19.07", x2: "7.76", y2: "16.24" }],
    ["line", { x1: "16.24", y1: "7.76", x2: "19.07", y2: "4.93" }],
  ],

  lock: [
    ["rect", { x: "3", y: "11", width: "18", height: "11", rx: "2" }],
    ["path", { d: "M7 11V7a5 5 0 0110 0v4" }],
  ],

  unlock: [
    ["rect", { x: "3", y: "11", width: "18", height: "11", rx: "2" }],
    ["path", { d: "M7 11V7a5 5 0 019.9-1" }],
  ],

  key: [
    ["circle", { cx: "7.5", cy: "15.5", r: "5.5" }],
    ["path", { d: "M21 2l-9.6 9.6M15.5 7.5l3 3L22 7l-3-3" }],
  ],

  moon: "M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z",

  sun: [
    ["circle", { cx: "12", cy: "12", r: "5" }],
    ["line", { x1: "12", y1: "1", x2: "12", y2: "3" }],
    ["line", { x1: "12", y1: "21", x2: "12", y2: "23" }],
    ["line", { x1: "4.22", y1: "4.22", x2: "5.64", y2: "5.64" }],
    ["line", { x1: "18.36", y1: "18.36", x2: "19.78", y2: "19.78" }],
    ["line", { x1: "1", y1: "12", x2: "3", y2: "12" }],
    ["line", { x1: "21", y1: "12", x2: "23", y2: "12" }],
    ["line", { x1: "4.22", y1: "19.78", x2: "5.64", y2: "18.36" }],
    ["line", { x1: "18.36", y1: "5.64", x2: "19.78", y2: "4.22" }],
  ],

  "arrow-up": [
    ["line", { x1: "12", y1: "19", x2: "12", y2: "5" }],
    ["polyline", { points: "5 12 12 5 19 12" }],
  ],

  "arrow-down": [
    ["line", { x1: "12", y1: "5", x2: "12", y2: "19" }],
    ["polyline", { points: "19 12 12 19 5 12" }],
  ],

  "arrow-right": [
    ["line", { x1: "5", y1: "12", x2: "19", y2: "12" }],
    ["polyline", { points: "12 5 19 12 12 19" }],
  ],

  "arrow-left": [
    ["line", { x1: "19", y1: "12", x2: "5", y2: "12" }],
    ["polyline", { points: "12 19 5 12 12 5" }],
  ],

  maximize: [
    ["path", { d: "M8 3H5a2 2 0 00-2 2v3M21 8V5a2 2 0 00-2-2h-3M3 16v3a2 2 0 002 2h3M16 21h3a2 2 0 002-2v-3" }],
  ],

  minimize: [
    ["path", { d: "M4 14h6v6M20 10h-6V4M14 10l7-7M3 21l7-7" }],
  ],

  sidebar: [
    ["rect", { x: "3", y: "3", width: "18", height: "18", rx: "2" }],
    ["line", { x1: "9", y1: "3", x2: "9", y2: "21" }],
  ],

  layers: [
    ["polygon", { points: "12 2 2 7 12 12 22 7 12 2" }],
    ["polyline", { points: "2 17 12 22 22 17" }],
    ["polyline", { points: "2 12 12 17 22 12" }],
  ],

  folder: [
    ["path", { d: "M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 2h9a2 2 0 012 2z" }],
  ],

  zap: "M13 2L3 14h9l-1 8 10-12h-9l1-8z",

  send: "M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z",

  mic: [
    ["path", { d: "M12 1a3 3 0 00-3 3v8a3 3 0 006 0V4a3 3 0 00-3-3z" }],
    ["path", { d: "M19 10v2a7 7 0 01-14 0v-2M12 19v4M8 23h8" }],
  ],

  "stop-circle": [
    ["circle", { cx: "12", cy: "12", r: "10" }],
    ["rect", { x: "9", y: "9", width: "6", height: "6" }],
  ],

  pause: [
    ["rect", { x: "6", y: "4", width: "4", height: "16" }],
    ["rect", { x: "14", y: "4", width: "4", height: "16" }],
  ],

  "skip-forward": [
    ["polygon", { points: "5 4 15 12 5 20 5 4" }],
    ["line", { x1: "19", y1: "5", x2: "19", y2: "19" }],
  ],

  list: [
    ["line", { x1: "8", y1: "6", x2: "21", y2: "6" }],
    ["line", { x1: "8", y1: "12", x2: "21", y2: "12" }],
    ["line", { x1: "8", y1: "18", x2: "21", y2: "18" }],
    ["line", { x1: "3", y1: "6", x2: "3.01", y2: "6" }],
    ["line", { x1: "3", y1: "12", x2: "3.01", y2: "12" }],
    ["line", { x1: "3", y1: "18", x2: "3.01", y2: "18" }],
  ],

  grid: [
    ["rect", { x: "3", y: "3", width: "7", height: "7" }],
    ["rect", { x: "14", y: "3", width: "7", height: "7" }],
    ["rect", { x: "14", y: "14", width: "7", height: "7" }],
    ["rect", { x: "3", y: "14", width: "7", height: "7" }],
  ],

  "rotate-ccw": [
    ["polyline", { points: "1 4 1 10 7 10" }],
    ["path", { d: "M3.51 15a9 9 0 101.85-4.15L1 10" }],
  ],

  download: [
    ["path", { d: "M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" }],
    ["polyline", { points: "7 10 12 15 17 10" }],
    ["line", { x1: "12", y1: "15", x2: "12", y2: "3" }],
  ],

  upload: [
    ["path", { d: "M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" }],
    ["polyline", { points: "17 8 12 3 7 8" }],
    ["line", { x1: "12", y1: "3", x2: "12", y2: "15" }],
  ],

  link: [
    ["path", { d: "M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71" }],
    ["path", { d: "M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71" }],
  ],

  unlink: [
    ["path", { d: "M18.84 12.25l1.72-1.71a4.5 4.5 0 00-6.37-6.37L12.5 5.86M5.17 11.75l-1.72 1.71a4.5 4.5 0 006.37 6.37l1.45-1.45" }],
    ["line", { x1: "8", y1: "2", x2: "8", y2: "5" }],
    ["line", { x1: "2", y1: "8", x2: "5", y2: "8" }],
    ["line", { x1: "16", y1: "19", x2: "16", y2: "22" }],
    ["line", { x1: "19", y1: "16", x2: "22", y2: "16" }],
  ],

  bell: "M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9zM13.73 21a2 2 0 01-3.46 0",

  "bell-off": [
    ["path", { d: "M13.73 21a2 2 0 01-3.46 0M18.63 13A17.89 17.89 0 0118 8a6 6 0 00-9.33-5M4.34 4.34A16 16 0 002 8c0 7-3 9-3 9h13M1 1l22 22" }],
  ],

  star: "M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z",

  heart: "M20.84 4.61a5.5 5.5 0 00-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 00-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 000-7.78z",

  bookmark: "M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z",

  filter: "M22 3H2l8 9.46V19l4 2v-8.54L22 3z",

  "sort-asc": [
    ["line", { x1: "3", y1: "6", x2: "21", y2: "6" }],
    ["line", { x1: "3", y1: "12", x2: "16", y2: "12" }],
    ["line", { x1: "3", y1: "18", x2: "11", y2: "18" }],
  ],

  "sort-desc": [
    ["line", { x1: "3", y1: "6", x2: "11", y2: "6" }],
    ["line", { x1: "3", y1: "12", x2: "16", y2: "12" }],
    ["line", { x1: "3", y1: "18", x2: "21", y2: "18" }],
  ],

  expand: [
    ["path", { d: "M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" }],
  ],

  compress: [
    ["path", { d: "M4 14h6v6M14 4h6v6M14 10l7-7M3 21l7-7" }],
  ],

  split: [
    ["path", { d: "M21 15a4 4 0 01-4 4H7M21 9a4 4 0 00-4-4H7" }],
    ["line", { x1: "7", y1: "5", x2: "7", y2: "19" }],
  ],

  merge: [
    ["path", { d: "M6 3v6a6 6 0 006 6 6 6 0 006-6V3" }],
    ["line", { x1: "6", y1: "21", x2: "6", y2: "15" }],
    ["line", { x1: "18", y1: "21", x2: "18", y2: "15" }],
  ],

  diff: [
    ["path", { d: "M11 3H5a2 2 0 00-2 2v14a2 2 0 002 2h6" }],
    ["path", { d: "M19 3h-4v4" }],
    ["path", { d: "M15 7l4-4" }],
    ["path", { d: "M13 17h6M16 14v6" }],
  ],

  compass: [
    ["circle", { cx: "12", cy: "12", r: "10" }],
    ["polygon", { points: "16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88 16.24 7.76" }],
  ],

  "map-pin": [
    ["path", { d: "M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0118 0z" }],
    ["circle", { cx: "12", cy: "10", r: "3" }],
  ],

  network: [
    ["circle", { cx: "12", cy: "5", r: "3" }],
    ["circle", { cx: "5", cy: "19", r: "3" }],
    ["circle", { cx: "19", cy: "19", r: "3" }],
    ["line", { x1: "10.27", y1: "7.22", x2: "6.73", y2: "16.78" }],
    ["line", { x1: "13.73", y1: "7.22", x2: "17.27", y2: "16.78" }],
    ["line", { x1: "8", y1: "19", x2: "16", y2: "19" }],
  ],

  wifi: [
    ["path", { d: "M5 12.55a11 11 0 0114.08 0M1.42 9a16 16 0 0121.16 0M8.53 16.11a6 6 0 016.95 0" }],
    ["line", { x1: "12", y1: "20", x2: "12.01", y2: "20" }],
  ],

  bluetooth: [
    ["polyline", { points: "6.5 6.5 17.5 17.5 12 23 12 1 17.5 6.5 6.5 17.5" }],
  ],

  usb: [
    ["circle", { cx: "10", cy: "7", r: "1" }],
    ["circle", { cx: "14", cy: "7", r: "1" }],
    ["path", { d: "M12 3v12M8 16h8M12 21v-5M8 16l-2 5M16 16l2 5" }],
  ],

  server: [
    ["rect", { x: "2", y: "2", width: "20", height: "8", rx: "2" }],
    ["rect", { x: "2", y: "14", width: "20", height: "8", rx: "2" }],
    ["line", { x1: "6", y1: "6", x2: "6.01", y2: "6" }],
    ["line", { x1: "6", y1: "18", x2: "6.01", y2: "18" }],
  ],

  microscope: [
    ["path", { d: "M6 18h8M3 22h18M14 22a7 7 0 100-14M11 7.5L11 14.5" }],
    ["path", { d: "M9 3.5L13 3.5M11 3.5L11 10.5" }],
    ["path", { d: "M7 7.5a4 4 0 004 4" }],
  ],

  flask: [
    ["path", { d: "M9 3h6M9 3v9L4 21h16L15 12V3" }],
    ["path", { d: "M5.5 18.5h13" }],
  ],

  "chart-bar": [
    ["rect", { x: "3", y: "12", width: "4", height: "9" }],
    ["rect", { x: "10", y: "6", width: "4", height: "15" }],
    ["rect", { x: "17", y: "9", width: "4", height: "12" }],
    ["line", { x1: "3", y1: "21", x2: "21", y2: "21" }],
  ],

  "chart-line": [
    ["polyline", { points: "3 20 8 14 13 17 21 7" }],
    ["line", { x1: "3", y1: "21", x2: "21", y2: "21" }],
  ],

  "pie-chart": [
    ["path", { d: "M21.21 15.89A10 10 0 118 2.83" }],
    ["path", { d: "M22 12A10 10 0 0012 2v10z" }],
  ],
};

// ── SVG renderer ──────────────────────────────────────────────────────────────

function renderElement(tag: string, attrs: Record<string, string | number>, key: number) {
  const props: Record<string, string | number> = { key, ...attrs };
  return React.createElement(tag as any, props);
}

/** Returns the SVG inner elements from a PathDef. */
function toElements(def: PathDef): React.ReactNode {
  if (typeof def === "string") {
    return React.createElement("path", { d: def });
  }
  return def.map(([tag, attrs], i) => renderElement(tag, attrs, i));
}

// ── Public component ──────────────────────────────────────────────────────────

/**
 * `<Icon name="search" size={16} />`
 *
 * Uses `currentColor` so it inherits from the parent's CSS `color` property.
 * Stroke width defaults to `var(--icon-stroke, 1.5)` which can be set per-theme.
 */
export function Icon({
  name,
  size = 16,
  strokeWidth,
  color = "currentColor",
  className,
  style,
  title,
  "aria-label": ariaLabel,
  "aria-hidden": ariaHidden,
}: IconProps) {
  const def = PATHS[name];
  if (!def) {
    console.warn(`[Icon] Unknown icon: "${name}"`);
    return null;
  }

  const sw = strokeWidth ?? "var(--icon-stroke, 1.5)";

  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={sw as number}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      style={style}
      role={ariaLabel ? "img" : undefined}
      aria-label={ariaLabel}
      aria-hidden={ariaHidden ?? (!ariaLabel ? "true" : undefined)}
    >
      {title && <title>{title}</title>}
      {toElements(def)}
    </svg>
  );
}

// Convenience: identical to <Icon> but always 20px — use in the Activity Bar.
export function ActivityIcon(props: Omit<IconProps, "size"> & { size?: number }) {
  return <Icon size={20} {...props} />;
}
