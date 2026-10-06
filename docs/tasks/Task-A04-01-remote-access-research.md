# Task-A04-01-remote-access-research

- **Outcome.** The owner can choose how web and mobile companions reach the desktop app from outside the local network (R78) at no or very low cost. The choice is based on a written comparison of options: their cost, setup effort, security and limits.
- **Decisive unknowns** (worker finds out from official documentation):
  - Which options have a free or very cheap tier that fits: for example a tunnel service, a private network service, peer-to-peer connection with a small helper server, or a relay on a free hosting tier.
  - What each one requires of the desktop user (an account, an installed program, a domain).
  - How each one protects the traffic, and how it combines with pairing by email and code (R79).
  - Which **free email-sending service** to use for the pairing code (owner decided on a free service, R84): free-tier limits, sign-up needs, and where its access key should live so it is not exposed (a small fliphopper-run helper, or a key per user).
- **Files owned.** `docs/research/remote-access.md` (evidence), updates to [0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md), this card.
- **External operations.** None. Public documentation only: no accounts created, no services set up, no emails sent.
- **Done means mechanically.** The research file compares at least three options. For each it gives: the cost and the free-tier limits, the setup steps, the security model, its fit with R77–R79, and source links with the date read. It ends with a recommendation for the owner.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-06 | "find some easy task to do for the next 6 hrs … find them and do them" — the agent picked this card (public documentation only, as written) | owner (chat) |

## Notes
- 2026-10-07 — Done in the card run (`claude/a01-run`). Public documentation only; no accounts, installs or emails.

## Result (2026-10-07)

**In plain words.** [remote-access.md](../research/remote-access.md) compares six ways for companions to reach the desktop from outside (Tailscale private network, Tailscale Funnel, Cloudflare Tunnel, Cloudflare Quick Tunnel, ngrok, a FlipHopper-run relay) and four ways to send the pairing email. Recommendation for the owner: the user's own free **Tailscale** account (private network first, Funnel as fallback) as the free path; a **FlipHopper relay** with end-to-end encryption later as a paid option; and **one FlipHopper-run email helper** (Cloudflare Workers calling Resend), because every email service needs a verified domain. 0009's open questions 4 and 5 point to it; they stay the owner's to decide.

| Check | Result |
|---|---|
| At least three options, each with cost and free-tier limits, setup, security, fit with R77–R79, sources and date read | PASS: six transport options, four email options; 12 sources read 2026-10-07 (three secondary, marked) |
| Ends with a recommendation | PASS |
| `git diff --check`, relative links | PASS |
