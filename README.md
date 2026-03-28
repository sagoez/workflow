# Workflow CLI

[![wf::build](https://github.com/sagoez/workflow/actions/workflows/ci.yml/badge.svg)](https://github.com/sagoez/workflow/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/wf-cli?logo=data:image/svg%2bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCA2NCA2NCIgd2lkdGg9IjY0IiBoZWlnaHQ9IjY0Ij48cmVjdCB3aWR0aD0iNjQiIGhlaWdodD0iNjQiIHJ4PSIxNCIgZmlsbD0iI2ZmZiIvPjx0ZXh0IHg9IjMyIiB5PSI0NCIgdGV4dC1hbmNob3I9Im1pZGRsZSIgZm9udC1mYW1pbHk9InN5c3RlbS11aSwtYXBwbGUtc3lzdGVtLHNhbnMtc2VyaWYiIGZvbnQtd2VpZ2h0PSI5MDAiIGZvbnQtc3R5bGU9Iml0YWxpYyIgZm9udC1zaXplPSIzMiIgZmlsbD0iIzAwMCI+d2Y8L3RleHQ+PC9zdmc+&label=crates.io)](https://crates.io/crates/wf-cli)

A terminal-native alternative to [Warp's Workflows](https://docs.warp.dev/knowledge-and-collaboration/warp-drive/workflows). Define parameterized commands in YAML, resolve arguments interactively, and get the final command copied to your clipboard — ready to paste.

[Documentation](https://wf.sagoez.com) | [Workflow Vault](https://vault.sagoez.com)

> Built with event sourcing, CQRS, hexagonal architecture, and an actor model — because I wanted to learn all of that and needed an excuse to build something. You could do this in 50 lines of bash. But your bash script wouldn't have an actor model.

## Quick Start

```bash
# Install from crates.io
cargo install wf-cli

# Run — select a workflow, fill in the prompts, paste the result
wf
```

## How It Works

1. You write YAML workflow files with `{{placeholders}}` in commands
2. Run `wf` — pick one from the interactive menu
3. Fill in the argument prompts (text, enums, numbers, booleans)
4. The resolved command is copied to your clipboard

## Workflow YAML Format

```yaml
name: "Deploy to K8s"
description: "Deploy an app to a Kubernetes cluster"
command: "kubectl apply -f {{file}} --namespace {{namespace}}"
tags: ["kubernetes", "deployment"]
shells: ["bash", "zsh"]

arguments:
  - name: file
    description: "Path to deployment manifest"
    default_value: "./deployment.yaml"

  - name: namespace
    arg_type: Enum
    description: "Target namespace"
    enum_variants:
      - "default"
      - "staging"
      - "production"
```

### Argument Types

| Type | Description |
|------|-------------|
| `Text` | Free text input (default) |
| `Enum` | Select from static variants or dynamically generated via shell command |
| `MultiEnum` | Select multiple from a list (joined with `,`) |
| `Number` | Numeric input |
| `Boolean` | True/false |

### Dynamic Enums

Enum options can be generated at runtime from a shell command:

```yaml
arguments:
  - name: namespace
    arg_type: Enum
    description: "Kubernetes namespace"
    enum_command: "kubectl get namespaces --no-headers | awk '{print $1}'"

  - name: pod
    arg_type: Enum
    description: "Pod to inspect"
    enum_command: "kubectl get pods -n {{namespace}} --no-headers | awk '{print $1}'"
    dynamic_resolution: "namespace"
```

The `dynamic_resolution` field tells the CLI to resolve the referenced argument first, then use its value when executing `enum_command`.

## Commands

```bash
wf                  # Interactive workflow selection (default)
wf --list           # List all available workflows

# Sync workflows from a remote Git repo
wf sync --remote-url https://github.com/user/workflows.git --branch main
wf sync --ssh-key ~/.ssh/id_rsa --remote-url git@github.com:user/workflows.git

# Language
wf lang set en      # Set language (en, es)
wf lang current     # Show current language
wf lang list        # List available languages

# Storage backend
wf storage set rocksdb   # Switch to persistent storage
wf storage set inmemory  # Switch to in-memory storage
wf storage current       # Show current backend
wf storage list          # List all workflow sessions
wf storage replay <id>   # Replay events for a workflow
wf storage delete <id>   # Delete a specific session
wf storage purge         # Clear all stored events
```

## Workflow File Location

Workflow YAML files (`.yaml` / `.yml`) go in:

| OS | Path |
|----|------|
| macOS | `~/Library/Application Support/workflow/workflows/` |
| Linux | `~/.config/workflow/workflows/` |
| Windows | `%APPDATA%/workflow/workflows/` |

You can populate this directory manually or use `wf sync` to pull from a Git repository. See [workflow-vault](https://github.com/sagoez/workflow-vault) for an example shared workflow repo.

## Installation

### Prerequisites

- **Rust** — install from [rustup.rs](https://rustup.rs/)
- **LLVM/Clang** — required for RocksDB native bindings

#### macOS

```bash
brew install llvm

export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
export LDFLAGS="-L/opt/homebrew/opt/llvm/lib"
export CPPFLAGS="-I/opt/homebrew/opt/llvm/include"
# Add these to ~/.zshrc for persistence
```

#### Linux (Ubuntu/Debian)

```bash
sudo apt-get install clang llvm-dev libclang-dev
```

#### Linux (Fedora/RHEL)

```bash
sudo dnf install clang llvm-devel clang-devel
```

### Build from Source

```bash
git clone https://github.com/sagoez/workflow.git
cd workflow
cargo install --path .
```
