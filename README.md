# git-rbx

Semantic diff, three-way merge, and conflict resolution for Roblox place and
model files (`.rbxl`, `.rbxlx`, `.rbxm`, `.rbxmx`), as a git extension.
```
$ git merge feature
Merged in 1.32s: 214 ours + 96 theirs ops applied, 40 deduped, 2 conflicts
CONFLICTS (2):
  ! Workspace.Lobby.Door — property 'CFrame' (base content kept)
  ! Workspace.Props.Crate — delete vs edit (base content kept)
The conflicts are stored inside the file itself (a __GitRbxConflicts folder you will
see in Studio — leave it; resolving removes it). Resolve with:
  git rbx resolve Workspace/map.rbxl --list        (or --studio)
```

## Contents

- [Install](#install)
- [Quick start](#quick-start)
- [Change types](#change-types)
- [Conflict types](#conflict-types)
- [Commands](#commands)
- [Automation and agents](#automation-and-agents)
- [GitHub](#github)
- [Git LFS](#git-lfs)
- [How it works](#how-it-works)
- [Limitations](#limitations)
- [Development](#development)

## Install

The binary has to be on your `PATH` as `git-rbx`; git then dispatches
`git rbx <subcommand>` to it. Prebuilt binaries ship on every
[release](https://github.com/revvy02/git-rbx/releases) for macOS (Apple
silicon and Intel), Linux x86_64, and Windows x86_64.

With [mise](https://mise.jdx.dev), add it to your project's `mise.toml` (or
`mise use -g github:revvy02/git-rbx@latest` for every project):

```toml
[tools]
"github:revvy02/git-rbx" = "latest"
```

The repository is private, so mise needs a `GITHUB_TOKEN` in the environment
with access to it to fetch the release asset. Recent mise versions also hide
releases younger than 24 hours (`minimum_release_age`); to pick up a release
sooner, pin its version (`"github:revvy02/git-rbx" = "0.0.1"`) or add
`minimum_release_age_excludes = ["github:revvy02/git-rbx"]` under
`[settings]`.

Or build from source:

```sh
cargo install --locked --git https://github.com/revvy02/git-rbx
```

Then wire git up. Once per machine:

```sh
git rbx install
```

## Quick start

**See what changed.** `git diff` and `git rbx changes` show instances and
properties, not bytes:

```sh
git diff                              # worktree vs index, semantic for Roblox files
git show --ext-diff HEAD              # git log/show need --ext-diff for external drivers
git rbx changes main..feature         # every Roblox file changed between two revisions
git rbx diff old.rbxl new.rbxl        # any two files, no git required
```

**Merge.** Nothing to do — `git merge`, `git rebase`, and `git cherry-pick`
call the driver. Non-overlapping edits compose; identical edits on both sides
dedupe; only genuinely competing edits become conflicts.

**Resolve.** A conflicted merge leaves the file in your worktree with the
conflict state stamped into it. Pick a route:

```sh
git rbx resolve map.rbxl --studio                       # visual: Ours/Theirs/Custom in Studio
git rbx resolve map.rbxl --list                         # or from the CLI …
git rbx resolve map.rbxl --take theirs --entry Conflict_1
git rbx resolve map.rbxl --take ours --path Workspace.Props.Crate
git rbx resolve map.rbxl --finalize                     # apply choices, strip conflict state
git add map.rbxl && git commit
```

`git mergetool` also works: `install` registers git-rbx as a mergetool that
opens the Studio resolver. `git rbx check <file>` exits nonzero while a
file still carries conflict state, which is what the optional pre-commit
hook enforces.

## Change types

Every diff — `diff`, `changes`, `git diff`, and the conflict reports — is
expressed in five kinds of change. Each entry is one primitive operation;
an instance that was reparented *and* edited appears as one Reparented entry plus one
Modified entry, never a blended record.

| Kind | Meaning |
|---|---|
| **Added** | An instance (with its whole subtree) exists only in the new version. |
| **Removed** | An instance (with its whole subtree) exists only in the old version. |
| **Modified** | The same instance has different property values. A rename is a Modified entry on the `Name` property. |
| **Reparented** | The same instance has a different parent (`old_path` → `path`). |
| **Pivoted** | A Model and its world-space descendants were transformed together as a rigid body. |

**Property granularity.** Container properties are diffed per key rather
than as one blob: an attribute change is reported as `Attributes.<key>` and
a tag as `Tags.<tag>`, so two branches editing different attributes of the
same instance compose instead of conflicting. Reference-valued properties
(`PrimaryPart`, `ObjectValue.Value`, adornee-style refs) are compared by the
*logical target* — a ref that points at the same instance in both versions
is unchanged even though the underlying referent ids differ between files.

**Pivoted, in detail.** Dragging a Model in Studio rewrites the `CFrame` of
every descendant part. A naïve diff reports thousands of modified CFrames
and cannot tell that from thousands of independent edits. git-rbx factors
hierarchical rigid transforms out first: if a Model's descendants all moved
by the same transform, that is one `Pivoted` entry carrying the delta
(`Δ` position and axis-angle rotation), and only the *residual* edits —
parts that moved differently from their model — appear as Modified. Nested
pivots are reported relative to the nearest pivoted ancestor (`order` /
`parent_order` in the JSON) so a moved building with a moved door inside
reads as two small deltas, not one large and one enormous one. Pivots are
also first-class merge operations: two branches pivoting different models
compose, and two branches pivoting the same model differently is a
[Pivot conflict](#conflict-types).

## Conflict types

A three-way merge computes two change sets against the common ancestor and
combines them. Changes to different targets compose; identical changes
dedupe; the remainder are conflicts. A conflict never silently picks a side:
the contested instance keeps its *base* content in the merged file, both
competing versions are stored alongside it, and you choose.

| Kind | What happened | Resolution options |
|---|---|---|
| **Property** | Both sides set the same property of the same instance to different values (including `Name`, and per-key `Attributes.<key>`). | `ours`, `theirs`, or `custom` with a value of your own. |
| **PropertyBundle** | Both sides changed properties that only make sense together. The canonical case is a MeshPart's `MeshContent` and `InitialSize`: taking the mesh from one side and the source extent from the other renders the mesh at a wildly wrong scale. | `ours` or `theirs`, applied atomically to the whole bundle. |
| **DeleteVsEdit** | One side deleted a subtree; the other edited inside it, reparented something into it, or reparented something out of it. Reported once per deleted root, however many edits the other side made underneath. | `ours` or `theirs` — the whole branch outcome for that subtree. If the surviving side had reparented edited descendants out before the delete, those *reparent-outs* are preserved with either choice. |
| **ReparentTarget** | Both sides reparented the same instance to different parents (or to destinations that cannot be proven equal across the two branches). | `ours` or `theirs`. |
| **Pivot** | Both sides pivoted the same Model by different transforms. Nested pivots resolve in top-down order. | `ours` or `theirs`; the chosen delta is applied to the whole rigid body. |

**Rigid groups.** When a spatial edit touches many parts that are not under
one Model — a hand-built assembly, say — the same rigid movement can surface
as dozens of CFrame conflicts. These are folded into a *rigid group*
(`Group_N`), shown as one decision with the two candidate deltas, and
resolvable as one: `--take theirs --entry Group_1`.

**What does not conflict.** Two branches making the *same* change dedupe,
including identical additions (and references pointing into each branch's
copy of the identical new content), identical reparents, and both branches
evacuating the same instances out of a container before deleting it.
Different attributes or tags on the same instance compose. Tags never
conflict at all (set semantics).

## Commands

```
git rbx diff <old> <new> [--format pretty|summary|json|markdown] [--max-rows N] [-t]
git rbx changes <base> <head> [--format markdown|json|pretty|summary] [--max-rows N]
git rbx merge <base> <ours> <theirs> [--output FILE] [--path REAL_PATH] [--json]
git rbx resolve <file> --list [--json]
git rbx resolve <file> --take ours|theirs (--entry NAME | --path BASE_PATH | --all)
git rbx resolve <file> --take custom --entry NAME --value JSON
git rbx resolve <file> --finalize
git rbx resolve <file> --studio
git rbx check <file> [--json]
git rbx install [--global|--local] [--no-attributes] [--hooks] [--exe PATH] [--check]
git rbx git-diff <git external-diff arguments>
```

- **`merge`** is the git merge driver (`git-rbx merge %O %A %B --path %P`).
  Git passes extensionless temporary copies; `--path` (git's `%P`) supplies
  the real filename, which decides the output encoding and model-vs-place
  behavior. Without it the encoding is sniffed from the file header. Exit 0
  means clean; exit 1 means conflicts were written into the output.
- **`resolve --take`** records a decision without changing content;
  `--finalize` applies every decision, strips the conflict container and
  tags, and writes a clean file. Studio never serializes the result — all
  writes go through git-rbx, so Studio's own load/save side effects never
  leak into the merge.
- **`resolve --studio`** opens the file in Roblox Studio through
  [rodeo](https://github.com/revvy02/rodeo) with conflicted instances
  highlighted and an Ours/Theirs/Custom panel. Decisions made there call
  back into the same `--take`/`--finalize` commands; the file on disk is
  the only source of truth, so CLI and Studio resolution can be interleaved.
- **`git-diff`** is the git external-diff entry point; you never call it
  directly.

## Automation and agents

Every command that produces a decision has a machine-readable form, and the
conflict state lives in the file, so an agent can drive a merge end to end
with no GUI:

```sh
git merge feature || true
git rbx check map.rbxl --json              # {"clean":false,"unresolvedCount":2}
git rbx resolve map.rbxl --list --json     # the full conflict report (below)
git rbx resolve map.rbxl --take theirs --entry Conflict_1
git rbx resolve map.rbxl --take custom --entry Conflict_2 --value 0.5
git rbx resolve map.rbxl --finalize
git rbx check map.rbxl && git add map.rbxl && git commit
```

The **conflict report** (`resolve --list --json`, and embedded in
`merge --json`) lists each conflict with its stable entry name, kind, base
path, property, resolution state, rigid-group membership, and — for each
side — whether it deleted the target, where it moved it, its pivot delta,
and an *impact*: the exact patch that choosing that side applies, with typed
before/after values. That is enough to decide on the merits ("theirs moved
the door two studs; ours only retextured it") rather than blindly picking a
side. Entry names are deterministic across runs, so a recorded decision
can be replayed. `changes --format json` and `diff --format json` use the
same typed value encoding.

## GitHub

GitHub cannot render Roblox file diffs and has no hook for custom
renderers; `.gitattributes` diff drivers are a local-git mechanism. The
review surface has to be fed from CI instead.
[`.github/workflows/roblox-changes.yml`](.github/workflows/roblox-changes.yml)
is a reference workflow: on every pull request and push it runs
`git rbx changes` between the two revisions, appends the markdown to the
check's step summary (so it exists on every commit), and posts a single
pull-request comment that later pushes update in place. Copy it into a
repository that stores Roblox files and adjust the install step.

## Git LFS

Large place files usually live in Git LFS, and LFS is a clean/smudge
filter that git does **not** run for merge drivers or external diff
commands: each side arrives as pointer text, and whatever a driver writes
back is stored verbatim. git-rbx handles this transparently — pointers are
resolved through `git lfs smudge` on read (before the extension is trusted,
so skip-smudge checkouts work too), and a result that replaces a pointer is
written back through `git lfs clean`, so the repository keeps a pointer and
the worktree gets real content. Put `git rbx install` *after* `git lfs
track` in `.gitattributes` history, or just re-run it: the managed block is
always kept last so it overrides LFS's own `merge=lfs diff=lfs` lines.

## Limitations

- Identity is heuristic. Truly ambiguous cases (identical twins under one
  parent, both edited on both branches) resolve positionally and
  consistently, but a sufficiently creative reorganization can still read
  as remove-and-add. Renamed *and* reparented *and* edited in one commit is the
  known gap: rename-and-reparent and reparent-and-edit are detected, all three
  together are not.
- Studio materializes content on load and save (services, a session
  camera, migration attributes). A fresh Rojo build compared with a Studio
  save is never a zero diff; compare save with save.
- `git log -p` and `git show` only use external diff drivers with
  `--ext-diff`; `git diff` uses them automatically.
- The Studio resolver currently runs from the git-rbx source checkout it
  was built in (it needs the `studio-resolver` sources); the CLI path has
  no such dependency.