# 0021 — Audit trail: what the history records, who did it, how tampering shows, and the settings for it

| Status | Raised | Decided | Approver |
|---|---|---|---|
| **Accepted** (as proposed) | 2026-11-04 | 2026-11-08 | will.w.hui |

Card: task `A01-mitten312`. Raised by the approver on 2026-11-04 from the point of view of a company with strict audits; the approver agreed that day to group the additions by what they do (not by standard), to the identity levels in [§ 2](#2-identity) and to doing tamper evidence ourselves rather than relying on Git alone ([§ 3](#3-tamper-evidence)). On 2026-11-08 the approver accepted the record as proposed (see [Decided](#decided)). Facts checked 2026-11-08 ([§ Sources](#sources)).

## In plain words

An auditor asks four things about every change: **who** did it, **on whose approval**, **was it checked**, and **can the record have been changed afterwards**. Allomos's history (one small event file per change, [FILE-FORMAT § Events](../product/FILE-FORMAT.md#events-allomos3)) already says what changed and when. It does not yet answer the four questions well: the "who" is a name a session picks for itself, approvals are sentences in a task's description, a person's check does not say who checked or against which steps, and anyone who can push to the taskbag's repository can rewrite the whole history without a trace.

**Decided** (the approver accepted the proposals on 2026-11-08, [Decided](#decided)):

1. **Always recorded, no switch** (cheap, no privacy cost, useful to a single user too): who ran each change (agent, model, session) and for whom, structured approvals, structured human checks, links to commits and releases, outside effects, risk class and change type, reasons, exceptions, AI details, and links to requirements and decisions ([§ 1](#1-what-the-history-adds)).
2. **Identity in levels, a pseudonym by default:** the history only ever holds a pseudonym ID; who is behind it is kept outside the taskbag ([§ 2](#2-identity)).
3. **Tamper evidence of our own:** each computer signs the events it writes; checkpoints fix a fingerprint of the whole taskbag; optionally an outside witness (a time-stamp service) countersigns the fingerprint, which proves the history existed in that form by that time ([§ 3](#3-tamper-evidence)).
4. **Switches, off by default** (they cost something or would block a single user): real-person identity levels, enforced separation of duties, re-authentication when signing, the outside witness, and retention with legal hold ([§ 1.2](#12-switches-off-by-default)).
5. **A new format, `allomos/8`**, because once events are signed an older Allomos must not keep writing unsigned ones ([§ 4](#4-format-upgrade)).
6. **A new Audit section in Project settings**, and a **This computer** section in the app's Settings ([§ 6](#6-settings)).
7. Allomos is a tool: it **supports** a company's controls and **helps meet** a standard; it never says it "complies". Auditors certify a company's whole practice, not a tool. Allomos does not claim to meet FDA 21 CFR Part 11, which also needs formal system-validation documents ([§ 5](#5-standards-map)).

The questions the approver answered are listed at the end ([§ Questions for the approver](#questions-for-the-approver)). The 16 implementation tasks are unblocked and ready.

## Context

### What the history records today (checked 2026-11-08)

The taskbag holds 3,653 event files (about 4.1 MB of content, 16 MB on disk because each small file takes at least one 4 KB block; the repository's `.git` is 59 MB). Each event has the UTC time, the command, the writer (`alm` or `app`), often the AI agent and its tool, and each changed field's value before and after ([FILE-FORMAT § Events](../product/FILE-FORMAT.md#events-allomos3)).

The gaps:

1. **Human checks say nothing about the check.** `alm checked` adds a "by human" label ([0020](./0020-built-in-labels-and-label-table.md)), but not who checked, against which version of the steps, with what result, or the person's own words.
2. **"Who" is missing or unverifiable.** 1,112 of the 3,653 events (30%) have no `by` (for example 194 `describe`, 114 `add task`, 94 `ai-decided`); `by` is the `--as` name a session chooses, which anyone can type.
3. **Approvals are prose.** They live in task descriptions ("approver, 2026-10-20: …"), with no structured scope, time, channel, expiry or withdrawal, so nothing can check that an outside effect had one.
4. **Events can be changed or removed without a trace.** Files are "never changed or removed once written" only because `alm` and the app do not do it; a person with the files can.

### Why Git alone is not enough

Git's commit hashes do pin every file and all history before them, but whoever can change every copy can rewrite the whole chain, hashes included, and force-push it. Measured 2026-11-04: `allomos/allomos-taskbag` is a private repository of an organization on GitHub's Free plan, and the branch-protection and rulesets APIs both answer 403 ("Upgrade to GitHub Pro or make this repository public"), so force-pushes cannot be forbidden today. Even where they can, an organization owner can turn protection off. Git also uses SHA-1 by default (with collision detection); an auditor may ask about it, so the fingerprints below are SHA-256.

[0007](./0007-git-like-project-sync.md) already says history is not rewritten in normal use. This record adds a way to **show** that it was not.

## 1. What the history adds

### 1.1 Always recorded (no switch)

Each row is one implementation task. Field names are proposals; the implementing task may adjust them (routine choice), but not what they record.

| What | Fields (in the event unless said) | Why an auditor wants it | Task |
|---|---|---|---|
| **Who ran it** | `actor`: `{agent, model, session, tool}` for an agent (`agent` and `tool` exist today); `{person: "<pseudonym ID>"}` for a person in the app. `for`: the pseudonym the agent works for. `approval`: the approval ID it relies on, when it needs one. `by` stays, meaning what it means today. | "Who did this, and on whose behalf?" | A01-black004 |
| **Approvals** | A new command (`alm approve`, and an app button) writes an event (command `approve`) with one change `approval`: `{id, approver, scope: {tasks, plan_sha256}, at, channel: "chat" \| "app" \| "confirm", words, expires}`. Withdrawing is an event with only a `from`. `plan_sha256` fixes the exact plan text approved: a changed plan does not match. | "Was this approved, by whom, for exactly what, and was it still valid?" | A01-coach782 |
| **Human checks** | `alm checked` / **Checked** add a change `human_check`: `{checker, steps_sha256, result: "pass" \| "fail" \| "partly", words, where: "local" \| "deployed"}` beside the label change. | "Who checked, against which steps, and what did they see?" | A01-table778 |
| **Tamper evidence** | `sig` on every event and `checkpoints/` ([§ 3](#3-tamper-evidence)). The outside witness is a switch. | "Could the record have been changed?" | A01-gusto404 |
| **Links to code and releases** | Change `link`: `{kind: "commit" \| "branch" \| "merge" \| "pr" \| "ci" \| "release", ref, url}` (an agent records it; `alm` never runs Git, [0017](./0017-agents-run-git-not-alm.md)). | "Which code went with this change, and was it tested?" | A01-mint242 |
| **Outside effects** | Change `effect`: `{target, action, rollback, approval, result}`, recorded before and after (for example a deployment). | "What touched production, and how would it be undone?" | A01-job650 |
| **Risk class and change type** | Task field `change_type`: `standard` (pre-approved routine), `normal`, `emergency` (approval recorded afterwards, with a deadline); `risk`: `low` \| `medium` \| `high`. | Change management asks for change types and post-approval of emergencies. | A01-palm011 |
| **Reasons** | Change `reason` on every state change and plan change (iteration, base branch, blocked by); asked for, not refused when empty (`alm` tells). | "Why was this moved, dropped, or re-planned?" | A01-air289 |
| **Exceptions** | Change `exception`: `{kind: "check_not_run" \| "without_human_check" \| …, why, accepted_by}`. `--without-human-check` already records the why; this adds who accepted the risk. | Auditors look for controls skipped and who accepted that. | A01-seed343 |
| **AI details** | In `actor`: `model` (exact ID), `effort`, `permission_mode`, `rules`: the commit hash of `AGENTS.md`, `alm`: `alm --version`. What the agent cannot learn is written `"unknown"`, never guessed. | "Which model, under which rules and permissions, made this change?" | A01-urchin867 |
| **Traceability** | Change `traces`: requirement IDs (`R77`) and decision numbers (`0021`). | "Which requirement or decision does this change serve?" | A01-fennel617 |

Lifecycle times (created, ready, started, done) are not stored again: they are worked out from the events.

**Size.** The always-on fields add about 0.4–0.5 KB to each event (measured on samples, 2026-11-04), about 45% more content and almost no more disk (files stay inside their 4 KB blocks). Packed months ([FILE-FORMAT](../product/FILE-FORMAT.md#events-allomos3), from the 28th) keep the file count down; this taskbag's first packing (October, about 2,600 files) happens from 2026-11-28. Storing `describe` as differences instead of whole descriptions (today more than half the content) is a later, separate question.

### 1.2 Switches (off by default)

| Switch | What it does when on | Why it is off by default | Task |
|---|---|---|---|
| **Identity level** above pseudonym | Identity from a sign-in account or the company's SSO ([§ 2](#2-identity)) | Needs a cloud project or an identity provider | A01-frolic587 |
| **Separation of duties** | `alm` and the app refuse that the same person or agent both does and checks (or does and approves) the same task | Blocks a single-person project completely | A01-grotto036 |
| **Re-authentication when signing** | Approving and checking ask for a fresh sign-in; the event keeps the identity provider's signed token | Needs a sign-in level; slows every approval | A01-frolic587 |
| **Outside witness** | Checkpoint fingerprints are countersigned by a time-stamp service ([§ 3.3](#33-outside-witness)) | Sends data (a fingerprint) to an outside service; some services cost money | A01-gusto404 |
| **Retention and legal hold** | A retention period; a hold during which nothing may be removed or redacted | Only meaningful with a company policy | A01-stem986 |

Separation of duties is the one switch that makes `alm` refuse a change on its own judgment. Under "`alm` tells; the agent checks" that needs the approver's approval (Question 6).

## 2. Identity

Agreed by the approver on 2026-11-04: levels, a pseudonym by default.

| Level | Where the identity comes from | For |
|---|---|---|
| **Pseudonym** (default) | A fixed pseudonym ID per person, made by the app; optionally Git commit signing (SSH or GPG key) | local-only projects |
| **Sign-in account** | The cloud project's account ID ([0006](./0006-local-and-cloud-projects.md)) | cloud projects |
| **Company SSO** | The company's identity provider (IdP), signing in with OpenID Connect or SAML 2.0; SCIM 2.0 keeps the member list in step, so a person who leaves is switched off | companies |
| **Re-authentication when signing** | A fresh sign-in at each approval and human check; the event keeps the IdP's signed token (an ID token or SAML assertion), so an auditor can check it was really that person at that time | regulated industries |

- **Events only ever hold the pseudonym ID** (for example `p-7k3q2f`). Real names or email addresses written into the taskbag stay in Git history for good ([AGENTS.md](../../AGENTS.md) keeps personal details out for the same reason).
- **The mapping from pseudonym to person is kept outside Git**: for a local-only project, in Allomos's settings folder on the computer (`identities.json`); for a cloud project, in the cloud. An audit joins the two.
- **Agents are not people.** An agent's events name the agent and session (`actor`) and the person it works for (`for`). Who approved is always a person.
- **Options for the pseudonym's scope:**
  - **A (proposed):** one pseudonym per person **per project**. Two projects cannot be linked to the same person from their files alone; the person's computer keeps one per taskbag.
  - **B:** one per person for every project. Simpler for a company joining records across projects, but leaks that the same person worked on two projects that may be shared with different people.
- **Names the person sees:** "Pseudonym", "Allomos account", "Company sign-in (SSO)", "Sign in again to approve" (Question 2).

## 3. Tamper evidence

Agreed by the approver on 2026-11-04 that we do this ourselves. The pieces, from the inside out:

### 3.1 Signed events

- Each computer (and each cloud writer) has its own **signing key** (Ed25519, RFC 8032), made by the app or `alm` on first use. The private key stays in the operating system's key store (macOS Keychain, Windows Credential Manager), never in the taskbag.
- Each event gets `sig`: `{key: "<key ID>", prev: "<SHA-256 of this key's previous event>", value: "<signature>"}`, computed over the event in canonical JSON (RFC 8785) without `sig`. Nobody without the key can forge an event, and **`prev` chains each computer's own events**: removing one of them from the middle breaks that computer's chain. Two computers never share a chain, so parallel branches still merge without conflict.
- The public keys live in the taskbag, `keys/<key ID>.json`: `{key, public_key, owner: "<pseudonym>", device: "<name the person gave>", added, retired}`. Adding or retiring a key is itself a signed event. A retired key stays, so old events can still be checked.
- **Agents in the cloud** (without this computer's key) write with their writer's own key, or unsigned; an unsigned event is listed as such, never hidden (Question 4).

### 3.2 Checkpoints

- A **checkpoint** fixes a fingerprint of the whole taskbag: `checkpoints/<date>-<time>-<random>.json` with `{at, git_commit (when known), sha256 (of the taskbag's content: every file's path and SHA-256, sorted; checkpoints themselves and `.lock` left out), events: N, heads: {key ID: last event}, receipts: […]}`, signed like an event.
- The Git commit already pins everything before it; the SHA-256 fingerprint is there because Git's default hash is SHA-1. Parallel branches do not matter: the first checkpoint after a merge covers every event from both.
- Each computer's Allomos also remembers, **outside the taskbag** (in its settings folder), the last commit and checkpoint it saw for each taskbag. When the history it pulls no longer contains them, it says "this history was rewritten since <date>": listing the fact, not judging it.
- **Packed months** ([FILE-FORMAT](../product/FILE-FORMAT.md#events-allomos3)) remove the single event files in Git. Each packed line keeps the original file name and the original event, signature included, so a check counts a removed single file as fine when the same signed event appears in that month's packed file; anything else removed is listed.

### 3.3 Outside witness

A checkpoint's fingerprint (never the content) is sent to a service outside the taskbag, which returns a **receipt** signed by it. The receipt is saved in the checkpoint. A receipt cannot be forged; removing one leaves a gap; a rewritten history would need new receipts, all dated after the rewrite.

**The protocol: RFC 3161 time-stamping** (the Time-Stamp Protocol, 2001; RFC 5816, 2010, lets the token name the signing certificate by SHA-256). Allomos sends the fingerprint and a random number over HTTP, and gets back a time-stamp token: the fingerprint, the time, a serial number and the service's signature. The token can later be checked offline against the service's published certificate. `openssl ts` and existing Rust libraries handle it; Allomos invents nothing.

**Rules by region:** in the EU, a **qualified electronic time stamp** under eIDAS, issued by a provider on the EU Trusted List, enjoys a legal presumption that its date and time are accurate (ETSI EN 319 421 and EN 319 422 set the provider policy and the token profile); in China, GB/T 20520-2025 (网络安全技术 公钥基础设施 时间戳规范, in force 2026-02-01, replacing GB/T 20520-2006) is the national time-stamp standard.

**Services, as their own sites state them (checked 2026-11-08):**

| Service | Address | Price | What matters |
|---|---|---|---|
| **DigiCert** public time-stamp server | `http://timestamp.digicert.com` | not stated (widely used without charge) | No terms, limits or service promise published; meant for code signing |
| **Sectigo** public time-stamp server | `http://timestamp.sectigo.com`; an eIDAS one at `…/qualified` | not stated | Asks scripts to wait **15 seconds or more** between requests; no terms or service promise published |
| **GlobalSign** public time-stamp server | `http://timestamp.globalsign.com/tsa/r45standard` (current; older addresses redirect) | not stated; its managed Timestamping-as-a-Service (qualified variant too) is "contact sales" | Account needed for the managed service |
| **FreeTSA.org** | `https://freetsa.org/tsr` | free, commercial use allowed; donations optional | 1 request per second per address; best effort, no service promise; operator not clearly named. Good for trials |
| **Disig** (Slovakia, EU qualified) | after purchase | 100 stamps €11.67, 1,000 €52.50, 10,000 €500.00 (without VAT; price list from 2025-01-01) | Packs last at least 365 days |
| **Namirial** (Italy, eIDAS) | after purchase | 100 €25, 1,000 €175, 5,000 €790 (without VAT) | Used through its own signing software; no public address |
| **联合信任时间戳服务中心** (tsa.cn, Beijing United Trust) | by agreement | not published (packages, enterprise quotes) | Says it is built on RFC 3161 and GB/T 20520-2006 (since replaced by the 2025 edition); shows the National Time Service Center's (国家授时中心) standard time; API access after an application and an agreement |
| Other Chinese CAs (CFCA, BJCA, Shanghai CA) | — | not published | Contact sales |
| EU qualified providers in general | — | — | Listed in the EU Trusted List browser (eIDAS Dashboard) |

No public cloud offers a general RFC 3161 service with a published price (Microsoft's Artifact Signing is code signing, not time-stamping of any fingerprint).

**Proposed presets:** none chosen for the person. The list offers FreeTSA (free, trial) and the three code-signing services; a company with audit needs adds a paid qualified provider as **Custom address…**. Whether the free code-signing servers may be used for this at all is not stated by any of them (Question 3).

**Other witnesses** (not RFC 3161):

| Witness | What | Price | What matters |
|---|---|---|---|
| **Sigstore Rekor** (`rekor.sigstore.dev`) | A public transparency log: anyone can look an entry up | no fee mentioned; run by the OpenSSF (Linux Foundation) | A 99.5% availability target, not a promise; entries are public and permanent; whether non-software records are welcome is an open question on its tracker (issue 2993) |
| **OpenTimestamps** | Anchors fingerprints in the Bitcoin blockchain through free calendar servers | free (donations) | Confirmation "takes a few hours"; no account |
| **Allomos's own cloud witness** | A service of ours that countersigns (cloud projects) | hosting cost | Less convincing than an independent third party; a hosted resource needs the approver's approval |

A checkpoint may be sent to two witnesses at once (the **Second service** setting), so one service closing does not end the chain.

**Lasting longer than a certificate:** a service's signing certificate expires (after some years). Receipts kept for longer get a new time-stamp over the old ones before it expires (RFC 4998, Evidence Record Syntax). Allomos lists receipts whose certificate expires within a year.

**Offline:** a local-only project with no network keeps its checkpoints and sends the fingerprints when it is online again; the receipt then carries the later time, which is honest.

Sending a fingerprint to an outside service is an outside effect: turning the switch on asks the person to confirm, and building it needs the approver's approval of the chosen service (Question 3).

### 3.4 Checking

`alm audit` (proposed name) recomputes the fingerprints, checks every signature and every computer's chain, matches each receipt to its checkpoint, and lists the facts: unsigned events, broken chains, removed files not found in a packed month, gaps between checkpoints, receipts that do not match, keys not in `keys/`. Following "`alm` tells; the agent checks", it says what it found and does not declare the history "valid" or "tampered"; making `alm` judge needs the approver's approval (Question 6).

### 3.5 What this can and cannot show

- **Can:** any change to what a checkpoint covered is found; events cannot be back-dated past a witnessed checkpoint; a forged event without a key is found; a computer's event removed from the middle of its chain is found.
- **Cannot:** a change made between two checkpoints, before the fingerprint left the computer, by someone holding that computer's key. Denser checkpoints make that window smaller. Without the outside witness, someone who controls every copy **and** every key can still rewrite everything; the witness is what closes that.

## 4. Format upgrade

`allomos/7` (decision [0020](./0020-built-in-labels-and-label-table.md), A01-moth111) is already released (0.1.41) and this taskbag is in it, so folding the audit fields into that same upgrade is no longer possible. Under the rule in [FILE-FORMAT § Format versions](../product/FILE-FORMAT.md#format-versions), new optional fields do not raise the number; a change an older Allomos would get wrong does.

| Option | What | For | Against |
|---|---|---|---|
| **A: `allomos/8` when signing starts (proposed)** | The always-on fields arrive as optional fields first (no format change, older builds read past them). Signing, `keys/` and `checkpoints/` come with `allomos/8`: every writer must sign; older builds see the newer format and ask to be updated. | An older Allomos cannot keep writing unsigned events unnoticed. | One more upgrade for every user. |
| B: no format change | Everything as optional fields and new folders | No upgrade step | An older `alm` or app writes unsigned events and never moves `prev`; the check fills with noise and a real gap hides in it. |
| C: wait and do it all at once in `allomos/8` | Fields and signing together | One release | The always-on fields, useful now, wait for the hardest part. |

**Events written before the upgrade** (Question 5):

| Option | What | Against |
|---|---|---|
| **A: mark "before signing" (proposed)** | The `upgrade` event is the first signed event; its checkpoint (and, if the witness is on, its receipt) fixes everything written before. Earlier events are shown as "recorded before signing". | Nothing proves earlier events were not changed before the upgrade. That is the truth. |
| B: sign old events at the upgrade | Each old event gets a signature now | The signature would suggest the event was signed when written: misleading. |
| C: leave old events as they are, no checkpoint | — | Earlier events can still be changed after the upgrade without a trace. |

## 5. Standards map

Product texts say Allomos **supports** a control or **helps meet** a standard. The map goes into the user guide (task A01-gnome953); settings are named by what they do, never by a standard.

| Need | Standard and clause (checked 2026-11-08) | What in Allomos | Turn on |
|---|---|---|---|
| Changes authorized, tested, approved | ISO/IEC 27001:2022 Annex A **8.32** Change management; SOC 2 **CC8.1**; SOX §404 (15 U.S.C. 7262) with PCAOB **AS 2201** (IT general controls) | approvals, change types, links to commits and CI, reasons | always on |
| Logging who did what, when, with what result | ISO/IEC 27001 **8.15** Logging, **8.16** Monitoring activities; SOC 2 **CC7.2**; GB/T 22239-2019 (等保 2.0) level 3 **8.1.4.3** a)–b) (安全计算环境) and **8.1.3.5** (安全区域边界); 21 CFR **11.10(e)** time-stamped audit trails | who ran it, every event | always on |
| Records protected from deletion or change | ISO/IEC 27001 **5.33** Protection of records; GB/T 22239-2019 **8.1.4.3** c)–d); 21 CFR 11.10(e) | signed events, checkpoints | always on; outside witness recommended |
| Trusted time | ISO/IEC 27001 **8.17** Clock synchronization; eIDAS (Reg. (EU) 910/2014 as amended by 2024/1183) **Art. 41–42**, definitions **Art. 3(33)–(34)** | outside witness | Outside witness (a qualified provider for eIDAS) |
| Separation of duties | ISO/IEC 27001 **5.3** Segregation of duties | Separation of duties | Separation of duties |
| Identity and authentication | ISO/IEC 27001 **5.16** Identity management, **8.5** Secure authentication; SOC 2 **CC6.1–CC6.3**; NIST SP 800-63-4 (final 2025-07-31: IAL, AAL, FAL) | identity levels | Allomos account or SSO |
| Electronic signatures linked to records | 21 CFR **11.50**, **11.70**, **11.200(a)(1)** (two distinct identification components); 中国《电子签名法》**第十三条** (reliable signature) and **第十四条** (same effect as handwritten) | approvals and human checks with re-authentication | SSO + Sign in again to approve or check (Allomos still does not claim Part 11: **11.10(a)** system validation is the company's) |
| Log retention | 《网络安全法》**第二十三条（三）**: network logs at least six months (Art. 21 before the amendment in force 2026-01-01); SEC Reg. S-X **Rule 2-06** (17 CFR 210.2-06): auditors keep audit records 7 years (18 U.S.C. 1520 from SOX §802 sets 5 and lets the SEC set more) | Retention, Legal hold | Retention |

Notes: SOC 2's numbers are from the 2017 criteria with revised points of focus (2022); the AICPA published an exposure draft of new 2027 criteria on 2026-10-28 that may renumber them, so the guide's map is checked again when they are adopted. ISO/IEC 27001:2022/Amd 1:2024 (climate action) does not change Annex A. The full texts of GB/T 22239-2019, ISO/IEC 27001 and the SOC 2 criteria are not free: their clause titles were checked against official previews, listings and tables of contents; GB/T 22239's items c)–d) only against secondary sources (Question 8).

## 6. Settings

Today there are two settings windows ([TaskbagSettings](../../apps/web/src/TaskbagSettings.tsx), [SettingsWindow](../../apps/web/src/SettingsWindow.tsx)): **Project settings** (one per taskbag, opened beside the taskbag's name; sections General, Project folders, Agents, Tasks, Handoff) and the app's **Settings** (the whole app; one section, General). Proposed: a new section **Audit** in Project settings, after Handoff, stored in `project.json` (field `audit`) so it travels with the taskbag and everyone on the team works under the same rules; and a new section **This computer** in the app's Settings, for what belongs to one computer only. Help texts below are the English; zh-CN and ja come with the implementation (task A01-tail719).

**Changing a setting is itself recorded:** every change writes an event (command `audit-settings`, kind `project`) with the old and new value and a reason. **Turning a control off** is shown in a warning colour and needs a reason (an auditor's first question is "when was this control switched off, and why?"). In a cloud project only the owner and admin roles ([0004](./0004-team-accounts-and-roles.md)) may change them.

### Project settings → Audit (top to bottom)

| # | Setting | Control and options | Default | Help text | When changed, records |
|---|---|---|---|---|---|
| 1 | **Always recorded** | Read-only list of [§ 1.1](#11-always-recorded-no-switch), no switches; link **See which settings each standard needs** (opens the user guide's map) | — | "Allomos always records who made each change, approvals, human checks, links to code, outside effects and reasons. These cannot be turned off." | nothing (no setting) |
| 2 | **Identity** | Single choice: **Pseudonym** / **Allomos account** / **Company sign-in (SSO)**. The last two are greyed out in a local-only project, with "Needs a cloud project". SSO opens: protocol (OpenID Connect / SAML 2.0), issuer address or metadata URL, client ID, **Test sign-in**. Checkbox **Sign in again to approve or check** (only with account or SSO). | Pseudonym; checkbox off | "The history only ever holds a pseudonym ID. Who is behind each one is kept on this computer (local project) or in the cloud (cloud project), never in the taskbag." With **Show in Finder** (local) or **View in the cloud** (cloud). | `audit-settings`, change `identity` from/to, reason |
| 3 | **Separation of duties** | Checkbox: "The same person or agent may not both do and check (or do and approve) a task" | off | "Turn on when your company requires a second person. A project with one person cannot finish tasks while this is on." | change `separation_of_duties`, reason (required when turned off) |
| 4 | **Outside witness** | Checkbox; turning it on opens a confirmation: "Only a fingerprint of the taskbag is sent to the service you choose, never its content." When on: **Service** (list of presets, see [§ 3.3](#33-outside-witness), or **Custom address…**), optional **Second service**; **How often**: Every day / At every approval and human check / Only when I ask; **Make a checkpoint now**; status: last checkpoint and its time, number of receipts, fingerprints waiting to be sent (offline), gaps and mismatches found (facts, not verdicts). | off; when first turned on: Every day | "A time-stamp service countersigns the taskbag's fingerprint, so the history can be shown to have existed in this form at that time. Some services charge per time-stamp." | change `witness` (service, frequency) from/to, reason; each checkpoint is its own event |
| 5 | **Retention** | **Keep the history**: Forever / N years (with the note "China's Cybersecurity Law (Art. 23) asks for network logs to be kept at least six months"); **Legal hold** checkbox (asks for a reason; while on, nothing can be removed or redacted); **Remove personal details…** button (choose the fields, give a reason; leaves an event that says what was removed, without the old values) | Forever; hold off | "Removing personal details changes the files from now on. Older versions stay in Git history unless the history is rewritten, which the audit check will then report." | change `retention` / `legal_hold`, reason; a removal writes `redact` with field names and reason only |
| 6 | **Change types** | Shows the fixed classes Standard / Normal / Emergency and the risk levels, with meanings; whether a project may rename them is open (Question 7) | — | "Emergency changes may go ahead first; their approval is recorded afterwards, by the deadline shown." | (only if renaming is allowed) change `change_types`, reason |

### Settings → This computer (new section)

| Setting | Control | Help text | When changed, records |
|---|---|---|---|
| **Signing key** | Shows the public key's fingerprint; **Export public key…** (to give an auditor or the team); **Replace key…** (the old public key stays in `keys/`, retired, so older events can still be checked) | "This computer signs every change it writes. The private key stays in this computer's key store." | Replacing writes a `key` event in every open taskbag (new key added, old one retired) |
| **Last seen history** | Read-only list, per taskbag: last commit and checkpoint seen, and whether the history still contains them | "If a taskbag's history is rewritten, Allomos tells you here and in the task list." | nothing |

### Shown in the task pane, not in settings

Each task shows its approvals, who checked it and how, its exceptions, its linked commits and CI runs, and its outside effects. Each implementation task builds its own part.

## Decided

On 2026-11-08, in this task's session, after the record was drafted, the approver wrote: "i approve your outcome, so the tasks can be proceed without stop". Taken as: **every proposal below is the answer to its question**, and the implementation tasks are unblocked and made ready. Not covered, and still needing the approver's exact approval when they come up: sending fingerprints to any outside service, paying for a service, and any hosted resource (Question 3; [AGENTS.md](../../AGENTS.md) § Always-on boundaries).

## Questions for the approver

Each has a proposal; the approver accepted all of them on 2026-11-08 ([Decided](#decided)).

1. **Which fields, in how many steps?** Proposed: step 1 the always-on fields of [§ 1.1](#11-always-recorded-no-switch) as optional fields (no format change); step 2 signing, keys and checkpoints with `allomos/8`; step 3 the outside witness; step 4 identity levels above pseudonym, re-authentication and separation of duties (they need the cloud). Drop or add any row.
2. **Identity:** the four levels and their names ("Pseudonym", "Allomos account", "Company sign-in (SSO)", "Sign in again to approve or check"); pseudonym per project (A) or per person (B); the mapping in the settings folder (local) and the cloud.
3. **Outside witness:** which service(s) to preset, and whether to pay for one; default frequency (proposed: every day); offline: send later with the later time (proposed). **Approval to send fingerprints to the chosen service** (an outside effect) once it is built.
4. **Signing:** Ed25519 keys per computer in the OS key store (proposed); what a cloud agent without a key does: sign with its writer's own key, or write unsigned events that are listed (proposed: listed).
5. **Format:** `allomos/8` when signing starts, fields first as optional fields (A, proposed), or B / C; old events marked "recorded before signing" (A, proposed), or B / C.
6. **Checks inside `alm`:** may `alm audit` say "valid" / "not valid" (a check of its own), or only list the facts (proposed)? May separation of duties make `alm` refuse a change when the switch is on (proposed: yes, as a switch the project turns on)?
7. **Settings:** the Audit section in Project settings after Handoff, and This computer in the app's Settings; the names and help texts above; may a project rename the change types (proposed: no, fixed)?
8. **Standards:** is the map of [§ 5](#5-standards-map) enough, or should each line be checked against the standard's full text by someone who has it (some, like ISO/IEC 27001 and SOC 2, are paid or member-only documents)?

## The implementation tasks

Blocked by `A01-mitten312` until the decision; on 2026-11-08 they were unblocked and made ready ([Decided](#decided)), each pointing to its part of this record.

| Task | Part |
|---|---|
| A01-black004 | who ran it, for whom, approval ID |
| A01-coach782 | structured approvals |
| A01-table778 | structured human checks |
| A01-grotto036 | separation of duties (switch) |
| A01-gusto404 | signing, checkpoints, outside witness |
| A01-mint242 | links to commits, PRs, CI and releases |
| A01-job650 | outside-effect ledger |
| A01-palm011 | risk class and change type |
| A01-air289 | reasons for state and plan changes |
| A01-seed343 | exceptions and who accepted them |
| A01-urchin867 | AI details |
| A01-frolic587 | identity levels and re-authentication |
| A01-fennel617 | links to requirements and decisions |
| A01-stem986 | retention and legal hold |
| A01-tail719 | Project settings → Audit |
| A01-gnome953 | the user guide's standards map |

## Consequences

- **Format:** [FILE-FORMAT](../product/FILE-FORMAT.md) gains the new event fields, `sig`, `keys/`, `checkpoints/` and `project.json`'s `audit`; `allomos/8` per Question 5.
- **`alm`:** `approve`, `audit`, the new options on `checked`, `done`, `wait`, `block` and plan changes (`--reason`, `--link`, …); `alm guide` explains them.
- **App:** the Audit and This computer sections, the task pane's audit parts, the confirmation when the witness is turned on.
- **AGENTS.md:** agents pass the new details (model, effort, permission mode) when `alm` asks; running sessions are told (shared rule change).
- **Not changed:** states, labels ([0020](./0020-built-in-labels-and-label-table.md)), and that `alm` never runs Git ([0017](./0017-agents-run-git-not-alm.md)).

## Sources

Read 2026-11-08 by this task's research (official sites where they could be opened; where one refused automated reading, its official listing or preview).

- ISO/IEC 27002:2022 table of contents (official preview): <https://assets.vde-verlag.de/iec-normen/preview-pdf/info_isoiec27002%7Bed3.0%7Den.pdf>; Amd 1:2024: ISO-IAF joint communiqué, Feb 2024.
- SOC 2: <https://www.aicpa-cima.com/resources/download/2017-trust-services-criteria-with-revised-points-of-focus-2022>; 2027 exposure draft: <https://www.aicpa-cima.com/news/article/aicpa-proposes-update-of-soc-2-r-trust-services-criteria-to-enhance-the-relevance-usability-and-value-of-cpas-reports>
- SOX: <https://www.law.cornell.edu/uscode/text/15/7262>, <https://www.law.cornell.edu/uscode/text/15/7241>, <https://www.law.cornell.edu/uscode/text/18/1520>; 17 CFR 210.2-06 (eCFR); <https://pcaobus.org/oversight/standards/auditing-standards/details/AS2201>
- GB/T 22239-2019: <https://openstd.samr.gov.cn/bzgk/gb/newGbInfo?hcno=BAFB47E8874764186BDB7865E8344DAF>; clause items: <https://support.huaweicloud.com/bestpractice-cbh/cbh_08_1011.html>
- 网络安全法 (amended 2025-10-28): <http://www.npc.gov.cn/npc/c2/c30834/202510/t20251028_449048.html>, <https://www.cac.gov.cn/2025-12/29/c_1768735112911946.htm>
- 21 CFR Part 11: eCFR, title 21 part 11 (version 2026-11-01)
- eIDAS consolidated: <https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=CELEX:02014R0910-20241018>
- 电子签名法 (amended 2019-04-23): <https://wap.miit.gov.cn/zwgk/zcwj/flfg/art/2022/art_4ed0e4a46946479c90a9e0f2c76a11ef.html>
- NIST SP 800-63-4: <https://csrc.nist.gov/pubs/sp/800/63/4/final>
- RFC 3161, 5816, 4998, 7643, 7644, 8032, 8785: <https://www.rfc-editor.org/>; ETSI EN 319 421 (V1.3.1, 2025-07) and EN 319 422 (V1.1.1): <https://www.etsi.org/deliver/etsi_en/319400_319499/>; GB/T 20520-2025: openstd.samr.gov.cn; OpenID Connect Core 1.0 (errata set 2): <https://openid.net/specs/openid-connect-core-1_0.html>; SAML 2.0 (OASIS, 2005)
- Git: <https://git-scm.com/docs/hash-function-transition>; GitHub: <https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches>
- Time-stamp services: <https://knowledge.digicert.com/general-information/rfc3161-compliant-time-stamp-authority-server>, <https://sectigo.com/resources/time-stamping-server>, <https://globalsign.com/timestamp-service>, <https://www.freetsa.org/index_en.php>, <http://eidas.disig.sk/en/qualified-electronic-time-stamps/pricelist/>, <https://www.namirial.global/?p=372>, <https://www.tsa.cn/>, <https://eidas.ec.europa.eu/efda/tl-browser/>, <https://azure.microsoft.com/en-us/products/artifact-signing>
- Other witnesses: <https://docs.sigstore.dev/logging/overview/>, <https://github.com/sigstore/rekor/issues/2993>, <https://opentimestamps.org/>
