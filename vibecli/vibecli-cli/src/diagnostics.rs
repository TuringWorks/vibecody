//! Diagnostics bundle generation.
//!
//! `vibecli --diagnostics` collects environment, config, and version info
//! into a single report to aid bug reports and support.

use anyhow::Result;

/// Generate a diagnostics bundle and print it to stdout.
/// `resume_id` is an optional session ID to include in the report.
pub async fn generate_bundle(resume_id: Option<&str>) -> Result<()> {
    println!("=== VibeCLI Diagnostics Bundle ===\n");

    // Version
    println!("Version: {}", env!("CARGO_PKG_VERSION"));

    // OS / platform
    println!("OS:      {}", std::env::consts::OS);
    println!("Arch:    {}", std::env::consts::ARCH);

    // Relevant environment variables (keys only, not values for secrets)
    let env_vars = [
        "OPENAI_API_KEY", "ANTHROPIC_API_KEY", "GEMINI_API_KEY",
        "GROK_API_KEY", "GROQ_API_KEY", "OLLAMA_HOST",
        "HOME", "PATH", "VIBECLI_CONFIG",
    ];
    println!("\nEnvironment:");
    for var in &env_vars {
        let present = std::env::var(var).is_ok();
        println!("  {}: {}", var, if present { "set" } else { "not set" });
    }

    // Config path
    let config_path = {
        let home = std::env::var("HOME").unwrap_or_default();
        std::path::PathBuf::from(home).join(".vibecli").join("config.toml")
    };
    println!("\nConfig: {} ({})", config_path.display(),
        if config_path.exists() { "found" } else { "not found" });

    // Session DB
    let db_path = crate::session_store::default_db_path();
    println!("Session DB: {} ({})", db_path.display(),
        if db_path.exists() { "found" } else { "not found" });

    if let Some(id) = resume_id {
        println!("\nResume session: {}", id);
    }

    println!("\n=== End of Diagnostics ===");
    Ok(())
}
