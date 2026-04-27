//! BDD tests for the macOS native (sandbox-exec) Tier-0 backend.
//! Run on macOS only; on other hosts the test is a compiled no-op.

#[cfg(target_os = "macos")]
mod macos_run {
    use cucumber::{World, given, then, when};
    use std::path::PathBuf;
    use tempfile::TempDir;
    use vibe_sandbox::Sandbox;
    use vibe_sandbox_native::macos::{MacosSandbox, SbProfile};

    #[derive(Default, World)]
    pub struct MWorld {
        sandbox: Option<MacosSandbox>,
        rw_dir: Option<TempDir>,
        ro_dir: Option<TempDir>,
        last_exit: Option<i32>,
        last_stdout: String,
        profile: Option<SbProfile>,
        profile_render: Option<String>,
        profile_error: Option<String>,
    }

    impl std::fmt::Debug for MWorld {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MWorld").finish()
        }
    }

    #[given("a fresh macOS sandbox")]
    fn fresh_sandbox(world: &mut MWorld) {
        world.sandbox = Some(MacosSandbox::new().unwrap());
    }

    #[given(expr = "a temporary directory bound rw at {string}")]
    fn bind_rw(world: &mut MWorld, _guest: String) {
        let td = tempfile::tempdir().unwrap();
        let host: PathBuf = td.path().to_owned();
        world
            .sandbox
            .as_mut()
            .unwrap()
            .bind_rw(&host, &PathBuf::from("/work"))
            .unwrap();
        world.rw_dir = Some(td);
    }

    #[given(expr = "a temporary directory bound ro at {string}")]
    fn bind_ro(world: &mut MWorld, _guest: String) {
        let td = tempfile::tempdir().unwrap();
        let host: PathBuf = td.path().to_owned();
        world
            .sandbox
            .as_mut()
            .unwrap()
            .bind_ro(&host, &PathBuf::from("/ro"))
            .unwrap();
        world.ro_dir = Some(td);
    }

    #[when(expr = "I spawn {string}")]
    fn spawn_cmd(world: &mut MWorld, cmd: String) {
        // The feature uses literal /bin/sh -c '...' strings; route through sh -c.
        // We strip the leading "/bin/sh -c '" and trailing "'" if present.
        let inner = if let Some(rest) = cmd.strip_prefix("/bin/sh -c '") {
            rest.trim_end_matches('\'').to_string()
        } else {
            cmd.clone()
        };
        // Substitute $WORKDIR / $RODIR with the actual host paths.
        let workdir = world
            .rw_dir
            .as_ref()
            .map(|d| d.path().to_string_lossy().into_owned())
            .unwrap_or_default();
        let rodir = world
            .ro_dir
            .as_ref()
            .map(|d| d.path().to_string_lossy().into_owned())
            .unwrap_or_default();
        let inner = inner.replace("$WORKDIR", &workdir).replace("$RODIR", &rodir);

        let sandbox = world.sandbox.as_ref().unwrap();
        let out = sandbox.run_capture("/bin/sh", &["-c", &inner]).unwrap();
        world.last_exit = out.status.code();
        world.last_stdout = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            eprintln!(
                "--- sandbox spawn failed ---\nexit: {:?}\nstdout: {:?}\nstderr: {:?}\nrendered profile:\n{}\n--- end ---",
                out.status.code(),
                world.last_stdout,
                stderr,
                world.sandbox.as_ref().unwrap().rendered_profile()
            );
        }
    }

    #[then(expr = "the spawn exit code is {int}")]
    fn exit_code(world: &mut MWorld, expected: i32) {
        assert_eq!(world.last_exit, Some(expected),
            "stdout was: {:?}", world.last_stdout);
    }

    #[then(expr = "the host file {string} inside the bound dir contains {string}")]
    fn file_contains(world: &mut MWorld, name: String, expected: String) {
        let path = world.rw_dir.as_ref().unwrap().path().join(&name);
        let contents = std::fs::read_to_string(&path).expect("file exists");
        assert!(contents.contains(&expected),
            "{} contained {:?}, expected to contain {:?}",
            path.display(), contents, expected);
    }

    #[then(expr = "the spawn stdout matches {string}")]
    fn stdout_matches(world: &mut MWorld, expected: String) {
        assert_eq!(world.last_stdout, expected);
    }

    #[given("a fresh macOS sandbox profile")]
    fn fresh_profile(world: &mut MWorld) {
        world.profile = Some(SbProfile::new());
    }

    #[when("I render the profile")]
    fn render_profile(world: &mut MWorld) {
        world.profile_render = Some(world.profile.as_ref().unwrap().render());
    }

    #[then(expr = "the rendered profile starts with {string}")]
    fn rendered_starts_with(world: &mut MWorld, expected: String) {
        let r = world.profile_render.as_ref().unwrap();
        assert!(r.starts_with(&expected), "got: {}", r);
    }

    #[then(expr = "the rendered profile contains {string}")]
    fn rendered_contains(world: &mut MWorld, expected: String) {
        let r = world.profile_render.as_ref().unwrap();
        assert!(r.contains(&expected), "missing {expected:?} in: {r}");
    }

    #[when(r#"I add a subpath that contains ".." in the middle"#)]
    fn add_traversal(world: &mut MWorld) {
        let result = world
            .profile
            .as_mut()
            .unwrap()
            .allow_rw_subpath(std::path::Path::new("/tmp/../etc"));
        world.profile_error = result.err().map(|e| e.to_string());
    }

    #[then("the profile build returns an error")]
    fn profile_error(world: &mut MWorld) {
        assert!(world.profile_error.is_some(), "expected error, got Ok");
    }

    pub fn run() {
        futures::executor::block_on(MWorld::run("tests/features/native_macos.feature"));
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    {
        macos_run::run();
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("native_macos_bdd: skipped (not macOS)");
    }
}
