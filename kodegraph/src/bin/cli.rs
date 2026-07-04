//! `kodegraph` binary entry point. Thin wrapper over [`kodegraph::cli::run`].

fn main() -> anyhow::Result<()> {
    kodegraph::cli::run()
}