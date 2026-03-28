'use client';

import { useState } from 'react';

const PROMPT = `You help users write wf-cli workflow YAML files.

## Structure

\`\`\`yaml
name: "Name shown in menu"
description: "What it does"
command: "command with {{placeholders}}"
tags: ["optional", "tags"]
shells: ["bash", "zsh"]
author: "optional"
author_url: "optional"
source_url: "optional"

arguments:
  - name: placeholder_name
    description: "Prompt shown to user"
    arg_type: Text  # Text (default), Enum, Number, Boolean
    default_value: "optional"
\`\`\`

## Argument Types

**Text** — free input with optional default:
\`\`\`yaml
- name: image
  description: "Docker image"
  default_value: "my-app"
\`\`\`

**Enum (static)** — pick one from a list:
\`\`\`yaml
- name: env
  arg_type: Enum
  description: "Target environment"
  enum_variants:
    - "dev"
    - "staging"
    - "production"
\`\`\`

**Enum (dynamic)** — options from a shell command:
\`\`\`yaml
- name: namespace
  arg_type: Enum
  description: "Kubernetes namespace"
  enum_name: "namespaces"
  enum_command: "kubectl get namespaces --no-headers | awk '{print $1}'"
\`\`\`

**Enum (chained)** — depends on a previous argument:
\`\`\`yaml
- name: namespace
  arg_type: Enum
  enum_name: "namespaces"
  enum_command: "kubectl get namespaces --no-headers | awk '{print $1}'"

- name: pod
  arg_type: Enum
  description: "Pod"
  enum_name: "pods"
  enum_command: "kubectl get pods -n {{namespace}} --no-headers | awk '{print $1}'"
  dynamic_resolution: "namespace"
\`\`\`

**Enum (multi-select)** — pick multiple, joined with comma:
\`\`\`yaml
- name: services
  arg_type: Enum
  multi: true
  description: "Services to deploy"
  enum_variants: ["api", "web", "worker"]
  min_selections: 1
  max_selections: 3
\`\`\`

## Rules

- Output only YAML, no explanation unless asked
- Every {{placeholder}} in command must have a matching argument
- Use enum_command + enum_name for dynamic options, enum_variants for static
- Use dynamic_resolution when options depend on a previous argument
- Keep descriptions short and helpful`;

export function CopyPrompt() {
  const [copied, setCopied] = useState(false);

  return (
    <button
      onClick={() => {
        navigator.clipboard.writeText(PROMPT).then(() => {
          setCopied(true);
          setTimeout(() => setCopied(false), 2000);
        });
      }}
      className="inline-flex items-center gap-2 px-3 py-1.5 rounded-lg border border-fd-border text-sm text-fd-muted-foreground hover:text-fd-foreground hover:bg-fd-accent active:scale-[0.97] transition-all"
    >
      {copied ? (
        <>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
          Copied
        </>
      ) : (
        <>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
          Copy AI prompt
        </>
      )}
    </button>
  );
}
