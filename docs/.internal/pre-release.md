# Pre-Release Checklist

Use this checklist before tagging a new release to ensure user-visible artifacts and docs are up to date.

1. `just ci` — run full CI-equivalent checks (build, tests, clippy, fmt).
2. `just docs-generate-tags` — regenerate tag-domain documentation with the latest data (requires `oxidex-tags/examples/render_domain`).
3. Review `docs/tag-domains/*.md` diffs for accuracy.
4. Update `CHANGELOG.md` with highlights for the release.
5. Ensure packaging manifests (Homebrew/RPM/Deb) reflect new version numbers.
6. Run `just release-check` to verify release-specific automation succeeds.
