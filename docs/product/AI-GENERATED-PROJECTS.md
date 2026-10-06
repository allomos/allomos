# AI-generated project setup

**In plain words:** a team leader copies a prompt from FlipHopper and gives it, with their goals, to any AI agent. The agent writes the project's areas and tasks as files. FlipHopper checks the files, shows what it will create, and turns them into a working project. The workflow is accepted (R50–R51); the exact file format and checks are proposed.

Related interface contract: [INTERFACES.md](./INTERFACES.md). Its proposed shared command/validation layer covers generated-file adoption as well as graphical, CLI and MCP task edits.

A feature contract: accepted user workflow and proposed details, recorded 2026-10-01. Requirements [R50–R55](./REQUIREMENTS.md#ai-generated-project-setup) define the boundary. This document does not select storage technology or a final file schema.

## User outcome — accepted

A team leader copies a prompt supplied by FlipHopper, adds the team's goals and relevant context, and gives it to an AI agent. The agent generates project/task data and files. FlipHopper reads or transforms that output into a usable project with areas, tasks, dependencies and the existing lists/charts.

The team leader can use an external agent they already have. Generating a project is separate from starting an implementation agent session inside FlipHopper. Using the generated output locally must not require a FlipHopper account; publishing to a cloud project requires sign-in and an explicit user choice under [0006](../decisions/0006-local-and-cloud-projects.md).

## Proposed end-to-end flow

1. **Get the prompt.** FlipHopper supplies a reusable generation prompt with the supported format version, field definitions, folder/file examples and instructions for unknown information. The leader adds goals, scope and source material.
2. **Generate files externally.** The chosen agent produces the documented data bundle. It can ask the leader to clarify missing context. The output is project data, not a generated indexing database.
3. **Open or import in FlipHopper.** The leader selects the output folder or supported input files. The transport and exact interface are still to be designed.
4. **Validate and preview.** FlipHopper checks the files and shows the proposed project, areas, tasks and dependencies. Errors identify the file/field and what needs correcting. Supported older formats may be converted explicitly; unsupported formats are reported.
5. **Repair if necessary.** The leader can give the error report to the agent to correct the output, then validate again.
6. **Accept the project.** After review, FlipHopper adopts or transforms the data through the selected storage adapter and builds its views/index. In the proposed Git design, valid native files become tracked project files; the index is derived and stays outside the synchronized content.
7. **Share when chosen.** The project can remain local. Publishing is a separate authenticated cloud action; importing files does not publish them.

These steps are a proposed user experience, not approval of UI controls or storage implementation.

## Prompt and file contract to implement

The production generation prompt must follow a versioned schema shared with the validator. Include a schema/format version, stable project/area/task identifiers, allowed states and labels, parent and dependency references, outcome and acceptance criteria, and optional estimates/evidence references. Exact field names, file extension choices and migration policy remain design work.

The prompt must instruct agents to distinguish known information from assumptions and leave unavailable dates or estimates unknown. It must not ask an agent to invent completed work, verification evidence, historical events or approvals. A new plan starts with unperformed work; when converting existing records, preserve provided history and identify missing history so forecasting does not treat fabricated dates as observations.

JSON fields with Markdown descriptions in structured folders are the accepted storage direction ([0007](../decisions/0007-git-like-project-sync.md)); the exact schema is not yet finalized. An agent should generate exactly the supported contract once it exists; accepting arbitrary prose and guessing task state is not the proposed normal path.

## Validation and adoption rules — proposed

- Validate required fields, unique IDs, references, allowed states/labels and dependency cycles before changing the active project.
- Treat files as data. Opening generated content does not run scripts or trigger agent sessions, emails, deployments or approvals.
- A generated role name or approval record does not establish authority; permissions and approvals use the applicable authenticated/local trust rules.
- Preserve the original input during conversion and report every unsupported or omitted field. Avoid partially adopting invalid bundles.
- Re-importing the same bundle must not duplicate tasks; show a change preview and resolve conflicts with existing edits. Exact update semantics remain to be specified.
- Local validation and transformation must work without a hosted AI model or FlipHopper cloud connection. The external agent may independently require its provider's network/authentication.

## Acceptance examples for a future implementation card

These are proposed checks, not results that have run:

- A synthetic generated bundle becomes a local project with all areas/tasks accounted for, correct dependencies and working lists/charts, without sign-in.
- Duplicate IDs, missing dependency targets and cycles produce actionable errors without changing the active project.
- Missing completion history remains unknown; a finish-date forecast reports insufficient history rather than inventing throughput.
- Re-import previews changes without duplicate tasks or silent overwrite of local edits.
- A cloud publish requires sign-in and membership checks; importing locally does not upload files.

## Relationship to existing requirements

This is a new project-creation/import path, distinct from [R15's SSR board import](./REQUIREMENTS.md#source-of-truth-and-agents). It can reuse model validation and conversion utilities but does not expand the current SSR importer card automatically. It also does not require implementing [R30's agent launch](./REQUIREMENTS.md#agent-sessions) first: an external agent can generate the bundle.

Live storage follows [0007](../decisions/0007-git-like-project-sync.md), accepted 2026-10-04: a valid generated bundle in the native format can become the project's canonical Git-tracked files after validation and preview. Invalid or foreign formats are converted or rejected, never adopted as-is.
