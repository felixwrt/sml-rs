# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Embedded examples for the ESP32-C3 (#37)

## [0.4.0] - 2024-06-04

### Added

- `SmlReader` type that provides an API for reading, decoding and parsing SML messages from several input sources.
- CI: Checks for SemVer violations
- Implement `std::error::Error` for all error types

### Changed

- CI script cleanup
- Changed `tests/libsml-testing` from submodule to subtree
- Updated maintenance badge for 2024
- Updated hex-literal to 0.4.1

## 0.3.0 - 2023-03-24

<!-- next-url -->
[Unreleased]: https://github.com/felixwrt/sml-rs/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/felixwrt/sml-rs/compare/v0.3.0...v0.4.0