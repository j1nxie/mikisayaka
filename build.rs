use anyhow::Error;
use vergen_gitcl::{Emitter, GitclBuilder};

pub fn main() -> Result<(), Error> {
    println!("cargo:rerun-if-changed=migrations");
    if let Ok(sha) = std::env::var("VERGEN_GIT_SHA") {
        if sha != "unknown" {
            println!("cargo:rustc-env=VERGEN_GIT_SHA={sha}");

            return Ok(());
        }
    }

    let gitcl = GitclBuilder::default().sha(true).build()?;

    Emitter::default().add_instructions(&gitcl)?.emit()?;
    Ok(())
}
