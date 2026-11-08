# Third-party media tools

Opul includes FFmpeg and ffprobe from [FFmpeg](https://ffmpeg.org/), with the [LAME MP3 encoder](https://lame.sourceforge.io/), under the GNU Lesser General Public License version 2.1 or later. These are separate executables; Opul does not link their libraries into its application binary. FFmpeg is a trademark of Fabrice Bellard.

Each app includes the complete FFmpeg and LAME LGPL texts and `SOURCE.txt` in `Contents/Resources/media-tools/`. That file identifies the exact source tarballs, SHA-256 checksums, configuration and source release. The unmodified sources and build recipe are distributed with the binary archives in [the pinned tools release](https://github.com/slovakian/opul/releases/tag/ffmpeg-1). You may rebuild and replace these tools under their respective licenses.

The exact unmodified source inputs for `ffmpeg-1` are:

| Component | Source | SHA-256 |
| --- | --- | --- |
| FFmpeg/ffprobe 9.0.2 | [ffmpeg-9.0.2.tar.xz](https://github.com/slovakian/opul/releases/download/ffmpeg-1/ffmpeg-9.0.2.tar.xz) | `8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e` |
| LAME 3.100 | [lame-3.100.tar.gz](https://github.com/slovakian/opul/releases/download/ffmpeg-1/lame-3.100.tar.gz) | `ddfe36cab873794038ae2c1210557ad34857a4b6bdc515785d1da9e175b1da1e` |

The same release distributes `build.sh` and `sources.env`; each architecture archive includes its configure arguments. The minimal static build disables GPL and nonfree components and uses LAME's LGPL encoder without its GPL decoder. FFmpeg's zlib and Apple system-framework dependencies remain identified by the recipe and executable load commands.

The application itself remains under the MIT license.

Optional video tools download only with consent. Runtime 1's top-level inventory is below. License texts, pinned source URLs/hashes and runtime metadata travel inside each signed runtime archive. Paths are relative to `Tools/python-runtime/1/`. See [the runtime build contract](docs/python-runtime.md), [the exact lock](tools/python-runtime/runtime.lock.json), and [the immutable runtime release](https://github.com/slovakian/opul/releases/tag/python-runtime-1).

| Component | Version | License and shipped notice |
| --- | --- | --- |
| Python | 3.13.18, standalone 20261026 | PSF and historical terms, `licenses/Python-LICENSE.txt`; bundled native-library notices in `licenses/Python-dependencies.rst` |
| Bellard QuickJS | 2026-06-04 | MIT, `licenses/QuickJS-LICENSE.txt`; custom Opul driver, no upstream engine changes |
| curl_cffi | 0.17.0 | MIT, `lib/python3.13/site-packages/curl_cffi-0.17.0.dist-info/licenses/LICENSE` |
| cffi | 2.1.1 | MIT, corresponding `.dist-info/licenses/LICENSE` |
| certifi | 2026.10.23 | MPL-2.0, corresponding `.dist-info/licenses/LICENSE`, including Mozilla CA provenance; unmodified source: [python-certifi](https://github.com/certifi/python-certifi) |
| Brotli | 1.2.1 | MIT, corresponding `.dist-info/licenses/LICENSE` |
| websockets | 17.3 | BSD-3-Clause, corresponding `.dist-info/licenses/LICENSE` |
| PyCryptodome (Cryptodome namespace) | 3.24.1 | BSD/public-domain terms, `pycryptodomex-3.24.1.dist-info/LICENSE.rst` |
| pycparser | 3.11 | BSD-3-Clause, corresponding `.dist-info/licenses/LICENSE` |

`Python-dependencies.rst` preserves upstream's bundled-library notice inventory, including libffi, OpenSSL, SQLite, bzip2, ncurses and zlib. Wheel metadata and notices are retained rather than replaced by this summary.

Runtime 2 repairs runtime 1's missing native curl notices without changing its executable pins. `licenses/curl-impersonate/` includes the upstream release's complete macOS notice bundle, plus BoringSSL's fiat-crypto license and author attribution. Every notice has a pinned SHA-256 and is checked before publication. The directory also retains curl_cffi's tagged Makefile, curl-impersonate's exact CMake recipe, and source/archive URLs and checksums. Those recipes identify the native source versions and patches used by the pinned wheel; AppleIDN is provided by macOS rather than bundled.

| Embedded component | Version | Notice file in `licenses/curl-impersonate/` |
| --- | --- | --- |
| curl-impersonate | 2.3.1 | `LICENSE` |
| curl | 8.23.0 | `LICENSE_CURL` |
| BoringSSL | `156c7b75ae9b8c3b3f847acf264f17594c3859fb` | `LICENSE_BORINGSSL` |
| fiat-crypto | BoringSSL's pinned copy | `LICENSE_BORINGSSL_FIAT`, `AUTHORS_BORINGSSL_FIAT` |
| Brotli | 1.2.1 | `LICENSE_BROTLI` |
| c-ares | 1.34.9 | `LICENSE_CARES` |
| nghttp2 | 1.63.0 | `LICENSE_NGHTTP2` |
| nghttp3 | 1.15.0 | `LICENSE_NGHTTP3` |
| ngtcp2 | 1.20.0 | `LICENSE_NGTCP2` |
| zlib | 1.3.1 | `LICENSE_ZLIB` |
| zstd | 1.5.7 | `LICENSE_ZSTD` |

Opul downloads yt-dlp's official bare script directly from [yt-dlp releases](https://github.com/yt-dlp/yt-dlp/releases). yt-dlp is under the Unlicense and includes yt-dlp-ejs under the Unlicense. It is not included in the app installer. Settings links to the project and its supported-sites list.
