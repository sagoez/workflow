# Workflow CLI

A powerful command-line tool for executing pre-defined workflows with interactive argument resolution. Perfect for automating complex tasks, DevOps operations, and repetitive commands with dynamic parameters.

## ✨ Features

- **🔍 Interactive Workflow Discovery** - Browse and select from available workflows
- **⚡ Direct Execution** - Run workflows by name or file path
- **🎯 Smart Argument Resolution** - Interactive prompts for required parameters
- **📋 Dynamic Options** - Enum arguments populated by executing commands
- **🌍 Multi-language Support** - Built-in internationalization (English/Spanish)
- **📂 Git Integration** - Sync workflows from remote repositories
- **📋 Clipboard Integration** - Generated commands copied automatically
- **🎨 Rich UI** - Progress indicators and intuitive prompts

## 🚀 Quick Start

### Installation

```bash
# Clone and build
git clone <repository-url>
cd workflow
cargo build --release

# Install globally (optional)
cargo install --path .
```

### First Run

```bash
# Initialize configuration
workflow init

# List available workflows
workflow --list

# Run interactively (choose from menu)
workflow

# Execute specific workflow
workflow "scale_kubernetes_pods.yaml"
```

## 📝 Usage

### Basic Commands

```bash
# Interactive workflow selection
workflow

# List all available workflows with descriptions
workflow --list

# Execute a specific workflow
workflow <filename>
workflow "My Workflow.yaml"

# Show help
workflow --help
```

### Configuration Commands

```bash
# Initialize configuration directories
workflow init

# Language management
workflow lang set en          # Set language to English
workflow lang set es          # Set language to Spanish
workflow lang list            # List available languages
workflow lang current         # Show current language

# Workflow repository management
workflow resource set <git-url>    # Set workflows repository URL
workflow resource current          # Show current repository URL

# Sync workflows from repository
workflow sync                      # Use configured repository
workflow sync <git-url>           # Use specific repository
workflow sync --ssh-key <path>    # Use SSH key for authentication
```

## 📋 Workflow Format

Workflows are defined in YAML files with the following structure:

```yaml
name: "My Workflow"
description: "Description of what this workflow does"
command: "echo {{message}} > {{output_file}}"
arguments:
  - name: message
    arg_type: Text
    description: "Message to write"
    default_value: "Hello World"
  
  - name: output_file
    arg_type: Text
    description: "Output file path"
  
  - name: namespace
    arg_type: Enum
    description: "Kubernetes namespace"
    enum_command: "kubectl get namespaces --no-headers | awk '{print $1}'"
    
tags: ["example", "demo"]
shells: ["bash", "zsh"]
```

### Argument Types

| Type | Description | Example |
|------|-------------|---------|
| `Text` | Free text input | File paths, names, messages |
| `Number` | Numeric input | Port numbers, counts, IDs |
| `Boolean` | True/false selection | Enable/disable flags |
| `Enum` | Selection from dynamic options | Namespaces, services, branches |

### Dynamic Enums

Enum arguments can populate options by executing commands:

```yaml
- name: git_branch
  arg_type: Enum
  description: "Git branch to checkout"
  enum_command: "git branch -r | sed 's/origin\///' | grep -v HEAD"
```

Or use static options:

```yaml
- name: environment
  arg_type: Enum
  description: "Deployment environment"
  enum_variants: ["dev", "staging", "prod"]
```

## 🌍 Internationalization

The tool supports multiple languages. Translation files are stored in the configuration directory:

```bash
# View current language
workflow lang current

# Switch to Spanish
workflow lang set es

# List available languages
workflow lang list
```

## 📁 Configuration

Configuration files are stored in platform-specific directories:

- **Linux/macOS**: `~/.config/workflow-rs/`
- **Windows**: `%APPDATA%\workflow-rs\`

## 🔧 Example Workflows

### Kubernetes Pod Scaling

```yaml
name: "Scale Kubernetes Pods"
description: "Scale deployments in a specific namespace"
command: "kubectl scale deployment --replicas={{replicas}} --namespace={{namespace}} --all"
arguments:
  - name: namespace
    arg_type: Enum
    description: "Target namespace"
    enum_command: "kubectl get namespaces --no-headers | awk '{print $1}'"
  
  - name: replicas
    arg_type: Number
    description: "Number of replicas"
    default_value: "3"
```

### Git Repository Cleanup

```yaml
name: "Git Branch Cleanup"
description: "Delete merged branches except main/master"
command: "git branch --merged | grep -v -E '(main|master|\\*)' | xargs -n 1 git branch -d"
arguments: []
```

### Service Health Check

```yaml
name: "Service Health Check"
description: "Check if a service is responding"
command: "curl -f {{protocol}}://{{host}}:{{port}}{{path}}/health || echo 'Service is down'"
arguments:
  - name: protocol
    arg_type: Enum
    description: "Protocol"
    enum_variants: ["http", "https"]
  
  - name: host
    arg_type: Text
    description: "Hostname or IP"
    default_value: "localhost"
  
  - name: port
    arg_type: Number
    description: "Port number"
    default_value: "8080"
  
  - name: path
    arg_type: Text
    description: "Base path"
    default_value: ""
```

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🆘 Support

- Create an issue for bug reports or feature requests
- Check existing workflows in the `resource/` directory for examples
- Use `workflow --help` for command reference
