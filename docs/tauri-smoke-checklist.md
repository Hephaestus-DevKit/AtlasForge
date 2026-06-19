# Tauri Smoke Test Checklist

Use this checklist after building the packaged `.exe` to verify core functionality.

## Build

```powershell
npm run tauri -- build --target x86_64-pc-windows-msvc
```

The installer should appear at `src-tauri/target/release/bundle/msi/`.

## Pre-Launch

- [ ] Installer runs without errors
- [ ] App appears in Start Menu / Desktop shortcut
- [ ] App launches without crash

## Core Flow

### Dashboard
- [ ] Dashboard page loads without blank screen
- [ ] Stats cards render (even if zero)

### Assets
- [ ] Assets page shows empty state when no workspace roots configured
- [ ] "Add Workspace Root" form opens
- [ ] Browse button opens native folder picker
- [ ] Adding a valid root succeeds and appears in table
- [ ] Adding duplicate root shows error toast
- [ ] Removing a root works with confirm modal

### Repositories
- [ ] Repositories page shows empty state when no repos scanned
- [ ] "Scan All" triggers scan and shows progress
- [ ] Repository list renders after scan
- [ ] Clicking a repo opens detail panel
- [ ] Filter by language works
- [ ] Filter checkboxes (No CI, No README) work

### Tasks
- [ ] Tasks page shows empty state when no tasks
- [ ] Tasks appear after running scan/audit operations
- [ ] Task status badges render correctly

### Knowledge
- [ ] Knowledge page shows search interface
- [ ] "No repositories found" empty state when no roots
- [ ] Search shows loading spinner while searching
- [ ] Search results render correctly when indexed

### Automations
- [ ] Automations page shows empty state when no rules
- [ ] Creating a rule via form works
- [ ] Toggling rule enabled/disabled works
- [ ] Deleting a rule shows confirm modal

### Settings
- [ ] Settings page renders
- [ ] Adding AI provider works
- [ ] Form fields have accessible labels (click label focuses input)

## UI Hardening

- [ ] No page shows a blank white screen on error (ErrorBoundary fallback visible)
- [ ] Long paths do not break table layout (horizontal scroll works)
- [ ] Buttons are disabled while operations are in progress
- [ ] Toast notifications appear for success/error feedback
- [ ] Confirm modal appears before destructive actions
- [ ] Loading spinners show during data fetches
- [ ] Empty states show when no data is available
- [ ] All form inputs are reachable via keyboard Tab

## Exit

- [ ] App closes cleanly via window X button
- [ ] No background process lingers after close
