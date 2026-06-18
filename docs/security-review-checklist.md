# Security Review Checklist

This checklist is the release gate for sensitive Meetily automation. The detailed checklist is maintained by CHA-1728 and must be completed or explicitly waived before any feature covered by [Privacy, Consent, and Access Controls](privacy-consent-access-controls.md) ships.

Minimum gate:

* The feature links to the privacy policy and names its default state, consent scope, revocation path, and audit surface.
* Permission, local file access, Apple Events, local server exposure, calendar sync, export destination, screenshot, and provider behavior are reviewed when applicable.
* Manual QA covers denied permissions, revoked permissions, deletion, and failed external writes when applicable.
* macOS development and release build checks are completed when the feature changes packaged behavior.
