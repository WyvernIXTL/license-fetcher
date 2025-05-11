# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## v0.8.2

## Fixed

- Some root id of cargo metadata output not being parsed correctly.

## v0.8.1

## Fixed

- Docs in docs.rs not building.

## v0.8.0

### Added

- Configuration
  - Configuration builder
  - From build environment
  - From manifest file
- Caching
- Execution without panic
- Nice error traces

### Changed

- Switched to MPL-2.0 license.
- Also fetches licenses in direct sub folders (`.*license.*` and the like).
