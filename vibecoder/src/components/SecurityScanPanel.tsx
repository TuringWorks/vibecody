import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

// -- Types --------------------------------------------------------------------

type Severity = "Critical" | "High" | "Medium" | "Low" | "Info";
type TabName = "Findings" | "Summary" | "Patterns" | "History";
type GroupMode = "none" | "cwe" | "file" | "severity";

interface Finding {
  id: string;
  title: string;
  severity: Severity;
  file: string;
  line: number;
  description: string;
  cwe: string;
  remediation: string;
  suppressed: boolean;
}

interface ScanPattern {
  id: string;
  name: string;
  vulnerabilityClass: string;
  cweIds: string[];       // CWE IDs covered by this pattern
  languages: string[];
  enabled: boolean;
  matchCount: number;
}

interface ScanRun {
  id: string;
  timestamp: string;
  findingCount: number;
  duration: string;
}

interface SecurityScanPanelProps {
  workspacePath?: string;
  onOpenFile?: (path: string, line?: number) => void;
}

// -- Default Patterns — comprehensive CVE-database-mapped set -----------------
// Organised by vulnerability class. Each entry lists the CWE IDs it covers so
// match counts can be computed without string-matching on names.

const DEFAULT_PATTERNS: ScanPattern[] = [
  // ── Injection ──────────────────────────────────────────────────────────────
  { id: "p-001", name: "SQL Injection",              vulnerabilityClass: "Injection",         cweIds: ["CWE-89", "CWE-564"],                        languages: ["Rust", "Python", "JavaScript", "Go", "Java", "C#", "PHP", "Ruby"], enabled: true,  matchCount: 0 },
  { id: "p-002", name: "Cross-Site Scripting (XSS)", vulnerabilityClass: "Injection",         cweIds: ["CWE-79", "CWE-80", "CWE-83", "CWE-116"],    languages: ["JavaScript", "TypeScript", "HTML", "PHP", "Ruby"],                  enabled: true,  matchCount: 0 },
  { id: "p-003", name: "Command Injection",          vulnerabilityClass: "Injection",         cweIds: ["CWE-78", "CWE-77", "CWE-88"],               languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-004", name: "LDAP Injection",             vulnerabilityClass: "Injection",         cweIds: ["CWE-90"],                                   languages: ["Java", "Python", "C#", "JavaScript", "PHP"],                       enabled: true,  matchCount: 0 },
  { id: "p-005", name: "XPath / XQuery Injection",   vulnerabilityClass: "Injection",         cweIds: ["CWE-643", "CWE-652"],                       languages: ["Java", "Python", "C#", "JavaScript"],                              enabled: true,  matchCount: 0 },
  { id: "p-006", name: "Template Injection (SSTI)",  vulnerabilityClass: "Injection",         cweIds: ["CWE-94", "CWE-1336"],                       languages: ["Python", "JavaScript", "Ruby", "PHP", "Java"],                     enabled: true,  matchCount: 0 },
  { id: "p-007", name: "Header / CRLF Injection",    vulnerabilityClass: "Injection",         cweIds: ["CWE-113", "CWE-93"],                        languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-008", name: "Log Injection",              vulnerabilityClass: "Injection",         cweIds: ["CWE-117"],                                  languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-009", name: "NoSQL Injection",            vulnerabilityClass: "Injection",         cweIds: ["CWE-943"],                                  languages: ["JavaScript", "TypeScript", "Python", "Go", "Java"],                 enabled: true,  matchCount: 0 },
  { id: "p-010", name: "XML External Entity (XXE)",  vulnerabilityClass: "Injection",         cweIds: ["CWE-611", "CWE-776"],                       languages: ["Java", "Python", "C#", "JavaScript", "PHP"],                       enabled: true,  matchCount: 0 },

  // ── Authentication & Secrets ───────────────────────────────────────────────
  { id: "p-011", name: "Hardcoded Secrets / API Keys",   vulnerabilityClass: "Authentication",    cweIds: ["CWE-798", "CWE-321", "CWE-259"],            languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-012", name: "Weak Password Policy",           vulnerabilityClass: "Authentication",    cweIds: ["CWE-521", "CWE-261"],                       languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-013", name: "Missing Authentication",         vulnerabilityClass: "Authentication",    cweIds: ["CWE-306", "CWE-862", "CWE-863"],            languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-014", name: "JWT Weakness",                   vulnerabilityClass: "Authentication",    cweIds: ["CWE-347", "CWE-345"],                       languages: ["JavaScript", "TypeScript", "Python", "Java", "Go", "Rust"],         enabled: true,  matchCount: 0 },
  { id: "p-015", name: "OAuth / OIDC Misconfiguration",  vulnerabilityClass: "Authentication",    cweIds: ["CWE-346", "CWE-601"],                       languages: ["JavaScript", "TypeScript", "Python", "Java", "Go"],                 enabled: true,  matchCount: 0 },
  { id: "p-016", name: "Credential Exposure in URLs",    vulnerabilityClass: "Authentication",    cweIds: ["CWE-312", "CWE-315"],                       languages: ["*"],                                                                enabled: true,  matchCount: 0 },

  // ── Access Control ─────────────────────────────────────────────────────────
  { id: "p-017", name: "Path Traversal",                 vulnerabilityClass: "Access Control",    cweIds: ["CWE-22", "CWE-23", "CWE-36"],               languages: ["Rust", "Python", "Go", "Java", "C", "C++", "PHP", "JavaScript"],    enabled: true,  matchCount: 0 },
  { id: "p-018", name: "Insecure Direct Object Ref (IDOR)", vulnerabilityClass: "Access Control", cweIds: ["CWE-639", "CWE-284"],                       languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-019", name: "Privilege Escalation",           vulnerabilityClass: "Access Control",    cweIds: ["CWE-269", "CWE-266", "CWE-732"],            languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-020", name: "Unrestricted File Upload",        vulnerabilityClass: "Access Control",   cweIds: ["CWE-434"],                                  languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-021", name: "Directory Listing / Open Index", vulnerabilityClass: "Access Control",    cweIds: ["CWE-548"],                                  languages: ["*"],                                                                enabled: true,  matchCount: 0 },

  // ── Session Management ─────────────────────────────────────────────────────
  { id: "p-022", name: "CSRF Vulnerability",             vulnerabilityClass: "Session",           cweIds: ["CWE-352"],                                  languages: ["Rust", "Python", "JavaScript", "TypeScript", "Java", "PHP", "Ruby"], enabled: true,  matchCount: 0 },
  { id: "p-023", name: "Session Fixation",               vulnerabilityClass: "Session",           cweIds: ["CWE-384"],                                  languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-024", name: "Insufficient Session Expiry",    vulnerabilityClass: "Session",           cweIds: ["CWE-613"],                                  languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-025", name: "Insecure Cookie / Flag Missing", vulnerabilityClass: "Session",           cweIds: ["CWE-614", "CWE-1004", "CWE-1275"],          languages: ["JavaScript", "TypeScript", "Python", "PHP", "Ruby", "Go", "Rust"],  enabled: true,  matchCount: 0 },
  { id: "p-026", name: "Clickjacking (missing X-Frame)", vulnerabilityClass: "Session",           cweIds: ["CWE-1021"],                                 languages: ["*"],                                                                enabled: false, matchCount: 0 },

  // ── Cryptography ───────────────────────────────────────────────────────────
  { id: "p-027", name: "Weak / Broken Cipher",           vulnerabilityClass: "Cryptography",      cweIds: ["CWE-327", "CWE-326"],                       languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-028", name: "Weak Password Hashing",          vulnerabilityClass: "Cryptography",      cweIds: ["CWE-916", "CWE-759", "CWE-760"],            languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-029", name: "Insecure Random / Predictable",  vulnerabilityClass: "Cryptography",      cweIds: ["CWE-330", "CWE-338"],                       languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-030", name: "Hardcoded / Missing IV or Nonce",vulnerabilityClass: "Cryptography",      cweIds: ["CWE-329", "CWE-1204"],                      languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-031", name: "Missing Encryption at Rest",     vulnerabilityClass: "Cryptography",      cweIds: ["CWE-311", "CWE-312"],                       languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-032", name: "TLS / Certificate Misconfiguration", vulnerabilityClass: "Cryptography",  cweIds: ["CWE-295", "CWE-296", "CWE-297", "CWE-599"],  languages: ["*"],                                                               enabled: true,  matchCount: 0 },
  { id: "p-033", name: "Side-Channel / Timing Attack",   vulnerabilityClass: "Cryptography",      cweIds: ["CWE-208", "CWE-385"],                       languages: ["*"],                                                                enabled: false, matchCount: 0 },

  // ── Transport & Network ────────────────────────────────────────────────────
  { id: "p-034", name: "Insecure HTTP (cleartext)",      vulnerabilityClass: "Transport",         cweIds: ["CWE-319"],                                  languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-035", name: "SSRF — Server-Side Request Forgery", vulnerabilityClass: "Transport",     cweIds: ["CWE-918"],                                  languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-036", name: "Open Redirect",                  vulnerabilityClass: "Transport",         cweIds: ["CWE-601"],                                  languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-037", name: "DNS Rebinding / Confused Deputy",vulnerabilityClass: "Transport",         cweIds: ["CWE-350"],                                  languages: ["JavaScript", "TypeScript", "Go", "Rust", "Python"],                 enabled: true,  matchCount: 0 },
  { id: "p-038", name: "CORS Misconfiguration",          vulnerabilityClass: "Transport",         cweIds: ["CWE-942", "CWE-346"],                       languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-039", name: "Missing Security Headers",       vulnerabilityClass: "Transport",         cweIds: ["CWE-693", "CWE-1021"],                      languages: ["*"],                                                                enabled: false, matchCount: 0 },

  // ── Memory Safety ──────────────────────────────────────────────────────────
  { id: "p-040", name: "Buffer Overflow (Stack/Heap)",   vulnerabilityClass: "Memory Safety",     cweIds: ["CWE-120", "CWE-121", "CWE-122", "CWE-131"], languages: ["C", "C++", "Rust"],                                                  enabled: true,  matchCount: 0 },
  { id: "p-041", name: "Use-After-Free",                 vulnerabilityClass: "Memory Safety",     cweIds: ["CWE-416"],                                  languages: ["C", "C++"],                                                         enabled: true,  matchCount: 0 },
  { id: "p-042", name: "Integer Overflow / Underflow",   vulnerabilityClass: "Memory Safety",     cweIds: ["CWE-190", "CWE-191", "CWE-193"],            languages: ["C", "C++", "Rust", "Go", "Java"],                                   enabled: true,  matchCount: 0 },
  { id: "p-043", name: "Null Pointer Dereference",       vulnerabilityClass: "Memory Safety",     cweIds: ["CWE-476"],                                  languages: ["C", "C++", "Java", "Go", "Rust"],                                   enabled: true,  matchCount: 0 },
  { id: "p-044", name: "Format String Vulnerability",    vulnerabilityClass: "Memory Safety",     cweIds: ["CWE-134"],                                  languages: ["C", "C++"],                                                         enabled: true,  matchCount: 0 },
  { id: "p-045", name: "Unsafe Rust / FFI",              vulnerabilityClass: "Memory Safety",     cweIds: ["CWE-758", "CWE-457"],                       languages: ["Rust"],                                                             enabled: true,  matchCount: 0 },
  { id: "p-046", name: "Double-Free",                    vulnerabilityClass: "Memory Safety",     cweIds: ["CWE-415"],                                  languages: ["C", "C++"],                                                         enabled: true,  matchCount: 0 },

  // ── Deserialization & Parsing ───────────────────────────────────────────────
  { id: "p-047", name: "Insecure Deserialization",       vulnerabilityClass: "Deserialization",   cweIds: ["CWE-502"],                                  languages: ["Java", "Python", "JavaScript", "C#", "Ruby", "PHP"],               enabled: true,  matchCount: 0 },
  { id: "p-048", name: "YAML / TOML Unsafe Load",        vulnerabilityClass: "Deserialization",   cweIds: ["CWE-502", "CWE-20"],                        languages: ["Python", "JavaScript", "TypeScript", "Rust", "Go"],                 enabled: true,  matchCount: 0 },
  { id: "p-049", name: "Prototype Pollution",            vulnerabilityClass: "Deserialization",   cweIds: ["CWE-1321"],                                 languages: ["JavaScript", "TypeScript"],                                         enabled: true,  matchCount: 0 },
  { id: "p-050", name: "Zip / Archive Slip",             vulnerabilityClass: "Deserialization",   cweIds: ["CWE-22"],                                   languages: ["*"],                                                                enabled: true,  matchCount: 0 },

  // ── Information Disclosure ─────────────────────────────────────────────────
  { id: "p-051", name: "Sensitive Data Exposure",        vulnerabilityClass: "Info Disclosure",   cweIds: ["CWE-200", "CWE-201", "CWE-202"],            languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-052", name: "Verbose Error / Stack Trace",    vulnerabilityClass: "Info Disclosure",   cweIds: ["CWE-209", "CWE-215"],                       languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-053", name: "Debug Mode in Production",       vulnerabilityClass: "Info Disclosure",   cweIds: ["CWE-489", "CWE-11"],                        languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-054", name: "Source Code / Path Disclosure",  vulnerabilityClass: "Info Disclosure",   cweIds: ["CWE-540", "CWE-548"],                       languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-055", name: "PII / PHI in Logs or Storage",   vulnerabilityClass: "Info Disclosure",   cweIds: ["CWE-359", "CWE-532"],                       languages: ["*"],                                                                enabled: true,  matchCount: 0 },

  // ── Supply Chain ───────────────────────────────────────────────────────────
  { id: "p-056", name: "Dependency CVE Scan",            vulnerabilityClass: "Supply Chain",      cweIds: ["CWE-1395", "CWE-829"],                      languages: ["Rust", "JavaScript", "Python", "Go", "Java", "C#"],                 enabled: true,  matchCount: 0 },
  { id: "p-057", name: "Typosquatting / Confused Dep.",  vulnerabilityClass: "Supply Chain",      cweIds: ["CWE-1357"],                                 languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-058", name: "Unpinned / Mutable Dependency",  vulnerabilityClass: "Supply Chain",      cweIds: ["CWE-1104"],                                 languages: ["Rust", "JavaScript", "Python", "Go", "Java"],                       enabled: true,  matchCount: 0 },
  { id: "p-059", name: "CI/CD Pipeline Injection",       vulnerabilityClass: "Supply Chain",      cweIds: ["CWE-77", "CWE-78"],                         languages: ["*"],                                                                enabled: false, matchCount: 0 },

  // ── Concurrency ────────────────────────────────────────────────────────────
  { id: "p-060", name: "Race Condition (TOCTOU)",        vulnerabilityClass: "Concurrency",       cweIds: ["CWE-362", "CWE-367"],                       languages: ["C", "C++", "Go", "Java", "Rust", "Python"],                         enabled: true,  matchCount: 0 },
  { id: "p-061", name: "Deadlock / Livelock",            vulnerabilityClass: "Concurrency",       cweIds: ["CWE-833", "CWE-820"],                       languages: ["*"],                                                                enabled: false, matchCount: 0 },

  // ── Input Validation & DoS ─────────────────────────────────────────────────
  { id: "p-062", name: "ReDoS (Regular Expression DoS)", vulnerabilityClass: "Input Validation",  cweIds: ["CWE-1333"],                                 languages: ["JavaScript", "TypeScript", "Python", "Java", "Go", "Rust"],         enabled: true,  matchCount: 0 },
  { id: "p-063", name: "Unvalidated / Untrusted Input",  vulnerabilityClass: "Input Validation",  cweIds: ["CWE-20", "CWE-1284"],                       languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-064", name: "Denial of Service (resource exhaustion)", vulnerabilityClass: "Input Validation", cweIds: ["CWE-400", "CWE-770"],               languages: ["*"],                                                                enabled: true,  matchCount: 0 },

  // ── API & Business Logic ───────────────────────────────────────────────────
  { id: "p-065", name: "Mass Assignment / Over-posting", vulnerabilityClass: "API Security",      cweIds: ["CWE-915"],                                  languages: ["*"],                                                                enabled: true,  matchCount: 0 },
  { id: "p-066", name: "Rate Limiting Missing",          vulnerabilityClass: "API Security",      cweIds: ["CWE-770", "CWE-799"],                       languages: ["*"],                                                                enabled: false, matchCount: 0 },
  { id: "p-067", name: "GraphQL Introspection / DoS",    vulnerabilityClass: "API Security",      cweIds: ["CWE-400", "CWE-770"],                       languages: ["JavaScript", "TypeScript", "Python", "Rust", "Go"],                 enabled: false, matchCount: 0 },

  // ── Rust / Tauri / Electron specific ──────────────────────────────────────
  { id: "p-068", name: "Tauri IPC Command Injection",    vulnerabilityClass: "App Security",      cweIds: ["CWE-78", "CWE-77"],                         languages: ["Rust", "JavaScript", "TypeScript"],                                 enabled: true,  matchCount: 0 },
  { id: "p-069", name: "Panic / Unwrap in Production",   vulnerabilityClass: "App Security",      cweIds: ["CWE-248", "CWE-617"],                       languages: ["Rust"],                                                             enabled: true,  matchCount: 0 },
  { id: "p-070", name: "Electron nodeIntegration / Remote Code", vulnerabilityClass: "App Security", cweIds: ["CWE-749"],                               languages: ["JavaScript", "TypeScript"],                                         enabled: false, matchCount: 0 },
];

// -- All vulnerability classes for filter dropdown ----------------------------

const VULN_CLASSES = [
  "All",
  "Injection", "Authentication", "Access Control", "Session",
  "Cryptography", "Transport", "Memory Safety", "Deserialization",
  "Info Disclosure", "Supply Chain", "Concurrency", "Input Validation",
  "API Security", "App Security",
] as const;

// -- CWE descriptions for group headers ---------------------------------------

const CWE_NAMES: Record<string, string> = {
  // Injection
  "CWE-77":  "Command Injection (generic)",
  "CWE-78":  "OS Command Injection",
  "CWE-79":  "Cross-Site Scripting (XSS)",
  "CWE-80":  "Basic XSS",
  "CWE-83":  "XSS in Attribute Value",
  "CWE-88":  "Argument Injection",
  "CWE-89":  "SQL Injection",
  "CWE-90":  "LDAP Injection",
  "CWE-93":  "CRLF Injection",
  "CWE-94":  "Code Injection / SSTI",
  "CWE-113": "HTTP Header Injection",
  "CWE-116": "Improper Encoding / Escaping",
  "CWE-117": "Log Injection",
  "CWE-564": "SQL Injection via Hibernate",
  "CWE-611": "XML External Entity (XXE)",
  "CWE-643": "XPath Injection",
  "CWE-652": "XQuery Injection",
  "CWE-776": "Recursive Entity Expansion (Billion Laughs)",
  "CWE-943": "NoSQL Injection",
  "CWE-1336": "Server-Side Template Injection",
  // Authentication
  "CWE-259": "Hardcoded Password",
  "CWE-261": "Weak Encoding for Password",
  "CWE-306": "Missing Authentication",
  "CWE-312": "Cleartext Storage of Sensitive Info",
  "CWE-315": "Cleartext Storage in Cookie",
  "CWE-321": "Hardcoded Cryptographic Key",
  "CWE-345": "Insufficient Verification of Data Authenticity",
  "CWE-346": "Origin Validation Error",
  "CWE-347": "Improper JWT Verification",
  "CWE-798": "Hardcoded Credentials / API Key",
  "CWE-862": "Missing Authorisation",
  "CWE-863": "Incorrect Authorisation",
  // Access Control
  "CWE-22":  "Path Traversal",
  "CWE-23":  "Relative Path Traversal",
  "CWE-36":  "Absolute Path Traversal",
  "CWE-269": "Improper Privilege Management",
  "CWE-266": "Incorrect Privilege Assignment",
  "CWE-284": "Improper Access Control",
  "CWE-434": "Unrestricted File Upload",
  "CWE-521": "Weak Password Requirements",
  "CWE-548": "Directory Listing / Info Exposure",
  "CWE-639": "IDOR — Authorisation Bypass",
  "CWE-732": "Incorrect Permission Assignment",
  // Session
  "CWE-352": "Cross-Site Request Forgery (CSRF)",
  "CWE-384": "Session Fixation",
  "CWE-613": "Insufficient Session Expiration",
  "CWE-614": "Sensitive Cookie without Secure Flag",
  "CWE-1004": "Cookie missing HttpOnly",
  "CWE-1021": "Clickjacking (missing X-Frame-Options)",
  "CWE-1275": "Cookie missing SameSite",
  // Cryptography
  "CWE-208": "Observable Timing Discrepancy",
  "CWE-295": "Improper Certificate Validation",
  "CWE-296": "Improper Following of Certificate Chain",
  "CWE-297": "Improper Validation of Certificate Expiry",
  "CWE-311": "Missing Encryption of Sensitive Data",
  "CWE-326": "Inadequate Encryption Strength",
  "CWE-327": "Broken / Risky Cryptographic Algorithm",
  "CWE-329": "Not Using Random IV / Nonce",
  "CWE-330": "Insufficient Random Values",
  "CWE-338": "Weak PRNG",
  "CWE-385": "Covert Timing Channel",
  "CWE-599": "No Verification of OpenSSL Certificate",
  "CWE-759": "Unsalted One-Way Hash",
  "CWE-760": "Predictable Salt",
  "CWE-916": "Weak Password Hash (MD5/SHA1)",
  "CWE-1204": "Weak Initialization Vector",
  // Transport
  "CWE-319": "Cleartext Transmission (HTTP)",
  "CWE-350": "DNS Rebinding / Reliance on Reverse DNS",
  "CWE-601": "Open Redirect",
  "CWE-693": "Missing Security Header",
  "CWE-918": "Server-Side Request Forgery (SSRF)",
  "CWE-942": "CORS Misconfiguration",
  // Memory Safety
  "CWE-120": "Buffer Copy without Size Check",
  "CWE-121": "Stack-Based Buffer Overflow",
  "CWE-122": "Heap-Based Buffer Overflow",
  "CWE-131": "Incorrect Calculation of Buffer Size",
  "CWE-134": "Uncontrolled Format String",
  "CWE-190": "Integer Overflow",
  "CWE-191": "Integer Underflow",
  "CWE-193": "Off-by-One Error",
  "CWE-415": "Double Free",
  "CWE-416": "Use After Free",
  "CWE-457": "Uninitialized Variable",
  "CWE-476": "Null Pointer Dereference",
  "CWE-758": "Undefined Behaviour (Unsafe Rust/C)",
  // Deserialization
  "CWE-20":  "Improper Input Validation",
  "CWE-502": "Deserialization of Untrusted Data",
  "CWE-1321": "Prototype Pollution",
  // Info Disclosure
  "CWE-11":  "ASP.NET Misconfiguration: Creating Debug Binary",
  "CWE-200": "Sensitive Information Exposure",
  "CWE-201": "Insertion of Sensitive Information into Log",
  "CWE-202": "Exposure of Sensitive Data Through Data Queries",
  "CWE-209": "Error Message Contains Sensitive Info",
  "CWE-215": "Sensitive Info in Debugging Code",
  "CWE-359": "Exposure of PII/PHI",
  "CWE-489": "Active Debug Code",
  "CWE-532": "Sensitive Info in Log Files",
  "CWE-540": "Source Code in Web Root",
  // Supply Chain
  "CWE-829": "Inclusion of Functionality from Untrusted Source",
  "CWE-1104": "Mutable / Unpinned Dependency",
  "CWE-1357": "Reliance on Insufficiently Trustworthy Component",
  "CWE-1395": "Dependency on Vulnerable Third-Party Component",
  // Concurrency
  "CWE-362": "Race Condition",
  "CWE-367": "TOCTOU Race Condition",
  "CWE-820": "Missing Synchronisation",
  "CWE-833": "Deadlock",
  // Input Validation / DoS
  "CWE-400": "Uncontrolled Resource Consumption",
  "CWE-770": "Allocation Without Limits",
  "CWE-799": "Missing Rate Limiting",
  "CWE-1284": "Improper Validation of Specified Quantity",
  "CWE-1333": "ReDoS — Inefficient Regular Expression",
  // API Security
  "CWE-915": "Improperly Controlled Modification (Mass Assignment)",
  // App Security
  "CWE-248": "Uncaught Exception (Rust panic!)",
  "CWE-617": "Reachable Assertion",
  "CWE-749": "Exposed Dangerous Method",
};

// -- Helpers ------------------------------------------------------------------

const severityColor = (s: Severity): string => {
  switch (s) {
    case "Critical": return "var(--accent-rose)";
    case "High": return "var(--accent-gold)";
    case "Medium": return "var(--accent-gold)";
    case "Low": return "var(--info-color)";
    case "Info": return "var(--text-secondary)";
  }
};

const severityOrder: Record<Severity, number> = { Critical: 0, High: 1, Medium: 2, Low: 3, Info: 4 };

/** Group an array by a key function. */
function groupBy<T>(items: T[], keyFn: (item: T) => string): Record<string, T[]> {
  const result: Record<string, T[]> = {};
  for (const item of items) {
    const key = keyFn(item);
    (result[key] ??= []).push(item);
  }
  return result;
}

// -- Component ----------------------------------------------------------------

const SecurityScanPanel: React.FC<SecurityScanPanelProps> = ({ workspacePath, onOpenFile }) => {
  const [tab, setTab] = useState<TabName>("Findings");
  const [findings, setFindings] = useState<Finding[]>([]);
  const [patterns, setPatterns] = useState<ScanPattern[]>(DEFAULT_PATTERNS);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [scanHistory, setScanHistory] = useState<ScanRun[]>([]);
  const [filterSeverity, setFilterSeverity] = useState<Severity | "All">("All");
  const [searchQuery, setSearchQuery] = useState("");
  const [lastScanTime, setLastScanTime] = useState<string | null>(null);
  const [groupMode, setGroupMode] = useState<GroupMode>("cwe");
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(new Set());
  const [patternClassFilter, setPatternClassFilter] = useState<string>("All");
  const [patternSearch, setPatternSearch] = useState("");

  const tabs: TabName[] = ["Findings", "Summary", "Patterns", "History"];

  useEffect(() => {
    loadScanResults();
    loadScanHistory();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workspacePath]);

  async function loadScanResults() {
    if (!workspacePath) return;
    try {
      const result = await invoke<Finding[]>("get_security_scan_results", { workspacePath });
      if (result.length > 0) setFindings(result);
    } catch {
      // No previous results
    }
  }

  async function loadScanHistory() {
    if (!workspacePath) return;
    try {
      const result = await invoke<ScanRun[]>("get_security_scan_history", { workspacePath });
      setScanHistory(result);
    } catch {
      // No history yet
    }
  }

  async function runScan() {
    if (!workspacePath) {
      setError("Open a workspace folder first.");
      return;
    }
    setScanning(true);
    setError(null);
    const startTime = Date.now();
    try {
      const enabledPatterns = patterns.filter((p) => p.enabled).map((p) => p.id);
      const result = await invoke<Finding[]>("run_security_scan", {
        workspacePath,
        patternIds: enabledPatterns,
      });
      setFindings(result);
      setLastScanTime(new Date().toLocaleString());

      // Update pattern match counts using cweIds for accurate mapping
      setPatterns((prev) =>
        prev.map((p) => ({
          ...p,
          matchCount: result.filter((f) => p.cweIds.includes(f.cwe)).length,
        }))
      );

      // Add to history
      const elapsed = ((Date.now() - startTime) / 1000).toFixed(1);
      setScanHistory((prev) => [
        { id: `scan-${Date.now()}`, timestamp: new Date().toLocaleString(), findingCount: result.length, duration: `${elapsed}s` },
        ...prev.slice(0, 19),
      ]);
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  }

  // Persist suppress/unsuppress to backend
  async function toggleSuppress(f: Finding) {
    const newSuppressed = !f.suppressed;
    setFindings((prev) => prev.map((x) => x.id === f.id ? { ...x, suppressed: newSuppressed } : x));
    try {
      await invoke("suppress_security_finding", {
        cwe: f.cwe,
        file: f.file,
        line: f.line,
        reason: "Suppressed via Security Scanner panel",
      });
    } catch {
      // Revert on failure
      setFindings((prev) => prev.map((x) => x.id === f.id ? { ...x, suppressed: !newSuppressed } : x));
    }
  }

  // Suppress all findings for a CWE project-wide
  async function suppressCwe(cwe: string) {
    setFindings((prev) => prev.map((f) => f.cwe === cwe ? { ...f, suppressed: true } : f));
    try {
      await invoke("suppress_security_cwe", {
        cwe,
        reason: `All ${cwe} findings suppressed via Security Scanner panel`,
      });
    } catch {
      // Revert on failure
      setFindings((prev) => prev.map((f) => f.cwe === cwe ? { ...f, suppressed: false } : f));
    }
  }

  const togglePattern = (id: string) => {
    setPatterns((prev) => prev.map((p) => p.id === id ? { ...p, enabled: !p.enabled } : p));
  };

  const toggleGroup = (key: string) => {
    setCollapsedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key); else next.add(key);
      return next;
    });
  };

  const activeFindings = findings.filter((f) => !f.suppressed);
  const suppressedFindings = findings.filter((f) => f.suppressed);

  const filteredFindings = activeFindings
    .filter((f) => filterSeverity === "All" || f.severity === filterSeverity)
    .filter((f) => {
      if (!searchQuery) return true;
      const q = searchQuery.toLowerCase();
      return f.title.toLowerCase().includes(q) || f.file.toLowerCase().includes(q) || f.cwe.toLowerCase().includes(q);
    })
    .sort((a, b) => severityOrder[a.severity] - severityOrder[b.severity]);

  const countBySeverity = (sev: Severity) => activeFindings.filter((f) => f.severity === sev).length;

  const handleFileClick = (file: string, line: number) => {
    if (onOpenFile && workspacePath) {
      const fullPath = file.startsWith("/") ? file : `${workspacePath}/${file}`;
      onOpenFile(fullPath, line);
    }
  };

  // Group findings for display
  const groupedFindings: [string, Finding[]][] = groupMode === "none"
    ? [["", filteredFindings]]
    : Object.entries(groupBy(filteredFindings, (f) => {
        if (groupMode === "cwe") return f.cwe;
        if (groupMode === "file") return f.file;
        return f.severity;
      })).sort((a, b) => {
        // Sort groups: by severity of worst finding, then alphabetically
        if (groupMode === "severity") return severityOrder[a[0] as Severity] - severityOrder[b[0] as Severity];
        const aWorst = Math.min(...a[1].map((f) => severityOrder[f.severity]));
        const bWorst = Math.min(...b[1].map((f) => severityOrder[f.severity]));
        return aWorst !== bWorst ? aWorst - bWorst : a[0].localeCompare(b[0]);
      });

  // Group suppressed findings by CWE
  const suppressedByCwe = groupBy(suppressedFindings, (f) => f.cwe);

  // Render a single finding row
  const renderFinding = (f: Finding) => (
    <div
      key={f.id}
      style={{
        borderRadius: "var(--radius-sm)", background: "var(--bg-tertiary)",
        borderLeft: `3px solid ${severityColor(f.severity)}`,
        border: `1px solid ${severityColor(f.severity)}44`,
      }}
    >
      <div role="button" tabIndex={0}
        onClick={() => setExpandedId(expandedId === f.id ? null : f.id)}
        style={{ padding: "8px 12px", cursor: "pointer", display: "flex", alignItems: "flex-start", gap: 8 }}
      >
        <span style={{
          fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: 3,
          background: `${severityColor(f.severity)}22`, color: severityColor(f.severity),
          fontWeight: 600, flexShrink: 0, marginTop: 1,
        }}>
          {f.severity}
        </span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>{f.title}</div>
          <div style={{ display: "flex", gap: 8, marginTop: 3, flexWrap: "wrap", alignItems: "center" }}>
            <span
              onClick={(e) => { e.stopPropagation(); handleFileClick(f.file, f.line); }}
              style={{
                fontSize: "var(--font-size-xs)", color: "var(--accent-blue)", fontFamily: "var(--font-mono)",
                cursor: onOpenFile ? "pointer" : "default",
                textDecoration: onOpenFile ? "underline" : "none",
              }}
              title="Open in editor"
            >
              {f.file}:{f.line}
            </span>
            {groupMode !== "cwe" && (
              <span style={{ fontSize: "var(--font-size-xs)", padding: "1px 4px", borderRadius: 3, background: "var(--bg-secondary)", color: "var(--text-secondary)" }}>
                {f.cwe}
              </span>
            )}
          </div>
        </div>
        <button
          onClick={(e) => { e.stopPropagation(); toggleSuppress(f); }}
          style={{
            padding: "2px 8px", fontSize: "var(--font-size-xs)", borderRadius: 3,
            border: "1px solid var(--border-color)", background: "none",
            color: "var(--text-secondary)", cursor: "pointer", flexShrink: 0,
          }}
        >
          Suppress
        </button>
      </div>

      {expandedId === f.id && (
        <div style={{ borderTop: "1px solid var(--bg-secondary)", padding: "12px 12px", display: "flex", flexDirection: "column", gap: 8 }}>
          <div>
            <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-secondary)", marginBottom: 3 }}>PROBLEM</div>
            <div style={{ fontSize: "var(--font-size-base)", lineHeight: 1.6 }}>{f.description}</div>
          </div>
          {f.remediation && (
            <div>
              <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-secondary)", marginBottom: 3 }}>REMEDIATION</div>
              <div style={{ fontSize: "var(--font-size-base)", lineHeight: 1.6, color: "var(--success-color)" }}>{f.remediation}</div>
            </div>
          )}
        </div>
      )}
    </div>
  );

  return (
    <div className="panel-container">
      {/* Header */}
      <div className="panel-header">
        <h3>Security Scanner</h3>
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
          {lastScanTime ? `Last scan: ${lastScanTime}` : "Static analysis for common vulnerabilities"}
        </div>
        <button
          onClick={runScan}
          disabled={scanning || !workspacePath}
          className={`panel-btn ${scanning ? "panel-btn-secondary" : "panel-btn-primary"}`}
          style={{ marginLeft: "auto" }}
        >
          {scanning ? "Scanning..." : "Run Scan"}
        </button>
      </div>

      <div className="panel-body">
      {error && (
        <div className="panel-error" style={{ display: "flex", justifyContent: "space-between" }}>
          <span>{error}</span>
          <button aria-label="Dismiss error" onClick={() => setError(null)} style={{ background: "none", border: "none", cursor: "pointer" }}>x</button>
        </div>
      )}

      {/* Severity badges + group toggle */}
      {activeFindings.length > 0 && (
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", alignItems: "center" }}>
          {(["Critical", "High", "Medium", "Low", "Info"] as Severity[]).map((sev) => {
            const count = countBySeverity(sev);
            if (count === 0) return null;
            return (
              <button
                key={sev}
                onClick={() => setFilterSeverity(filterSeverity === sev ? "All" : sev)}
                style={{
                  padding: "2px 8px", borderRadius: "var(--radius-xs-plus)",
                  border: `1px solid ${severityColor(sev)}`,
                  background: filterSeverity === sev ? `${severityColor(sev)}33` : "transparent",
                  color: severityColor(sev), cursor: "pointer", fontSize: "var(--font-size-sm)", fontWeight: 600,
                }}
              >
                {count} {sev}
              </button>
            );
          })}
          {filterSeverity !== "All" && (
            <button
              onClick={() => setFilterSeverity("All")}
              style={{ padding: "2px 8px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", background: "transparent", color: "var(--text-secondary)", cursor: "pointer", fontSize: "var(--font-size-sm)" }}
            >
              Clear filter
            </button>
          )}
          <span style={{ flex: 1 }} />
          <select
            value={groupMode}
            onChange={(e) => { setGroupMode(e.target.value as GroupMode); setCollapsedGroups(new Set()); }}
            style={{
              padding: "2px 8px", fontSize: "var(--font-size-xs)", borderRadius: 3,
              background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
              color: "var(--text-secondary)", cursor: "pointer",
            }}
          >
            <option value="cwe">Group by CWE</option>
            <option value="severity">Group by Severity</option>
            <option value="file">Group by File</option>
            <option value="none">No grouping</option>
          </select>
        </div>
      )}

      {/* Tab bar */}
      <div className="panel-tab-bar">
        {tabs.map((t) => (
          <button className={`panel-tab${tab === t ? " active" : ""}`} key={t} onClick={() => setTab(t)}>
            {t} {t === "Findings" && activeFindings.length > 0 ? `(${filteredFindings.length})` : ""}
          </button>
        ))}
      </div>

      {/* Content area - nested scroll within panel-body */}
      <div style={{ flex: 1, overflowY: "auto" }}>
        {/* Findings Tab */}
        {tab === "Findings" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {/* Search */}
            {activeFindings.length > 0 && (
              <input
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search findings by title, file, or CWE..."
                className="panel-input panel-input-full"
                style={{ marginBottom: 4 }}
              />
            )}

            {scanning && (
              <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
                Scanning workspace for vulnerabilities...<br />
                <span style={{ fontSize: "var(--font-size-sm)", opacity: 0.7 }}>Checking {patterns.filter((p) => p.enabled).length} patterns</span>
              </div>
            )}

            {!scanning && findings.length === 0 && (
              <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: "var(--font-size-md)", lineHeight: 1.7 }}>
                No scan results yet.<br />
                Click <strong>Run Scan</strong> to analyze your workspace for security issues.
              </div>
            )}

            {/* Grouped findings */}
            {groupedFindings.map(([groupKey, groupFindings]) => (
              <div key={groupKey || "__ungrouped"}>
                {groupKey && (
                  <div role="button" tabIndex={0}
                    onClick={() => toggleGroup(groupKey)}
                    style={{
                      display: "flex", alignItems: "center", gap: 8, padding: "8px 8px",
                      cursor: "pointer", userSelect: "none", marginTop: 4, marginBottom: 2,
                      background: "var(--bg-secondary)", borderRadius: 5,
                    }}
                  >
                    <span style={{ fontSize: "var(--font-size-xs)", opacity: 0.6 }}>
                      {collapsedGroups.has(groupKey) ? "\u25B6" : "\u25BC"}
                    </span>
                    <span style={{ fontWeight: 600, fontSize: "var(--font-size-base)", flex: 1 }}>
                      {groupMode === "cwe" ? `${groupKey} — ${CWE_NAMES[groupKey] || "Unknown"}` : groupKey}
                    </span>
                    <span style={{
                      fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: "var(--radius-md)",
                      background: "var(--bg-tertiary)", color: "var(--text-secondary)", fontWeight: 600,
                    }}>
                      {groupFindings.length}
                    </span>
                    {groupMode === "cwe" && (
                      <button
                        onClick={(e) => { e.stopPropagation(); suppressCwe(groupKey); }}
                        style={{
                          padding: "2px 8px", fontSize: "var(--font-size-xs)", borderRadius: 3,
                          border: "1px solid var(--border-color)", background: "none",
                          color: "var(--text-secondary)", cursor: "pointer",
                        }}
                        title={`Suppress all ${groupKey} findings`}
                      >
                        Suppress All
                      </button>
                    )}
                  </div>
                )}
                {!collapsedGroups.has(groupKey) && groupFindings.map(renderFinding)}
              </div>
            ))}

            {/* Suppressed findings section — grouped by CWE */}
            {suppressedFindings.length > 0 && (
              <div style={{ marginTop: 12, padding: "8px 12px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)" }}>
                <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, color: "var(--text-secondary)", marginBottom: 6 }}>
                  {suppressedFindings.length} suppressed finding(s)
                </div>
                {Object.entries(suppressedByCwe)
                  .sort((a, b) => a[0].localeCompare(b[0]))
                  .map(([cwe, cweFindngs]) => (
                  <div key={cwe} style={{ marginBottom: 6 }}>
                    <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-secondary)", padding: "4px 0 2px" }}>
                      {cwe} — {CWE_NAMES[cwe] || "Unknown"} ({cweFindngs.length})
                    </div>
                    {cweFindngs.slice(0, 5).map((f) => (
                      <div key={f.id} style={{ display: "flex", alignItems: "center", gap: 8, padding: "2px 0" }}>
                        <span style={{ textDecoration: "line-through", fontSize: "var(--font-size-sm)", flex: 1, opacity: 0.6 }}>
                          {f.file}:{f.line}
                        </span>
                        <button
                          onClick={() => toggleSuppress(f)}
                          style={{ padding: "2px 8px", fontSize: "var(--font-size-xs)", borderRadius: 3, border: "1px solid var(--border-color)", background: "none", color: "var(--text-secondary)", cursor: "pointer" }}
                        >
                          Restore
                        </button>
                      </div>
                    ))}
                    {cweFindngs.length > 5 && (
                      <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", padding: "2px 0" }}>
                        ...and {cweFindngs.length - 5} more
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Summary Tab */}
        {tab === "Summary" && (
          <div>
            <div style={{ display: "flex", gap: 12, flexWrap: "wrap", marginBottom: 16 }}>
              {[
                { label: "Total", value: findings.length, color: "var(--text-primary)" },
                { label: "Active", value: activeFindings.length, color: "var(--success-color)" },
                { label: "Suppressed", value: suppressedFindings.length, color: "var(--text-secondary)" },
              ].map(({ label, value, color }) => (
                <div key={label} style={{ background: "var(--bg-tertiary)", padding: "12px 16px", borderRadius: "var(--radius-sm)", textAlign: "center", minWidth: 80, border: "1px solid var(--border-color)" }}>
                  <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color }}>{value}</div>
                  <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 2 }}>{label}</div>
                </div>
              ))}
            </div>

            <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 8 }}>Severity Breakdown</div>
            {(["Critical", "High", "Medium", "Low", "Info"] as Severity[]).map((sev) => {
              const count = countBySeverity(sev);
              const pct = activeFindings.length > 0 ? Math.round((count / activeFindings.length) * 100) : 0;
              return (
                <div key={sev} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                  <span style={{ minWidth: 55, fontSize: "var(--font-size-base)", color: severityColor(sev), fontWeight: 500 }}>{sev}</span>
                  <div style={{ flex: 1, background: "var(--bg-tertiary)", borderRadius: 3, height: 10, overflow: "hidden" }}>
                    <div style={{ width: `${pct}%`, height: "100%", background: severityColor(sev), borderRadius: 3, transition: "width 0.3s" }} />
                  </div>
                  <span style={{ minWidth: 25, textAlign: "right", fontSize: "var(--font-size-sm)" }}>{count}</span>
                  <span style={{ minWidth: 35, textAlign: "right", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{pct}%</span>
                </div>
              );
            })}

            {/* CWE Breakdown */}
            {activeFindings.length > 0 && (
              <div style={{ marginTop: 16 }}>
                <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 8 }}>By CWE Category</div>
                {Object.entries(groupBy(activeFindings, (f) => f.cwe))
                  .sort((a, b) => b[1].length - a[1].length)
                  .map(([cwe, items]) => (
                    <div key={cwe} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 0", fontSize: "var(--font-size-sm)" }}>
                      <span style={{ fontWeight: 600, minWidth: 60 }}>{cwe}</span>
                      <span style={{ flex: 1, color: "var(--text-secondary)" }}>{CWE_NAMES[cwe] || "Unknown"}</span>
                      <span style={{ fontSize: "var(--font-size-xs)", padding: "1px 8px", borderRadius: "var(--radius-md)", background: "var(--bg-tertiary)", color: "var(--text-secondary)", fontWeight: 600 }}>
                        {items.length}
                      </span>
                    </div>
                  ))}
              </div>
            )}

            {/* Top affected files */}
            {activeFindings.length > 0 && (
              <div style={{ marginTop: 16 }}>
                <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 8 }}>Most Affected Files</div>
                {Object.entries(
                  activeFindings.reduce<Record<string, number>>((acc, f) => { acc[f.file] = (acc[f.file] || 0) + 1; return acc; }, {})
                )
                  .sort((a, b) => b[1] - a[1])
                  .slice(0, 5)
                  .map(([file, count]) => (
                    <div key={file} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 0", fontSize: "var(--font-size-sm)" }}>
                      <span
                        style={{ flex: 1, fontFamily: "var(--font-mono)", color: "var(--accent-blue)", cursor: onOpenFile ? "pointer" : "default" }}
                        onClick={() => handleFileClick(file, 1)}
                      >
                        {file}
                      </span>
                      <span style={{ fontSize: "var(--font-size-xs)", padding: "1px 8px", borderRadius: "var(--radius-md)", background: "var(--bg-tertiary)", color: "var(--text-secondary)", fontWeight: 600 }}>
                        {count}
                      </span>
                    </div>
                  ))}
              </div>
            )}
          </div>
        )}

        {/* Patterns Tab */}
        {tab === "Patterns" && (() => {
          const visiblePatterns = patterns.filter((p) => {
            const matchClass = patternClassFilter === "All" || p.vulnerabilityClass === patternClassFilter;
            const matchSearch = !patternSearch || p.name.toLowerCase().includes(patternSearch.toLowerCase()) || p.cweIds.some(c => c.toLowerCase().includes(patternSearch.toLowerCase()));
            return matchClass && matchSearch;
          });
          const enabledCount = patterns.filter((p) => p.enabled).length;
          return (
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              {/* Stats + bulk controls */}
              <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                  {enabledCount}/{patterns.length} enabled
                </span>
                <span style={{ flex: 1 }} />
                <button
                  onClick={() => setPatterns((prev) => prev.map((p) => ({ ...p, enabled: true })))}
                  style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: 3, border: "1px solid var(--border-color)", background: "none", color: "var(--text-secondary)", cursor: "pointer" }}
                >
                  Enable All
                </button>
                <button
                  onClick={() => setPatterns((prev) => prev.map((p) => ({ ...p, enabled: false })))}
                  style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: 3, border: "1px solid var(--border-color)", background: "none", color: "var(--text-secondary)", cursor: "pointer" }}
                >
                  Disable All
                </button>
              </div>

              {/* Search + class filter */}
              <div style={{ display: "flex", gap: 6 }}>
                <input
                  value={patternSearch}
                  onChange={(e) => setPatternSearch(e.target.value)}
                  placeholder="Search patterns or CWE…"
                  className="panel-input"
                  style={{ flex: 1, fontSize: "var(--font-size-sm)" }}
                />
                <select
                  value={patternClassFilter}
                  onChange={(e) => setPatternClassFilter(e.target.value)}
                  style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: 3, background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-secondary)", cursor: "pointer" }}
                >
                  {VULN_CLASSES.map((c) => (
                    <option key={c} value={c}>{c}</option>
                  ))}
                </select>
              </div>

              {/* Pattern list */}
              {visiblePatterns.length === 0 && (
                <div style={{ textAlign: "center", padding: 16, color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>No patterns match the current filter.</div>
              )}
              {visiblePatterns.map((p) => (
                <div key={p.id} style={{
                  padding: "8px 12px", borderRadius: "var(--radius-sm)",
                  background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
                  opacity: p.enabled ? 1 : 0.5,
                }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <input type="checkbox" checked={p.enabled} onChange={() => togglePattern(p.id)} style={{ cursor: "pointer" }} />
                    <span style={{ fontWeight: 600, fontSize: "var(--font-size-base)", flex: 1 }}>{p.name}</span>
                    <span style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: 3, background: "var(--bg-secondary)", color: "var(--text-secondary)" }}>
                      {p.vulnerabilityClass}
                    </span>
                    {p.matchCount > 0 && (
                      <span style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: "var(--radius-md)", background: "var(--error-bg)", color: "var(--error-color)", fontWeight: 600 }}>
                        {p.matchCount}
                      </span>
                    )}
                  </div>
                  <div style={{ display: "flex", gap: 4, marginTop: 5, flexWrap: "wrap" }}>
                    {p.cweIds.map((cwe) => (
                      <span key={cwe} style={{ fontSize: 9, padding: "1px 4px", borderRadius: 3, background: "var(--bg-secondary)", color: "var(--accent-blue)", fontFamily: "var(--font-mono)" }}>
                        {cwe}
                      </span>
                    ))}
                  </div>
                  <div style={{ display: "flex", gap: 4, marginTop: 4, flexWrap: "wrap" }}>
                    {p.languages.map((lang) => (
                      <span key={lang} style={{ fontSize: "var(--font-size-xs)", padding: "1px 4px", borderRadius: 3, background: "var(--bg-secondary)", color: "var(--text-secondary)" }}>
                        {lang}
                      </span>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          );
        })()}

        {/* History Tab */}
        {tab === "History" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {scanHistory.length === 0 ? (
              <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
                No scan history yet. Run a scan to start tracking.
              </div>
            ) : (
              scanHistory.map((run) => (
                <div key={run.id} style={{
                  padding: "8px 12px", borderRadius: "var(--radius-sm)", background: "var(--bg-tertiary)",
                  border: "1px solid var(--border-color)", display: "flex", alignItems: "center", gap: 12,
                }}>
                  <div style={{ flex: 1 }}>
                    <div style={{ fontSize: "var(--font-size-base)", fontWeight: 500 }}>{run.timestamp}</div>
                  </div>
                  <span style={{
                    fontSize: "var(--font-size-sm)", padding: "2px 8px", borderRadius: "var(--radius-md)",
                    background: run.findingCount > 0 ? "color-mix(in srgb, var(--accent-rose) 13%, transparent)" : "color-mix(in srgb, var(--accent-green) 13%, transparent)",
                    color: run.findingCount > 0 ? "var(--accent-rose)" : "var(--accent-green)",
                    fontWeight: 600,
                  }}>
                    {run.findingCount} findings
                  </span>
                  <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{run.duration}</span>
                </div>
              ))
            )}
          </div>
        )}
      </div>

      {/* Footer */}
      {findings.length > 0 && (
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", flexShrink: 0 }}>
          {activeFindings.length} active issue{activeFindings.length !== 1 ? "s" : ""} — click to expand, file links open in editor
        </div>
      )}
      </div>
    </div>
  );
};

export default SecurityScanPanel;
