# Releasing PBS Exporter

Release Please manages versions on `main`. Promote application changes from `develop`
without a separate release-version bump there. Merge the generated release PR into
`main` when its version, changelog and CI are ready, then sync the resulting release
metadata back to `develop` through a PR.

## Publication paths

- A normal `main` push publishes only the `main` and commit-SHA Docker tags.
- A merged Release Please PR creates a `v<version>` GitHub release. The same workflow
  calls the publisher directly, using Release Please's `release_created` and
  `tag_name` outputs. It does not depend on a bot-created tag starting another run.
- A manually pushed `v<version>` tag also invokes the publisher.
- A versioned publication checks out that exact tag, validates that Cargo.toml,
  Cargo.lock and the release manifest agree with it, builds the binary, and publishes
  versioned Docker tags, `latest`, the Linux amd64 archive and its SHA-256 checksum.
  Source revision labels identify the checked-out commit, including manual retries.

Release Please uses plain `v<version>` tags, matching the publisher. It must be able
to find the previous release tag; changing the manifest alone does not create a
release baseline. Do not move an existing release tag to a different commit.

## Recover an incomplete release

Use **Actions → Release → Run workflow** on `main` and supply the existing
`release_tag`, for example `v0.4.6`. This rebuilds that tag and replaces its uploaded
assets if needed. The validator comes from the workflow revision, so recovery also
works when the older release tag predates the validator script. The tag's version
files must still agree. This operation publishes release artifacts; use it only for
an intended release, not a feature branch.

When repairing a missing historical release, first identify the exact source commit
of the already-published image. Create a new, previously absent version tag at that
commit and let its publication finish. Then refresh the pending Release Please PR
against that baseline; do not merge a PR that repeats already-released history.

## Checks

All seven existing CI jobs run for PRs targeting either `main` or `develop`.
The Check job also runs the release-context regression tests:

```sh
python3 -m unittest discover -s tests -p release_context_test.py
```

GitHub may require maintainer approval before running a bot-created PR's CI.
Approving that run does not approve merging the PR.
