# Third-party media tools

Opul includes FFmpeg and ffprobe from [FFmpeg](https://ffmpeg.org/), with the [LAME MP3 encoder](https://lame.sourceforge.io/), under the GNU Lesser General Public License version 2.1 or later. These are separate executables; Opul does not link their libraries into its application binary. FFmpeg is a trademark of Fabrice Bellard.

Each app includes the complete FFmpeg and LAME LGPL texts and `SOURCE.txt` in `Contents/Resources/media-tools/`. That file identifies the exact source tarballs, SHA-256 checksums, configuration and source release. The unmodified sources and build recipe are distributed with the binary archives in [the pinned tools release](https://github.com/slovakian/opul/releases/tag/ffmpeg-1). You may rebuild and replace these tools under their respective licenses.

The application itself remains under the MIT license.

Optional video tools download only with consent. The Python runtime includes Python under the PSF license, curl_cffi under the MIT license, cffi under the MIT license, certifi under MPL-2.0, Brotli under the MIT license, websockets under the BSD-3-Clause license, PyCryptodome under BSD/public-domain terms, and Bellard QuickJS under the MIT license. License texts, pinned sources and runtime metadata travel inside each signed runtime archive. See [the runtime build contract](docs/python-runtime.md) and [the immutable runtime release](https://github.com/slovakian/opul/releases/tag/python-runtime-1).

Opul downloads yt-dlp's official bare script directly from [yt-dlp releases](https://github.com/yt-dlp/yt-dlp/releases). yt-dlp is under the Unlicense and includes yt-dlp-ejs under the Unlicense. It is not included in the app installer. Settings links to the project and its supported-sites list.
