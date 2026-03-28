Analyze commits since the last release using git-cliff, display the proposed release notes and version bump, then execute the release.

## Steps

1. **Get the current version** by reading `version` from `Cargo.toml`.

2. **Generate release preview** by running:
   ```
   git-cliff --bump --unreleased
   ```
   Capture the output. If it is empty or contains no version header, stop and tell the user there is nothing to release (no releasable commits since the last tag).

3. **Determine the bump type** by parsing the new version from the first `## [X.Y.Z]` line in git-cliff's output and comparing to the current version:
   - Major component changed → `major`
   - Minor component changed → `minor`
   - Patch component changed → `patch`

4. **Display a summary** showing:
   - Current version → New version
   - Bump type and the commit types that drove it (e.g. "feat commits present → minor bump")
   - The full generated release notes from git-cliff's output

5. **Execute the release**:
   ```
   cargo release <type> --execute --no-push
   ```
   where `<type>` is the bump type determined in step 3. The `--no-push` flag is required because this repo uses jj; cargo-release cannot push directly.

   cargo-release will:
   - Bump the version in `Cargo.toml`
   - Run the `pre-release-hook` (git-cliff writes the full `CHANGELOG.md`)
   - Commit with message `chore(release): v<new-version>`
   - Create tag `v<new-version>`

6. **Push via jj** (this repo uses jj, not plain git):
   ```
   jj bookmark set main --revision <release-commit-hash>
   jj git push
   git push origin v<new-version>
   ```
   - `jj bookmark set main` moves the main bookmark to the release commit that cargo-release just made
   - `jj git push` pushes the main branch to origin
   - `git push origin v<new-version>` pushes the tag (jj does not push tags)

7. **Report completion** with the new version and the tag that was pushed. Remind the user that the GitHub Actions release workflow will now build and publish the binaries.
