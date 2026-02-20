# Changelog

All notable changes to this project will be documented in this file.

## [0.3.4] - 2026-02-20

### Bug Fixes

- Actions hashes

## [0.3.3] - 2026-02-20

### Bug Fixes

- Deserialization error

### Chore

- Add MIT License to the project
- Update taiki-e/install-action digest to 2723513
- Update rust crate anyhow to v1.0.102
- Update actions/checkout action to v6
- Update actions/setup-node action to v6
- Update base image
- Update taiki-e/install-action digest to 71b4839
- Update rust crate sqlx to 0.8.0 [security]
- Bump sqlx from ^0.7 to ^0.8

## [0.3.2] - 2026-02-19

### Bug Fixes

- Set required fields in Cargo.toml

## [0.3.1] - 2026-02-19

### Bug Fixes

- Publish releases

## [0.3.0] - 2026-02-19

### Bug Fixes

- Index wrongly identify source_url as unique
- Date encoding

### Chore

- Improve dev compile time
- Bump
- Optimize compilation
- Bump development environment
- Add prek
- Add agent guidelines
- Save query preparation

### Documentation

- Update to reflect new module structure
- Add project description
- Update agent guidelines

### Features

- Save last sync time to sync_state table

### Refactoring

- Claude full review
- Split main.rs into separate files

## [0.2.0] - 2024-03-04

### Features

- Track changes history

## [0.1.1] - 2024-03-03

### Bug Fixes

- Bump to Bookworm due to Glibc requirements

## [0.1.0] - 2024-03-03

### Chore

- Add Dockerfile

### Features

- Initial commit
- Serialize API response
- Save to database
- Save function and on conflict do nothing
- Take Retry-After header into account
- Deserialize word_count as 0 if null
- Deserialize title as "Untitled" if null

[0.3.4]: https://github.com/NSenaud/readwise-reader-sync/compare/v0.3.3...0.3.4
[0.3.3]: https://github.com/NSenaud/readwise-reader-sync/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/NSenaud/readwise-reader-sync/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/NSenaud/readwise-reader-sync/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/NSenaud/readwise-reader-sync/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/NSenaud/readwise-reader-sync/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/NSenaud/readwise-reader-sync/compare/v0.1.0...v0.1.1

