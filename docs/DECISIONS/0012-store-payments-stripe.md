# 0012. Store payments: Stripe Checkout + Stripe Tax, real-money prices, no virtual currency

- Status: accepted (spec, 2026-11-10); merchant-of-record alternative is an open question for the user

## Context

EU consumer law for digital content, VAT OSS, minors, and Mojang's usage guidelines (LEGAL). Horizon sells through
Paddle as merchant of record (`horizon/docs/cosmetics.md:386-388,476-480`).

## Decision

- Stripe Checkout opened in the **system browser** (no card ever typed in the launcher), Stripe Tax for VAT (EU OSS
  registration by the company), invoices/receipts by Stripe, webhooks → idempotent ownership grants.
- Prices in **real currency** (EUR first; GBP, CHF, USD later), final price incl. VAT shown before checkout. **No
  premium/virtual currency, no bundles of points, no random items**, no countdowns or fake scarcity.
- Checkout requires the two ticked statements of LEGAL §3 (immediate delivery + loss of withdrawal right), recorded with
  timestamp and wording version.
- Minors: parental controls (LEGAL §4) checked server-side before a Checkout Session is created.
- Refund (voluntary within 14 days if the item was never equipped, or any legal refund, or chargeback) ⇒ ownership
  revoked.

## Consequences

The company is the seller: it must register for VAT OSS and publish legal notices (TODO_HUMAN).

## Alternatives

Paddle / Lemon Squeezy as merchant of record (they handle VAT and act as seller; higher fees ~5 %; simpler for a small
team). Open question for the user.
