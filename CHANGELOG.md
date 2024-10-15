# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking

### Added

### Changed

### Fixed

### Removed

- see 'Breaking' section above

[Unreleased]: https://github.com/jnioche/carbonintensity-api/compare/0.3.0...HEAD

## [0.3.0] - 2024-10-15

### Breaking

- variants added to public enum `ApiError`
- `get_intensity_postcode()`/`get_intensity_region()` removed, use `get_intensity()`
  instead. This takes a `Target` (e.g. national/region/postcode)
- `get_intensities_postcode()`/`get_intensities_region()` removed, use `get_intensities()`
  instead. This takes a `Target` (e.g. national/region/postcode)
- `get_intensities()` signature changed, instead of taking just a URL now
  takes a `Target` (e.g. national/region/postcode) and the start/end date

### Added

- ability to retrieve national data ([issue #9](https://github.com/jnioche/carbonintensity-api/issues/9))
- added `Cargo.lock` to improve compilation reproducibility of the binary
  ([PR #21](https://github.com/jnioche/carbonintensity-api/pull/21))
- added `rust-version` to `Cargo.toml` to make it explicit which version
  of Rust compiles the crate ([commit](https://github.com/jnioche/carbonintensity-api/commit/f92d03673181f3be8f0954724b60dd38b1808145))
- run clippy in CI ([PR #22](https://github.com/jnioche/carbonintensity-api/pull/22))
- check for semver violations in CI ([PR #28](https://github.com/jnioche/carbonintensity-api/pull/28))
- more tests
- added a changelog

### Changed

- make requests to Carbon Intensity API concurrently ([issue #7](https://github.com/jnioche/carbonintensity-api/issues/7))

### Fixed

- prevent start date to be before first data available ([issue #15](https://github.com/jnioche/carbonintensity-api/issues/15))
- Prevent CLI to panic when the pipe is broken ([issue #5](https://github.com/jnioche/carbonintensity-api/issues/5))

### Removed

- see 'Breaking' section above

[0.3.0]: https://github.com/jnioche/carbonintensity-api/compare/0.2.0...0.3.0

## [0.2.0] - 2023-12-04

### Added

- improvements to output legibility ([issue #2](https://github.com/jnioche/carbonintensity-api/issues/2))
- run tests in CI

### Fixed

- handle date ranges of more than 13 days ([issue #3](https://github.com/jnioche/carbonintensity-api/issues/3))
- accepts postcode's outward codes which are 4 characters long ([issue #1](https://github.com/jnioche/carbonintensity-api/issues/1))

[0.2.0]: https://github.com/jnioche/carbonintensity-api/compare/0.1.0...0.2.0

## [0.1.0] - 2023-11-21

### Added

- Initial release

[0.1.0]: https://github.com/jnioche/carbonintensity-api/compare/99759e1a889065d473bacd2958692ab8bbeb3ae0...0.1.0
