# Remote access for companions, and the pairing email

**Evidence collected 2026-10-07** for task `A04-01` and [decision 0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md) open questions 4 and 5. Public sources only: vendors' own documentation and pricing pages, plus a few secondary articles where a vendor page could not be read (marked *secondary*). No account was created, nothing was installed or set up, and no email was sent. Free tiers change often; re-check a figure before relying on it.

## In plain words

The phone and browser companions must reach the FlipHopper desktop app when the phone is **away from home** (R78), at no or very low cost. The desktop sits behind a home router, so something has to carry the traffic in. There are three kinds of answer:

- **A private network** (Tailscale): the phone and the computer join the user's own private network. Nothing is exposed on the internet. The phone needs the Tailscale app.
- **A tunnel** (Tailscale Funnel, Cloudflare Tunnel, ngrok): the desktop gets a public web address that leads to it. The phone needs nothing extra, but the address is reachable by anyone, so FlipHopper's pairing has to be the lock.
- **A relay FlipHopper runs** (for example on Cloudflare Workers): the desktop and the companion both connect out to FlipHopper's small server, which passes messages between them. Easiest for users, but FlipHopper pays for it as use grows.

The pairing code (R79, R84) needs an email-sending service. Each service needs a **verified sending domain** to reach arbitrary addresses, so the realistic choice is one small FlipHopper-run sender, not one per user.

**Recommendation** (for the owner to decide; details below):
1. **Free path: the user's own free Tailscale account.** Private network by default (most secure, nothing public); Funnel when a companion device cannot install Tailscale.
2. **Later, a paid option: a FlipHopper relay** with end-to-end encryption, as 0012 already considers ("paid hosted relay").
3. **Pairing email: one FlipHopper-run sender** — a tiny helper on Cloudflare Workers' free tier calling Resend's free tier from a FlipHopper domain, with the key stored only as the helper's secret.

## What a solution must satisfy

| Need | Source |
|---|---|
| Free or very cheap for the user and for FlipHopper; free users should cost FlipHopper (almost) nothing | R78, [0012](../decisions/0012-free-local-app-paid-cloud.md) |
| Companion actions are requests the desktop checks and records | R77 |
| Pairing by email address and a long code the desktop sends; codes single-use, short-lived, revocable | R79, R80 |
| Traffic encrypted | 0009 proposed pairing details |
| The email service's key is never committed or embedded in the app | R84 |
| Works on macOS and Windows | R81 |

## Options for reaching the desktop

| | A. Tailscale private network | B. Tailscale Funnel | C. Cloudflare Tunnel (named) | D. Cloudflare Quick Tunnel | E. ngrok free | F. FlipHopper relay (Cloudflare Workers + Durable Objects) |
|---|---|---|---|---|---|---|
| **Who runs it** | user's own account | user's own account | user's own account | nobody (no account) | user's own account | FlipHopper |
| **Cost** | free Personal plan: up to 6 users, unlimited user devices [1] | included on all plans [2] | free on every plan [4, 5 *secondary*] | free [3] | free: 1 GB/month, 20k HTTP requests/month, 3 endpoints; paid from $10/month [6] | free tier: 100,000 requests/day [7], Durable Objects on the free plan (SQLite backend) [8]; paid from $5/month plus use [7] |
| **Needs from the user** | Tailscale app on the computer **and** on every companion device; one sign-in | Tailscale on the computer only; MagicDNS and HTTPS certificates on, a Funnel policy line [2] | a Cloudflare account and **a domain whose DNS is on Cloudflare** (about the price of a domain a year) [5 *secondary*]; `cloudflared` installed [4] | `cloudflared` installed | an ngrok account and its agent | nothing |
| **Address** | private name, reachable only inside the user's network | public `*.ts.net` HTTPS address, ports 443 / 8443 / 10000 only [2] | the user's own hostname | random `*.trycloudflare.com`, **changes every start** [3] | assigned dev domain; **a warning page** in front of browser visits on the free plan [6] | FlipHopper's address |
| **Security** | WireGuard encryption end to end; nothing exposed | TLS to a public address: anyone can knock, pairing must hold | TLS through Cloudflare; Cloudflare Access can add a login | TLS; optional email PIN gate (`--allowed-mail`) [3] | TLS; public | must add end-to-end encryption so the relay cannot read projects |
| **Limits** | — | "non-configurable bandwidth" limits [2] | — | **testing only**: no uptime guarantee, 200 in-flight requests, no Server-Sent Events [3] | monthly caps above | FlipHopper pays beyond the free tier; must limit abuse |
| **Fit with R77–R80** | good: pairing still names each device; transport already private | good, if pairing and rate limits are strict | good, for users who own a domain | poor: address changes, so pairing breaks on each restart | poor: warning page and monthly caps | good; the natural "paid hosted relay" of 0012 |

### Notes on each

- **A — Tailscale private network.** The strongest privacy: the desktop's local server never faces the internet. The cost is one install and sign-in on each phone. FlipHopper would only need to show "connect your phone to Tailscale" and the private address; pairing (R79) still decides which device may act.
- **B — Tailscale Funnel.** Same account as A, but the phone needs nothing. FlipHopper's server becomes public, so pairing codes, device removal and request rate limits are the only lock; fine if done well, but a stronger demand on our security work.
- **C — Cloudflare Tunnel.** Free and robust, but the domain requirement puts it out of reach of most users. Worth documenting as an advanced choice.
- **D — Quick Tunnel.** Useful for our own testing; Cloudflare itself says it is for testing and development only [3].
- **E — ngrok.** The free plan's warning page and monthly caps make it a poor default; $10/month is not "very cheap" for a free app's users.
- **F — FlipHopper relay.** The best experience (no setup at all), and the only option where FlipHopper carries a running cost per user. The free tier covers early, small use; beyond it, it matches 0012's idea of a paid feature. Not for the first companions release.

## Options for the pairing email

| | 1. FlipHopper-run sender: Worker + Resend | 2. FlipHopper-run sender: Worker + Brevo | 3. User's own email-service account | 4. User's own Gmail through SMTP |
|---|---|---|---|---|
| **Free tier** | 3,000 emails/month, 100/day [9] | 300 emails/day [11 *secondary*] | as 1 or 2, per user | Gmail's own sending limits |
| **Domain** | **a verified domain is required** to send at all [10] | domain authentication expected; without it Brevo rewrites the sender to `@brevosend.com` [11 *secondary*] | each user would need their own domain | none: sends as the user |
| **Where the key lives** | as a secret of the Worker only; never in the app (R84) | the same | on each user's computer | an **app password** on each user's computer; needs 2-Step Verification; not for work or school accounts or Advanced Protection; Google discourages app passwords [12] |
| **Cost to FlipHopper** | a domain; free tiers until about 100 pairings a day | the same | none | none |
| **Fit** | good: pairing emails are rare (a few per user) | good, as a second provider | poor: too much setup for users | weak: fragile, and Google advises against it |

The helper (a few dozen lines on Cloudflare Workers' free tier, [7]) receives "send this code to this address" from a desktop app, applies a rate limit per desktop and per address, and calls the email service with its secret key. Abuse control matters because anyone could call it; the rate limit and a short, fixed email text keep it from being useful to spammers.

## Recommendation for the owner

1. **Iteration 9 (at home)** needs none of this: companions reach the desktop on the home network.
2. **Iteration 10 (away from home), free path:** support the user's own **Tailscale** account. Private network first (option A); Funnel (B) as the fallback for devices that cannot install Tailscale. Document Cloudflare Tunnel (C) for users who own a domain.
3. **Later, paid option:** a **FlipHopper relay** (F) with end-to-end encryption, as the hosted, no-setup choice. This answers 0012's open question 2 with "both".
4. **Pairing email:** one **FlipHopper-run helper** on Cloudflare Workers calling **Resend** (option 1), with Brevo (2) as a backup provider. It needs a FlipHopper domain and two accounts; each is an external operation for the owner to approve.

**What this does not settle** (owner decisions): whether FlipHopper buys a domain now; who holds the Cloudflare and Resend accounts; whether Funnel's public exposure is acceptable without the relay's end-to-end encryption.

## Sources (read 2026-10-07)

1. Tailscale pricing — <https://tailscale.com/pricing>
2. Tailscale Funnel — <https://tailscale.com/kb/1223/funnel> ("available for all plans"; ports, requirements, bandwidth limits)
3. Cloudflare Quick Tunnels — <https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/do-more-with-tunnels/trycloudflare/>
4. Cloudflare Tunnel overview — <https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/>
5. *Secondary:* Flavio Copes, "The complete guide to Cloudflare Tunnel" — <https://flaviocopes.com/cloudflare-tunnel/> (free on every plan; domain on Cloudflare required for a named tunnel)
6. ngrok pricing — <https://ngrok.com/pricing>
7. Cloudflare Workers pricing — <https://developers.cloudflare.com/workers/platform/pricing/>
8. Cloudflare Durable Objects pricing — <https://developers.cloudflare.com/durable-objects/platform/pricing/>
9. Resend pricing — <https://resend.com/pricing>
10. Resend domains — <https://resend.com/docs/dashboard/domains/introduction> ("You must add and verify at least one domain to send emails")
11. *Secondary:* Brevo free plan and domain authentication — <https://www.brevo.com/blog/free-smtp-servers/>, <https://www.captaindns.com/en/blog/brevo-transactional-email-technical-guide> (Brevo's own help pages refused automated reading)
12. Google, "Sign in with app passwords" — <https://support.google.com/accounts/answer/185833>
