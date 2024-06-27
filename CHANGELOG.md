# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Embedded examples for the ESP32-C3 (#37)
- Added `serde` feature which implements `Serialize` and `Deserialize` on most error types (#33)
- **BREAKING:** The `DecodeErr::InvalidMessage` variant has a new boolean member `invalid_padding_bytes` (#43)

### Changed

- Decoder improvements (#43)
- **BREAKING:** Renamed `*Reader` types to `*ByteSource` (e.g. `IoReader` to `IoByteSource`) (#45)
- Refactored `ByteSourceErr` trait (#46)
- **BREAKING:** Renamed feature `embedded_hal` to `embedded-hal-02` (#47)


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