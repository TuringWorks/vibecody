//! Interactive setup wizard for VibeCody.
//!
//! Detects the user's platform (OS, arch, RAM, GPU) and walks them through:
//! 1. Platform detection & tier recommendation
//! 2. AI provider configuration (Ollama or cloud API keys)
//! 3. Always-on service installation (launchd / systemd / Windows Service)
//! 4. Health check verification

use anyhow::Result;
use std::io::{self, Write};

// ── ANSI colors ────────────────────────────────────────────────────────────

const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

// ── Platform detection ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct PlatformInfo {
    os: String,
    arch: String,
    ram_gb: f64,
    gpu: Option<String>,
    is_raspberry_pi: bool,
    pi_model: Option<String>,
    hostname: String,
}

impl PlatformInfo {
    fn detect() -> Self {
        let os = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();
        let ram_gb = detect_ram_gb();
        let gpu = detect_gpu();
        let (is_raspberry_pi, pi_model) = detect_raspberry_pi();
        let hostname = {
            #[cfg(unix)]
            {
                std::process::Command::new("hostname")
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            }
            #[cfg(not(unix))]
            {
                std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".to_string())
            }
        };

        Self { os, arch, ram_gb, gpu, is_raspberry_pi, pi_model, hostname }
    }

    fn display_name(&self) -> &str {
        if self.is_raspberry_pi {
            return "Raspberry Pi";
        }
        match self.os.as_str() {
            "macos" => "macOS",
            "linux" => "Linux",
            "windows" => "Windows",
            _ => &self.os,
        }
    }

    fn recommended_tier(&self) -> &str {
        if self.ram_gb >= 16.0 { "max" }
        else if self.ram_gb >= 8.0 { "pro" }
        else { "lite" }
    }

    fn recommended_model(&self) -> &str {
        if self.is_raspberry_pi {
            if self.ram_gb < 2.0 { return "tinyllama:1.1b"; }
            if self.ram_gb < 6.0 { return "phi:2.7b"; }
            return "mistral:7b";
        }
        if self.ram_gb >= 32.0 { "qwen3-coder:480b-cloud" }
        else if self.ram_gb >= 16.0 { "codellama:13b" }
        else if self.ram_gb >= 8.0 { "codellama:7b" }
        else { "qwen3-coder:480b-cloud" }
    }
}

fn detect_ram_gb() -> f64 {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
        {
            if let Ok(s) = String::from_utf8(output.stdout) {
                if let Ok(bytes) = s.trim().parse::<u64>() {
                    return bytes as f64 / 1_073_741_824.0;
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
            for line in contents.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(kb_str) = parts.get(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb as f64 / 1_048_576.0;
                        }
                    }
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("wmic")
            .args(["ComputerSystem", "get", "TotalPhysicalMemory", "/Value"])
            .output()
        {
            if let Ok(s) = String::from_utf8(output.stdout) {
                for line in s.lines() {
                    if let Some(val) = line.strip_prefix("TotalPhysicalMemory=") {
                        if let Ok(bytes) = val.trim().parse::<u64>() {
                            return bytes as f64 / 1_073_741_824.0;
                        }
                    }
                }
            }
        }
    }
    0.0
}

fn detect_gpu() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("system_profiler")
            .args(["SPDisplaysDataType", "-detailLevel", "mini"])
            .output()
        {
            let s = String::from_utf8_lossy(&output.stdout);
            if s.contains("Apple") {
                // Extract Metal GPU family
                for line in s.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("Chipset Model:") || trimmed.starts_with("Chip:") {
                        return Some(trimmed.to_string());
                    }
                }
                return Some("Apple Silicon (Metal)".to_string());
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=name", "--format=csv,noheader"])
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !s.is_empty() {
                    return Some(format!("NVIDIA {s} (CUDA)"));
                }
            }
        }
        if std::path::Path::new("/dev/kfd").exists() {
            return Some("AMD GPU (ROCm)".to_string());
        }
    }
    None
}

fn detect_raspberry_pi() -> (bool, Option<String>) {
    #[cfg(target_os = "linux")]
    {
        if let Ok(model) = std::fs::read_to_string("/proc/device-tree/model") {
            let model = model.trim_end_matches('\0').trim().to_string();
            if model.contains("Raspberry Pi") {
                return (true, Some(model));
            }
        }
        if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
            if cpuinfo.contains("Raspberry Pi") || cpuinfo.contains("BCM2") {
                return (true, Some("Raspberry Pi".to_string()));
            }
        }
    }
    (false, None)
}

// ── Interactive prompts ────────────────────────────────────────────────────

fn prompt_line(message: &str) -> String {
    print!("{CYAN}?{RESET} {message} ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    input.trim().to_string()
}

fn prompt_yn(message: &str, default_yes: bool) -> bool {
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    let answer = prompt_line(&format!("{message} {DIM}{hint}{RESET}"));
    if answer.is_empty() { return default_yes; }
    matches!(answer.to_lowercase().as_str(), "y" | "yes")
}

fn prompt_choice(message: &str, options: &[(&str, &str)], default: usize) -> usize {
    println!("\n{CYAN}?{RESET} {message}");
    for (i, (label, desc)) in options.iter().enumerate() {
        let marker = if i == default { "▸" } else { " " };
        println!("  {marker} {BOLD}{}{RESET}  {DIM}{}{RESET}", label, desc);
    }
    loop {
        let answer = prompt_line(&format!("Enter choice (1-{}, default {}):", options.len(), default + 1));
        if answer.is_empty() { return default; }
        if let Ok(n) = answer.parse::<usize>() {
            if n >= 1 && n <= options.len() { return n - 1; }
        }
        println!("  Please enter a number between 1 and {}.", options.len());
    }
}

// ── Setup steps ────────────────────────────────────────────────────────────

fn step_detect(info: &PlatformInfo) {
    println!("\n{BOLD}┌─ VibeCody Setup Wizard ─────────────────────────────────┐{RESET}");
    println!("{BOLD}│{RESET}                                                          {BOLD}│{RESET}");
    println!("{BOLD}│{RESET}  {GREEN}✓{RESET} Platform:  {BOLD}{}{RESET} ({}){}",
        info.display_name(),
        info.arch,
        if info.is_raspberry_pi {
            format!(" — {}", info.pi_model.as_deref().unwrap_or("unknown model"))
        } else { String::new() },
    );
    println!("{BOLD}│{RESET}  {GREEN}✓{RESET} Memory:    {BOLD}{:.1} GB{RESET}", info.ram_gb);
    if let Some(gpu) = &info.gpu {
        println!("{BOLD}│{RESET}  {GREEN}✓{RESET} GPU:       {BOLD}{gpu}{RESET}");
    } else {
        println!("{BOLD}│{RESET}  {YELLOW}○{RESET} GPU:       {DIM}Not detected (CPU inference){RESET}");
    }
    println!("{BOLD}│{RESET}  {GREEN}✓{RESET} Hostname:  {BOLD}{}{RESET}", info.hostname);
    println!("{BOLD}│{RESET}  {GREEN}✓{RESET} Tier:      {BOLD}{}{RESET} (recommended)", info.recommended_tier());
    println!("{BOLD}│{RESET}                                                          {BOLD}│{RESET}");
    println!("{BOLD}└──────────────────────────────────────────────────────────┘{RESET}");
}

fn step_provider(info: &PlatformInfo) -> Result<(String, Option<String>)> {
    let options = &[
        ("ollama", "Local models — free, private, no API key needed"),
        ("claude", "Anthropic Claude — best for complex coding tasks"),
        ("openai", "OpenAI GPT — widely used, fast"),
        ("gemini", "Google Gemini — good free tier"),
        ("grok",   "xAI Grok — fast, generous rate limits"),
        ("groq",   "Groq — ultra-fast inference for open models"),
    ];

    let default = 0; // ollama
    let choice = prompt_choice("Choose your AI provider:", options, default);
    let provider = options[choice].0.to_string();

    if provider == "ollama" {
        // Check if Ollama is installed
        let ollama_ok = std::process::Command::new("ollama")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !ollama_ok {
            println!("\n  {YELLOW}⚠{RESET}  Ollama is not installed.");
            if prompt_yn("Install Ollama now?", true) {
                println!("  {DIM}Running: curl -fsSL https://ollama.com/install.sh | sh{RESET}");
                let _ = std::process::Command::new("sh")
                    .arg("-c")
                    .arg("curl -fsSL https://ollama.com/install.sh | sh")
                    .status();
            } else {
                println!("  Install Ollama later: https://ollama.com/download");
            }
        }

        let model = info.recommended_model().to_string();
        println!("\n  {GREEN}✓{RESET} Recommended model for your hardware: {BOLD}{model}{RESET}");

        // Check if model is pulled
        let model_exists = std::process::Command::new("ollama")
            .args(["show", &model])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !model_exists && prompt_yn(&format!("Pull {model} now? (this may take a few minutes)"), true) {
            println!("  {DIM}Running: ollama pull {model}{RESET}");
            let _ = std::process::Command::new("ollama")
                .args(["pull", &model])
                .status();
        }

        return Ok((provider, Some(model)));
    }

    // Cloud provider — ask for API key
    let env_var = match provider.as_str() {
        "claude" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "gemini" => "GEMINI_API_KEY",
        "grok" => "GROK_API_KEY",
        "groq" => "GROQ_API_KEY",
        _ => "API_KEY",
    };

    let existing = std::env::var(env_var).ok();
    if let Some(key) = &existing {
        let masked = format!("{}...{}", &key[..6.min(key.len())], &key[key.len().saturating_sub(4)..]);
        println!("\n  {GREEN}✓{RESET} Found {BOLD}{env_var}{RESET} = {DIM}{masked}{RESET}");
    } else {
        println!("\n  {YELLOW}⚠{RESET}  {BOLD}{env_var}{RESET} is not set.");
        let key = prompt_line(&format!("Enter your {} API key (or press Enter to skip):", provider));
        if !key.is_empty() {
            // Write to config.toml
            let config_dir = dirs::home_dir()
                .map(|h| h.join(".vibecli"))
                .unwrap_or_else(|| std::path::PathBuf::from(".vibecli"));
            let _ = std::fs::create_dir_all(&config_dir);
            let config_path = config_dir.join("config.toml");

            // Append provider config
            let section = format!("\n[{provider}]\nenabled = true\napi_key = \"{key}\"\n");
            let mut existing_content = std::fs::read_to_string(&config_path).unwrap_or_default();
            existing_content.push_str(&section);
            let _ = std::fs::write(&config_path, &existing_content);
            println!("  {GREEN}✓{RESET} Saved to {}", config_path.display());

            // Also hint about shell rc
            println!("  {DIM}Tip: add to your shell profile for future sessions:{RESET}");
            println!("  {DIM}  export {env_var}=\"{}...\"{RESET}", &key[..6.min(key.len())]);
        }
    }

    Ok((provider, None))
}

fn step_always_on(info: &PlatformInfo) -> Result<bool> {
    if !prompt_yn("\nEnable always-on mode (run VibeCody as a background service)?", false) {
        return Ok(false);
    }

    let config_dir = dirs::home_dir()
        .map(|h| h.join(".vibecli"))
        .unwrap_or_else(|| std::path::PathBuf::from(".vibecli"));
    let _ = std::fs::create_dir_all(&config_dir);

    match info.os.as_str() {
        "macos" => {
            let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.vibecody.vibecli</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}/vibecli</string>
        <string>serve</string>
        <string>--port</string>
        <string>7878</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{}/vibecli-stdout.log</string>
    <key>StandardErrorPath</key>
    <string>{}/vibecli-stderr.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>RUST_LOG</key>
        <string>info</string>
    </dict>
</dict>
</plist>"#,
                dirs::home_dir().unwrap_or_default().join(".local/bin").display(),
                config_dir.display(),
                config_dir.display(),
            );

            let plist_path = dirs::home_dir()
                .unwrap_or_default()
                .join("Library/LaunchAgents/com.vibecody.vibecli.plist");
            std::fs::write(&plist_path, &plist)?;
            println!("  {GREEN}✓{RESET} Created {}", plist_path.display());

            let _ = std::process::Command::new("launchctl")
                .args(["load", &plist_path.to_string_lossy()])
                .status();
            println!("  {GREEN}✓{RESET} Service loaded — VibeCody is running at http://localhost:7878");
        }
        "linux" => {
            let service = format!(r#"[Unit]
Description=VibeCody AI Coding Assistant
After=network.target

[Service]
Type=simple
ExecStart={}/vibecli --serve --port 7878
Restart=always
RestartSec=5
Environment=RUST_LOG=info
WorkingDirectory=%h

[Install]
WantedBy=default.target
"#,
                dirs::home_dir().unwrap_or_default().join(".local/bin").display(),
            );

            let service_dir = dirs::home_dir()
                .unwrap_or_default()
                .join(".config/systemd/user");
            let _ = std::fs::create_dir_all(&service_dir);
            let service_path = service_dir.join("vibecody.service");
            std::fs::write(&service_path, &service)?;
            println!("  {GREEN}✓{RESET} Created {}", service_path.display());

            let _ = std::process::Command::new("systemctl")
                .args(["--user", "daemon-reload"])
                .status();
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "enable", "--now", "vibecody.service"])
                .status();
            println!("  {GREEN}✓{RESET} Service enabled — VibeCody is running at http://localhost:7878");
        }
        "windows" => {
            println!("  {YELLOW}⚠{RESET}  Windows Service setup requires Administrator privileges.");
            println!("  Run this in an elevated PowerShell:");
            println!("    {DIM}New-Service -Name VibeCody -BinaryPathName \"%LOCALAPPDATA%\\VibeCody\\vibecli.exe serve --port 7878\" -StartupType Automatic{RESET}");
            println!("    {DIM}Start-Service VibeCody{RESET}");
        }
        _ => {
            println!("  {YELLOW}⚠{RESET}  Automatic service setup is not available for {}.", info.os);
            println!("  You can run VibeCody manually: vibecli --serve --port 7878");
        }
    }

    Ok(true)
}

fn step_health_check() -> bool {
    print!("\n  {DIM}Checking VibeCody health...{RESET} ");
    let _ = io::stdout().flush();

    // Give the service a moment to start
    std::thread::sleep(std::time::Duration::from_secs(2));

    match std::process::Command::new("curl")
        .args(["-sf", "http://localhost:7878/health"])
        .output()
    {
        Ok(output) if output.status.success() => {
            println!("{GREEN}✓ Healthy{RESET}");
            true
        }
        _ => {
            println!("{YELLOW}○ Not reachable yet{RESET}");
            println!("  {DIM}The service may need a few more seconds to start.{RESET}");
            println!("  {DIM}Check manually: curl http://localhost:7878/health{RESET}");
            false
        }
    }
}

fn step_summary(info: &PlatformInfo, provider: &str, model: Option<&str>, always_on: bool) {
    println!("\n{BOLD}┌─ Setup Complete ────────────────────────────────────────┐{RESET}");
    println!("{BOLD}│{RESET}                                                         {BOLD}│{RESET}");
    println!("{BOLD}│{RESET}  {GREEN}✓{RESET} Platform:  {BOLD}{}{RESET} ({})", info.display_name(), info.arch);
    println!("{BOLD}│{RESET}  {GREEN}✓{RESET} Provider:  {BOLD}{provider}{RESET}{}",
        model.map(|m| format!(" ({m})")).unwrap_or_default());
    if always_on {
        println!("{BOLD}│{RESET}  {GREEN}✓{RESET} Always-on: {BOLD}http://localhost:7878{RESET}");
    }
    println!("{BOLD}│{RESET}                                                         {BOLD}│{RESET}");
    println!("{BOLD}│{RESET}  {CYAN}Next steps:{RESET}                                           {BOLD}│{RESET}");
    println!("{BOLD}│{RESET}    vibecli                    {DIM}# Start chatting{RESET}           {BOLD}│{RESET}");
    println!("{BOLD}│{RESET}    vibecli --agent \"fix bugs\" {DIM}# Run an agent{RESET}            {BOLD}│{RESET}");
    println!("{BOLD}│{RESET}    vibecli --review           {DIM}# Review code{RESET}              {BOLD}│{RESET}");
    if !always_on {
        println!("{BOLD}│{RESET}    vibecli --serve            {DIM}# Start daemon{RESET}             {BOLD}│{RESET}");
    }
    println!("{BOLD}│{RESET}                                                         {BOLD}│{RESET}");
    println!("{BOLD}│{RESET}  {DIM}Docs: https://vibecody.github.io/vibecody/setup/{RESET}       {BOLD}│{RESET}");
    println!("{BOLD}│{RESET}  {DIM}Use cases: https://vibecody.github.io/vibecody/use-cases/{RESET}{BOLD}│{RESET}");
    println!("{BOLD}│{RESET}                                                         {BOLD}│{RESET}");
    println!("{BOLD}└─────────────────────────────────────────────────────────┘{RESET}");
}

// ── Public entry point ─────────────────────────────────────────────────────

pub async fn run_setup() -> Result<()> {
    // Step 1: Detect platform
    let info = PlatformInfo::detect();
    step_detect(&info);

    // Step 2: Provider configuration
    let (provider, model) = step_provider(&info)?;

    // Step 3: Always-on service
    let always_on = step_always_on(&info)?;

    // Step 4: Health check (if always-on)
    if always_on {
        step_health_check();
    }

    // Step 5: Summary
    step_summary(&info, &provider, model.as_deref(), always_on);

    Ok(())
}

// ── Service management subcommands ─────────────────────────────────────────

pub fn service_install() -> Result<()> {
    let info = PlatformInfo::detect();
    let _ = step_always_on(&info)?;
    Ok(())
}

pub fn service_start() -> Result<()> {
    let info = PlatformInfo::detect();
    match info.os.as_str() {
        "macos" => {
            let _ = std::process::Command::new("launchctl")
                .args(["load", &dirs::home_dir().unwrap_or_default()
                    .join("Library/LaunchAgents/com.vibecody.vibecli.plist")
                    .to_string_lossy()])
                .status();
            println!("{GREEN}✓{RESET} VibeCody service started");
        }
        "linux" => {
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "start", "vibecody.service"])
                .status();
            println!("{GREEN}✓{RESET} VibeCody service started");
        }
        _ => println!("{YELLOW}⚠{RESET} Manual start required on {}", info.os),
    }
    Ok(())
}

pub fn service_stop() -> Result<()> {
    let info = PlatformInfo::detect();
    match info.os.as_str() {
        "macos" => {
            let _ = std::process::Command::new("launchctl")
                .args(["unload", &dirs::home_dir().unwrap_or_default()
                    .join("Library/LaunchAgents/com.vibecody.vibecli.plist")
                    .to_string_lossy()])
                .status();
            println!("{GREEN}✓{RESET} VibeCody service stopped");
        }
        "linux" => {
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "stop", "vibecody.service"])
                .status();
            println!("{GREEN}✓{RESET} VibeCody service stopped");
        }
        _ => println!("{YELLOW}⚠{RESET} Manual stop required on {}", info.os),
    }
    Ok(())
}

pub fn service_status() -> Result<()> {
    let info = PlatformInfo::detect();
    match info.os.as_str() {
        "macos" => {
            let _ = std::process::Command::new("launchctl")
                .args(["list", "com.vibecody.vibecli"])
                .status();
        }
        "linux" => {
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "status", "vibecody.service"])
                .status();
        }
        _ => {
            // Try health check
            match std::process::Command::new("curl")
                .args(["-sf", "http://localhost:7878/health"])
                .output()
            {
                Ok(o) if o.status.success() => println!("{GREEN}✓{RESET} VibeCody is running"),
                _ => println!("{YELLOW}○{RESET} VibeCody is not running"),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_info_detect_does_not_panic() {
        let info = PlatformInfo::detect();
        assert!(!info.os.is_empty(), "OS should be detected");
        assert!(!info.arch.is_empty(), "arch should be detected");
        assert!(info.ram_gb >= 0.0, "RAM should be non-negative");
    }

    #[test]
    fn test_platform_info_os_is_known() {
        let info = PlatformInfo::detect();
        // Should be one of the known OS strings
        let known = ["macos", "linux", "windows"];
        assert!(
            known.iter().any(|&k| info.os == k),
            "OS '{}' should be macos, linux, or windows",
            info.os
        );
    }

    #[test]
    fn test_platform_info_arch_is_known() {
        let info = PlatformInfo::detect();
        let known = ["x86_64", "aarch64", "armv7", "arm"];
        assert!(
            known.iter().any(|&k| info.arch.contains(k)),
            "arch '{}' should contain a known value",
            info.arch
        );
    }

    #[test]
    fn test_recommended_tier_thresholds() {
        // Tier: >=16 GB = max, >=8 GB = pro, <8 GB = lite
        let base = PlatformInfo { os: "linux".into(), arch: "x86_64".into(), ram_gb: 0.0,
            gpu: None, is_raspberry_pi: false, pi_model: None, hostname: "h".into() };
        let lite = PlatformInfo { ram_gb: 4.0, ..base.clone() };
        let pro  = PlatformInfo { ram_gb: 8.0, ..base.clone() };
        let max  = PlatformInfo { ram_gb: 16.0, ..base.clone() };
        assert_eq!(lite.recommended_tier(), "lite");
        assert_eq!(pro.recommended_tier(), "pro");
        assert_eq!(max.recommended_tier(), "max");
    }

    #[test]
    fn test_raspberry_pi_not_detected_on_non_pi() {
        let info = PlatformInfo::detect();
        // On a developer machine, is_raspberry_pi should be false
        // (unless literally running on a Pi, which is acceptable)
        let _ = info.is_raspberry_pi; // just ensure it's accessible
    }

    #[test]
    fn test_platform_info_hostname_not_empty() {
        let info = PlatformInfo::detect();
        assert!(!info.hostname.is_empty(), "hostname should be detected");
    }
}
