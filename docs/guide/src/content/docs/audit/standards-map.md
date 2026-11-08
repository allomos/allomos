---
title: Audit and standards
description: What Allomos always records, what each audit setting does, and which settings help meet ISO/IEC 27001, SOC 2, 等保 2.0 and other standards.
sidebar:
  order: 1
---

An auditor asks four things about every change: **who** did it, **on whose approval**, **was it checked**, and **can the record have been changed afterwards**. Allomos keeps a record that answers them, in the taskbag's history (one small file per change).

This page says what Allomos records on its own, what each setting in **Project settings → Audit** adds, and which settings to turn on for some common standards.

:::note[Allomos helps; it does not certify]
Allomos is a tool. It **supports** a company's controls and **helps meet** a standard; it never makes a project "compliant". Auditors certify a company's whole practice, not a tool. Allomos does not claim to meet FDA 21 CFR Part 11, which also needs the company's own system-validation documents.
:::

## Always recorded

These need no setting and cannot be turned off. They cost little and help a single user too.

| What | What is kept |
|---|---|
| **Who ran it** | For an agent: which agent, model and session, and the person it works for. For a person: a pseudonym ID (see [Identity](#identity)). |
| **Approvals** | Who approved, for exactly which tasks and plan, when, through which channel, in their own words, and when it expires or was withdrawn. |
| **Human checks** | Who checked, against which version of the check steps, the result (pass, fail, partly) and what they wrote. |
| **Signed changes** | Each computer signs the changes it writes; checkpoints fix a fingerprint of the whole taskbag. Removing or changing a change afterwards shows up in `alm audit`. |
| **Links to code** | Commits, branches, merges, pull requests, CI runs and releases that went with a task. |
| **Outside effects** | What touched an outside system (for example a deployment): target, action, how to undo it, the approval and the result. |
| **Change type and risk** | Standard, normal or emergency; low, medium or high risk. An emergency change records its approval afterwards. |
| **Reasons** | Why a task was moved, dropped or re-planned. |
| **Exceptions** | A check that was not run or a task finished without a human check, why, and who accepted the risk. |
| **AI details** | The exact model ID, effort, permission mode, the version of the rules (`AGENTS.md`) and of `alm`. What an agent cannot know is written as "unknown", never guessed. |
| **Traceability** | The requirements and decisions a task serves. |

## The audit settings

**Project settings → Audit** (open the settings beside the taskbag's name). The settings are stored in the taskbag, so everyone on the project works under the same rules. Every change to them is itself recorded, with a reason; turning a control off is shown in a warning colour and needs a reason.

### Identity

Where the identity behind each change comes from:

- **Pseudonym** (default): the history only ever holds a pseudonym ID. Who is behind it is kept on this computer, outside the taskbag.
- **Allomos account**: the cloud project's account. Needs a cloud project.
- **Company sign-in (SSO)**: the company's identity provider (OpenID Connect or SAML 2.0). Needs a cloud project.
- **Sign in again to approve or check**: each approval and human check asks for a fresh sign-in (with an account or SSO only).

### Separation of duties

When on, the same person or agent may not both do and check, or do and approve, the same task: `alm` and the app refuse it. Off by default, because a project with one person could not finish any task.

### Outside witness

When on, a time-stamp service (RFC 3161) countersigns each checkpoint's fingerprint, so the history can be shown to have existed in that form by that time. **Only the fingerprint is sent, never the taskbag's content.** You choose the service (some charge per time-stamp) and how often: every day, at every approval and human check, or only when you ask. Off by default, because it sends data to an outside service.

### Retention

How long the history is kept (forever, or a number of years), and a **legal hold**, during which nothing may be removed. **Remove personal details…** removes chosen fields from a task and leaves a record of what was removed and why, without the old values.

### Change types

Shows the fixed change types and risk levels with their meanings. Nothing to set.

## Which settings each standard needs

The map below says, for each need, which clauses ask for it, what in Allomos helps, and what to turn on. "Always on" means nothing to turn on.

| Need | Standard and clause | What in Allomos helps | Turn on |
|---|---|---|---|
| Changes authorized, tested and approved | ISO/IEC 27001:2022 Annex A **8.32** Change management; SOC 2 **CC8.1**; SOX §404 (15 U.S.C. 7262) with PCAOB **AS 2201** | Approvals, change types, links to commits and CI, reasons | Always on |
| A log of who did what, when, with what result | ISO/IEC 27001 **8.15** Logging, **8.16** Monitoring activities; SOC 2 **CC7.2**; GB/T 22239-2019 (等保 2.0) level 3 **8.1.4.3** a)–b) and **8.1.3.5**; 21 CFR **11.10(e)** | Who ran it, every change | Always on |
| Records protected from deletion or change | ISO/IEC 27001 **5.33** Protection of records; GB/T 22239-2019 **8.1.4.3** c)–d); 21 CFR **11.10(e)** | Signed changes, checkpoints | Always on; **Outside witness** recommended |
| Trusted time | ISO/IEC 27001 **8.17** Clock synchronization; eIDAS (Regulation (EU) 910/2014 as amended by 2024/1183) **Art. 41–42**, definitions **Art. 3(33)–(34)** | Outside witness | **Outside witness** (for eIDAS, a qualified provider from the EU Trusted List) |
| Separation of duties | ISO/IEC 27001 **5.3** Segregation of duties | Separation of duties | **Separation of duties** |
| Identity and authentication | ISO/IEC 27001 **5.16** Identity management, **8.5** Secure authentication; SOC 2 **CC6.1–CC6.3**; NIST SP 800-63-4 | Identity levels | **Identity**: Allomos account or Company sign-in (SSO) |
| Electronic signatures linked to records | 21 CFR **11.50**, **11.70**, **11.200(a)(1)**; China's Electronic Signature Law (电子签名法) **Art. 13–14** | Approvals and human checks with a fresh sign-in | **Identity**: SSO, and **Sign in again to approve or check** |
| Log retention | China's Cybersecurity Law (网络安全法) **Art. 23(3)**: network logs at least six months; SEC Regulation S-X **Rule 2-06** (17 CFR 210.2-06): auditors keep audit records seven years (see also 18 U.S.C. 1520) | Retention, legal hold | **Retention** (at least what your rules ask) |

### Quick lists by standard

- **ISO/IEC 27001:2022**: always on, plus **Outside witness** (8.17), **Separation of duties** (5.3), **Identity** with an account or SSO (5.16, 8.5).
- **SOC 2**: always on (CC7.2, CC8.1), plus **Identity** with an account or SSO (CC6.1–CC6.3).
- **等保 2.0 (GB/T 22239-2019), level 3**: always on, plus **Outside witness** recommended (8.1.4.3 c)), and **Retention** of at least six months under the Cybersecurity Law.
- **SOX (IT general controls)**: always on (change management), plus **Retention** long enough for audit records (seven years under Rule 2-06).
- **21 CFR Part 11**: always on, plus **Identity** with SSO and **Sign in again to approve or check**. The system validation of 11.10(a) is the company's own work; Allomos does not claim Part 11.
- **eIDAS time stamps**: **Outside witness** with a qualified provider.

## Notes on the map

- The clause numbers were checked on 2026-11-09, one by one: laws and regulations (China's Cybersecurity Law and Electronic Signature Law, 21 CFR Part 11, eIDAS, 17 CFR 210.2-06, 18 U.S.C. 1520, 15 U.S.C. 7262) and NIST SP 800-63-4 against their official texts; ISO/IEC 27001 Annex A against the official table of contents of ISO/IEC 27002:2022, which uses the same numbers; GB/T 22239-2019 against a full copy of the standard (not the official SAMR listing, which could not be opened); the SOC 2 criteria only against secondary sources, because the AICPA text needs a sign-in. If your audit depends on a clause, check it against the full text you hold.
- SOC 2's numbers are from the 2017 Trust Services Criteria with revised points of focus (2022). The AICPA proposed new criteria for 2027 on 2026-10-28; this map will be checked again when they are adopted.
- ISO/IEC 27001:2022/Amd 1:2024 (climate action) does not change Annex A.
- China's Cybersecurity Law was amended with effect from 2026-01-01; the six-month log rule was Article 21 before.
- To check the record itself, run `alm audit`: it lists what it finds (unsigned changes, broken chains, gaps, receipts that do not match) and leaves the judgement to you.
