# Memories

## Patterns

### mem-1778347421-d386
> web-notes uses tao + wry for the native macOS shell, persists the imported notes folder at config_dir/web-notes/config.json, and serves notes through a custom app:// protocol so HTML files keep working with relative JS/CSS assets.
<!-- tags: app, rust, webkit | created: 2026-05-09 -->

## Decisions

## Fixes

### mem-1778349389-40db
> failure: cmd=ralph tools task complete task-1778347646-ef8b, exit=2, error=unrecognized subcommand 'complete', next=check ralph tools task --help for close/done command
<!-- tags: tooling, error-handling | created: 2026-05-09 -->

### mem-1778349373-8f15
> failure: cmd=ralph tools interact progress "Completed Step 7...", exit=1, error=No bot token / RALPH_TELEGRAM_BOT_TOKEN unset, next=skip external progress update in this environment
<!-- tags: tooling, error-handling | created: 2026-05-09 -->

### mem-1778349222-af15
> failure: cmd=cargo fmt -- --check, exit=1, error=formatting drift in src/app.rs after Step 7 recovery edits, next=run cargo fmt before rechecking
<!-- tags: rust, formatting, error-handling | created: 2026-05-09 -->

### mem-1778348930-2f36
> failure: cmd=cargo fmt -- --check, exit=1, error=formatting drift in src/domain.rs and src/notes.rs after Step 6 navigation/refresh tests, next=run cargo fmt before rechecking
<!-- tags: rust, formatting, error-handling | created: 2026-05-09 -->

### mem-1778348730-f533
> failure: cmd=cargo fmt -- --check, exit=1, error=src/app.rs formatting drift after Step 5 note-rendering edits, next=run cargo fmt before rechecking
<!-- tags: rust, formatting, error-handling | created: 2026-05-09 -->

### mem-1778348550-ab65
> failure: cmd=cargo fmt -- --check, exit=1, error=src/domain.rs formatting drift after tree-state edits, next=run cargo fmt before rechecking
<!-- tags: rust, formatting, error-handling | created: 2026-05-09 -->

### mem-1778348188-20be
> failure: cmd=cargo fmt -- --check, exit=1, error=src/app.rs formatting drift after edits, next=run cargo fmt before rechecking
<!-- tags: rust, formatting, error-handling | created: 2026-05-09 -->

### mem-1778347962-881b
> failure: cmd=ralph tools task start task-1778347627-1701 --format json, exit=2, error=unexpected argument '--format' for task start, next=run task start without --format because only list supports formatted output
<!-- tags: tooling, error-handling | created: 2026-05-09 -->

### mem-1778347585-75c7
> failure: cmd=ls -la .agents/planning && file .agents/planning/2026-05-09-rust-notes-app && wc -l .agents/planning/2026-05-09-rust-notes-app && sed -n '1,260p' .agents/planning/2026-05-09-rust-notes-app, exit=1, error=planning target is a directory so wc read failed, next=list directory contents and inspect contained plan files
<!-- tags: tooling, error-handling | created: 2026-05-09 -->

### mem-1778347577-8ef7
> failure: cmd=sed -n '1,220p' .ralph/agent/scratchpad.md, exit=1, error=.ralph/agent/scratchpad.md: No such file or directory, next=create scratchpad file before planning updates
<!-- tags: tooling, error-handling | created: 2026-05-09 -->

## Context
