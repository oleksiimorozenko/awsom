use crate::cli::{Cli, Shell};
use clap::CommandFactory;
use clap_complete::{generate, Shell as ClapShell};
use std::io;

pub fn execute(shell: Shell) {
    let mut cmd = Cli::command();
    let bin_name = "awsom";

    let clap_shell = match shell {
        Shell::Bash => ClapShell::Bash,
        Shell::Zsh => ClapShell::Zsh,
        Shell::Fish => ClapShell::Fish,
        Shell::PowerShell => ClapShell::PowerShell,
        Shell::Elvish => ClapShell::Elvish,
    };

    // Generate completions to stdout without any extra messages
    // Users typically use this with eval or redirect to a file
    generate(clap_shell, &mut cmd, bin_name, &mut io::stdout());
}
