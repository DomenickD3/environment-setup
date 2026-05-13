# Environment Setup

Personal bootstrap scripts for setting up a development environment from a fresh shell.

## Install

After this repository is pushed to GitHub, run the full setup with one line:

```bash
curl -fsSL https://raw.githubusercontent.com/DomenickD3/environment-setup/main/install | bash
```

This clones or updates the repo at:

```text
~/.local/share/environment-setup
```

Then it updates apt packages when apt is available, installs Nix if needed,
installs Codex, GitHub CLI, Neovim, tmux, and stow into your Nix profile, runs
the GitHub SSH key setup interactively, clones or updates your dotfiles and
dotfiles-private repos, runs their install commands, writes a current-shell
activation script, and prompts you to log in to your Codex subscription.

Installer output is appended to:

```text
~/.local/state/environment-setup/install.log
```

If a step fails, the terminal output includes the failing command and the log
path. Use a different log file with:

```bash
curl -fsSL https://raw.githubusercontent.com/DomenickD3/environment-setup/main/install | INSTALL_LOG="$HOME/install.log" bash
```

Use a different location with:

```bash
curl -fsSL https://raw.githubusercontent.com/DomenickD3/environment-setup/main/install | INSTALL_DIR="$HOME/dev/environment-setup" bash
```

Override the dotfiles repo, checkout path, or install command with:

```bash
curl -fsSL https://raw.githubusercontent.com/DomenickD3/environment-setup/main/install | \
  DOTFILES_REPO_URL="git@github.com:DomenickD3/.dotfiles.git" \
  DOTFILES_DIR="$HOME/src/dotfiles" \
  DOTFILES_INSTALL_CMD="./bootstrap" \
  DOTFILES_PRIVATE_REPO_URL="git@github.com:DomenickD3/.dotfiles-private.git" \
  DOTFILES_PRIVATE_DIR="$HOME/src/dotfiles-private" \
  DOTFILES_PRIVATE_INSTALL_CMD="./bootstrap" \
  bash
```

To only clone/update and install the dotfiles repos without installing Nix,
packages, GitHub SSH setup, or Codex login, run:

```bash
curl -fsSL https://raw.githubusercontent.com/DomenickD3/environment-setup/main/install | bash -s -- --dotfiles-only
```

## Run A Command

Run a command after installing:

```bash
~/.local/share/environment-setup/bin/setup-github-ssh --help
```

Or dispatch a command directly through the installer:

```bash
curl -fsSL https://raw.githubusercontent.com/DomenickD3/environment-setup/main/install | bash -s -- install-nix --help
```

Pass command arguments after the command name:

```bash
curl -fsSL https://raw.githubusercontent.com/DomenickD3/environment-setup/main/install | bash -s -- setup-github-ssh --help
```

## Commands

- `bin/install-nix`: install Nix using the official installer URL.
- `bin/setup-github-ssh`: generate a GitHub SSH key, optionally upload it with `gh`, and test SSH authentication.

The default install flow also installs Codex, GitHub CLI, Neovim, tmux, and stow persistently with:

```bash
nix --extra-experimental-features nix-command --extra-experimental-features flakes profile add path:$INSTALL_DIR#codex
nix --extra-experimental-features nix-command --extra-experimental-features flakes profile add path:$INSTALL_DIR#gh
nix --extra-experimental-features nix-command --extra-experimental-features flakes profile add path:$INSTALL_DIR#neovim
nix --extra-experimental-features nix-command --extra-experimental-features flakes profile add path:$INSTALL_DIR#tmux
nix --extra-experimental-features nix-command --extra-experimental-features flakes profile add path:$INSTALL_DIR#stow
```

For dotfiles, the installer defaults to:

- cloning `git@github.com:DomenickD3/.dotfiles.git` into `~/.dotfiles`
- cloning `git@github.com:DomenickD3/.dotfiles-private.git` into `~/.dotfiles-private`
- auto-detecting one of `./bootstrap`, `./install`, `./install.sh`, `./setup`, or `script/install`
- running that command from inside each checkout

If either repo uses a different entrypoint, set `DOTFILES_INSTALL_CMD` or
`DOTFILES_PRIVATE_INSTALL_CMD`.

After dotfiles install, the installer writes an activation script that sources
`~/.profile` and `~/.bashrc` by default. The installer does not source those
files itself because it cannot change the parent terminal when run with
`curl ... | bash`.

To apply the installed Nix profile and shell startup files to the terminal that
launched the installer, source the generated activation file after install:

```bash
. ~/.local/state/environment-setup/activate.sh
```

Override the generated file path with `ACTIVATE_SCRIPT`. Override or disable the
startup files included in that activation script with `SOURCE_AFTER_INSTALL_FILES`,
using a colon-separated list of files.

## Nix

Install Nix:

```bash
./bin/install-nix
```

Preview the installer command first:

```bash
./bin/install-nix --dry-run
```

## Local Development

From a local checkout:

```bash
./install --help
./bin/install-nix --help
./bin/setup-github-ssh --help
```
