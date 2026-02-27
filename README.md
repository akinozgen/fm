# FM

Cross-platform file manager built with Tauri + Vue + Rust.

## Status

Actively developed and already usable for core navigation, selection, and basic file operations.

## Stack

- Frontend: Vue + Vite (inside Tauri webview)
- Backend: Rust + Tauri commands/events
- Persistence: `tauri-plugin-store` + SQLite (`rusqlite`)

## Current Capabilities

- Navigation: back/forward/up, breadcrumb + manual path input
- Views: list + grid, per-directory view mode persistence
- Selection: single, multi, range, rectangle, keyboard-first behavior
- File ops: open, inline rename, create file/folder, delete, permanent delete
- Virtual locations: `fm://welcome`, `fm://home`, `fm://trash`
- Sidebar: user folders, drives, removable devices

## Known Gaps

- Context menu actions are still mostly placeholders
- Copy/cut/paste and transfer queue are not complete
- Search and drag/drop flows are not finalized
- Trash restore flow is not implemented yet

## Development

Requirements:

- Node.js + npm
- Rust toolchain
- Tauri OS prerequisites

Commands:

```bash
npm install
npm run tauri dev
```

Frontend only:

```bash
npm run dev
```

## Project Layout

- `src/` — Vue UI
- `src/lib/` — frontend services/utilities
- `src/components/` — UI components
- `src-tauri/src/` — Rust backend

## License

MIT. See [LICENSE](LICENSE).
