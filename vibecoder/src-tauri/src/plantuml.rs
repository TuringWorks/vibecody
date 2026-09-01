//! Rendering PlantUML diagrams.
//!
//! Mermaid runs in the browser; PlantUML does not. It is a Java program, and
//! there is no port of it that could draw a diagram inside a webview. So this
//! module hands the source to whatever PlantUML the machine already has and
//! reads back SVG.
//!
//! Two things it deliberately does not do:
//!
//! * **Send the diagram anywhere.** The usual way to render PlantUML on the web
//!   is to encode the source into a URL and fetch it from `plantuml.com`. That
//!   is someone else's server reading your architecture, quietly, because a
//!   picture appeared. If PlantUML is not installed this says so and stops.
//! * **Show a picture of an error.** With `-pipe` PlantUML happily renders an
//!   SVG that *contains* the error text and exits 0. `-failfast2` makes a
//!   syntax error an error, which is what the caller can act on.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// How this machine can run PlantUML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Renderer {
    /// A `plantuml` launcher on `PATH`, or wherever one was found.
    Command(PathBuf),
    /// `java -jar plantuml.jar`, for a machine that has the jar but no wrapper.
    Jar { java: PathBuf, jar: PathBuf },
}

impl Renderer {
    /// What to show a person who asks what is being used.
    pub fn describe(&self) -> String {
        match self {
            Renderer::Command(path) => path.display().to_string(),
            Renderer::Jar { java, jar } => format!("{} -jar {}", java.display(), jar.display()),
        }
    }
}

/// Why a diagram could not be drawn. Each variant is a different thing for the
/// caller to say, which is the whole reason they are kept apart.
#[derive(Debug)]
pub enum PlantUmlError {
    /// Nothing on this machine can run PlantUML.
    NotInstalled,
    /// A jar was found but there is no Java to run it with.
    NoJava { jar: PathBuf },
    /// The source is larger than this will hand to a subprocess.
    TooLarge { bytes: usize, limit: usize },
    /// PlantUML rejected the diagram — its own message, verbatim.
    Diagram(String),
    /// PlantUML did not finish in time.
    TimedOut { seconds: u64 },
    /// The process could not be run, or its output was not usable.
    Process(String),
}

impl std::fmt::Display for PlantUmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlantUmlError::NotInstalled => write!(
                f,
                "PlantUML is not installed. It is a Java program and there is no \
                 in-browser version of it, so a diagram cannot be drawn without it. \
                 Install it with `brew install plantuml` (macOS), `apt install plantuml` \
                 (Debian/Ubuntu), or download plantuml.jar and point PLANTUML_JAR at it. \
                 Nothing is sent to a remote renderer."
            ),
            PlantUmlError::NoJava { jar } => write!(
                f,
                "found {} but no `java` to run it with — install a JRE, or install the \
                 `plantuml` launcher instead",
                jar.display()
            ),
            PlantUmlError::TooLarge { bytes, limit } => write!(
                f,
                "this diagram is {bytes} bytes; the renderer is given at most {limit}"
            ),
            PlantUmlError::Diagram(message) => write!(f, "{message}"),
            PlantUmlError::TimedOut { seconds } => write!(
                f,
                "PlantUML did not finish within {seconds}s and was stopped; \
                 a diagram this large may need to be split up"
            ),
            PlantUmlError::Process(message) => write!(f, "could not run PlantUML: {message}"),
        }
    }
}

/// The most source this will hand to a subprocess.
///
/// A diagram is text someone wrote; a megabyte of it is not a diagram any more.
/// The bound is here so the size comes from a rule rather than from whatever
/// happened to be in the file.
pub const MAX_SOURCE_BYTES: usize = 1 << 20;

/// How long PlantUML gets before it is stopped.
pub const TIMEOUT: Duration = Duration::from_secs(30);

/// Where a jar tends to land, per packager.
const JAR_LOCATIONS: &[&str] = &[
    "/opt/homebrew/opt/plantuml/libexec/plantuml.jar",
    "/usr/local/opt/plantuml/libexec/plantuml.jar",
    "/usr/share/plantuml/plantuml.jar",
    "/usr/local/share/plantuml/plantuml.jar",
    "/opt/plantuml/plantuml.jar",
];

/// Find PlantUML on this machine.
pub fn discover() -> Option<Renderer> {
    discover_with(
        |path| Path::new(path).is_file(),
        |name| which(name),
        |key| std::env::var(key).ok(),
    )
}

/// The search itself, with the machine passed in so it can be tested.
///
/// Order matters: an explicitly configured jar beats whatever is on `PATH`,
/// because someone who set `PLANTUML_JAR` chose that one on purpose.
pub fn discover_with(
    is_file: impl Fn(&str) -> bool,
    on_path: impl Fn(&str) -> Option<PathBuf>,
    env: impl Fn(&str) -> Option<String>,
) -> Option<Renderer> {
    let jar_from_env = env("PLANTUML_JAR")
        .filter(|jar| is_file(jar))
        .map(PathBuf::from);
    if let Some(jar) = jar_from_env {
        return Some(match on_path("java") {
            Some(java) => Renderer::Jar { java, jar },
            // Reported as `NoJava` by the caller rather than skipped silently:
            // a jar that was pointed at deliberately should not be ignored.
            None => Renderer::Jar {
                java: PathBuf::new(),
                jar,
            },
        });
    }

    if let Some(command) = on_path("plantuml") {
        return Some(Renderer::Command(command));
    }

    let jar = JAR_LOCATIONS.iter().find(|jar| is_file(jar))?;
    let java = on_path("java")?;
    Some(Renderer::Jar {
        java,
        jar: PathBuf::from(jar),
    })
}

/// Look a program up on `PATH`.
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| is_executable(candidate))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// The command line for one render, as `(program, args)`.
///
/// `-pipe` reads the source from stdin and writes the picture to stdout, so
/// nothing is written to disk. `-failfast2` turns a syntax error into a
/// non-zero exit instead of a picture of the error.
pub fn command_line(renderer: &Renderer) -> (PathBuf, Vec<String>) {
    let render_args = ["-tsvg", "-pipe", "-charset", "UTF-8", "-failfast2"]
        .iter()
        .map(|arg| (*arg).to_string());
    match renderer {
        Renderer::Command(path) => (path.clone(), render_args.collect()),
        Renderer::Jar { java, jar } => (
            java.clone(),
            std::iter::once("-jar".to_string())
                .chain(std::iter::once(jar.display().to_string()))
                .chain(render_args)
                .collect(),
        ),
    }
}

/// Draw a diagram, returning SVG.
pub async fn render_svg(source: &str) -> Result<String, PlantUmlError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(PlantUmlError::TooLarge {
            bytes: source.len(),
            limit: MAX_SOURCE_BYTES,
        });
    }
    let renderer = discover().ok_or(PlantUmlError::NotInstalled)?;
    if let Renderer::Jar { java, jar } = &renderer {
        if java.as_os_str().is_empty() {
            return Err(PlantUmlError::NoJava { jar: jar.clone() });
        }
    }
    run(&renderer, source).await
}

async fn run(renderer: &Renderer, source: &str) -> Result<String, PlantUmlError> {
    use tokio::io::AsyncWriteExt;

    let (program, args) = command_line(renderer);
    let mut child = tokio::process::Command::new(&program)
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| PlantUmlError::Process(format!("{}: {e}", program.display())))?;

    if let Some(mut stdin) = child.stdin.take() {
        // A closed pipe means PlantUML gave up on its own; the exit status below
        // is what says why, so the write error is not the interesting one.
        let _ = stdin.write_all(source.as_bytes()).await;
        let _ = stdin.shutdown().await;
    }

    let finished = tokio::time::timeout(TIMEOUT, child.wait_with_output()).await;
    let output = match finished {
        Ok(result) => result.map_err(|e| PlantUmlError::Process(e.to_string()))?,
        Err(_) => {
            return Err(PlantUmlError::TimedOut {
                seconds: TIMEOUT.as_secs(),
            })
        }
    };

    if !output.status.success() {
        return Err(PlantUmlError::Diagram(diagnostic(
            &output.stderr,
            &output.stdout,
        )));
    }
    let svg = String::from_utf8(output.stdout)
        .map_err(|_| PlantUmlError::Process("PlantUML wrote something that is not text".into()))?;
    if svg.trim().is_empty() {
        return Err(PlantUmlError::Diagram(diagnostic(&output.stderr, &[])));
    }
    Ok(svg)
}

/// PlantUML's own complaint, tidied into one line-broken message.
fn diagnostic(stderr: &[u8], stdout: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }
    // Some builds put the complaint on stdout and nothing on stderr.
    let fallback = String::from_utf8_lossy(stdout);
    match fallback.trim() {
        "" => "PlantUML rejected the diagram but said nothing about why".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nothing(_: &str) -> Option<PathBuf> {
        None
    }

    #[test]
    fn a_launcher_on_path_is_preferred_over_a_packaged_jar() {
        let found = discover_with(
            |path| path == "/usr/share/plantuml/plantuml.jar",
            |name| match name {
                "plantuml" => Some(PathBuf::from("/usr/bin/plantuml")),
                "java" => Some(PathBuf::from("/usr/bin/java")),
                _ => None,
            },
            |_| None,
        );
        assert_eq!(found, Some(Renderer::Command("/usr/bin/plantuml".into())));
    }

    #[test]
    fn a_configured_jar_wins_over_everything() {
        // Someone who sets PLANTUML_JAR picked that one on purpose.
        let found = discover_with(
            |path| path == "/opt/mine/plantuml.jar",
            |name| (name == "java").then(|| PathBuf::from("/usr/bin/java")),
            |key| (key == "PLANTUML_JAR").then(|| "/opt/mine/plantuml.jar".to_string()),
        );
        assert_eq!(
            found,
            Some(Renderer::Jar {
                java: "/usr/bin/java".into(),
                jar: "/opt/mine/plantuml.jar".into(),
            })
        );
    }

    #[test]
    fn a_configured_jar_that_does_not_exist_is_not_used() {
        let found = discover_with(|_| false, nothing, |_| Some("/nope.jar".to_string()));
        assert_eq!(found, None);
    }

    #[test]
    fn a_packaged_jar_is_found_when_there_is_no_launcher() {
        let found = discover_with(
            |path| path == "/opt/homebrew/opt/plantuml/libexec/plantuml.jar",
            |name| (name == "java").then(|| PathBuf::from("/opt/java")),
            |_| None,
        );
        assert_eq!(
            found,
            Some(Renderer::Jar {
                java: "/opt/java".into(),
                jar: "/opt/homebrew/opt/plantuml/libexec/plantuml.jar".into(),
            })
        );
    }

    #[test]
    fn a_jar_with_no_java_is_reported_rather_than_skipped() {
        // Skipping it would end in "PlantUML is not installed", which is the
        // wrong thing to tell someone who has the jar and is missing a JRE.
        let found = discover_with(
            |_| true,
            nothing,
            |key| (key == "PLANTUML_JAR").then(|| "/opt/mine/plantuml.jar".to_string()),
        );
        assert!(matches!(found, Some(Renderer::Jar { java, .. }) if java.as_os_str().is_empty()));
    }

    #[test]
    fn nothing_installed_is_nothing_found() {
        assert_eq!(discover_with(|_| false, nothing, |_| None), None);
    }

    #[test]
    fn the_command_line_reads_stdin_and_fails_on_a_bad_diagram() {
        let (program, args) = command_line(&Renderer::Command("/usr/bin/plantuml".into()));
        assert_eq!(program, PathBuf::from("/usr/bin/plantuml"));
        assert!(args.contains(&"-pipe".to_string()), "{args:?}");
        assert!(args.contains(&"-tsvg".to_string()), "{args:?}");
        assert!(args.contains(&"-failfast2".to_string()), "{args:?}");

        let (program, args) = command_line(&Renderer::Jar {
            java: "/usr/bin/java".into(),
            jar: "/opt/plantuml.jar".into(),
        });
        assert_eq!(program, PathBuf::from("/usr/bin/java"));
        assert_eq!(
            &args[..2],
            &["-jar".to_string(), "/opt/plantuml.jar".to_string()]
        );
    }

    #[tokio::test]
    async fn an_oversized_diagram_is_refused_before_anything_is_spawned() {
        let huge = "a".repeat(MAX_SOURCE_BYTES + 1);
        let error = render_svg(&huge).await.expect_err("too large");
        assert!(matches!(error, PlantUmlError::TooLarge { .. }), "{error:?}");
    }

    #[test]
    fn every_failure_says_a_different_thing() {
        let messages = [
            PlantUmlError::NotInstalled.to_string(),
            PlantUmlError::NoJava {
                jar: "/x.jar".into(),
            }
            .to_string(),
            PlantUmlError::TooLarge { bytes: 2, limit: 1 }.to_string(),
            PlantUmlError::Diagram("syntax error at line 3".into()).to_string(),
            PlantUmlError::TimedOut { seconds: 30 }.to_string(),
            PlantUmlError::Process("no such file".into()).to_string(),
        ];
        for (i, message) in messages.iter().enumerate() {
            assert!(!message.is_empty());
            assert!(
                messages.iter().skip(i + 1).all(|other| other != message),
                "two failures read the same: {message}"
            );
        }
        assert!(messages[0].contains("brew install plantuml"));
        assert!(
            messages[0].contains("Nothing is sent to a remote renderer"),
            "the one thing someone will wonder about a diagram that did not render"
        );
    }

    /// Draws a real diagram, on a machine that can.
    ///
    /// Skipped — loudly — where PlantUML is absent. A test that passes because
    /// nothing ran is the failure this repo's eval harness exists to prevent,
    /// and it would be the same failure here.
    #[tokio::test]
    async fn a_real_diagram_becomes_real_svg() {
        let Some(renderer) = discover() else {
            eprintln!("skipped: no PlantUML here — set PLANTUML_JAR to run this");
            return;
        };
        let svg = render_svg("@startuml\nAlice -> Bob: hello\n@enduml")
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "{} could not draw a two-line diagram: {e}",
                    renderer.describe()
                )
            });
        assert!(
            svg.contains("<svg"),
            "not SVG: {}",
            &svg[..svg.len().min(120)]
        );
        assert!(svg.contains("Alice"), "the diagram lost its own text");
    }

    #[tokio::test]
    async fn a_broken_diagram_is_an_error_not_a_picture_of_one() {
        if discover().is_none() {
            eprintln!("skipped: no PlantUML here — set PLANTUML_JAR to run this");
            return;
        }
        // Without `-failfast2` this returns 8 KB of SVG *containing* the words
        // "Syntax Error", exit code 0 — a picture of a failure, reported as a
        // drawing that worked.
        let error = render_svg("@startuml\nthis is not a diagram >>> <<<\n@enduml")
            .await
            .expect_err("a syntax error must not render");
        match error {
            PlantUmlError::Diagram(message) => {
                assert!(message.to_lowercase().contains("error"), "{message}")
            }
            other => panic!("wrong failure for a syntax error: {other:?}"),
        }
    }

    /// The Tauri command is three lines, and all three can be wrong: the wrong
    /// module, an error that loses its message, a success that returns nothing.
    #[tokio::test]
    async fn the_command_returns_the_drawing_or_the_reason() {
        if discover().is_none() {
            let refused = crate::commands::render_plantuml("@startuml\nA -> B\n@enduml".into())
                .await
                .expect_err("nothing installed, nothing drawn");
            assert!(refused.contains("brew install plantuml"), "{refused}");
            return;
        }
        let svg = crate::commands::render_plantuml("@startuml\nAlice -> Bob: hi\n@enduml".into())
            .await
            .expect("a two-line diagram");
        assert!(svg.contains("<svg"), "not SVG");
        assert_eq!(
            crate::commands::plantuml_renderer()
                .await
                .unwrap()
                .is_some(),
            true,
            "the renderer that just drew a diagram must be reportable"
        );
    }

    #[test]
    fn the_diagnostic_prefers_what_plantuml_actually_said() {
        assert_eq!(diagnostic(b"  boom  ", b"ignored"), "boom");
        assert_eq!(diagnostic(b"", b" fallback "), "fallback");
        assert!(diagnostic(b"", b"").contains("said nothing"));
    }
}
