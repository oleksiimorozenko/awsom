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

    eprintln!("Generating completion file for {:?}...", shell);
    generate(clap_shell, &mut cmd, bin_name, &mut io::stdout());
    eprintln!("\n# Installation instructions:");

    match shell {
        Shell::Bash => {
            eprintln!("# Add to ~/.bashrc:");
            eprintln!("#   eval \"$(awsom completions bash)\"");
            eprintln!("# Or save to completion directory:");
            eprintln!("#   awsom completions bash > /usr/local/etc/bash_completion.d/awsom");
        }
        Shell::Zsh => {
            eprintln!("# Add to ~/.zshrc:");
            eprintln!("#   eval \"$(awsom completions zsh)\"");
            eprintln!("# Or save to completion directory:");
            eprintln!("#   awsom completions zsh > ~/.zfunc/_awsom");
            eprintln!("#   Then add to ~/.zshrc: fpath=(~/.zfunc $fpath)");
        }
        Shell::Fish => {
            eprintln!("# Save to fish completion directory:");
            eprintln!("#   awsom completions fish > ~/.config/fish/completions/awsom.fish");
        }
        Shell::PowerShell => {
            eprintln!("# Add to PowerShell profile:");
            eprintln!("#   awsom completions powershell | Out-String | Invoke-Expression");
        }
        Shell::Elvish => {
            eprintln!("# Add to Elvish config:");
            eprintln!("#   eval (awsom completions elvish | slurp)");
        }
    }
}
