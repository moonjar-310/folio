# Personal Planner Note App

A lightweight self-hosted personal planner and Markdown notebook for the web.

## Purpose

Combine:
- Long-term Goals
- Daily / Weekly Planner
- Todo management
- Markdown Notes

The product should feel like a calm personal planner, not a SaaS dashboard.

## Core Principles

- Simple
- Fast
- Web-first
- Self-hosted
- Zero-cost oriented
- Portable
- Markdown-first

## Main Navigation

- Home
- Planner
- Todo
- Notes
- Folders
- Settings

## Tech Direction

### Runtime
- Rust
- Cloudflare Workers
- WebAssembly

### UI
- SSR HTML
- CSS
- HTMX
- Minimal Vanilla JavaScript

### Storage
Cloud:
- R2: canonical Markdown data
- D1: metadata, search, tasks, goals, sessions

Future Local Mode:
- Filesystem
- SQLite

See:
- PRODUCT_SPEC.md
- ARCHITECTURE.md
- DESIGN.md
- DEVELOPMENT.md
- AGENTS.md
