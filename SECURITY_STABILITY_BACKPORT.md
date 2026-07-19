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
