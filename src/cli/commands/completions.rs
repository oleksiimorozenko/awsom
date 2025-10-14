use crate::cli::{Cli, Shell};
use clap::CommandFactory;
use clap_complete::{generate, Shell as ClapShell};
use std::io::{self, IsTerminal};

pub fn execute(shell: Shell, show_install: bool) {
    if show_install {
        // Just show installation instructions
        print_installation_instructions(&shell);
        return;
    }

    // Generate the completion script
    let mut cmd = Cli::command();
    let bin_name = "awsom";

    let clap_shell = match shell {
        Shell::Bash => ClapShell::Bash,
        Shell::Zsh => ClapShell::Zsh,
        Shell::Fish => ClapShell::Fish,
        Shell::PowerShell => ClapShell::PowerShell,
        Shell::Elvish => ClapShell::Elvish,
    };

    // Generate completions to stdout
    generate(clap_shell, &mut cmd, bin_name, &mut io::stdout());

    // Only show hint when running interactively (not when being eval'd or piped)
    // When stdout is captured (not a terminal), we're being piped/eval'd - don't show hints
    if io::stdout().is_terminal() {
        let shell_name = match shell {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
            Shell::PowerShell => "powershell",
            Shell::Elvish => "elvish",
        };

        eprintln!();
        eprintln!("# Completion script generated successfully!");
        eprintln!("# To see installation instructions, run:");
        eprintln!("#   awsom completions {} --show-install", shell_name);
    }
}

fn print_installation_instructions(shell: &Shell) {
    let instructions = match shell {
        Shell::Bash => {
            r#"
awsom shell completion for Bash

COPY-PASTE INSTALLATION (recommended):

  eval "$(awsom completions bash)"
  echo 'eval "$(awsom completions bash)"' >> ~/.bashrc

This enables completions immediately and adds them to ~/.bashrc for future shells.

Alternative - Save to system completion directory (faster startup):

  awsom completions bash > /tmp/awsom_completion.bash
  sudo mv /tmp/awsom_completion.bash /usr/local/etc/bash_completion.d/awsom
  # or on Linux:
  sudo mv /tmp/awsom_completion.bash /etc/bash_completion.d/awsom
  source ~/.bashrc

"#
        }
        Shell::Zsh => {
            r#"
awsom shell completion for Zsh

COPY-PASTE INSTALLATION (recommended):

  eval "$(awsom completions zsh)"
  echo 'eval "$(awsom completions zsh)"' >> ~/.zshrc

This enables completions immediately and adds them to ~/.zshrc for future shells.

Alternative - Save to completion directory (faster startup):

  mkdir -p ~/.zfunc
  awsom completions zsh > ~/.zfunc/_awsom
  echo 'fpath=(~/.zfunc $fpath)' >> ~/.zshrc
  echo 'autoload -Uz compinit && compinit' >> ~/.zshrc
  source ~/.zshrc

"#
        }
        Shell::Fish => {
            r#"
awsom shell completion for Fish

COPY-PASTE INSTALLATION:

  mkdir -p ~/.config/fish/completions
  awsom completions fish > ~/.config/fish/completions/awsom.fish

Fish will automatically load completions on next shell startup.

To activate immediately in current shell:

  source ~/.config/fish/completions/awsom.fish

"#
        }
        Shell::PowerShell => {
            r#"
awsom shell completion for PowerShell

COPY-PASTE INSTALLATION:

  awsom completions powershell | Out-String | Invoke-Expression
  Add-Content -Path $PROFILE -Value "`nawsom completions powershell | Out-String | Invoke-Expression"

This enables completions immediately and adds them to your PowerShell profile for future sessions.

Note: If $PROFILE doesn't exist, create it first:
  New-Item -Path $PROFILE -ItemType File -Force

"#
        }
        Shell::Elvish => {
            r#"
awsom shell completion for Elvish

COPY-PASTE INSTALLATION:

  eval (awsom completions elvish | slurp)
  echo 'eval (awsom completions elvish | slurp)' >> ~/.config/elvish/rc.elv

This enables completions immediately and adds them to rc.elv for future shells.

Note: rc.elv location may vary:
  Unix: ~/.config/elvish/rc.elv
  Windows: ~\AppData\Roaming\elvish\rc.elv

"#
        }
    };

    println!("{}", instructions);
}
