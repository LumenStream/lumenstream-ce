# LumenStream CE

## What This Is

LumenStream CE is a Jellyfin-like community edition media service built for self-hosted users who want Jellyfin/Emby-compatible playback, library management, metadata scraping, and an integrated web/admin experience. The CE repository is the shared product foundation, while `commercial` extends it with business capabilities such as billing and subscription features. The current product baseline already includes core playback and streaming; the next stage is to evolve scraping and agent-assisted request workflows into a faster, higher-quality, more usable end-user experience.

## Core Value

Ordinary users can express what they want through the product and reliably get the right media result with minimal friction.

## Requirements

### Validated

- ✓ Jellyfin / Emby-compatible media server APIs, login flows, and client-facing playback endpoints are already implemented — existing
- ✓ Base streaming and playback routing are already working, including direct play and LumenBackend-assisted stream routing — existing
- ✓ Media library management, scan, search, and metadata ingestion already exist in CE — existing
- ✓ CE already ships built-in metadata scraping/providers for TMDB, TVDB, and Bangumi — existing
- ✓ CE already includes a web frontend and admin surface for users, sessions, media, and request workflows — existing
- ✓ Agent request capability already belongs to CE rather than commercial-only overlays — existing

### Active

- [ ] Improve scraper hit rate so media and metadata resolution succeed more often on the first attempt
- [ ] Improve scraper metadata completeness so matched items have richer, more trustworthy details
- [ ] Improve multi-turn agent request flows so users can refine intent and still reach a successful result quickly
- [ ] Increase agent response speed and answer quality while keeping the interaction flow smooth for ordinary users
- [ ] Redesign admin views that currently expose raw JSON so administrators get concise, visual, readable management surfaces

### Out of Scope

- CE billing, wallet, subscription, and other commercial monetization features — these belong in `commercial`, not this repository's current scope
- Reworking the already-finished base streaming foundation from scratch — the current milestone builds on top of the existing playback baseline
- Isolating core agent capability out of CE — agent functionality is intended to remain available in the community edition

## Context

This is a brownfield Rust workspace with established Jellyfin/Emby compatibility, playback, scraping, and web/admin foundations. The backend is built around Actix Web, SQLx/Postgres, Meilisearch, and a multi-crate Rust workspace; the frontend is Astro with React islands and Tailwind. Existing CE boundaries already document that agent workflows, scraping, playback routing, and compatibility work stay in CE, while commercial monetization and related advanced business features stay downstream.

The current project phase starts from a meaningful shipped baseline: core playback/streaming is already considered done, so this initialization is not about proving basic media serving. Instead, the immediate product problem is quality of outcomes: scraping needs to match more accurately, return fuller metadata, and do so fast enough to support better downstream agent behavior. On top of that, the user-facing request flow needs to feel more fluid, with multi-turn agent interactions that help users reach the right media result rather than forcing rigid one-shot requests.

There is also an admin UX gap. Some administrator surfaces still expose raw JSON too directly, which is functionally useful but not acceptable as the intended long-term experience. This milestone should treat admin visualization as a product concern, not just a debugging convenience.

## Constraints

- **Compatibility**: Preserve Jellyfin / Emby-compatible behavior — existing clients and playback flows must not regress
- **Edition Boundary**: Keep agent capability in CE while leaving billing and monetization in `commercial` — the repo split is intentional
- **Brownfield Architecture**: Build on the existing Rust + Astro workspace rather than introducing a platform rewrite — the codebase already has working foundations and shared patterns
- **Product Priority**: Scraper quality improvements are ordered as hit rate first, metadata completeness second, speed third — optimization work should follow that sequence
- **User Experience**: Agent flows must work for ordinary users, not just admins or power users — the product should reduce friction rather than expose internals
- **Admin UI Quality**: Admin interfaces should favor concise, visual presentation over raw JSON dumps — readability and aesthetics are part of the requirement

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Treat this initialization as brownfield CE planning | The repository already contains validated playback, scraping, compatibility, frontend, and agent foundations | ✓ Good |
| Keep agent capability inside CE scope | You explicitly want agent functionality available in community edition; commercial stays focused on billing-style extensions | — Pending |
| Prioritize scraper hit rate before completeness and speed | Better agent and user outcomes depend first on finding the right media target | — Pending |
| Focus the next milestone on multi-turn agent UX and admin visualization | Product value now depends on request quality and usability, not just backend capability breadth | — Pending |

---
*Last updated: 2026-03-20 after initialization*
