# Rustic Launcher: legal requirements (engineering view)

Owner: spec. **This is an engineering requirements document, not legal advice.** Every item marked ⚖ must be
reviewed by a lawyer before launch (TODO_HUMAN). The publishing entity ("the Company") is not decided yet; its
country determines the supervisory authority, the OSS member state and some age thresholds.

## 1. Legal notices and identity

- Legal notice page (web + launcher About): company name, form, address, registration number, VAT number, director of
  publication, host, contact email (EU e-Commerce Directive Art. 5; France LCEN art. 6 III if French).
- Required trademark disclaimer everywhere the game is mentioned (launcher About, website footer, store):
  **"NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT."** (the same line
  Horizon uses, `horizon/README.md:171`). "Minecraft" never in product, binary, crate or domain names.
- Open-source notices: Rustic is MIT OR Apache-2.0; the Rustic edition redistributes Horizon (MIT, "Copyright (c)
  2026 Ludovic Marie and the Horizon contributors") and its `THIRD_PARTY_NOTICES.md`: shown in Settings › About.

## 2. GDPR

### 2.1 Lawful bases

| Purpose | Basis (Art. 6(1)) |
|---|---|
| Rustic account, sign-in, linked MC profile, friends, presence, servers, domains verification, cosmetics ownership/equip | (b) contract |
| Purchases, invoices, tax records, VAT location evidence | (b) contract + (c) legal obligation |
| Security logs, fraud and abuse prevention, rate limiting, moderation, enforcement of the fair-play policy | (f) legitimate interest (balancing test documented) |
| Friend suggestions of players met ("Played with recently", "Seen recently", ADR 0145): co-presence in a party or on a server the user shares in their activity | (f) legitimate interest with an easy opt-out (Settings › Privacy & data, "Appear in suggestions of players I meet", on by default for adults); **never for minors** (not recorded, not suggested, cannot opt in) |
| Usage analytics, performance reports, crash reports | (a) consent, **off by default**, per category, withdrawable at any time (ADR 0017) |
| Marketing emails / newsletters | (a) consent (unticked box), unsubscribe link |
| Parental consent records | (c) / (f) |

Children: where a consent-based service is offered to a child under the national age of Art. 8 (table §4), consent
must be given/authorised by the parent; we therefore keep all consent-based processing off for those accounts unless
a verified parent enables it.

### 2.2 Data inventory

| Data | Where | Retention |
|---|---|---|
| Email, password hash (argon2id), display name, birth date (or age bracket), country, language | Postgres | Account lifetime; deleted 30 days after a deletion request |
| Microsoft account subject id + email ("Continue with Microsoft", ADR 0090) | Postgres | Until unlinked or account deletion |
| Minecraft UUID + username history | Postgres | Account lifetime |
| Microsoft / Minecraft tokens | **Only the user's OS keychain**, never on our servers | Until sign-out |
| Porkbun API keys, SSH private keys, VPS host keys | **Only the user's OS keychain / local store** | Until removed |
| Friends graph, blocks, party | Postgres | Account lifetime |
| Presence (online/in game/server address if shared) | Redis, TTL | Minutes (ephemeral) |
| Friend-suggestion sightings (ADR 0145): kind (server/party), SHA-256 of the normalised server address or of the party id (never the raw address), the user's own server name, first/last seen | Postgres `sightings` | 30 days after last seen; erased at once on opt-out or deletion request; in the export |
| Direct and party messages | Postgres | 90 days, then deleted (moderation window) |
| Reports, moderation actions, statements of reasons | Postgres | 2 years after the case closes (bans: while the ban lasts + 1 year) |
| Orders, invoices, consent to immediate delivery, VAT evidence (billing country, IP country) | Postgres + Stripe | 10 years (accounting law, e.g. FR Code de commerce L123-22) ⚖ |
| Servers, hostnames, domains, TXT tokens, route ids | Postgres/Redis | Server lifetime + 30 days |
| Security logs (IP, user agent, auth events) | Log store | 90 days; IPs truncated after 7 days |
| Telemetry (only with consent) | Self-hosted analytics (EU) | 13 months aggregated; raw 90 days |
| Crash reports (only with consent) | Self-hosted (EU) | 90 days |
| Relay connection metadata (source IP, hostname, bytes, duration) | Relay logs | 7 days (abuse/DDoS investigation) |
| Waitlist (Rustic Cloud hosting) | Postgres | Until launch + 6 months or unsubscribe |

### 2.3 Rights

- **Export** (Art. 15/20): Settings › Privacy › Export my data → JSON archive (account, profile, friends, messages,
  orders, cosmetics, servers, consents) generated asynchronously, download link valid 7 days, emailed notice.
- **Delete** (Art. 17): Settings › Privacy › Delete my account → re-authentication → 30-day grace (sign-in cancels) →
  hard delete + anonymisation of what must be kept (invoices keep billing data; moderation records keep a pseudonymous
  id). Linked MC UUID released.
- Rectification in Settings; objection/restriction by email to the DPO/contact; response within one month.
- Consent log: purpose, version of the text, timestamp, method; withdrawal as easy as giving.

### 2.4 Other GDPR duties

- Records of processing (Art. 30); **DPIA recommended** (minors at scale, communications, location-like presence) ⚖.
- Processors with DPAs: hosting provider, email provider, CDN/WAF, Stripe (payment processing; Stripe is an independent
  controller for some fraud/AML data), error tracking (self-hosted preferred). Transfers outside the EEA: EU-US Data
  Privacy Framework certification or SCCs.
- Breach procedure: assess, notify the authority within 72 h when required, users without undue delay if high risk.
- Privacy by design: tokens never leave the device; the cosmetics/MC link proof uses Mojang `hasJoined` so we never see
  Microsoft credentials; the Rustic edition does not send Horizon's heartbeat (ADR 0006, 0017).

### 2.5 Web cookies

Website (account page, checkout return pages, web invite fallback): only strictly necessary cookies (session,
CSRF, language) without banner; any analytics only after opt-in through a banner where **Reject** is as easy as
**Accept** (CNIL / EDPB guidance); no third-party ad cookies. The launcher itself stores local data needed for the
service (ePrivacy Art. 5(3) exemption for strictly necessary storage); telemetry identifiers only after consent.

## 3. EU consumer law for digital content (cosmetics)

### 3.1 Pre-contract information and checkout (CRD 2011/83/EU as amended by Omnibus 2019/2161)

- Show: item description, total price incl. VAT, the Company's identity and address, the functionality and
  interoperability (visible only to players using the Rustic edition; purely visual; requires a Rustic account and an
  owned Minecraft: Java Edition), withdrawal conditions, complaint handling.
- The final button reads **"Order with obligation to pay"** or an unambiguous equivalent ("Pay €4.99") (Art. 8(2)).
- **Consent to immediate execution + acknowledgement of loss of withdrawal right** (Art. 16(m)), two separate,
  unticked checkboxes, both required, before the payment page:

  > ☐ I ask for the item to be added to my account immediately, before the 14-day withdrawal period ends.
  >
  > ☐ I understand that I lose my right of withdrawal once the item has been added to my account.

  French: « ☐ Je demande que l'objet soit ajouté à mon compte immédiatement, avant la fin du délai de rétractation de
  14 jours. » « ☐ Je reconnais perdre mon droit de rétractation dès que l'objet a été ajouté à mon compte. »

- **Confirmation on a durable medium** (Art. 8(7)): the receipt email includes the order, price, VAT, and the
  recorded consent + acknowledgement (text version, timestamp).
- No pre-ticked boxes, no hidden costs, no drip pricing; discounts show the lowest price of the previous 30 days
  (Price Indication Directive, Omnibus).

### 3.2 Refunds and revocation

- Statutory: Digital Content Directive 2019/770 remedies when an item does not conform (doesn't render, wrong item):
  fix or refund. On refund, **the item is revoked** (DCD Art. 16(5) allows preventing further use).
- Voluntary goodwill policy (decision): refund on request within 14 days **if the item was never equipped**; refund ⇒
  revocation. Chargeback ⇒ revocation + purchases paused pending review.
- Gifts: refund goes to the payer; the item is revoked from the recipient (stated at gifting time).
- Account deletion does not refund owned items (stated in Terms).

### 3.3 Invoices and VAT

- Stripe Checkout + **Stripe Tax** compute VAT at the customer's location (B2C digital services, place of supply =
  customer); two non-contradictory location evidences (billing country, IP/country of payment card) stored.
- The Company registers for the **EU VAT One-Stop Shop (OSS)** in its member state (below €10 000 EU-wide cross-border
  sales the home VAT may apply, OSS still simplest) ⚖. UK, Switzerland, Norway and US states have their own registration
  rules: **launch EU-only first** (decision), then extend with registrations or switch to a merchant of record
  (ADR 0012 open question).
- Receipts/invoices issued by Stripe with sequential numbering, VAT rate and amount; B2B reverse charge not needed
  (consumer product) but VAT id field allowed.

### 3.4 Fair commercial practices

- No paid random items, loot boxes or premium currency (ADR 0012; aligned with the CPC Network key principles on
  in-game virtual currencies, 2024, and the Unfair Commercial Practices Directive).
- **No direct exhortation to children to buy** (UCPD Annex I, No. 28): no "Buy now!" copy, no pressure, no countdowns,
  no "limited" wording, no buyer counts; the Store is never advertised by badges or pop-ups (same rules as Horizon:
  `horizon/docs/cosmetics.md:450-453`).

## 4. Minors

### 4.1 Ages (⚖ to confirm per launch country)

| Country | Digital consent age (GDPR Art. 8) |
|---|---|
| Belgium, Denmark, Estonia, Finland, Latvia, Malta, Portugal, Sweden, Norway, UK (UK GDPR) | 13 |
| Austria, Bulgaria, Cyprus, Italy, Lithuania, Spain | 14 |
| France, Czechia, Greece, Slovenia | 15 |
| Germany, Croatia, Hungary, Ireland, Luxembourg, Netherlands, Poland, Romania, Slovakia | 16 |

Rules (decision):
- **Minimum age for a Rustic account: 13** everywhere (younger: refused; the game can still be played with the Microsoft
  account alone, without social/store features). Birth date asked at sign-up, neutral age screen (no hint of the
  threshold), re-asking blocked for 24 h.
- From 13 to the national age above: account allowed; consent-based processing (telemetry, marketing) stays off; **chat
  (DMs, party chat) requires parental consent** and is friends-only; presence friends-only; public server exposure off
  by default.
- Under 18 (whatever the guardian settings): never recorded for nor shown in **friend suggestions of players met**
  (ADR 0145); the "Appear in suggestions" setting is forced off. A minor's linked Minecraft name is not shown when an
  adult looks them up by Rustic name.
- Under 18: **purchases** are subject to parental controls: default "Ask a parent" (request email to the linked parent,
  approved on the web account page); a parent can allow purchases up to a **monthly spending limit** (default €0, set by
  the parent) ⚖ (contract capacity of minors varies).
- Microsoft child accounts: XSTS error 2148916238 means the child's Microsoft account needs family settings; the
  launcher explains it.

### 4.2 Moderation, reporting, DSA

- Report button on every message, profile, server listing and Store gift; categories: harassment, hate, sexual content
  involving minors, grooming, spam/scam, cheating, impersonation, other.
- Block user (hides presence, messages, invites both ways).
- Queue with SLAs (grooming/CSAM: immediate suspension + preservation + report to law enforcement / national hotline
  (INHOPE member)); others within 72 h.
- Account restrictions come with a **statement of reasons** and an appeal route (DSA Art. 17, 20 for platforms ⚖);
  single point of contact for authorities and users (DSA Art. 11-12); **no profiling-based advertising to minors**
  (DSA Art. 28) — we have no ads at all; follow the Commission's 2025 Art. 28 guidelines (age assurance proportionate,
  private-by-default settings for minors).
- Chat filter (word lists per language) + rate limits; links in DMs disabled for minors.

## 5. Platform terms we depend on

- **Microsoft / Mojang**: the Minecraft EULA and Usage Guidelines ⚖ (re-read the current official page before launch):
  owning the game is required (owner gate, ADR 0009); no Mojang assets distributed (game files prepared from the
  player's own copy, as Horizon does); no use of "Minecraft" in names; disclaimer §1; anything sold must not affect
  gameplay; **no capes or cape-like items** (capes are Mojang's; Horizon applies the same rule,
  `horizon/docs/cosmetics.md:17`); our own Azure app + Mojang App ID review approval (TODO_HUMAN); Xbox/Microsoft
  identity terms (no storage of credentials). Servers created by users: each server owner accepts the Minecraft EULA
  (`horizon-server init --eula`), shown as their own checkbox.
- **Modrinth**: API use with a unique, identifying **User-Agent** (`rustic-launcher/<version> (contact@<domain>)`),
  respect rate limits (300 requests/min per IP as documented ⚖ re-check), download files from Modrinth's CDN rather
  than rehosting, show project attribution and links, respect each project's licence.
- **CurseForge** (later): API key under the CurseForge third-party API terms ⚖; files whose authors disabled third-party
  distribution (`allowModDistribution: false`) must not be downloaded by the launcher (link to the website instead);
  attribution required.
- **Horizon registry**: MIT code; content under each project's SPDX licence; ratings API is Horizon's (read-only use).
- **Horizon project**: MIT allows redistribution of the modified client (Rustic edition) with the notice; we must not
  present it as an official Horizon build, nor use Horizon's MSA client id (`horizon/docs/forking.md`).
- **Native mods built on the player's PC** (ADR 0157): the launcher downloads, never redistributes, the tools it
  builds with: Rust (rustup-init and the toolchain from static.rust-lang.org, MIT/Apache-2.0), WinLibs' GCC/binutils
  on Windows (GPL with the GCC runtime exception, from its GitHub release, pinned by SHA-256 as Horizon's launcher
  does), horizon-mixins (MIT, built from its repository) and crates.io crates of the edition's lockfile. The edition
  source archive Rustic publishes is Horizon (MIT, with its notices) + Rustic's patches and companion (MIT OR
  Apache-2.0). `crates/rusticmc-native` follows rules of Horizon's launcher (`native_mods*.rs`, `rust_toolchain.rs`,
  MIT, the Horizon contributors), rewritten, credited in its documentation. A mod's own licence is the registry's.
- **Porkbun** (domains): the user's contract is **with Porkbun**. Rustic acts only as a technical tool using the user's
  own API keys. Purchase screen states: seller = Porkbun; Porkbun's registration agreement and terms apply; prices and
  renewals are Porkbun's; refunds/withdrawal for domains are handled by Porkbun under its terms; Rustic receives no
  payment (if an affiliate programme is ever used, it must be disclosed). The user's explicit ticks are required before
  `agreeToTerms: "yes"` is sent. WHOIS/registrant data is entered at Porkbun, not stored by Rustic.
- **Stripe**: Stripe Services Agreement, restricted businesses list (digital goods for games are allowed), Stripe Tax
  terms.

## 6. Cosmetics policy (public)

Purely visual; no gameplay effect of any kind; no capes or cape-like back items; no random draws, loot boxes, or
premium currency; prices in real money incl. VAT; no artificial scarcity; items may be visible only to players using the
Rustic edition; offensive or IP-infringing designs are refused (creators confirm originality; takedown process for
third-party IP claims).

## 7. Fair play

Public policy text: [FAIR_PLAY.md](FAIR_PLAY.md). Content catalogue enforcement in ADR 0016.

## 8. Hosting (VPS today, Rustic Cloud later)

- **VPS mode**: the user's own server and provider contract; Rustic installs software with the user's credentials.
  Terms state the user is responsible for their VPS, its security updates and their provider's acceptable-use policy;
  game files uploaded are the user's own copy.
- **Rustic Cloud hosting (future, paid)**: our own Terms of Service + acceptable-use policy; consumer law for services:
  pre-contract information, 14-day withdrawal right unless the consumer expressly requests the service to start
  during the period (then pro-rata payment, CRD Art. 14(3)) ⚖; subscription cancellation as easy as sign-up (e.g.
  FR "résiliation en trois clics", DE "Kündigungsbutton"); renewal reminders; SLA wording; DSA hosting-service duties
  (notice-and-action for illegal content on hosted servers); the server jar and resources would be obtained per owner
  as hosting providers do ⚖.

## 9. Documents to publish before launch (⚖)

Terms of Service, Privacy Policy, Cookie notice (web), Legal notice, Store terms (sale conditions incl. §3 wording),
Refund policy, Community guidelines + moderation/appeals, Fair-play policy, Parental guide, Open-source notices,
Domain purchase notice (Porkbun), later Hosting terms + AUP.
