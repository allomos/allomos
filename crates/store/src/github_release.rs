//! `alm guide github-release` (Task-A03-planet530), moved out of `guide.rs` to keep that file
//! from growing (Task-A03-dapper008): the procedure an agent follows when the approver writes
//! `alm guide github release` in the conversation. `guide.rs` prints it with the other topics.

/// `alm guide github-release` (Task-A03-planet530): the procedure an agent follows when the
/// approver writes `alm guide github release` in the conversation: publish a new GitHub release
/// with release notes, check it, and (only with the word `install`) install it on this computer
/// (Task-A03-hen326). The same text for every project:
/// it names no repository, address or file; the project's facts come from the project notes'
/// `## Releases` section.
pub(crate) const GITHUB_RELEASE_EN: &str = "# GitHub release: publish a new release, and install it on this computer when asked

The approver writes `alm guide github release` (or `alm guide github-release`) in the
conversation; you ran it, and this is what to do. One outcome: **a new release on GitHub, with
notes that say what changed** (and, when the command says `install`, installed on this computer).
The command in the conversation is the approval for this release, and with `install` for
installing it here too. Anything it does not cover (another version jump than its words say,
publishing somewhere new, installing without `install`) still goes to the approver.

Words after the command, which you read from the approver's line (alm does not act on them):
- `minor` or `major`: raise that part of the version instead of the patch.
- `install`: after step 6, also download and install the release on this computer (step 7).
  Without it, skip step 7: the release is published and checked, nothing is downloaded or
  installed here.
The words combine, for example `alm guide github release minor install`.
Any other word: ask the approver what it means before you tag.

This text names no repository, address or file: every fact of the project comes from the
taskbag's `project-notes.md`, its `## Releases` section (step 0). Where the project's rules
(AGENTS.md) say something else, they win.

## 0. Read the project's release facts

Read the `## Releases` section of the project notes. It is a table with these keys:

| Key | What it says |
|---|---|
| `REMOTE` | the GitHub repository |
| `BASE_BRANCH` | the branch releases are cut from |
| `VERSION_FILES` | every file that holds the version, how it is written there, and what derives from it (a lockfile) |
| `TAG_FORMAT` | how a tag is spelled |
| `BUMP` | which part of the version goes up by default |
| `RELEASE_WORKFLOW` | what builds and publishes on a tag push, how long it takes, how to run it again for an existing tag |
| `NOTES_DIR` | where release-notes pages live, the file name pattern, and the shape of a page |
| `NOTES_LANGUAGES` | every language a page must exist in |
| `NOTES_INDEX` | the index pages that list releases and gain a line |
| `NOTES_URL` | the published address of a release's page, per language |
| `NOTES_PUBLISH` | what publishes the pages, how long it takes, and how to build them locally |
| `GH_NOTES` | who writes the GitHub release's own notes, and whether they link to the pages |
| `INSTALL_ASSET` | the asset to install on this computer, and its checksum file (used only with `install`) |
| `INSTALL_COMMAND` | how it is installed, and whether a person must approve (a password, a dialog) (only with `install`) |
| `VERIFY_INSTALLED` | how to read the installed version afterwards (only with `install`) |
| `APPROVAL_RECORD` | where this release's approval and evidence are written |

When the section is missing, or a key in it: derive what the repository settles (the last tag
and its shape, the workflow under `.github/workflows` that runs on a tag, the files whose version
equals the last tag's, the folder holding the last release's notes page, the assets of the last
GitHub release for this computer), write the section into the project notes, and show it to the
approver before tagging. Ask only for what nothing in the repository settles. Next time the
section is the source.

## 1. Preflight: stop on any failure and say which

- You are on `BASE_BRANCH`, the working tree is clean, pulled, and nothing is ahead of the remote.
- The last release (`gh release list --limit 1`) and the last tag agree.
- No run of `RELEASE_WORKFLOW` is still going (`gh run list --workflow <file> --limit 1`).
- There are commits since the last tag. None: say so and stop, there is nothing to release.
- `gh auth status` passes; with `install`, so does every tool `INSTALL_COMMAND` needs.

## 2. Collect what changed

- `git log <last tag>..HEAD --no-merges` lists the commits; a version bump is not a change.
- When a commit names a task, read it (`alm show <id>`), so the notes say what a person using the
  product notices, not what the code did.
- Sort the changes under **New**, **Fixed** and **Also** (or the headings the last release's page
  uses): one bullet per outcome, a bold lead-in, plain words, present tense; no commit hashes and
  no task IDs in the text. A change nobody using the product can notice (refactoring, tests, agent
  rules) goes under Also in one line, or is left out; say in your handoff what you left out.
- Write the notes in each of `NOTES_LANGUAGES`, translating meaning, not words; product and
  command names stay as they are.

## 3. Choose the version

`BUMP` (or the command's `minor` / `major`) applied to the last tag's version. Say the version you
chose before you write anything: until the tag exists, everything is easy to undo.

## 4. Write the notes

- One page per language in `NOTES_LANGUAGES`, in the same shape as the last release's page (front
  matter, headings, the download line, the date).
- A line for the new release at the top of every page in `NOTES_INDEX`.
- Build the pages locally (`NOTES_PUBLISH` says how), so a broken page is found before it is
  published.
- When `GH_NOTES` links to the pages, every language's page exists before the tag: a missing page
  is a dead link on the GitHub release.

## 5. Bump, commit, tag

- Change every file in `VERSION_FILES` to the new version; update what derives from them (a
  lockfile) with the project's own tool, never by hand.
- One commit holds the notes and the bump, in the project's commit style with the attribution line
  its rules ask for. Push `BASE_BRANCH` and confirm it landed.
- Tag in `TAG_FORMAT`, annotated (`git tag -a <tag> -m <tag>`), and push the tag. The tag push is
  the outside effect; everything before it is local.

## 6. Wait and check

- Wait for the release run and for the notes' publish in the background
  (`gh run watch <run id> --exit-status`); do not poll by hand.
- A failed run: read its log, fix the cause on `BASE_BRANCH`, and run it again the way
  `RELEASE_WORKFLOW` says (often `workflow_dispatch` with the tag). Never delete a tag or a release
  and tag again; say what happened instead.
- Check, and keep the evidence: `gh release view <tag>` lists every expected asset and checksum
  file; the release's own notes point to the pages (`GH_NOTES`); `NOTES_URL` for the new version
  answers 200 in every language, with the version in the page's title.

## 7. Download and install (only with `install`)

Without `install` in the command, skip this step and go on with step 8.

- Download into a new, empty folder outside the project:
  `gh release download <tag> --pattern <INSTALL_ASSET> --pattern <its checksum file>`.
- Check the checksum first; a mismatch stops the install, and you report it.
- Install with `INSTALL_COMMAND`. When it asks for a password or an approval in a dialog, open it
  for the approver and say what to click; never type a password.
- `VERIFY_INSTALLED`: the installed version equals the released one. Report what it shows.

## 8. Record and report

- Write the approval (the approver's command line, and when) and the evidence where
  `APPROVAL_RECORD` says, and in your handoff.
- Report in plain words first: the version, the notes' headings and how many items each has, the
  links to the release and to the notes, and the version installed (without `install`: that it
  was not installed here). Then the evidence: the run IDs, the asset list, and with `install` the
  checksum result. Last, what was not done or not checked.

## What you never do here

- Release from a working tree with changes, from another branch than `BASE_BRANCH`, or while a
  release run is going.
- Guess a value the repository does not settle: ask the approver.
- Force-push, delete a tag or a release, or tag again.
- Type a password or approve an installer's dialog.
";

/// `alm guide github-release` in `language`, or `None` when it is not written in that language
/// (the agent then follows a faithful translation of the English text). Only English so far.
pub fn github_release(language: &str) -> Option<&'static str> {
    match crate::guide::written_in(language)? {
        "en" => Some(GITHUB_RELEASE_EN),
        _ => None,
    }
}
