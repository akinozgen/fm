Project: Cross-platform file manager (macOS/Windows/Linux)

Purpose
- Build a fast, reliable, desktop-grade file manager with web UI + Rust core.
- Prioritize native-feeling behavior over web conventions.
- Keep architecture reusable so core logic can be consumed by other frontends later.

Non-Negotiable Product Goals
- Snappy UI with zero filesystem work on the UI thread.
- Operations must feel immediate: optimistic UX where safe, clear progress/state where needed.
- No visible “laggy app” behavior during navigation, selection, or large-directory browsing.
- Feature implementations must be end-to-end, not partial stubs.

Architecture Contract
- UI Layer (Tauri webview):
  - Renders state and dispatches user intents only.
  - No direct filesystem reads/writes.
  - Minimal business logic, mostly view and interaction orchestration.
- Core Layer (Rust):
  - Owns filesystem access, long-running operations, indexing, thumbnails, metadata.
  - Exposes typed commands and streams typed events/chunks.
  - Designed to run headless and be reusable by future non-web UIs.
- IPC Layer:
  - Stable, versioned contracts for command args/results and event payloads.
  - Incremental/diff/chunk updates preferred over full-state reloads.
  - Include cancellation IDs / request IDs for async workflows.

Interaction/UX Principles (Desktop-Native First)
- Favor native desktop interaction patterns (Explorer/Finder conventions) over web-app patterns.
- Cursor behavior:
  - Keep default cursor for clickable UI unless a dedicated resize/drag affordance is needed.
- Keyboard is first-class:
  - Implement expected file-manager shortcuts and navigation semantics.
  - New features should include keyboard flow when practical.
- Selection behavior:
  - Must support robust single, multi, range, and rectangle patterns.
  - Selection state should remain predictable across mouse + keyboard combinations.
- Modals/dialogs:
  - Prefer inline/in-context editing for lightweight actions (rename/new item).
  - Prefer OS-native dialogs for destructive confirmations where appropriate.
- Visual style:
  - Compact, clean, native-like.
  - Avoid heavy web-looking effects and default browser UI feel.

Performance Rules
- Never perform filesystem operations in frontend code.
- Stream directory entries in chunks; do not block until whole directory is resolved.
- Cancel in-flight listing/walk jobs when navigation target changes.
- Keep UI rendering incremental and bounded (virtualization-ready patterns).
- Separate IO-bound and CPU-bound workloads in core design.
- Avoid large synchronous transforms in render paths.

State & Data Rules
- Single source of truth for operational data is backend/core.
- Frontend may keep ephemeral interaction state (selection, edit mode, focus cursor).
- Persisted settings:
  - Store preferences in platform config directory via Tauri store plugin.
  - Store queryable operational data/history in SQLite.
- Preferences split:
  - Global preferences (show hidden, show extensions, selection checkboxes, etc.).
  - Per-directory preferences (view mode and future directory-specific options).

Feature Delivery Rules
- Implement features as vertical slices:
  - UI behavior
  - command wiring
  - backend operation
  - state refresh/reconciliation
  - error handling path
- Do not ship “UI only” placeholders for core file operations unless explicitly requested.
- If a feature changes selection/navigation, preserve consistency with existing logic.

Error Handling Rules
- Backend commands return clear, actionable error strings.
- Frontend should fail gracefully and keep UI consistent (no stuck edit/mode states).
- On operation failure, avoid silent corruption of selection/history/path state.

Code Organization Rules
- Keep components modular and extensible; avoid giant mixed-responsibility files where possible.
- Keep protocols and interfaces explicit; avoid ad-hoc payload shapes.
- Prefer small composable utilities for keyboard/input/state logic.
- Avoid inline style/script sprawl for complex behaviors.

Development Workflow Contract
- User runs the dev loop; agent should not run builds/tests unless explicitly requested.
- Iterate quickly on reported UX bugs before moving to new feature layers.
- Preserve existing user changes; never revert unrelated modifications.
- Make focused edits with minimal blast radius.
- When adding behavior, align with already-established app conventions first.

Current Milestone Focus (Living)
- Usable daily-driver baseline:
  - fast navigation and listing
  - robust selection and keyboard workflows
  - foundational file operations (rename/create/delete/copy/move pipeline)
  - stable persistence for preferences/history
- Polish and extensibility:
  - continue reducing web feel
  - keep new features modular for later plugin/advanced capabilities

Quality Bar for “Done”
- Behavior matches expected desktop file manager semantics for the scope implemented.
- No obvious regressions in keyboard navigation, selection, or path navigation.
- No UI-thread filesystem work introduced.
- Contracts remain coherent and reusable.
