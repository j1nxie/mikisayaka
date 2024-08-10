use anyhow::Error;
use vergen_gitcl::{Emitter, GitclBuilder};

pub fn main() -> Result<(), Error> {
    let gitcl = GitclBuilder::default().sha(true).build()?;

    Emitter::default().add_instructions(&gitcl)?.emit()?;
    Ok(())
}
