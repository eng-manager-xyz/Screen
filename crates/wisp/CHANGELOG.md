# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

This file is maintained by [release-plz](https://release-plz.dev). Do not edit
manually — write your changes with [conventional commit](https://www.conventionalcommits.org)
messages (`feat(wisp): …`, `fix(wisp): …`, `feat(wisp)!: …` for breaking
changes) and release-plz will fold them into the next Release PR.

## [Unreleased]

### Added

- Initial publish to crates.io. Pixi-shaped 2D scene graph
  (`Stage` / `Container` / `Sprite` / `Graphics` / `Text` /
  `Mesh`), filter chain (blur, drop shadow, motion blur, color
  matrix, advanced blends), mask system (rounded clip, privacy
  blur, solid redaction, spotlight, dim-outside, ellipse, freehand
  path), text rendering (atlas + cosmic-text/glyphon `FlexibleText`),
  vector primitive bridge, headless export via `RenderTexture`.
