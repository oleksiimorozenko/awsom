# awsom v1.0.0 Release Plan

## Overview

This document tracks the tasks required before releasing awsom v1.0.0.
Based on comprehensive code review performed 2024-11-23.

**Current Version**: 0.11.0
**Target Version**: 1.0.0
**Status**: In Progress

---

## Phase 1: Fix Critical Panics (C2-C5)

Quick wins that prevent runtime crashes.

### C2: Unsafe unwrap() in aws_config.rs
- [x] **File**: `src/aws_config.rs:2550-2626`
- [x] Replace `.unwrap()` with proper error handling
- [x] Affected code: `get_static_credentials()` function
- **Completed**: 2024-11-23

### C3: Panic-prone Default impl
- [x] **File**: `src/session/mod.rs:49`
- [x] Remove `impl Default for SessionManager` or use factory pattern
- **Completed**: 2024-11-23

### C4: Unhandled stdin panics
- [x] **File**: `src/sso_config.rs:78-102`
- [x] Handle `io::stdin().read_line()` Result properly
- [x] Add user-friendly error messages for input failures
- **Completed**: 2024-11-23

### C5: No log fallback
- [x] **File**: `src/main.rs:58`
- [x] Add graceful fallback to stderr if log file creation fails
- **Completed**: 2024-11-23

**Phase 1 Total**: ✅ Complete

---

## Phase 2: Refactor ui/app.rs (C1)

The biggest technical debt - 4,955 LOC monolithic file.

### Proposed Module Structure
```
src/ui/
├── mod.rs              # Public exports
├── app.rs              # Core App struct, lifecycle (~500 LOC)
├── state.rs            # AppState enum, transitions
├── input/
│   ├── mod.rs
│   ├── main.rs         # Main screen key handlers
│   ├── dialogs.rs      # Dialog key handlers
│   └── config.rs       # Config input handlers
├── render/
│   ├── mod.rs
│   ├── main_screen.rs  # Main UI rendering
│   ├── dialogs.rs      # Modal/dialog rendering
│   ├── help.rs         # Help screen
│   └── widgets.rs      # Reusable widgets
└── actions/
    ├── mod.rs
    ├── session.rs      # Session operations
    ├── profile.rs      # Profile operations
    └── accounts.rs     # Account loading
```

### Tasks
- [ ] Extract `AppState` enum to `state.rs`
- [ ] Extract render functions to `render/` module
- [ ] Extract input handlers to `input/` module
- [ ] Extract business logic to `actions/` module
- [ ] Update imports throughout codebase
- [ ] Verify all tests pass after each extraction

**Phase 2 Total**: ~4-6 hours

---

## Phase 3: Refactor aws_config.rs (H2, H3)

Second largest file - 2,650 LOC with repeated INI parsing.

### Proposed Module Structure
```
src/config/
├── mod.rs              # Public exports
├── parser.rs           # Generic INI parser
├── profiles.rs         # Profile CRUD operations
├── sso_sessions.rs     # SSO session management
├── credentials.rs      # Credentials file operations
└── backup.rs           # Backup/restore logic
```

### Tasks
- [ ] Create generic INI parser to replace 5+ similar functions
- [ ] Extract profile management to `profiles.rs`
- [ ] Extract SSO session handling to `sso_sessions.rs`
- [ ] Extract credentials operations to `credentials.rs`
- [ ] Remove dead marker system code (L7)
- [ ] Update imports throughout codebase

**Phase 3 Total**: ~3-4 hours

---

## Phase 4: Fix Unsafe Async Patterns (H4, H8)

Replace `Arc<Mutex<>>` with proper async channels.

### Tasks
- [ ] **File**: `src/ui/app.rs:135, 859-860, 918`
- [ ] Replace `Arc<Mutex<Option<DeviceAuthorizationInfo>>>` with `tokio::sync::watch`
- [ ] Update all readers/writers to use channel API
- [ ] Test login flow still works correctly

**Phase 4 Total**: ~1 hour

---

## Phase 5: Clean Up Dead Code

Remove unused code and `-A dead_code` suppression.

### Tasks
- [ ] Remove `-A dead_code` from CI clippy command
- [ ] Fix or remove all dead code warnings (37+ warnings)
- [ ] Remove legacy marker system in `aws_config.rs:104-230`
- [ ] Implement or remove TODOs in `aws_config.rs:470, 521`

**Phase 5 Total**: ~2 hours

---

## Phase 6: SSM Browser Feature

New major feature for v1.

### Dependencies to Add
```toml
aws-sdk-ssm = "1.x"
aws-sdk-ec2 = "1.x"
```

### Tasks
- [ ] Add AWS SDK dependencies
- [ ] Create SSM client module in `src/ssm/`
- [ ] Add `AppState::SsmBrowser` variant
- [ ] Create SSM browser pane UI
- [ ] Implement instance listing per profile
- [ ] Add "Start Session" action (opens terminal)
- [ ] Add "Copy Command" action (clipboard)
- [ ] Add keyboard shortcut (`s` from main screen)
- [ ] Add CLI command `awsom ssm list`
- [ ] Add CLI command `awsom ssm connect`

**Phase 6 Total**: ~6-8 hours

---

## Phase 7: Add Tests

Increase test coverage before release.

### Priority Areas
- [ ] `aws_config.rs` - INI parsing tests
- [ ] `auth/` - Token cache tests
- [ ] `credentials/` - Credential fetching tests
- [ ] Integration tests for config operations
- [ ] Error path tests for `models.rs`

**Phase 7 Total**: ~4-6 hours

---

## Phase 8: Release v1.0.0

Final release tasks.

### Pre-Release Checklist
- [ ] All critical/high issues resolved
- [ ] All tests passing
- [ ] Clippy clean (no `-A dead_code`)
- [ ] README.md updated with new features
- [ ] CHANGELOG.md created/updated
- [ ] Version bumped to 1.0.0

### Release Tasks
- [ ] Create release commit
- [ ] Tag v1.0.0
- [ ] Push to trigger CI/CD
- [ ] Verify Homebrew formula generated
- [ ] Announce release

---

## Timeline Estimate

| Phase | Description | Estimate |
|-------|-------------|----------|
| 1 | Fix Critical Panics | 50 min |
| 2 | Refactor ui/app.rs | 4-6 hours |
| 3 | Refactor aws_config.rs | 3-4 hours |
| 4 | Fix Async Patterns | 1 hour |
| 5 | Clean Up Dead Code | 2 hours |
| 6 | SSM Browser Feature | 6-8 hours |
| 7 | Add Tests | 4-6 hours |
| 8 | Release | 1 hour |

**Total**: ~22-30 hours of development work

---

## Progress Tracking

### Completed
- [x] Create code-reviewer agent
- [x] Run full code review
- [x] Create this plan file
- [x] Phase 1: Fix Critical Panics (C2-C5)

### In Progress
- [ ] Phase 2: Refactor ui/app.rs

### Blocked
- None

---

## Notes

- Each phase should be committed separately for easy rollback
- Run tests after each significant change
- Update this file as tasks complete
- Consider releasing 0.12.0, 0.13.0 etc. for major milestones before 1.0.0
