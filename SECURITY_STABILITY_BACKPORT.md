# Security & Stability Backport — audit log

This document records the security- and stability-relevant commits backported from
`ogulcancelik/herdr` (upstream) onto `vitalybe/herdr` (the fork this repository was
forked from), on the `security-stability-backport` branch, and why. It is the audit
trail requested for this task: what was reviewed, what was picked, what was
deliberately skipped, and how conflicts were resolved.

## Fork setup

- `sasha-dn/herdr` was created as a proper GitHub fork of `vitalybe/herdr` via
  `gh repo fork vitalybe/herdr --clone=true`, so the fork network is
  `sasha-dn/herdr` → forked from `vitalybe/herdr` → forked from `ogulcancelik/herdr`.
- Remotes in the clone: `origin` = `sasha-dn/herdr` (push target), `parent` =
  `vitalybe/herdr` (read-only reference), `upstream` = `ogulcancelik/herdr` (read-only
  reference).

## Divergence point

`git merge-base parent/master upstream/master` resolves to `f54d8e8c0aee5d3ec9724234d1bcd418bff8ff2f`,
dated 2026-06-30 — this matches the expected divergence point exactly, no discrepancy
to note. From that point:

- `vitalybe/herdr` (`parent/master`) has 38 commits of its own (sidebar/agent-hierarchy
  UX work: parent/child agent tree, `agent set-parent`/drag-to-reparent, `agent
  children` CLI, sidebar band/divider/same-name-pane collapsing, undo-close, previous/
  next-pane bindings, last-selected-pane memory, and its own fix for
  plugin-registry-reload-after-live-handoff).
- `ogulcancelik/herdr` (`upstream/master`) has 194 commits `vitalybe/herdr` lacks (not
  ~192 as estimated, but close).

## Method

1. Listed all 194 upstream-only commits in chronological order
   (`git log --reverse f54d8e8c..upstream/master`).
2. Classified each by reading its subject line, and its diff whenever the subject was
   ambiguous, against the rubric in the task: security (crashes/panics from untrusted
   input, memory-safety, auth/permission/socket handling, path/symlink issues,
   injection, privilege handling, TOCTOU, CVE/RUSTSEC-adjacent) and stability (hangs/
   deadlocks/races, zombie/orphaned processes, data loss/corruption, protocol/version-
   mismatch handling, reconnect/live-handoff robustness, resource leaks, other crash
   fixes) vs. skip (features, cosmetic/UX changes, docs/translation, behavior-neutral
   refactors).
3. Cherry-picked the selected commits (in upstream chronological order) onto a
   `security-stability-backport` branch created off `origin/master`
   (`vitalybe/herdr`'s master, i.e. including its 38 commits), using
   `git cherry-pick -x` throughout so every resulting commit message carries a
   `(cherry picked from commit <original-upstream-sha>)` trailer.
4. While cherry-picking, found and backported 3 additional commits that either turned
   out to be missed on the first classification pass, or were required as
   compile-time prerequisites for commits already selected (see "Commits added during
   cherry-picking" below).
5. Found 2 originally-selected commits that, on hitting cherry-pick conflicts, turned
   out not to actually apply — one because its target feature was itself correctly
   skipped, one because the fork had already fixed the identical bug independently.
   Both were dropped (see "Commits dropped during cherry-picking" below).
6. Verified the result builds (`cargo check` / `cargo build`, using Homebrew
   `zig@0.15` — see "Build and test results") and ran the full test suite, fixing two
   small test-expectations issues that the newly-active unknown-config-key detector
   correctly surfaced.
7. Merged `security-stability-backport` into `master` with a regular (non-squash)
   merge, preserving every individual commit and its `-x` trailer, and pushed both
   `master` and `security-stability-backport` to `origin` (`sasha-dn/herdr`).

**Total: 194 upstream-only commits reviewed, 27 backported, 167 skipped.**

## Commits backported (27), in the order cherry-picked

Each entry: our commit SHA on `security-stability-backport` — original upstream SHA —
one-line message — why it's security/stability — conflict notes (if any).

1. **`fadc682`** — upstream `e9bcea9c` — *fix: reuse remote ssh connection* —
   **Stability**: reworks `herdr --remote`'s SSH connection handling to reuse one
   OpenSSH connection across setup/install/check/bridge steps instead of one per
   step, avoiding repeated password prompts and connection churn. Clean cherry-pick.

2. **`a2ce427`** — upstream `26db26e4` — *fix: keep remote sessions alive across
   disconnects* — **Stability**: reconnect robustness — remote sessions are kept in
   their own login-independent session so a network drop disconnects only the
   client, not the remote pane. Clean cherry-pick.

3. **`04ade38`** — upstream `74a771d1` — *fix: replace stale agent session from
   foreground reports* (refs #943) — **Stability/security-adjacent**: fixes session-
   owner-conflict arbitration in `TerminalState` so a stale/wrong agent's persisted
   session ref can't linger or be incorrectly reused across a resume; this is
   correctness of session ownership, not just cosmetics. Clean cherry-pick.

4. **`2459f97`** — upstream `aa0768b7` — *fix: stop sgr mouse sequences leaking into
   panes* — **Stability**: prevents SGR mouse escape sequences from leaking into the
   pane's input stream as spurious characters (same class of issue as the upstream
   "spurious characters written to command line" bug class). Clean cherry-pick.

5. **`799d509`** — upstream `6f148588` — *fix: scope empty image paste bridge to
   remote* (refs #986) — **Security**: previously, a *local* (non-`--remote`) Herdr
   client would also trigger the clipboard-image-paste bridge on an empty bracketed
   paste; this scopes that bridge to remote clients only, so a local client no longer
   over-reaches into clipboard-image handling it doesn't need. Conflicts in
   `docs/next/CHANGELOG.md`, `docs/next/website/src/content/docs/configuration.mdx`,
   and `src/client/mod.rs` (see notes below).

6. **`13c4f27`** — upstream `4eceb75d` — *fix: kitty graphics streams freeze on
   partial updates and leak host images* (#948) — **Stability + security-adjacent**:
   fixes a render-loop freeze under continuous kitty-graphics streams (a dropped
   notify could leave a pane showing a stale frame indefinitely) and a host-image
   leak/misattribution bug (weak fingerprinting let a retransmitted image with
   different content be treated as unchanged). Clean cherry-pick.

7. **`8328d89`** — upstream `797981c7` — *fix: allow foreground agent session
   takeover* (refs #943) — **Stability/security-adjacent**: extends the same session-
   ownership-arbitration mechanism as #3 above, so a foreground agent can correctly
   take over hook authority from a conflicting owner instead of getting silently
   rejected or wedged. Clean cherry-pick.

8. **`c405700`** — upstream `c78349f4` — *fix: wait for server stop socket cleanup* —
   **Stability**: `herdr server stop` now waits until both server sockets are
   actually unreachable before returning, closing a resource-cleanup race that could
   cause an immediate restart to fail. Clean cherry-pick.

9. **`3e99e5e`** — upstream `d5521e3d` — *fix: preserve pi and omp working status
   across reloads* (#984) — **Stability** — **included as a prerequisite** for #10
   below (it creates `src/integration/assets/herdr-agent-state.test.ts`, which #10's
   diff modifies), but also reclassified as a genuine stability fix in its own right
   on inspection: it's the same class of "agent working-status gets reset/lost across
   a session reload" fix as several other pi/omp commits already selected. Clean
   cherry-pick once inserted ahead of #10.

10. **`adf1cec`** — upstream `b1d1a07b` — *fix: retry pi state socket reports* (refs
    #1049) — **Stability**: adds retry robustness so a dropped pi status-socket report
    doesn't leave the pane's displayed agent state stuck/incorrect. Required #9 as a
    prerequisite (see above); clean once applied after it.

11. **`9bfcb19`** — upstream `d9230f51` — *fix: clarify remote ssh auth failures*
    (#1084) — **Security**: replaces a raw Rust `Debug`-formatted I/O error with a
    clear SSH-authentication-denied message and guidance — auth-failure handling
    clarity, in scope per the "auth ... handling bugs" rubric bucket. Conflict in
    `docs/next/CHANGELOG.md` only (see notes).

12. **`e71af52`** — upstream `e4a7095c` — *fix: guard windows process tree cycles*
    (refs #1083) — **Stability**: adds a visited-set cycle guard to Windows process-
    tree walking so a PID-reuse cycle can't cause unbounded traversal/memory growth
    (changelog: "ignores cyclic process-parent snapshots instead of growing memory
    until the server aborts"). Conflicts in `docs/next/CHANGELOG.md` and
    `src/platform/windows.rs` (see notes).

13. **`b18cc57`** — upstream `adbaae68` — *fix: reanchor pi status after session
    replacement* (#1189) — **Stability**: three squashed sub-fixes, all in the pi
    session-state family — reanchoring status after replacement, guarding active pi
    sessions against nested startup (race guard), and serializing pi session-
    replacement reports (race/ordering guard). Clean cherry-pick.

14. **`cc472ef`** — upstream `9c453427` — *fix: preserve explicit session sockets
    during live handoff* (refs #1180, reported by dvic) — **Stability**: live-handoff
    robustness — an explicitly-named session socket path is preserved across a live
    handoff instead of being dropped. Clean cherry-pick.

15. **`d097c57`** — upstream `749e85e0` — *fix: detach windows server from host
    terminal* (refs #1329) — **Stability**: the Windows server process is detached
    (`DETACHED_PROCESS` creation flag) from the terminal console that launched it, so
    closing the launching terminal no longer kills persistent pane processes.
    Substantial conflicts in `src/platform/windows.rs` and `docs/next/CHANGELOG.md`
    (see notes — this is the largest non-cherry-pick-order conflict after the Linux
    one below).

16. **`c272645`** — upstream `045f506e` — *fix: keep windows wait connections open*
    (refs #963) — **Stability** — **missed on the first classification pass**,
    discovered only as a prerequisite for #17 (see "Commits added during cherry-
    picking"). Fixes `herdr wait`/API connections dropping prematurely on Windows.
    Clean cherry-pick.

17. **`8f2faef`** — upstream `d64cf40c` — *fix: keep windows api connections open*
    (refs #1279) — **Stability**: Windows named-pipe API clients now stay connected
    while waiting for the initial request bytes, fixing intermittent `BrokenPipe`
    failures on `status server`/`api snapshot`/etc. Conflicts in
    `docs/next/CHANGELOG.md` and `src/api/server.rs` (see notes; the `src/ipc.rs`
    conflict resolved cleanly as "take theirs" since it's this commit's own
    self-contained new code).

18. **`fc8d043`** — upstream `64de9279` — *fix: allow slow server shutdowns* —
    **Stability**: relaxes an overly-tight timeout so a legitimately slow server
    shutdown isn't treated as a failure. Clean cherry-pick.

19. **`3f6b9f0`** — upstream `b425260f` — *perf: move session autosaves off main
    loop* — **Stability** — **missed on the first classification pass** (identified
    as a stability candidate during initial triage — moving blocking session-save
    disk I/O off the render/input loop prevents a real UI-hang class of bug — but
    never added to the final selection list). Discovered as a prerequisite for #20
    (see "Commits added during cherry-picking"). One conflict in `src/app/session.rs`
    (see notes).

20. **`7dff5b2`** — upstream `75ed6abc` — *fix: reap detached custom command
    children* (#1384, refs #1360) — **Stability**: reaps detached custom-command
    child processes after exit instead of letting them accumulate as zombies —
    matches the rubric's "zombie or orphaned process handling" bucket directly.
    Required #19 as a prerequisite; clean once applied after it.

21. **`7eb2743`** — upstream `9e4fdd61` — *fix: retry dropped omp lifecycle reports*
    (refs #1310) — **Stability**: retries OMP lifecycle status reports dropped by a
    startup race, preventing a pane from getting stuck showing stale/incorrect
    status. Clean cherry-pick.

22. **`845ecdc`** — upstream `b88aa130` — *fix: flush live handoff responses before
    exit* — **Stability**: the old server now waits (up to a timeout) for its live-
    handoff response to actually be written to the client before exiting, preventing
    a race where the client never sees the handoff acknowledgement. Conflicts in
    `src/server/headless.rs` (two large diff-context-noise blocks; see notes).

23. **`e4d9058`** — upstream `80d2958a` — *fix(linux): scope foreground process
    discovery to panes* (refs #1399) — **Security/stability**: replaces a global,
    whole-`/proc`-scanning process-group-membership lookup with one that only ever
    walks the pane's own process tree (via `/proc/<pid>/task/<tid>/children`),
    so foreground-process detection can no longer match an unrelated process on the
    host that happens to share a recycled process-group id. This is the largest
    single conflict of the backport (see notes) — reconstructed the equivalent
    upstream end-state by hand rather than also backporting an intermediate,
    security-irrelevant caching commit.

24. **`fa3969c`** — upstream `600753b4` — *fix: guard cli requests against protocol
    mismatches* (#1453, refs #1435) — **Stability**: adds a `protocol_guard` that
    rejects/handles CLI-to-server socket requests when client/server protocol
    versions mismatch, instead of behaving unpredictably — directly matches the
    rubric's "protocol or version-mismatch handling" bucket. Conflicts in `src/cli.rs`
    and `tests/cli_wrapper.rs` (two unrelated tests dropped as diff-context noise for
    features we don't have; see notes).

25. **`40dfd35`** — upstream `8915f01c` — *fix: stop agent wait when pane closes*
    (#1488) — **Stability**: `herdr wait agent-status` now returns `pane_not_found`
    promptly when its target pane closes, instead of hanging until the full timeout
    — a direct hang fix. Conflict in `docs/next/CHANGELOG.md` only (see notes — this
    is also where a stray unresolved conflict-marker line was briefly, mistakenly
    left in the file and then caught and amended out before proceeding, see below).

26. **`49a0b29`** — upstream `f11be93e` — *fix: report unknown config keys* (#1575,
    refs #1573) — **Stability**: config keys that don't match any known field
    (typos, stale/renamed keys) are no longer silently discarded; they're now
    surfaced as `"unknown config key <path>; ignoring key"` diagnostics through the
    existing diagnostic pipeline. This is a real robustness improvement — it
    directly caught a pre-existing stale `agent_panel_scope` key in one of this
    fork's own tests that had been silently swallowed until now (see "Test fixes"
    below). Large, multi-file conflict — deliberately did not adopt two upstream
    prerequisite commits that rebuild `config_diagnostic_summary`'s presentation
    layer and add a new `herdr config check` CLI subcommand neither of which exist
    in this fork; see notes.

27. **`e68da37`** — upstream `60c04005` — *fix: serialize opencode lifecycle
    reports* (refs #1519) — **Stability**: serializes concurrent opencode lifecycle
    report requests through a promise chain instead of racing them, preventing out-
    of-order status updates. Conflict was just an integration-asset version-number
    bump (see notes).

Two further commits on this branch are **not** upstream cherry-picks — they are
build/test fixups for mistakes this backport introduced while merging the above (see
"Build and test results" below): `a4a9b8c` and `740acb8`.

## Commits added during cherry-picking (not on the original selection list)

Discovered mid-backport, either because a cherry-pick failed to apply cleanly and
tracing the missing symbol led to an upstream commit that should have been selected,
or because a candidate flagged during initial triage was never carried through to the
final list:

- **`d5521e3d`** ("preserve pi and omp working status across reloads", #984) —
  needed by `b1d1a07b`; reclassified as a genuine stability fix in its own right, not
  merely a prerequisite (see entry #9 above).
- **`045f506e`** ("keep windows wait connections open", #963) — needed by `d64cf40c`;
  its title didn't match the keyword search used in the first classification pass.
  Genuine stability fix, same family as the other "keep windows X connections open"
  commits already selected (see entry #16 above).
- **`b425260f`** ("move session autosaves off main loop") — needed by `75ed6abc`;
  flagged as a stability candidate during initial triage but never added to the
  final list. Moves blocking session-save I/O off the main render/input loop,
  preventing a real UI-freeze class of bug (see entry #19 above).

## Commits dropped during cherry-picking (were selected, turned out not to apply)

- **`e9cbcf2f`** ("fix: handle pane graphics stream disconnect race") — its target
  file, `src/api/server/pane_graphics_stream.rs`, does not exist anywhere in this
  tree; it belongs entirely to upstream feature commit `88370e15` ("add isolated
  pane graphics streaming"), which was correctly classified as a feature and not
  backported. With the feature absent, this race-condition fix inside it has nothing
  to apply to. Not backported.
- **`a22454f2`** ("fix: preserve plugins during live handoff", #893) — this is
  exactly the bug the task description flagged as a likely conflict source: the fork
  (`vitalybe/herdr`) already carries its own independent fix for
  "plugin-registry-reload-after-live-handoff". Confirmed by diff: our tree's
  `HandoffApp` constructor already has
  `app.state.installed_plugins = load_plugin_registry(false)` with an explanatory
  comment describing the exact same bug upstream's commit fixes; upstream's change to
  `load_plugin_registry(app.no_session)` is functionally identical (`app.no_session`
  is `false` at that point). Not backported — nothing to add.

## Borderline commits skipped, and why

A representative sample of the closer calls (the full 194-commit review is
summarized by the totals above; these are the ones that could plausibly have gone
either way):

- **`be901fead`** *fix: opencode — report subagent permission prompts as blocked +
  handle session.status object form* (#838) — reads like a permission-handling fix,
  but it's really a third-party agent-integration status-label heuristic (detecting
  when an opencode UI is showing a permission prompt, for the sidebar status
  indicator), not a security control of Herdr itself. Classified as feature/cosmetic.
- **`c71c6c10`** *fix: resolve split current from caller pane* — a CLI
  `pane split --current` correctness fix (resolves to the wrong pane in one edge
  case), not a crash/hang/leak. Classified as feature/behavior fix, skipped.
- **`d6285aaa`** *fix: avoid render-time plain url scans* — moves URL-hyperlink
  detection out of the render hot path; plausibly a perf/responsiveness fix, but
  with no crash/hang/leak language in the commit and a large, invasive diff, kept on
  the "skip" side of the line as a performance refactor rather than a stability fix.
- **`e0758c32`** *feat: harden named agent cli workflows* — despite "harden" in the
  name, this is a large new-feature commit (named-agent CLI/API schema additions),
  not a hardening fix to existing functionality. Skipped as a feature.
- **`3e8f9df1`** *fix: improve config diagnostic delivery* and **`a6905364`**
  *fix: clarify config fallback diagnostics* — both are prerequisites of `f11be93e`
  (backported, see above) that rebuild the human-readable config-diagnostic summary
  wording and add a new `herdr config check` CLI subcommand. Message-clarity work,
  not itself security/stability, and this fork's own `config_diagnostic_summary` is
  a materially different (simpler) implementation; adopting these would mean
  pulling in a new user-facing CLI command as pure scaffolding. Skipped; only the
  underlying unknown-key-detection data (from `f11be93e`) was kept.
- **`ef67a970`** *perf(linux): cache foreground process group scans* — pure
  performance commit (adds a TTL cache), correctly skipped; its cache design was
  later superseded by `80d2958a`'s scoped-lookup rewrite (backported, see above), so
  the cache was never introduced at all rather than added-then-removed.

## Conflict-resolution notes (the non-trivial ones)

Nearly every cherry-pick that touched `docs/next/CHANGELOG.md` conflicted, for a
structural reason worth calling out once: upstream's `CHANGELOG.md` accumulated
entries from the ~167 commits *not* backported, and later got split into dated
release sections (`## [0.7.2]`, `## [0.7.3]`, `## [0.7.4]`) by `release:` commits
that were out of scope. Each actual cherry-pick's own diff to this file only ever
added one or two new bullet lines; every conflict was resolved by checking
`git show <original-sha> -- docs/next/CHANGELOG.md` to find exactly which line(s)
that commit added, keeping this fork's own `## Unreleased` section, and discarding
release-section headers/unrelated bullets that were pulled in only as merge context.

Other notable resolutions:

- **`6f148588`** (paste bridge): a `draw_host_cursor` local pulled in as diff context
  came from a skipped, purely-cosmetic commit (`d1471e64`, "draw host cursor on
  windows"); dropped it since the paste-bridge fix doesn't use it.
- **`e4a7095c`** (Windows process-tree cycles): `src/platform/windows.rs` conflict
  context included two unrelated pre-existing tests from a skipped scrollback-editor
  commit; kept only the two new cycle-guard tests this commit actually adds.
- **`b1d1a07b`** (pi socket retries): required `d5521e3d` as a prerequisite (see
  above) because it modifies a shared test file that commit creates.
- **`749e85e0`** (detach Windows server): `src/platform/windows.rs` diff context
  pulled in ~80 lines of unrelated custom-command-process/scrollback-editor helpers
  and tests from skipped feature commits; applied only the three real changes
  (`DETACHED_PROCESS` creation flag, `IsProcessInJob`-based detection, one new test).
- **`b425260f`** (session autosave off main loop): one real conflict in
  `src/app/session.rs` where the fork's own sidebar drag-to-reparent feature
  computes two extra persistence keys (`agent_manual_order_keys`,
  `pane_section_order_keys`) at the same call site upstream renamed a local
  variable; kept the fork's extra computation, applied upstream's rename.
- **`d64cf40c`** (keep Windows API connections open): `src/api/server.rs` wanted to
  import `is_connection_closed_error`/`local_stream_peer_closed` from `crate::ipc`,
  but this fork's `server.rs` still had its own local, duplicate copy of that helper
  (the refactor that unified it into `ipc.rs` was itself `045f506e`, added as a
  prerequisite - see above). Once `045f506e` was in, this applied cleanly.
- **`b88aa130`** (flush live handoff responses): `src/server/headless.rs` conflict
  context included an entire unrelated dispatch-refactor function
  (`dispatch_headless_runtime_mutation`/`handle_deferred_requests_headless`, from
  skipped "route X through runtime dispatch" refactors) and an unrelated
  spinner/title-flooding test; discarded both, kept only the
  `response_write_complete` threading through the live-handoff path and all 8
  `ApiRequestMessage` test-construction sites already in this tree.
- **`80d2958a`** (scope Linux foreground process discovery): the largest conflict.
  This fork never had the intermediate cache implementation
  upstream's diff assumes as its starting point (that cache came from the
  correctly-skipped, purely-perf `ef67a970`), so instead of also backporting that
  perf commit as a stepping stone, reconstructed the equivalent upstream end-state
  by hand: added `ProcGroupMember`, `foreground_process_group_members[_with]`,
  `process_tree_pids`, `process_task_ids`, `process_task_children`,
  `numeric_file_name`, `live_process_group_member`, rewired `foreground_job`, and
  kept only the 3 new tests that exercise this logic (discarding unrelated
  WSL-cursor and scrollback-editor context pulled in from other skipped commits).
  `src/detect/mod.rs` and `tests/api_ping.rs` auto-merged cleanly and needed no
  changes.
- **`600753b4`** (CLI protocol guard): `src/cli.rs` wanted `mod runtime;` alongside
  the new `mod protocol_guard;` — `mod runtime` belongs to a skipped refactor and
  doesn't exist here, so only `protocol_guard` was added. `tests/cli_wrapper.rs`
  conflicts were two unrelated tests (`workspace_report_metadata_sends_token_patch`,
  `api_snapshot_prints_live_session_snapshot`) for CLI surface (`workspace
  report-metadata`, `api snapshot`) this fork doesn't have; dropped both.
- **`8915f01c`**: while resolving its `CHANGELOG.md` conflict, a stray unresolved
  `>>>>>>>` marker line was accidentally left in the file and briefly committed.
  Caught immediately by a full-tree grep for leftover conflict markers
  (`grep -rln '^<<<<<<< \|^=======$\|^>>>>>>> '`) before any further commits were
  built on top, and fixed with `git commit --amend` on that same, not-yet-shared
  commit.
- **`f11be93e`** (unknown config keys): the big one, alongside `80d2958a`. Adopted
  the core `serde_ignored`-based unknown-key-path detection
  (`ConfigKeyPathSegment`/`config_key_path`/`format_config_key_path`/
  `unknown_config_key_diagnostics`/`deserialize_with_ignored`) and wired it into
  `Config::load()` and `load_live_config_from_str()`. Deliberately did **not** adopt
  this commit's two prerequisites (`3e8f9df1`, `a6905364` — see "Borderline commits"
  above), so left `config_diagnostic_summary` as this fork's own simple
  implementation untouched, and dropped 4 tests in `src/config/io.rs` and 5 in
  `tests/cli_wrapper.rs` that depended on that abandoned presentation layer or on a
  `herdr config check` CLI subcommand this fork doesn't have. Kept the one new test
  that validates the real backported behavior without any such dependency
  (`reload_config_applies_known_sibling_and_summarizes_unknown_key`), and one
  pre-existing generic test (`config_loaders_report_unreadable_path`) that's
  compatible as-is.
- **`60c04005`** (serialize opencode reports): integration asset version bump
  conflict (`OPENCODE_INTEGRATION_VERSION` / `HERDR_INTEGRATION_VERSION` comment) —
  this fork was at `7` (having skipped an intervening upstream bump to `8`); bumped
  to `8` rather than jumping to upstream's `9`, since `9` would imply also having
  picked up the version-8 change this backport doesn't include.

## Test fixes required after backporting `f11be93e`

Making the unknown-config-key detector live surfaced three pre-existing issues in
tests, all fixed in a follow-up commit (`740acb8`):

1. A test I'd kept from `f11be93e`'s own diff
   (`reload_config_applies_known_sibling_and_summarizes_unknown_key`) asserted the
   abandoned impact-classification wording (`"config.toml has unknown keys; herdr
   config check"`); changed it to check the actual diagnostic substring this fork's
   `config_diagnostic_summary` produces.
2. A test fixture (`load_live_config_warns_about_unknown_keys_and_applies_known_siblings`)
   included a `[ui.sidebar.agents.rows_by_agent]` section from an upstream
   sidebar-tokens feature this fork never had in that shape (this fork's `ui` config
   has no `sidebar` field at all); removed the section.
3. Most importantly: `reload_config_updates_live_state`, a pre-existing test on
   `vitalybe/herdr`'s own master (**not** something this backport introduced),
   asserted a fully clean `Applied` reload with zero diagnostics, but its fixture
   carried a stale `agent_panel_scope = "current"` key that isn't a real config
   field anywhere in this schema (confirmed: the only other reference to
   `agent_panel_scope` in the codebase is a test literally verifying it's an
   ignored/legacy key). Before this backport, unknown keys were silently discarded
   with zero diagnostic, so the test passed by accident. The newly backported
   detector now (correctly) flags it as `"unknown config key ui.agent_panel_scope;
   ignoring key"` — this is the fix working exactly as intended, not a regression.
   Removed the stale key from the fixture, since nothing the test checks depends on
   it.

Fixing these three cleared what looked like ~37 additional failures in the full
`cargo test --bins` run; those were a cascading effect of one of the three tests
panicking while holding a shared `Mutex` used by many tests to serialize environment-
variable mutation (once poisoned, every later test sharing that lock fails with
`PoisonError` regardless of its own correctness). Confirmed by running the affected
tests in isolation before and after the fix.

There is a small, separate build-fix commit (`a4a9b8c`) for two auto-merge artifacts
that referenced code this fork doesn't have: a `mastracode` entry in a bundled-asset-
version regression test (MastraCode support was correctly never backported — a
feature, not security/stability), and a missing `parent` field (this fork's own
agent-hierarchy addition to `PaneInfo`) in a test helper introduced while cherry-
picking `045f506e`.

## Build and test results

- **Toolchain**: this project vendors `libghostty-vt` and builds it via `zig build`
  in `build.rs`; the pinned version is `0.15.2`. The sandbox this backport was
  performed in did not have `zig` preinstalled; `brew install zig` alone installs
  `0.16.0` (too new — `build.zig` calls a since-changed `zig.Build.Dir` API and
  fails outright), so `zig@0.15` was installed via `brew install zig@0.15`
  (keg-only) and pointed to explicitly with `ZIG=/opt/homebrew/opt/zig@0.15/bin/zig`.
  This is an environment/toolchain detail, not a defect in the backport.
- **`cargo check --all-targets`**: clean, zero errors/warnings-as-errors, after
  fixing two build breaks introduced by auto-merged (not hand-written) hunks during
  the cherry-picking process — see `a4a9b8c` above.
- **`cargo build`**: succeeds.
- **`cargo test --bins`** (2530 tests): **2528 passed, 2 failed**, both confirmed
  pre-existing/environment-specific and unrelated to this backport:
  - `api::server::tests::dispatched_request_reports_response_write_completion` — a
    new test added while cherry-picking `b88aa130`, but its failure here is purely
    this sandbox's environment: it builds a Unix-domain-socket path under
    `std::env::temp_dir()`, and this sandbox's `TMPDIR` is unusually long (this
    repo's own scratch path is over 100 characters deep before the socket file name
    is even appended), pushing the generated path past the OS's `sun_path` length
    limit for `sockaddr_un`. Confirmed by re-running with a short `TMPDIR=/tmp/hb/`:
    this specific test then passes (though a *different* set of tests starts failing
    instead, because several other tests independently assume the default macOS
    `TMPDIR` shape — a pre-existing, unrelated test-suite fragility around temp-path
    assumptions in general, not something introduced here).
  - `ui::panes::tests::pane_border_renderer_places_adjacent_cjk_by_display_width` —
    confirmed via `git show origin/master:src/ui/panes.rs` to exist byte-identical
    on `vitalybe/herdr`'s own master, and none of this backport's commits touch
    `src/ui/panes.rs` at all. Pre-existing, unrelated to this backport.
- Two test files this backport's cherry-picks touched are excluded from
  compilation/execution on this (Darwin) machine by their own `cfg` gates
  (`tests/cli_wrapper.rs` is `#![cfg(not(target_os = "macos"))]`; several test
  functions in `tests/api_ping.rs` and `src/detect/mod.rs` are
  `#[cfg(target_os = "linux")]`). Verified these still type-check correctly by
  temporarily forcing their `cfg` gates on (`cargo check`/`cargo check --all-targets`
  with the gates locally patched to always-true) and then reverting the temporary
  patch before committing anything — confirmed clean, no syntax/type errors, no
  permanent changes.
- Ran the non-platform-gated integration suite that overlaps this backport's changes
  directly: `cargo test --test api_ping` — **11/11 passed**.

## Final state

- `security-stability-backport` was merged into `master` with a regular merge commit
  (not squashed), preserving every individual commit above and its `-x` provenance
  trailer.
- Both `security-stability-backport` and `master` were pushed to `origin`
  (`sasha-dn/herdr`).

## Follow-up: protocol version gap

After the initial 27-commit backport, this fork's wire protocol (15) was found to be
incompatible (`compatible: no` per `herdr status`) with a locally-running, more-current
installation on protocol 16. Investigation found the exact upstream commit that bumped
15→16:

- `2bc1724` — "fix: switch prefix ASCII input source on the foreground client (#1016)"
  Fixes a real macOS bug: the prefix-mode ASCII input-source switch ran in the headless
  server, whose keyboard-input-source read is a stale per-process cache, leaving the
  terminal on the wrong input source after using prefix commands. Forwards the switch to
  the foreground client instead. Self-contained (17 files, adds one new protocol message
  variant + roundtrip test), single-parent squash-merge commit — clean cherry-pick target.

Cherry-picked as `b8b74c2`. One conflict (`docs/next/CHANGELOG.md`, the recurring
Unreleased-vs-0.7.2-section conflict noted above) resolved by keeping both protocol-bump
changelog lines. Rebuilt clean; `herdr status` protocol check against a live protocol-16
server now reports `compatible: yes`. `live_handoff` integration tests: 14/18 pass; the 4
failures (all named/non-default session socket paths under `/tmp`) reproduce identically
on the pre-cherry-pick commit, confirming they're pre-existing environment flakiness, not
a regression from this pick.

## Follow-up audit: `2bc1724` → v0.8.2 (2026-08-24) — list only, not yet backported

Continuation of the audit above. `2bc1724` (the last upstream commit incorporated, see
directly above) is roughly 450 commits behind live `ogulcancelik/herdr` `master`, which
has released through **v0.8.2** (2026-08-19) plus 29 further commits as of this review.
This pass is **list-only** — nothing below has been cherry-picked yet; it's the backlog
for the next backport round.

**Scope change from the original audit**: this round classifies into four buckets
instead of two. **Security** and **Stability** use the exact same rubric as the original
audit above. **Notable improvement** is a *new* category, not part of that original
rubric — real UX/feature value judged worth a personal daily-driver fork carrying it, as
opposed to a new agent integration, a platform this fork's user doesn't run, or pure
cosmetics. **Skip** covers everything else (docs, CI, i18n, Windows/WSL/Nix-only, new
agent integrations, single-theme cosmetics).

**Method**: cross-referenced upstream's `CHANGELOG.md` entries for every release after
the baseline (0.7.2 through 0.8.2) against the actual new-commit list. **This method has
a confirmed gap, found by the review pass below**: a changelog bullet whose commit
wasn't in the naive date-filtered "new since baseline" list got silently treated as
already-reviewed, but that date filter was flawed (see Revision note), and changelog
bullets themselves don't cover every commit — several genuine fixes have no changelog
line at all. Treat the totals below as a lower bound, not a complete accounting of all
~450 commits; a full commit-by-commit pass (the method the original 194-commit audit
above used) is the trustworthy way to close that gap and is the recommended next step
before actually cherry-picking from this list.

### Revision note (post-review)

An independent codex-agent review pass (via a Herdr pane, see task notes) checked this
section against upstream's real CHANGELOG and individual commits and found concrete
errors, corrected here:

- The initial "34 unreleased commits after v0.8.2" count was wrong (actual: **29**) —
  caused by a string-comparison date filter (`date > "2026-08-19"`) that also matched
  same-day commits timestamped *before* the actual v0.8.2 release commit
  (`9eb52145`, 17:49:54Z). Fixed by using `gh api compare/v0.8.2...master` directly.
- `adbaae68` ("reanchor pi status after session replacement", #1189) was wrongly
  re-listed as new — it's already backported (see entry 13, fork SHA `b18cc57`, in the
  original audit above). Root cause: the "new since baseline" list used a date filter
  (`since=<2bc1724's commit date>`), but `2bc1724` was a *later* addition on top of the
  original 27-commit backport, some of which (like `adbaae68`) have upstream commit
  dates *after* `2bc1724`'s date despite being incorporated earlier. **This means the
  same false-positive class could affect other already-backported commits with
  post-`2bc1724` dates** — not fully re-audited against all 27 in this pass; worth
  checking before actually cherry-picking anything below.
- Several classifications were too generous or too strict; moved accordingly (see
  buckets below). A representative sample of ~18 additional genuine commits the
  changelog-bullet method missed entirely were found via targeted keyword search and
  are now included in Stability.
- One SHA had a typo: `52960a5b` → `52960ea5`.

### Totals (revised)

**~119 commits/bullets classified this pass** (of ~450 raw commits since baseline) —
**3 Security, 39 Stability, ~29 Notable improvement, rest Skip** (Skip is *not*
exhaustively enumerated — see Method note above).

### Security (3)

1. **`0f14e279`** — *fix(cli): report non-UTF-8 arguments instead of panicking* (#2207)
   — a malformed/adversarial CLI argument crashed the process instead of erroring
   cleanly.
2. **`1079fff4`** — *fix: use the running binary in agent hooks* (#2722) — agent hooks
   previously invoked whichever `herdr` binary resolved first on `PATH`, not the one
   actually running the server; PATH-confusion class of bug.
3. **`624dfd47`** — *fix(graphics): isolate oversized kitty images* (#3035) — bounds an
   untrusted/oversized inbound kitty-graphics payload instead of letting it grow
   unbounded; same family as the already-backported kitty-graphics leak fix.

(`e3c3d443` and `d8838082`, originally listed here, were reclassified — see Stability
and Notable improvement below; codex's review: `e3c3d443` is ordinary SIGPIPE handling,
not a security boundary, and `d8838082` resolves a pane via the caller's own
`HERDR_PANE_ID` rather than "another client's" pane — same-user/server authority either
way, not a cross-client isolation issue as originally described.)

### Stability (39)

1. `f76f895e` — preserve plugin command arguments (#1613) — whitespace-only plugin args
   were being dropped; relevant since this fork already uses plugins (`claude-tab`, etc.)
2. `1491b7dd` — ignore nested codex session reports (#1927) — stale-session-ownership
   class, same as several already-backported fixes
3. `5eab32da` — restrict pi state reporting to TUI sessions (#2159)
4. `1997b88b` — keep pending URL clicks across host focus loss (#2291)
5. `d57cefb8` — preserve modifyOtherKeys key releases (#2303)
6. `ee8429fb` — parse default mouse reports instead of leaking escape bytes into the pane
   (#2312) — same bug family as the already-backported SGR-mouse-leak fix
7. `e9222d18` — restore keyboard reporting on detach, preventing enhanced keyboard
   sequences leaking into the shell after Herdr exits (#2395)
8. `7ae4b056` — wait for agent prompt readiness instead of racing/reporting premature
   readiness (#2410, #2537)
9. `37644cab` — wait for new pane shell readiness (#2774) — same family as #8, separate
   commit
10. `b8387818` — derive repo name for embedded bare-repo layouts (#2657, #2660)
11. `d46d9a22` — preserve workspace labels for root repositories across update restarts
    (#2594, #2727)
12. `8b635c91` — focus Copilot pane before prompting, preventing silently dropped input
    (#2734, #1698)
13. `a5c69bea` — eliminate redundant terminal wake work (#2962, #2550, #2901) — CPU
    regression fix under high-rate background output / many panes
14. `7d56b4c5` — batch kitty image row lifecycle (#3166)
15. `20a500a7` — start lifecycle subscriptions from live events (#3134)
16. `1c76079f` — copy selections before delayed mouse release (#3102)
17. `2d5f91dd` — match generated characters in prefix bindings (#3085)
18. `f72a5ef9` — keep background Claude MCP tasks working (#3094) — Claude-relevant
19. `8fe09d5d` — preserve multiline paste in terminal attach (#3056)
20. `ffc4e263` — add Claude screen activity fallbacks — Claude-relevant
21. `7d35ebe7` — synchronize plugin pane pwd (#2985) — plugin-relevant
22. `1b566016` — honor mouse capture config in terminal attach (#2995)
23. `52960ea5` — preserve mouse forwarding through handoff (#3002)
24. `e3c3d443` — exit quietly when output pipes close instead of panicking (#2996, was
    listed as Security, corrected to Stability per review)
25. `6f7ef04e` — bound stalled agent prompt waits instead of hanging indefinitely
26. `fd29addd` — bound Linux sound player lifetime, reaping players that don't exit
    (#1677)
27. `6382bd43` — reject oversized text pastes safely instead of disconnecting the client
    (#1676)
28. `ef85fa0c` — keep reading PTY output while pane input is blocked
29. `d4e0dd3d` — eliminate repeated workspace Git discovery, removing blocking work from
    a hot path (#1842)
30. `0a800b8f` — avoid blocking on foreground cwd checks (#2213)
31. `47104169` — ignore empty clipboard writes instead of erasing existing content
32. `b4112743` — restore terminal state on SIGHUP/SIGTERM
33. `374d3edd` — restore blocking bridge streams, preventing immediate remote
    disconnect (#2485)
34. `ca1af383` — avoid abort on terminal hangup (#2427)
35. `b660cad8` — handle remote terminal hangup cleanly (#2827)
36. `7e4ab7b0` — prioritize shutdown requests over busy traffic, preventing shutdown
    from blocking (#2624)
37. `36074530` — compact large terminal redraws, preventing freezes on oversized remote
    frames (#2675)
38. `76211757` — reap ctrl-click URL opener child processes (#2906)
39. `2d24950a` — isolate hidden-pane render cadence so it doesn't delay foreground
    input (#2892)

### Notable improvement (~29 SHAs, new category — see scope note above)

- `38d2b078` — live filtering in keybind help (#1832)
- `a79b3d55` — close workspace when API closes its last tab, matching TUI behavior
  (#1899)
- `a4b18b6e` — preserve navigate mode after pane focus change (#1993)
- `ecddecec` — `pane read`/`pane wait-output` accept `--flag=value` and reordered
  options (#2183)
- `fc824b99` — recognize versioned Python agent wrappers e.g. `python3.13` (#2188)
- `fae0b236` — ignore Cursor "Run Everything" false-blocked status (#2220)
- `d78e3d3b`, `2d011c39` — keep collapsed workspace status visible and highlight the
  active agent at 10+ workspaces (#2239, #2382, #2216)
- `09cdd88d` — return focus to the pane a split was opened from (#2266)
- `fd0e4ff4` — copy-mode `B`/`E`/`W` big-word motions (#2270)
- `93e0086f` — search single-tab names in the session navigator (#2320)
- `4a8a8f90` — ignore modified j/k/arrow keys in session navigator (#2377, #1981)
- `3825c0c3` — refresh host terminal appearance on focus, keeping auto light/dark theme
  current (#2417, #2416)
- `4e310844` — direct pane-resize keybindings (#2558)
- `f5067ed8` — move-tab keybind actions (#2561)
- `2e8cc906` — anchor host IME cursor to modal name-input fields (#2569) — worth
  prioritizing if this fork's user types non-Latin/IME input into name prompts
- `350f0013` — `ui.window_title` keeps the outer terminal window title in sync with the
  session (#2627)
- `f76d7df5` — keep tab/sidebar clicks registered through a stray drag report, e.g.
  Ghostty native fullscreen (#2736, #796)
- `165dca45`, `a9799165`, `25e35f07` — focus-preservation fixes (closing a non-focused
  workspace, a background workspace self-closing, background worktree removal) —
  reclassified from Stability per review: real UX correctness, not crash/hang class
  (#1877, #1912, #3099)
- `f99647b2` — clear CLI error instead of a raw I/O error when no server is running
  (#1963) — reclassified from Stability: message-clarity, not a hang/crash fix
- `d950fdfe` — decode 0x1f as Ctrl+_ per ASCII standard (#2165) — reclassified from
  Stability: a key-mapping correctness fix, not a leak/crash
- `d76657f2` — speed up alternate-screen history reads (#2426) — reclassified from
  Stability: a latency improvement, no hang/failure being fixed
- `49e333ae`, `2075de5d` — Claude Code title-spinner/status detection fixes (#2709,
  #2762) — reclassified from Stability: agent-status *detection* accuracy, not a
  crash/hang class, though still worth prioritizing since this fork runs Claude Code
- `1e1d0632`, `010afe53` — preserve navigator search query and refresh tab-bar labels
  after a plugin rename (#1140, #1179) — reclassified from Stability: UX polish
- `d8838082` — `pane current`/`pane get`/`pane layout --current` resolve via the
  caller's own `HERDR_PANE_ID` instead of the UI-focused pane (#2298) — reclassified
  from Security: same-authority correctness fix, not a client-isolation boundary

### Skip — representative sample, not exhaustive

- All Windows-only, WSL-only, or Nix-packaging-only fixes/features (roughly half of
  what was reviewed): e.g. Windows Korean IME support (#1823), Windows ConPTY bundling
  (#1828), Windows installer atomic directory swap (#2530), WSL cell-size query (#2160),
  Nix `xcbuild` packaging (#995) — this fork builds and runs on macOS.
- New agent integrations not currently relevant to this fork's usage: Grok CLI (#1807),
  Antigravity CLI (#2087), Qwen Code (#2743), `3b8aeee1` Maki agent support (#1301,
  #1302).
- Single-locale/theme/border cosmetics: Rose Pine `surface_dim` fix (#2002), halfwidth
  Katakana rendering (#2257), copy-mode CJK line-wrap crossing (#2145), `36a8d36a`
  `ui.pane_outer_borders` option, `db6f301f` per-mode theme overrides, `5203a5dc` keep
  active row visible under cursor — all reclassified from Notable improvement per
  review: real but skippable cosmetics, not "must-add" value.
- Detection-tuning-only fixes with no correctness or crash implication:
  `b04c6496` (Grok Build status tuning), `2bbb7c91` (OMP scheduled-continuation status
  tuning), `26628b6a` (published Claude detection manifest, not a runtime fix),
  `4d17957c` (tab-bar ANSI-stripping cosmetic), `5dd20c0a` (Kiro-specific idle
  detection) — all reclassified from Stability/Notable improvement per review.
- `2863b715` Windows remote-attach-to-Unix-hosts support (#2329) and `b7317c97`
  Simplified Chinese README (#1990) — reclassified from Notable improvement per review:
  out of scope for this fork's macOS/English usage.
- Docs/CI/release-process noise: website manifest updates, release-notes commits, CI
  workflow hardening.

**Governance note (not a code fix, flagged for awareness):** three commits in the
reviewed range (`7c5c410b` "keep kennel authority private", `d9f6d1c5` "pin kennel
maintainer identities", `ace91050` "remove akbash maintainer authority") modify
`.github/MAINTAINERS`/`.github/MAINTAINER_IDS` to drop `akbash-bot` — a bot account that
authored a large share of the commits already backported in the original audit above —
from upstream's maintainer list. Diff-confirmed via `gh api .../commits/<sha>`; no code
changed, not itself actionable. Worth knowing the account behind many of this fork's
already-backported fixes no longer has upstream maintainer authority.

### Known limitations of this pass

- **Not a full commit audit.** ~450 raw commits exist since baseline; this pass reviewed
  ~119 of them (those covered by a changelog bullet, plus ~18 more found through
  targeted keyword searches during the codex review). The remainder have not been
  individually classified and could contain more Security/Stability fixes the changelog
  never mentioned — the review pass demonstrated this gap is real, not hypothetical.
- **Possible additional stale "already backported" entries.** The `adbaae68` false
  positive (see Revision note) suggests other commits from the original 27 with
  post-`2bc1724` dates could be hiding in a future re-run's "new" list; check against
  the original audit's SHA list above before trusting any future automated diff.
- **Recommended next step**: before cherry-picking anything above, run a proper
  ancestry-based diff (not date-filtered) of this fork's actual `master` against
  upstream `master`, then classify every remaining commit individually — the same
  method the original 194-commit audit used — rather than continuing to rely on
  changelog coverage.

---

## Pass 2 — full ancestry-based audit (2026-08-24), superseding Pass 1's coverage gap

Pass 1 (above) used upstream's dated `CHANGELOG.md` bullets plus a date-filter to find
"new" commits. That method had a real, demonstrated flaw: cherry-picks get new SHAs and
new commit dates, so a date filter cannot distinguish a genuinely new upstream commit
from one whose *content* is already sitting in this fork under a different SHA. This
pass replaces that method entirely and should be treated as authoritative for the range
it covers; Pass 1's "Known limitations" section (immediately above) is resolved by this
pass.

### Corrected method

1. `git fetch upstream && git fetch origin` — Pass 1's cached `upstream/master` ref was
   confirmed stale (9450b168, itself hundreds of commits behind live upstream).
2. **Patch-equivalence diff**, not SHA ancestry and not date-filtering:
   `git log --cherry-pick --right-only --no-merges origin/master...upstream/master`.
   This uses `git`'s patch-id matching to recognize a cherry-picked commit as already
   present even under a new SHA — the correct fix for Pass 1's flaw.
3. **Patch-id has its own limitation, handled explicitly**: any of this fork's 27
   originally-backported commits (plus the `2bc1724` protocol-16 fix) that hit a merge
   *conflict* during their original cherry-pick no longer has a byte-identical diff to
   upstream's version, so patch-id matching alone still produces false positives for
   those specific 17 commits (`045f506e`, `600753b4`, `60c04005`, `6f148588`,
   `749e85e0`, `75ed6abc`, `80d2958a`, `8915f01c`, `aa0768b7`, `b425260f`, `b88aa130`,
   `c78349f4`, `d64cf40c`, `d9230f51`, `e4a7095c`, `f11be93e`, `2bc1724`).
   All 28 original upstream SHAs were explicitly excluded from the candidate list by
   SHA regardless of what patch-id matching reported (not just the 17 above — the
   other 11, correctly recognized as already-present by patch-id alone, were excluded
   the same way, for a complete and verifiable list): `045f506e`, `26db26e4`,
   `4eceb75d`, `600753b4`, `60c04005`, `64de9279`, `6f148588`, `749e85e0`, `74a771d1`,
   `75ed6abc`, `797981c7`, `80d2958a`, `8915f01c`, `9c453427`, `9e4fdd61`, `aa0768b7`,
   `adbaae68`, `b1d1a07b`, `b425260f`, `b88aa130`, `c78349f4`, `d5521e3d`, `d64cf40c`,
   `d9230f51`, `e4a7095c`, `e9bcea9c`, `f11be93e`, `2bc1724` (27 original + the
   protocol-16 fix = 28), and verified absent from the final list before classification
   began (spot-checked by codex's independent review against 5 of the 28, all
   confirmed correctly excluded).
4. Raw right-only count: 492. After excluding the 28 known-already-backported SHAs:
   **475 commits**, in chronological order, none previously reviewed.
5. Bucketed by conventional commit prefix first (`fix`/`feat`/`perf`/`docs`/`chore`/
   `ci`/`test`/`release`/`refactor`), then classified using the same rubric as the
   original 194-commit audit (Security / Stability, read subject + diff when
   ambiguous) plus Pass 1's new **Notable improvement** category (real UX/feature value
   plausibly worth this macOS fork adopting — new integrations for agents this fork
   actually uses, configurable keybindings/`ui.*` options — as opposed to Windows/WSL/
   Nix-only, unused third-party-agent integrations, docs/website/CI/i18n noise, or
   single-locale cosmetics, all Skip).
6. `docs`(54) + `chore`(40) + `ci`(15) + `test`(12) + `release`(7) = 128 commits and all
   20 `refactor:` commits were bulk-classified Skip after spot-checking the bucket
   contents for anything hiding real behavior change (none found — contributor
   approvals, org-migration/relicensing chores, docs/i18n, CI hardening, and a batch of
   internal "route X through runtime dispatch" refactors, all behavior-neutral).
7. All 44 Windows/WSL/ConPTY/PowerShell/Nix-only `fix:` commits Skipped — this fork
   builds and runs on macOS only.

### Totals

**475 commits reviewed (1 additional commit, `4a3302d1`, explicitly out of scope —
see Skip section). Kept: 151 (2 Security, 83 Stability, 66 Notable improvement).
Skipped: 324. Totals below reflect codex's independent review and its
reclassifications (originally 4/83/65 kept, 323 skipped before review).**

### Security (2)

Codex's independent review (below) diff-verified the original 4 and reclassified 2 to
Stability: `6382bd43` (oversized pastes were already bounded/disconnected before this
fix — it makes them recoverable, not a DoS/crash-boundary fix) and `0f14e279` (argv is
caller-controlled, not untrusted external input, so the panic-avoidance framing is
Stability, not Security). See those entries under Stability below.

- `69a07fdf` — publish stable release checksums
- `322ab6a5` — verify stable release checksums — together: stable direct installs,
  self-updates, and remote helper downloads now require and verify the published
  SHA-256 digest for each GitHub release asset — supply-chain integrity for the update
  mechanism itself.

### Stability (83)

*Session/lifecycle ownership and agent-status correctness (same family as several
already-backported fixes):*
- `e608a751` — make known-agent exit process-owned
- `1491b7dd` — ignore nested codex session reports (#1927)
- `ac47b9e6` — resume agents after headless restore (#2088)
- `9351b058` — reject prompts to blocked agents (#2790)
- `20a500a7` — start lifecycle subscriptions from live events (#3134)
- `d8838082` — resolve `pane current`/`pane get`/`layout --current` from the calling
  pane instead of the UI-focused pane (#2298) — automation/API correctness, not a
  client-isolation boundary (Pass 1 already reviewed this exact commit and reached the
  same conclusion via its codex review)

*Claude Code detection/status correctness (highest-relevance cluster — this is the
fork's primary agent):*
- `a47bc3e0` — keep Claude working during `/btw`
- `4473e391` — detect claude confirmation prompts
- `49e333ae` — strip claude title spinner frames (#2709)
- `2075de5d` — detect all claude half-circle spinner frames (#2762)
- `9ba32c25` — detect claude 2.1.228 half-circle busy spinner
- `d742e515` *(Notable, see below — moved out of this cluster, it's a settings-format
  fix not a detection fix)*

*Codex/Copilot/OMP/agent-automation reliability (this pass includes Codex fixes given
this very review used a Codex agent as its independent reviewer):*
- `74db2f3a` *(Notable, see below)*
- `4800ff54` *(Notable, see below)*
- `7ae4b056` — wait for agent prompt readiness (#2537)
- `37644cab` — wait for new pane shell readiness (#2774) — same race-avoidance cluster
  as `7ae4b056`
- `f72a5ef9` — keep background Claude MCP tasks working (#3094)
- `b499e611` — resolve relative plugin commands from plugin root
- `f76f895e` — preserve plugin command arguments (#1613)
- `1079fff4` — use the running binary in agent hooks (#2722) — prevents hooks invoking
  the wrong on-`PATH` binary, a real version-mismatch class of bug

*Input/escape-sequence integrity (same class as already-backported `aa0768b7` "stop sgr
mouse sequences leaking into panes" — spurious/duplicate characters reaching pane apps):*
- `e98c4965` — preserve shifted punctuation for alt legacy input
- `39ba6b7a` — preserve alt-shift letters
- `d190c1f5` — send shifted punctuation as text in kitty mode
- `464c658f` — reassemble split sgr mouse input
- `5d24d0d2` — preserve escape before sgr mouse reports
- `c1f7b6f3` — preserve lone escape input boundaries
- `30232287` — preserve associated emoji input (#1410)
- `ea047db8` — avoid ctrl-tab escape sequences leaking into legacy panes
- `d950fdfe` — decode 0x1f as Ctrl+_ not Ctrl+- per ASCII standard (#2165)
- `d277d2f8` — preserve alt-prefixed control keys (#2543)
- `b0723b79` — preserve shift in kitty alternate reports (#2479)
- `ee8429fb` — parse default mouse reports instead of leaking escape bytes (#2312)
- `d57cefb8` — preserve modifyOtherKeys key releases (#2303)
- `b76adc15` — preserve native key lifecycle across routing (#2142)
- `adb50cba` — preserve shift-tab in pane automation (#2259) — matters for
  programmatic agent-permission-mode cycling
- `d004089d` — disambiguate shifted punctuation keybinds (#2676)
- `2d5f91dd` — match generated characters in prefix bindings (#3085)
- `19cd372f` — preserve prefix mode for modifier events
- `b708f85e` — match shifted indexed number keybinds
- `4a8a8f90` — ignore modified j/k/up/down in session navigator (#2377) — input was
  being wrongly consumed instead of passed through
- `169a4fd9` — answer XTWINOPS size queries in panes (protocol-completeness, prevents
  pane apps misbehaving without a size-query response)
- `89011394` — query host cell size when ioctl reports no pixels (#2160)
- `e9222d18` — restore keyboard reporting on detach (#2395) — prevents enhanced
  keyboard sequences leaking into the shell after Herdr exits
- `cfa112e7` — preserve kitty associated text (#2051)
- `0bd44502` — preserve macOS IME commits in report-all mode (#2947) — macOS-specific,
  directly applicable to this fork
- `58e812b2` — consume handled URL click gestures — suppresses the unmatched mouse
  release left after a handled click, preventing duplicate actions in mouse-reporting
  pane apps (reclassified from Skip per codex review — broader than cosmetic)
- `1997b88b` — keep pending URL clicks handled across host focus loss (#2291) — extends
  the same guard as `58e812b2` across a focus-loss window (reclassified from Skip)
- `f76d7df5` — keep tab and sidebar clicks working when the terminal reports a stray
  drag (#2736) — replaces global click/drag state with per-client state, preventing
  concurrent clients from swallowing or completing each other's gestures (reclassified
  from Skip per codex review — broader than minor UX)
- `6382bd43` — reject oversized text pastes safely (#1676, refs #1665) — recoverable
  handling of oversized pastes, not a crash/DoS boundary fix (reclassified from
  Security per codex review)
- `0f14e279` — report non-UTF-8 CLI arguments instead of panicking, and avoid echoing
  malformed arguments back (#2207) — argv is caller-controlled, so this is a
  panic-avoidance fix, not an untrusted-input security fix (reclassified from Security
  per codex review)

*Data-loss / config-diagnostic correctness (the config-diagnostic entries are direct
follow-ons to this fork's own already-backported `f11be93e` unknown-config-key
detector):*
- `47104169` — ignore empty clipboard writes (#1893) — previously an empty write could
  erase existing clipboard contents
- `1f1e4342` — accept retired `ui.agent_panel_scope` as a known-but-retired key instead
  of flagging it via the unknown-key detector (#2295) — this fork's own test fixtures
  hit exactly this stale key during the original `f11be93e` backport (see Pass-0 "Test
  fixes" section above); this is upstream's own fix for the same false-positive class
- `bdb5222e` — report unknown *theme* names via the same diagnostic path (#2488)

*Shutdown/reconnect/remote robustness:*
- `3459798b` — extend local server startup wait
- `7e4ab7b0` — prioritize shutdown requests (#2624)
- `374d3edd` — restore blocking bridge streams (#2485), remote
- `ca1af383` — avoid abort on terminal hangup (#2427) — client-side core-dump
  prevention
- `b660cad8` — handle remote terminal hangup (#2827) — same underlying issue (#2424)
  as `ca1af383`, different code path
- `08640bb3` — fall back to `sh` for remote path discovery when the login shell can't
- `c3df61b5` — make remote helper installs shell-portable
- `52960ea5` — preserve mouse forwarding through handoff (#3002)
- `1b566016` — honor mouse capture config in terminal attach (#2995)
- `7d35ebe7` — synchronize plugin pane pwd (#2985)
- `ffc4e263` — add claude screen-activity detection fallbacks

*Rendering/CPU correctness (hangs, freezes, and real CPU regressions, not pure
micro-perf):*
- `36074530` — compact large terminal redraws (#2675) — prevents remote clients
  freezing when a full ANSI frame exceeds the transport limit
- `61f64006` — repaint after transient terminal resizes
- `d76657f2` — speed up alternate-screen history reads (#2426) — reduces read latency
  specifically for TUI agents, directly relevant to this fork's use case
- `a5c69bea` — eliminate redundant terminal wake work (#2962)
- `2d24950a` — isolate hidden pane render cadence (#2892)
- `e7c38ab3` — avoid full pane mode reads for scrollbars (#2554)
- `b01fc37e` — avoid full pane renders for spinner animation (#1868)
- `81f355fa` — replace agent spinners with static status marks (CPU reduction)
- `e27a4ea4` — skip rendering hidden pty updates
- `547aba8e` — avoid renders for passive mouse motion (#1872)
- `8d2af4e0` — adopt libghostty integration improvements (perf)
- `02a6e874` — batch contiguous terminal diff writes (perf)
- `d4e0dd3d` — eliminate repeated workspace git discovery (#1842)
- `5600197f` — avoid repeated Git config reads (#2688)
- `e3c3d443` — exit quietly when a downstream pipe closes instead of panicking with
  exit 101 (#2996, diff-confirmed) — crash fix, not itself an untrusted-input issue so
  kept in Stability rather than Security
- `3f752a72` — synchronously terminate cancelled status commands — prevents orphaned
  background status-check processes

*Workspace/pane lifecycle:*
- `a79b3d55` — close workspace when API closes its last tab (#1899) — makes the API
  path cascade tab→workspace closure the same way the TUI already does, avoiding a
  workspace left open with no reachable tab
- `76211757` — reap ctrl-click url openers after they exit (#2906) — prevents defunct
  process accumulation
- `ef85fa0c` — keep reading while pty input is blocked
- `6f7ef04e` — bound stalled agent prompt waits
- `bb29eedb` — delay agent prompt submission — prevents text sitting in the composer
  without a turn actually starting
- `0a800b8f` — avoid blocking on foreground cwd checks (#2213)
- `b4112743` — restore terminal state on SIGHUP/SIGTERM

### Notable improvement (66) — new category, not part of the original audit's rubric

*Focus-management (high relevance given this fork's multi-agent/multi-pane workflow):*
- `fbc3f084` focus parent after worktree removal (#1075)
- `165dca45` preserve focus when closing a non-focused workspace (#1877)
- `a9799165` keep focus on the current workspace when another self-closes (#1912)
- `25e35f07` preserve focus after background worktree removal (#3099)
- `09cdd88d` return focus to the pane a split was opened from (#2266)
- `a4b18b6e` preserve Navigate mode during directional pane focus (#1993) — belongs with
  this cluster (reclassified from Skip per codex review)

*cwd-following (useful for how panes/tabs track a shell's actual location):*
- `48d58648` follow cached cwd for new tabs
- `4f7fcc48` follow foreground cwd for new terminals
- `550e9f8c` follow foreground process group leader cwd
- `cf3769fe` follow focused pane cwd for new workspaces
- `9450b168` avoid rechecking cached pane cwd (#1616)

*Codex-specific detection (directly relevant — this review's own independent-reviewer
step runs as a Codex agent):*
- `74db2f3a` detect codex working with static titles (#1574)
- `4800ff54` detect reordered codex title spinners

*Sidebar/status visibility while monitoring many agents (this fork's core use case):*
- `d78e3d3b` keep collapsed workspace status visible (#2239)
- `2d011c39` highlight active agent in collapsed sidebar (#2382)
- `d2c317ad` keep active sidebar rows visible (#2794)
- `93e0086f` search single-tab names in navigator (#2320)

*Copy/paste/selection correctness:*
- `b0d46fb9` stop copy mode line-end at text (#1411)
- `f43a6d4a` preserve selection when auto-copy is disabled (#1496)
- `1c76079f` copy selections before delayed mouse release (#3102)
- `bd287b01` route VS Code Remote Tunnel copies through OSC52 (#2173)
- `c8850a2e` add copy mode search and tmux word motions
- `fd0e4ff4` add copy-mode big-word motions (#2270)
- `46174563` add copy-on-select setting

*Pane-app protocol/appearance completeness:*
- `6f311498` forward pane terminal bells (#2498)
- `10eb0bc7` propagate host color scheme to pane apps (#2214)
- `3825c0c3` refresh host appearance on focus (#2417)
- `503613c8` inherit host palette in pane queries (#1759)
- `6ea68f9a` forward page keys to primary-screen pagers
- `2e5da968` forward outer focus events to panes (#1388)
- `4c9a3f38` forward horizontal wheel events to panes (#1402)
- `eacea2da` page keys scroll pane scrollback at zsh/REPL prompts (#2191)
- `bc1c052d` read alternate-screen agent history
- `c0fb777e` report truncated pane reads (`truncated: true`)

*CLI/automation ergonomics:*
- `13cb5d9a` add shell completions
- `5d9212a5` expose session snapshot through CLI
- `3f809476` make agent automation a first-class CLI primitive
- `d956a6b5` show agent targets before options in help
- `f99647b2` report a clear CLI error when no herdr server is running (#1963)
- `ecddecec` accept `--flag=value` and reordered options in `pane read`/`wait-output`
  (#2183)
- `fc824b99` recognize versioned Python agent wrappers such as `python3.13` (#2188)
- `1bfabbab`, `1af1a5b8` — CLI help and bundled docs now point coding agents at
  Herdr's own guide/skill

*Plugin system (relevant — this fork uses the `claude-tab` and
`vbelman.claude-resume` plugins):*
- `d30ab1b5` add plugin-driven agent views and startup hooks
- `1955406e` link plugins without a running server
- `f6060cf6` scope agent skill installation
- `947328fa` support agent environment hints (macOS)
- `69d07dba` make plugin state global across sessions — real value, but codex review
  found this is a **breaking migration**: existing named-session plugin state must be
  reinstalled/relinked after adopting it, not a drop-in fix (moved here from Stability,
  flagged as needing care, not a blind cherry-pick)

*UI/keybinding configurability (real, opt-in, low-risk):*
- `2c7c8beb` add floating popup panes
- `38d2b078` filter keybind help (#1832)
- `cb2e17a4` print bundled agent skill and hint next CLI steps
- `02fe7d76` support bottom tab bar placement (#2118)
- `f6031f7f` make pane scrollbars optional (#2166)
- `cc9fa475` add distinct agent status indicators (#2282)
- `4e310844` add direct pane resize keybindings (#2558)
- `e48d8306` add configurable tab bar status (#2586)
- `f5067ed8` add move-tab keybind actions (#2561)
- `350f0013` keep the outer terminal window title in sync with the session (#2627)
- `a26a6543` connected tree glyphs, workspace grouping, and search auto-select in
  navigator
- `15539d3d` make headless terminal size configurable (#2829)

*Misc correctness worth adopting:*
- `d742e515` preserve claude settings formatting (#2089)
- `b8387818` derive repo name for embedded bare layouts (#2660)
- `d46d9a22` preserve workspace labels for root repositories across update restarts
  (#2727)
- `952729ee` preserve logical lines in scrollback editor (#2735)
- `e16d7d8c` preserve linked-worktree workspace labels — same correctness class as
  `d46d9a22` (reclassified from Skip per codex review)
- `08409d16` show the current (not stale/wrong) workspace name in its close
  confirmation dialog — matters because this guards a destructive action (reclassified
  from Skip per codex review)

### Skip — representative sample, not exhaustive (324 total)

- All 44 Windows/WSL/ConPTY/PowerShell/Nix-only `fix:` commits — this fork builds and
  runs on macOS.
- Reclassified to Skip per codex review (agent- or platform-specific code paths this
  fork's actual usage doesn't touch, despite sounding stability-relevant on the
  subject line alone): `5eab32da` (pi-lifecycle-only, not a shared code path),
  `8b635c91` (Copilot-specific, not Claude/Codex), `38b7b540` (Linux-only,
  `src/platform/linux.rs`), `fd29addd` (Linux `mpg123`-only), `fae0b236` (Cursor-only
  detection tuning), `7e821ba7` (OMP/Pi integration dirs, not the `claude-tab`/
  `vbelman.claude-resume` plugin mechanism this fork uses), `0065a0ef` (bundled Pi
  worktree extension only). Note: the original 194-commit audit did treat several
  Pi/OMP session-ownership fixes as in-scope Stability (shared lifecycle-arbitration
  code, not integration-specific) — these 7 are integration-specific code, not that
  shared class, which is why they're Skip here rather than an inconsistency with that
  precedent.
- 128 `docs`/`chore`/`ci`/`test`/`release` commits and 20 `refactor:` commits — website,
  contributor-approval bot noise, org-migration/relicensing chores, i18n, CI hardening,
  internal "route X through runtime dispatch" refactors with no behavior change.
- Third-party-agent integrations/detection this fork doesn't use: Grok (`b04c6496`,
  `b42c6e66`, `99760f77`), Kimi (`28b628d7`), Amp (`b03f033d`), OpenCode/OpenCode2
  (`be901fea`, `6fb0d803`, `ccccda54`), Kiro (`5dd20c0a`), Qwen (`9e6c2b4e`),
  `2bbb7c91` (OMP scheduled-continuation status tuning, reclassified from Stability on
  review — detection-tuning only).
- Pane-graphics feature cluster with no base to apply to: `88370e15` ("add isolated
  pane graphics streaming") was itself correctly classified as a feature and not
  backported; everything built on it — `e9cbcf2f`, `b7dca4bb`, `624dfd47`, `7d56b4c5`,
  `1777e9bb` — has nothing to apply to in this tree, same reasoning as the original
  audit's `e9cbcf2f` call.
- `a22454f2` "preserve plugins during live handoff" — this fork already has its own
  independent fix for the identical bug (confirmed in the original audit); genuinely a
  duplicate, not new.
- `a0a6e13f` "count ptmx master fds under devpts layouts" (#1984) — diff-confirmed this
  is a **test-fixture** fix (a WSL2/container-specific test assertion), not a
  production code change.
- Single-locale/cosmetic/theme: `be1891ec` (halfwidth katakana), `f459a0df` (CJK
  word-end edge case), `2e8cc906` (IME cursor anchor), `6c6ddcd4`/`97f8140f`/`5203a5dc`/
  `9166e07b`/`4d17957c` (tab/row/scrollbar cosmetics), `d78e3d3b`'s sibling cosmetic
  variants, `f83980db` (OSC4 palette index cosmetic).
- Minor UX with no crash/hang/data-loss implication: `84855451`, `974a4817`, `552aa8ca`,
  `7db744ab`, `cbc31d34`, `e6fa5289`, `3a8490f6`, `0cd0b1ae`, `9a4ce5e1`, `885c9a6b`
  (dead-code removal). (`08409d16`, `e16d7d8c`, `a4b18b6e` moved to Notable improvement
  and `58e812b2`/`1997b88b`/`f76d7df5` moved to Stability per codex review — see those
  sections.)
- Already reviewed and reclassified to Skip in Pass 1's codex-review step, reconfirmed
  here: `1e1d0632`/`010afe53` (navigator search/tab-rename UX polish), `26628b6a`
  (published detection manifest, not a runtime fix), `4d17957c` (tab-bar ANSI-stripping
  cosmetic).
- `c71c6c10` "resolve split current from caller pane" and `be901fea` — same calls the
  original 194-commit audit already made on these exact SHAs (behavior/feature fixes,
  not crash/hang).
- Niche/dev-tooling: `de94cf83` (mise remote installs), `bbe78f51` (repo commit-hook
  restore), `6b0e6165` (build tooling).
- **Explicit exception, not actually classified**: `4a3302d1` (vendored `libghostty-vt`
  bump) is *not* one of the 475 reviewed-and-classified commits despite appearing in
  this fork's commit range — codex's review correctly flagged that folding it into
  "Skip" without reading its diff would silently contradict the "475 reviewed" total.
  A vendored VT-parser bump is exactly the kind of change that can carry upstream
  security fixes; it needs its own dedicated diff review (upstream changelog for the
  vendored version bump, not just this fork's wrapping commit) before any verdict,
  security or otherwise.
- Website/marketplace/docs-site features: `e1bbc252`, `82487675`, `d36a6ded`,
  `c9676e46`, `22bb476d`, `b12da239`, `d16af266`, `79a953e9`, `eac54b84`, `a45ce225`,
  `d6dae883`, `4cf7cff3`, `2baebb50`, `ca7ae086`, `0f10e145`, `78a356b1`.
- Unused-relative-to-this-fork feature adds: `d0e33345` (MastraCode), `3b8aeee1`
  (Maki), `e9b22084` (Grok CLI), `679584fd` (Antigravity), `a4d52ab6` (Qwen Code
  detection+restore), `2863b715`/`9fac5172` (Windows-only), `db6f301f` (per-mode theme
  overrides — reclassified from Notable in Pass 1's review, reconfirmed here as
  cosmetic), `e0758c32` ("harden named agent cli workflows" — misleadingly named, it's
  a large new-feature commit, not a hardening fix — same call the original 194-commit
  audit made on this exact SHA), `5cfe5e5e`/`5b91dae1`/`b16465af`/`9c9490d7`/
  `add6c14f`/`36a8d36a` (sidebar/theme cosmetics), `2a1a8d64` (hide single-tab row),
  `e298e458` (workspace name prompt), `15cab964`/`56edcbcb` (low-level plugin-API
  internals with no direct end-user value), `ef67a970` (perf commit superseded by
  already-backported `80d2958a`, same call as the original audit).

### Codex-agent independent review of this pass

Spawned as a Herdr pane (`codex`, cwd `herdr-src`) with git/gh access to both remotes.
Scope given: verify all 4 originally-classified Security entries against their actual
diffs; spot-check 20+ Stability and 15+ Notable-improvement entries by reading diffs;
re-verify the `a0a6e13f`/`e3c3d443` diff-confirmations specifically; and check the
step-3 exclusion list for completeness against the original audit's 27+1 backported
SHAs. Findings, written to `CODEX_REVIEW_PASS2.md` in this directory and folded into
the sections above:

- **Both** original Security entries were downgraded to Stability on diff inspection:
  `6382bd43` (oversized pastes were already bounded/disconnected — this makes them
  recoverable, not a crash/DoS-boundary fix) and `0f14e279` (argv is caller-controlled,
  not untrusted external input). Security is now `69a07fdf`/`322ab6a5` only (release
  checksum verification) — a smaller but more defensible bucket.
- 7 Stability/Notable entries were scope-specific to agents or platforms this fork
  doesn't use and were moved to Skip (`5eab32da`, `8b635c91`, `38b7b540`, `fd29addd`,
  `fae0b236`, `7e821ba7`, `0065a0ef`) — codex read the actual diffs and found
  Pi/Copilot/Linux/Cursor/OMP-specific code paths, not the shared logic the subject
  lines implied.
- `69d07dba` ("make plugin state global across sessions") is real but is a **breaking
  migration** requiring existing named-session plugin state to be reinstalled/relinked
  — moved from Stability to Notable improvement with that caveat attached, not a
  blind cherry-pick candidate.
- 3 entries were upgraded from Skip to Stability after diff review found them broader
  than assumed: `58e812b2`, `1997b88b` (both prevent a stray mouse-release/click from
  leaking into or duplicating actions in pane apps), and `f76d7df5` (replaces global
  click/drag state with per-client state — a real multi-client correctness fix, not
  cosmetic).
- 3 entries were upgraded from Skip to Notable improvement: `08409d16` (destructive
  close-confirmation dialog was showing a stale/wrong workspace name), `e16d7d8c`, and
  `a4b18b6e` (belongs with the focus-management cluster).
- `a0a6e13f` and `e3c3d443` were re-verified and confirmed as originally stated
  (test-fixture-only, and a genuine panic fix, respectively) — no change.
- The step-3 exclusion list was confirmed logically complete (all 28 SHAs, verified by
  spot-check) but the document's own prose only *named* the 17 patch-id-mismatch
  SHAs, making the other 11 look missing to a reader who didn't already know they were
  correctly excluded by patch-id alone; fixed by listing all 28 explicitly in that
  section — no actual gap in the exclusion, a documentation-clarity fix only.
- `4a3302d1` (vendored `libghostty-vt` bump) was flagged as inconsistent with the
  "475 reviewed" total while also being described as unaudited; carved out into its
  own explicit-exception line in the Skip section rather than folded into "Skip" as if
  reviewed.

All corrections above are already applied to the Security/Stability/Notable
improvement/Skip sections and the Totals line earlier in this Pass 2 section; nothing
below this point needs further changes based on this review round.

### Recommended next step

Both this pass and Pass 1 still rely primarily on subject-line classification with
diff-verification only on a sampled subset (this pass: the 4 Security entries in full,
plus the ~30 Stability/Notable entries the codex review actually opened diffs for).
Before cherry-picking anything, diff-verify the remaining Security and Stability
entries individually — the same per-commit rigor the original 194-commit audit applied
throughout — since a category with only 2 Security and 83 Stability commits is a
tractable size for that pass. `4a3302d1` (vendored `libghostty-vt` bump) also still
needs its own dedicated look before any release, per the exception note above.

## Scope decision (2026-08-24): what actually gets cherry-picked next

Owner decision, made after reviewing the Pass 2 category breakdown above, on what to
actually backport (superseding the "recommended next step" above — no separate
dedicated diff-verification pass; verification happens during cherry-picking itself,
same as the original 27-commit backport):

- **Security (2)** — both, in full.
- **Stability (83)** — all, in full.
- **Notable improvement — included groups**: Focus-management (6), cwd-following (5),
  Codex-specific detection (2), Sidebar/status visibility (4), Copy/paste/selection
  (7), Pane-app protocol/appearance (10), Plugin system **excluding** the breaking
  migration `69d07dba` (4 of 5), Misc correctness (6). **44 commits total.**
- **Notable improvement — explicitly excluded for now**: CLI/automation ergonomics
  (8), UI/keybinding configurability (12), and `69d07dba` (breaking plugin-state
  migration, needs its own explicit go/no-go later) — **21 commits deferred, not
  rejected.**
- **`4a3302d1`** (vendored `libghostty-vt` bump) — still excluded pending its own
  dedicated look, per the exception note above.

**Total scoped for this backport round: 129 commits** (2 + 83 + 44).

## Phase 1 — Security (2026-08-24)

Branch `backport-pass2-20260824`, off `origin/master` at `2c8c6731`.

Both Security commits landed, but neither cherry-picked cleanly as-is — both needed
scoping/adaptation, and an independent codex review (spawned as a Herdr pane) then
found the scoping had been cut too narrowly on the first pass. Final state:

- **`69a07fdf`** ("publish stable release checksums") — **not backported.** Pure
  `website/latest.json` data for upstream's own release identity (version, asset
  URLs) — no reusable code, and adopting the data verbatim would misrepresent this
  fork's own (unpublished) build as upstream's 0.8.0 release.
- **`322ab6a5`** ("verify stable release checksums") — adopted in **two** separate
  commits after the review found the first one incomplete:
  - `2c8c6731` — the core fix: `src/update.rs`'s stable self-update path now rejects
    a manifest asset missing a `sha256` field instead of silently installing it
    unverified. Deliberately **not** adopted from the same upstream commit: the
    `website/latest.json`/`install.sh`/`install.ps1`/`scripts/changelog.py`/`justfile`
    release-pipeline changes (see below), and upstream's top-level `manifest.sha256`
    map fallback (this fork only supports the simpler inline per-asset form; no
    verification bypass results, just narrower manifest-format support).
  - `fb36cc12` — **codex review finding, High severity**: the same commit's
    `src/remote/attach.rs` hunk (this fork's equivalent is `src/remote/unix.rs`) was
    missed entirely on the first pass, leaving the identical unverified-install gap
    open on the `herdr --remote` SSH-install path. Adopted the same enforcement there.
- Also required an unplanned build-fix, `dc009faf`: the branch's build was already
  broken before this task's Phase 1 work started — a conflict-resolution defect in
  an earlier, unrelated commit (`08ed0acc`, landed before this backport round properly
  began) left `navigate.rs`'s test-only `handle_navigate_key` free function calling
  helpers that a correct upstream refactor had scoped to `#[cfg(test)]`. Fixed by
  gating the fork's own free function (and its two now-orphaned helpers) the same
  way, matching its own doc comment ("exercised in input unit tests"). No behavior
  change; build-fix only.

**Codex review's other findings, not acted on / lower priority**:
- **High** — `scripts/changelog.py`/`.github/workflows/release.yml` still produce a
  checksum-free manifest; if this fork ever cuts a release through that inherited
  pipeline, the now-mandatory consumer would reject it. Checked whether this is a live
  risk: `sasha-dn/herdr` has **zero GitHub releases and zero release-workflow runs
  ever** (confirmed via `gh api`), so this is a latent inconsistency in inherited,
  never-exercised CI, not an active vulnerability. Left unfixed for now — worth
  closing before this fork's first real release, not urgent today.
- **High** — `website/install.sh` drops verification too; same "never actually
  deployed" caveat applies (`has_pages: false`, no Cloudflare Pages deploy has run).
  Not fixed this round.
- **Medium** — dropping the top-level `manifest.sha256` map fallback narrows format
  support but introduces no bypass (noted above as a deliberate, documented choice).
- **Medium** — test coverage gap: the new tests cover the enforcement logic itself
  but not the checked-in manifest, remote path, or installer scripts end-to-end.

**Build/test**: `cargo check --all-targets` clean (only the same 8 pre-existing
dead-code warnings present since before this backport round, confirmed unrelated —
see Phase 2 below). `cargo test --bins`: same two pre-existing/environment-specific
failures as every prior pass (`dispatched_request_reports_response_write_completion`,
`pane_border_renderer_places_adjacent_cjk_by_display_width`), no new failures.

## Phase 2 — Stability, partial (2026-08-24)

Delegated to a background agent with instructions to batch in groups of ~10-12,
hard-cap effort per commit, and check build health after each batch. **13 of 83
landed** before the run ended without producing its planned write-up (its own final
summary was truncated/incomplete — likely ran out of runway mid-task, not a clean
stop). Commits landed, oldest to newest:

`8ba63696` `51384af4` `1a98102f` `52468f94` `08ed0acc` `c530b5cc` `a8ef9f5d`
(these last few were already on the branch from an earlier, separately-killed run —
see the branch's own history for exact provenance) — then this run's own batch:
`71b2fc4b` `de7df0c6` `770da5d7` `4c0746e9` `dda83845` `4caf45dc` `1ac5885e`
`b7dd14e2` `48ff4262` `77301bbd` `cef79b9d` `4f69cfa0` `a2dbd92e`.

One self-correction already visible in that list: `1ac5885e` ("repair CHANGELOG.md
corruption from an earlier commit in this backport") — the run caught and fixed its
own mistake mid-batch, which is a good sign for the commits it did finish, but is not
a substitute for independent review (none was run on Phase 2 yet — see below).

**Picking up manually where it left off surfaced a real ordering problem the
changelog-driven audit couldn't have caught**: the next commit in the list,
**`ea047db8`** ("avoid ctrl-tab escape sequences in legacy panes"), doesn't apply
against this fork's current tree — its diff assumes the `encode_terminal_key` /
`encode_terminal_key_once` split (repeat-count handling, Windows ConPTY fallback)
that upstream introduced via **`b76adc15`** ("preserve native key lifecycle across
routing", #2142, also in this Stability(83) list at a *later* line than `ea047db8`,
despite predating it by 5 days upstream). `ea047db8` is a prerequisite-dependent
commit that needs `b76adc15` applied first — the same class of issue the original
194-commit audit called out explicitly (see "Commits added during cherry-picking"
near the top of this document).

`b76adc15` itself does not cherry-pick cleanly: conflicts in `src/raw_input.rs`,
`src/server/client_transport.rs`, `src/server/headless.rs`, `tests/api_ping.rs`,
`tests/support/mod.rs`, plus a modify/delete conflict on `tests/cli/sessions.rs`
(deleted in this fork's own history, modified by `b76adc15` upstream). This is a
large, non-trivial reconciliation — **deliberately not attempted in this session**,
per the same "stop and report rather than grind" discipline given to the delegated
run. **`ea047db8` and `b76adc15` are both deferred, not dropped** — they need a
dedicated pass that first understands why this fork deleted `tests/cli/sessions.rs`
before reconciling the rest of `b76adc15`'s conflicts.

**Remaining Stability backlog**: 83 − 13 landed − 2 deferred = **68 commits not yet
attempted**, plus 44 Notable improvements not yet started, plus the codex review this
phase still needs (per the leaner one-reviewer-pass-per-phase structure agreed for
this round — not run yet, since the phase isn't finished).

**Operational note for whoever continues this**: mid-session, a manual conflict
investigation (aborting and retrying `ea047db8`/`b76adc15`) accidentally destroyed an
already-written, not-yet-committed fix (`fb36cc12`'s content, at the time uncommitted)
via `git cherry-pick --abort`, which hard-resets the working tree to `HEAD` —
including unrelated uncommitted changes, not just the conflicted files. It was
recovered by reapplying the same edit and committing immediately. **Lesson: commit or
stash any unrelated working-tree changes before attempting a cherry-pick that might
need `--abort`.**

**Build/test**: clean at `fb36cc12` (see Phase 1's build/test note above — same
result, no regressions from the 13 landed Stability commits).

