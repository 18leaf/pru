# Pur — Config Validation LSP (WIP)

Catch-all configuration validation exposed via an LSP, with plans for a CLI and more tooling. The goal is to validate “whatever config file you have” with the right schema or ruleset, and surface diagnostics directly in your editor.

> Status: **Working in progress**. Usable for JSON Schema validation today; more formats and schema-discovery features are planned.

![Alt text image demo (shows the Diagnostic warnings at the current stage)](docs/images/diagnostics-demo.png)

---

## What it is

`pur` (project name) is an **LSP-first** validator for configuration files. It aims to:

- Detect the config “dialect” (JSON today; YAML/TOML/XML planned)
- Find and load the right schema/rules (JSON Schema today)
- Provide editor diagnostics (warnings/errors), with consistent messaging and fast feedback

---

## Current capabilities (today)

- **Language Server Protocol (LSP)** integration (editor-agnostic in principle)
- **JSON Schema validation** for JSON files
- Publishes **diagnostics** (warnings/errors) to the editor as you type/save (depending on client settings)

---

## Missing / planned work

### Schema discovery improvements
- Support “shebang-like” inline schema hints, e.g.:
  - `$schema: schema` (or similar inline syntax you want recognized)
- More robust schema resolution strategies, likely including:
  - File-local hints (`$schema` fields, comments, directives)
  - Project-level mappings (workspace config)
  - Conventional paths (e.g. `.schemas/`, `schema.json`, etc.)

### More data languages (not started yet)
Planned support for validating additional config formats, including:
- **YAML**
- **TOML**
- **XML**

The intent is that each format can still participate in the same “find schema → validate → publish diagnostics” pipeline.

### Tooling roadmap
- **CLI** validator (batch validation in CI, pre-commit hooks, etc.)
- Better UX features over time (examples):
  - Code actions / quick fixes (where appropriate)
  - Schema cache + offline behavior
  - Workspace-level configuration options
  - Richer diagnostic metadata

---

## Non-goals (for now)

- Full IDE features beyond validation (formatting, refactors, etc.)
- Perfect schema auto-detection in every edge case (will improve iteratively)

---

## Contributing / feedback

This is an early-stage project. If you’re testing it, useful feedback includes:
- Example configs + schemas that *should* validate but don’t
- Desired schema-discovery syntax (especially the `$schema: schema` style you mentioned)
- Which editors/clients you’re using (VS Code, Neovim, etc.) and any LSP quirks you see

---
