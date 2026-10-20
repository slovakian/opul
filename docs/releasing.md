# Releasing Opul

## Ship a version

Open **Actions → Release → Run workflow**, pick `patch`, `minor`, or `major`, and run it. From a terminal:

```bash
gh workflow run release.yml -R slovakian/opul -f bump=patch
```

The workflow takes the newest `v*` tag, increases the part you chose, and then:

1. runs the test suite;
2. builds a universal (Apple silicon and Intel) `Opul.app` and wraps it in `Opul.dmg`;
3. signs and notarizes it when a Developer ID certificate is configured (see below);
4. writes release notes from merged pull requests and commits, and an `appcast.xml` signed with the Sparkle key;
5. tags the commit and publishes a GitHub release with `Opul.dmg` and `appcast.xml`;
6. downloads both files back through the public "latest" links and checks the version, checksum, update signature, and code signature.

If any step before publishing fails, nothing is tagged or published. Fix the problem and run the workflow again.

Nothing else needs updating. The website's download button and the in-app updater both follow GitHub's latest release:

- Download: `https://github.com/slovakian/opul/releases/latest/download/Opul.dmg`
- Update feed: `https://github.com/slovakian/opul/releases/latest/download/appcast.xml`

Keep the asset names `Opul.dmg` and `appcast.xml`. Installed copies of Opul look for that feed URL. Don't mark a release as a pre-release unless it should be skipped by both the download button and the updater.

## Version numbers

Git tags are the only source of version numbers. A release build sets both `CFBundleShortVersionString` and `CFBundleVersion` to the tag's version, and Sparkle compares `CFBundleVersion`. Local builds report the most recent tag.

## The Sparkle key

Updates are signed with an EdDSA key. The public half is `SPARKLE_PUBLIC_KEY` in `app.env`. The private half is the `SPARKLE_PRIVATE_KEY` Actions secret and is also stored in the maintainer's login keychain under the account `opul`. Export a backup with:

```bash
.build/artifacts/sparkle/Sparkle/bin/generate_keys --account opul -x opul-sparkle-private.key
```

If the private key is lost while builds are still ad-hoc signed, installed copies cannot verify new updates, and users would have to download Opul again by hand.

## Turning on Developer ID signing and notarization

Once the project joins the Apple Developer Program, add these Actions secrets. The workflow detects them; no code changes are needed.

| Secret | Value |
| --- | --- |
| `DEVELOPER_ID_CERTIFICATE_P12` | Base64 of a `.p12` export of the "Developer ID Application" certificate and its private key (`base64 -i cert.p12 \| pbcopy`) |
| `DEVELOPER_ID_CERTIFICATE_PASSWORD` | The password chosen when exporting the `.p12` |
| `NOTARY_APPLE_ID` | The Apple Account email of the developer account |
| `NOTARY_TEAM_ID` | The 10-character Team ID from the developer account's Membership page |
| `NOTARY_PASSWORD` | An app-specific password from account.apple.com |

Existing users keep updating: Sparkle accepts an update whose code signature changes as long as its EdDSA signature matches, so keep the same Sparkle key.

After the first notarized release, remove the "Opening Opul for the first time" steps from the website's `/download/` page and from this repository's README.
