# Changelog

## [Unreleased]

## [0.1.1] - 2026-10-27

- CALDAV_APP_PASSWORD is optional, so the webhook starts and answers the Pebble test event before CalDAV is configured, with the two secrets on separate lifecycles.
- The image carries the org.opencontainers.image.source label and the release checks it can be pulled anonymously, so the ghcr.io package is public and a private one fails the release instead of the deployment.
