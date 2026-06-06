# AGENTS.md — BootLoader

This repository is a workspace for an ARM Cortex-M bootloader project.

## Structure

- `Literature/The Embedonomicon.pdf` — reference document on `cortex-m-rt` vector tables and startup code (Rust embedded)
- `opencode.json` — OpenCode agent permission rules (must ask before: editing, destructive `bash`, git mutating writes, `sudo` is denied)

## Status

No build system, toolchain config, or source code has been scaffolded yet. Future work should set up:
- A Rust embedded toolchain (`cortex-m-rt`, `cortex-m-semihosting`, target triple)
- Build/test/lint configuration
- CI workflows

## Agent constraints (from `opencode.json`)

- **Read**: allowed broadly; `.env*` and `.ssh/` files denied
- **Edit**: must ask the user before every edit
- **Bash**: most commands allowed; `rm`, `touch`, `mv`, `git commit/push/pull/reset/clean`, `npm install -g`, `brew`, `apt` require asking; `sudo` is denied
- **External directory** access requires asking
