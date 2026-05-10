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

Then it installs Nix if needed, installs Codex into your Nix profile, runs the
GitHub SSH key setup interactively, and prompts you to log in to your Codex
subscription.
The GitHub CLI is provided temporarily through Nix and is not installed globally.

Use a different location with:

```bash
curl -fsSL https://raw.githubusercontent.com/DomenickD3/environment-setup/main/install | INSTALL_DIR="$HOME/dev/environment-setup" bash
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
- `bin/setup-github-ssh`: generate a GitHub SSH key and optionally derive the GitHub noreply email with `gh`.

The default install flow also installs Codex persistently with:

```bash
nix --extra-experimental-features nix-command --extra-experimental-features flakes profile add path:$INSTALL_DIR#codex
```

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
