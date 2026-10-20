# Security

Please do not open a public issue for a suspected vulnerability. Report it through GitHub's private vulnerability reporting for this repository. Include reproduction steps, affected versions, and the impact you observed.

Opul currently distributes ad-hoc-signed builds that are not notarized by Apple. Download Opul only from `github.com/slovakian/opul` or `opul.procka.org`. GitHub lists a SHA-256 digest next to every release file.

Updates are delivered with [Sparkle](https://sparkle-project.org). Every update is signed with the project's EdDSA key, and Opul refuses any update whose signature does not match the public key built into the installed app.
