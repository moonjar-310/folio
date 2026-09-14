# Design System

## Theme and naming update — 2026-09-14

- The navigation label is **Folders**, replacing Notebooks. It represents the Markdown folder hierarchy; Notes remains the full note browser.
- Support Light and Dark modes on every screen, including dialogs, inputs, planner rows, and editor preview.
- First visit follows the system preference. The sidebar and Settings expose a theme toggle; an explicit choice persists locally and applies before paint.
- Dark surfaces come from the updated Stitch reference: canvas `#141312`, raised sheet `#1C1B1A`, inset surface `#211F1E`, primary text `#EDE8E1`, editorial accent `#D67754`.
- Preserve a restrained sage interaction accent (`#A8C5B0` in dark mode) and visible focus/disabled/error states. Do not invert screenshots or use pure black.
- Theme changes affect presentation only and must not reset drafts, selected folders, or tasks.

## Direction

A premium paper planner translated into a calm web app.

Keywords:
- Clean
- Minimal
- Editorial
- Planner
- Calm
- Readable
- Focused

References:
- Sunsama: calm planning
- Todoist: task interaction
- Obsidian: note hierarchy
- Notion: typography only

## Visual Rules

Use:
- warm off-white page background
- white or subtle neutral surfaces
- charcoal primary text
- muted gray secondary text
- restrained muted green accent
- subtle borders
- 6–10px radius
- minimal shadows
- generous whitespace
- thin outline icons

Avoid:
- gradients
- glassmorphism
- heavy shadows
- colorful dashboard cards
- analytics widgets
- decorative animations
- excessive rounded containers

## Navigation

Top-level:
- Home
- Planner
- Todo
- Notes

Below:
- Folders
  - Daily
  - Projects
  - Personal
  - Archive

Bottom:
- Settings

Important:
`Notes` = full Notes screen.
`Folders` = Markdown folder hierarchy.

## Home

Hierarchy:

Date

What Matters
- Goal
- Goal
- Goal

Today
- Task
- Task

Quick Note

Recently Edited

Goals should stay visible but quiet.

## Tasks

Tasks should look like planner lines, not cards.

Example:

○ Finish authentication        09:00
○ Write design.md              11:00
✓ Review architecture

Metadata should have low visual priority.

## Planner

Weekly planner should feel like paper planning, not enterprise calendar software.

Use:
- clear day labels
- simple task rows
- subtle current-day highlight
- generous spacing

Avoid:
- dense hourly grid
- excessive controls

## Notes Browser

Show:
- compact note rows
- title
- preview
- updated time
- selected state

Do not show:
- tags
- graph controls
- backlink panels

## Note Editor

Highest-priority screen.

Requirements:
- focused writing area
- comfortable readable width
- clear title
- subtle metadata
- clean Markdown typography
- restrained toolbar

Support visual styling for:
- headings
- paragraphs
- lists
- checkboxes
- links
- code blocks
- blockquotes
- tables

## Search

Shortcut:
Ctrl/Cmd + K

Results:
- title
- excerpt
- path

Keep command palette sparse.

## Responsive

Tablet:
- collapsible sidebar

Mobile:
- drawer navigation
- prioritize writing and task completion
- do not simply shrink desktop columns
