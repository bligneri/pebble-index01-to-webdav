# Changelog

## [Unreleased]

## [0.3.0] - 2026-10-27

- journal.mode new, now the default, writes one journal entry per dictation dated at its recording time instead of appending lines to one entry per day, so each dictation is its own card with its own preview in jtx Board.

## [0.2.0] - 2026-10-27

- Dictations are written to CalDAV: notes and todos as new entries named after the delivery ID (a redelivery is not duplicated), journals appended to one entry per day and domain with ETag checks, so dictations land in jtx Board.
- The CalDAV user can come from CALDAV_USER in the secret, with caldav.user in the config as fallback, so the user name can live with the password.
- One log line per request (trigger, delivery ID, status, reason), so every Pebble delivery can be traced, test events included.
- Without CALDAV_APP_PASSWORD, dictations get 503 instead of 501, so the missing secret is named in the Pebble app.

## [0.1.1] - 2026-10-27

- CALDAV_APP_PASSWORD is optional, so the webhook starts and answers the Pebble test event before CalDAV is configured, with the two secrets on separate lifecycles.
- The image carries the org.opencontainers.image.source label and the release checks it can be pulled anonymously, so the ghcr.io package is public and a private one fails the release instead of the deployment.
