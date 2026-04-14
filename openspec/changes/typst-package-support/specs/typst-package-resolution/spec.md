## ADDED Requirements

### Requirement: Resolve Typst package imports
The system SHALL resolve Typst package imports in the format `@namespace/name:version` (e.g., `@preview/letter-pro:3.0.0`) when compiling templates. Package files SHALL be resolved from a local package cache directory.

#### Scenario: Template imports a package
- **WHEN** a template contains `#import "@preview/letter-pro:3.0.0": letter-simple`
- **AND** the package is available in the cache
- **THEN** the system SHALL resolve the import and make the package's exports available to the template

#### Scenario: Template imports multiple packages
- **WHEN** a template imports `@preview/letter-pro:3.0.0` and `@preview/tablex:0.0.8`
- **AND** both packages are available in the cache
- **THEN** the system SHALL resolve both imports independently

### Requirement: Automatic package download
The system SHALL automatically download packages from the official Typst package registry at `https://packages.typst.org/{namespace}/{name}-{version}.tar.gz` when a package is not yet in the local cache. The downloaded archive SHALL be extracted to the cache directory at `{cache_dir}/{namespace}/{name}/{version}/`.

#### Scenario: First use of a package
- **WHEN** a template imports `@preview/letter-pro:3.0.0`
- **AND** the package is not in the local cache
- **THEN** the system SHALL download the package from `https://packages.typst.org/preview/letter-pro-3.0.0.tar.gz`
- **AND** extract it to `{cache_dir}/preview/letter-pro/3.0.0/`
- **AND** resolve the import from the extracted files

#### Scenario: Package already cached
- **WHEN** a template imports `@preview/letter-pro:3.0.0`
- **AND** the package directory `{cache_dir}/preview/letter-pro/3.0.0/` already exists
- **THEN** the system SHALL resolve the import from the cached files without making any network requests

#### Scenario: Package does not exist in registry
- **WHEN** a template imports `@preview/nonexistent-package:1.0.0`
- **AND** the registry returns HTTP 404
- **THEN** the system SHALL return a render error indicating the package was not found

#### Scenario: Network error during download
- **WHEN** a template imports a package that is not cached
- **AND** the download fails due to a network error
- **THEN** the system SHALL return a render error indicating the download failure

### Requirement: Package cache configuration
The system SHALL read the package cache directory from the `TYPST_PACKAGE_CACHE` environment variable. If the variable is not set, the system SHALL use a default path of `./typst-packages`.

#### Scenario: Custom cache directory
- **WHEN** `TYPST_PACKAGE_CACHE` is set to `/var/cache/typst-packages`
- **THEN** packages SHALL be cached in `/var/cache/typst-packages/{namespace}/{name}/{version}/`

#### Scenario: Default cache directory
- **WHEN** `TYPST_PACKAGE_CACHE` is not set
- **THEN** packages SHALL be cached in `./typst-packages/{namespace}/{name}/{version}/`

### Requirement: Package dependency resolution
The system SHALL support packages that import other packages. When a package's source file imports another package, the system SHALL resolve and download the dependency package using the same mechanism.

#### Scenario: Package with transitive dependency
- **WHEN** a template imports `@preview/package-a:1.0.0`
- **AND** `package-a` internally imports `@preview/package-b:2.0.0`
- **THEN** the system SHALL also download and cache `package-b` and resolve the transitive import
