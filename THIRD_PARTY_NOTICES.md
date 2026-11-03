# Third-party media tools

Opul includes FFmpeg and ffprobe from [FFmpeg](https://ffmpeg.org/), with the [LAME MP3 encoder](https://lame.sourceforge.io/), under the GNU Lesser General Public License version 2.1 or later. These are separate executables; Opul does not link their libraries into its application binary. FFmpeg is a trademark of Fabrice Bellard.

Each app includes the complete FFmpeg and LAME LGPL texts and `SOURCE.txt` in `Contents/Resources/media-tools/`. That file identifies the exact source tarballs, SHA-256 checksums, configuration and source release. The unmodified sources and build recipe are distributed with the binary archives in [the pinned tools release](https://github.com/slovakian/opul/releases/tag/ffmpeg-1). You may rebuild and replace these tools under their respective licenses.

The application itself remains under the MIT license. yt-dlp is currently installed separately and is not redistributed by this slice.
