use failure::Error;
use rustwide::cmd::{Command, CommandError, SandboxBuilder};
use rustwide::{Crate, PrepareError, Toolchain, Workspace};

#[test]
fn test_fetch() -> Result<(), Error> {
    let workspace = crate::utils::init_workspace()?;
    let toolchain = Toolchain::dist("stable");
    toolchain.install(&workspace)?;

    let krate = Crate::registry("nemo157-cli", "0.1.0", "https://github.com/rust-lang/staging.crates.io-index");
    krate.fetch(&workspace)?;

    let krate2 = Crate::registry("rand", "0.3.14", "https://github.com/rust-lang/staging.crates.io-index");
    krate2.fetch(&workspace)?;

    Ok(())
}
