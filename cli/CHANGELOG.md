# 0.2.3

## Changes:

* Pick TypeScript tests based on naming conventions in specs directory instead of using hard-coded list.

# 0.2.2

## Features:

* Add custom specifications CLI argument for tests and implement it for wrapper test to enable users form running tests for a specific file. Example:
  - `cargo chipmunk test wrapper -s spec/build/spec/session.jobs.spec.js`: Runs single specification.
  - `cargo chipmunk test wrapper -s spec/build/spec/session.jobs.spec.js -s spec/build/spec/session.search.spec.js`: Runs multiple specifications.
* Support Code sign process on MacOS.

## Fixes:

* Fix dependencies to build rust core on linting and testing of binding, wrapper and app targets.

# 0.2.1

## Fixes:

* Disable skip checks for building Chipmunk while creating releases.
* Fix potential freezes if graceful shutdown fails, by forcing the app to close after a timeout of 4 seconds.

## Changes:

* Deactivate reinstalling TypeScript in production mode after building for local builds, keeping it for creating releases only.
* Remove `--production` from `Reset-Checksum` sub-command, resetting the scores for both environments since only one can exist at a time.

# 0.2.0

## Fixes:

* Fix dependencies between Rust core and binding targets.
* Remove false positives in the checksum records when builds run between development and production simultaneously.
  This has been achieved by resetting the checksums for one environment once the other is built.
* Fix copying files between `App` and `Client` targets.
* Add missing copy command for `json.packages` after building the `App` target.

## Changes:

* Logs can no longer be printed to a file using CLI arguments. This can be easily achieved by piping the command output to a file from the shell directly.

## Features:

* Provide release sub-commands to build Chipmunk, bundle, and compress it for use within the GitHub CD pipeline.
* Provide four UI modes:
  - **Bars (default):** Displays TUI progress-bars, providing logs at the end for failed jobs only.
  - **Report:** Displays TUI progress-bars and provides the full logs once all jobs are done.
  - **Print:** Hides TUI progress-bars, printing the logs of each job once it's done without waiting for other jobs.
  - **Immediate:** Hides TUI progress-bars, printing each log immediately when produced.
* Add `fail-fast` flag to stop running jobs once one of them fails, enabled by default for the run command.
* Implement graceful shutdown to close the spawned jobs gracefully on fail-fast or when `Ctrl-C` is called.
* Add ANSI terminal colors to log reports.
* Provide documentation for all modules, generated with the command `cargo doc`.

# 0.1.0

## Features:

* Sub-commands for build, clean, lint, test, and run for Chipmunk's multiple parts.
* Resolving dependencies upfront and running tasks concurrently when possible.
* Displays the current progress via TUI progress bars.
* Provide the option to print logs after all jobs finish, with the option to write these logs to a file as well.
* Track changes in source code files by saving their checksum scores to skip jobs where no change has been detected.
* Provide CLI sub-commands to reset the checksum scores to bypass skipping the targets.
* Provide CLI sub-commands to print the dependencies in `print-dot` format for `Graphviz`.
* Provide CLI sub-command to generate shell completion for a variety of shells.
* Check build environment and provide information about it.
* Provide integration tests in a Python script, which can be used in CI checks for PRs related to this tool.