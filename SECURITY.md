# Security Policy

## Reporting a vulnerability

Please report suspected vulnerabilities privately through GitHub Security Advisories at https://github.com/excelano/portal-theme/security/advisories/new. If you would rather not use GitHub, email david.anderson@excelano.com instead. I aim to respond within seven days.

Please do not open public issues for security problems.

## Supported versions

The latest release receives security fixes. Older versions are not supported.

## What portal-theme can access

portal-theme is a library, not a service. On Linux it connects to the session D-Bus and asks the XDG desktop portal for one setting, the appearance namespace's `color-scheme`, then listens for that setting changing. It reads nothing else from the bus, opens no files, makes no network calls, runs no subprocesses, and writes nothing anywhere: the one thing it does with an answer is set a theme preference on the egui context it was given. On Windows and macOS it does nothing at all.

The crate's own source is `forbid(unsafe_code)`. It compiles no C and runs no build script. Its D-Bus dependency is platform-gated to Linux and used through safe functions only.

## What portal-theme stores

Nothing. No telemetry, no analytics, no caching, no files. The watching thread holds a D-Bus connection and a clone of the egui context for as long as the program runs, and nothing survives the process.
