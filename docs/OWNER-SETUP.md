# Owner setup

The steps only the owner of a deployment can take: accounts, secrets and settings no builder and
no script may perform. Concrete names (secret names, host names, ids) are the owner's and live in
the owner's private notes, never in this repository. Every value that is a secret is stored from
standard input, never typed on a command line.

## 1. The Telegram bot

1. In Telegram, open `@BotFather` and send `/newbot`. Choose the display name `DeckStreak - XP and
   streaks for Anki` and the first free username of: `DeckStreakBot`, `deck_streak_bot`,
   `DeckStreak_bot`, `DeckStreakAppBot`, `DeckStreakHQBot`.
2. Set the about text (`/setabouttext`): `XP, levels, streaks and quests for your Anki reviews,
   right in Telegram. Not affiliated with Anki.`
3. Store the token BotFather gives you in your secret manager, from standard input. With Google
   Secret Manager, for example:

   ```sh
   read -rs BOT_TOKEN && printf '%s' "$BOT_TOKEN" | \
     gcloud secrets create <your-bot-token-secret> --data-file=- --project=<your-project>
   unset BOT_TOKEN
   ```

   Record the secret's name in your private notes. Never paste the token into a file, a chat or
   an issue.

## 2. The owner's identity

DeckStreak answers one Telegram user. Store your numeric Telegram user id as a secret the same
way (the bot's first `/start` from you logs a hint to look it up; it is never written to a file).

## 3. The Anki sync server

DeckStreak reads your own Anki sync server; it never logs in to AnkiWeb, and it never uploads to
your server (ADR-037). Set the server's URL as the setting `DECKSTREAK_SYNC_ENDPOINT` in the host's
private configuration, never in this repository. Store the account's username and password as the
secrets `anki-sync-username` and `anki-sync-password`; they reach the service as credentials at
start (ADR-038).

The transport is your decision. Serve the sync server over HTTPS if you can: with a plain `http:`
URL the credentials cross the network in the clear, and the service says so once at start, in a
WARN line that names the setting and never its value.

## 4. The AI agent's device key (optional; only when you enable the AI route)

DeckStreak runs whole without an AI route: the readings say they are not enabled, and nothing alerts
(ADR-054). To enable the subscription route, create the agent's own device key, add it to the proxy's
device roster (which restarts the proxy), and store it as a secret. It reaches the agent's unit
through the credential socket at start and is never written to disk (ADR-038).

## 5. Hosting and HTTPS

1. Choose the Mini App's host name: a subdomain of a domain you own, or a wildcard-DNS name of the
   host's static address (it works at no cost). The public landing page needs a domain you own.
2. If you own a domain, point an `A` record for the apex and the Mini App's subdomain at the host's
   static address.
3. The deployment adds one Caddy site block (`deploy/caddy/deck-streak.caddy`) with your host names
   filled in from your private configuration. Approve that change before it is made: Caddy may be
   shared with other services on the host.

## 6. The Mini App

1. Once HTTPS works, send `@BotFather` the command `/newapp`, choose your bot, set the title
   `DeckStreak` and the short name `deckstreak`, and give the Mini App's HTTPS URL.
2. Enable it as the bot's Main Mini App (`/mybots`, then Bot Settings), so the bot's profile shows
   "Launch app".
3. Set the menu button to open the Mini App.

## 7. The optional doomscroll tripwire

The tripwire listens through a second bot that posts into a private channel. Make the new
DeckStreak bot an administrator of that channel, and bind the channel in DeckStreak's settings;
otherwise the tripwire and every penalty that depends on it stay off.

## 8. The repository

After you have reviewed the public-readiness report and made the repository public, run once:

```sh
bash scripts/github-setup.sh
```

It sets the default branch, applies the rulesets for `main`, `dev` and release tags, and turns on
secret scanning, push protection, Dependabot alerts and private vulnerability reporting.
Dependabot's automated security fixes stay off, because they would open against `main` (ADR-036);
each alert is triaged, and a fixable one becomes a pull request into `dev`.

## 9. Each deploy

Deploys run from your machine, from a tag on `main`: `bash deploy/deploy.sh vX.Y.Z` (RELEASING.md).
The first deploy, and any change to a unit, Caddy, the firewall or a bucket, waits for your go.
