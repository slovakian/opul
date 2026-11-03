# 0025: FFmpeg and ffprobe are built-in tools

Status: accepted

Issue #59 implements the built-in tools portion of #58. This supersedes ADR 0021's discovery policy for FFmpeg. The existing yt-dlp preferences and discovery route stay in place until managed video tools replace them.

FFmpeg and ffprobe live together in `Contents/Resources/media-tools/`. The media engine resolves only those resources, checks both versions, and passes that directory explicitly to yt-dlp. Missing tools disable media post-processing with a reinstall message; direct files and torrents continue to work. The FFmpeg path preference is removed. Settings reports FFmpeg as Built in, with its version and with ffprobe. There is no tool-path restart advice.

Each installer contains one architecture. Both target macOS 13.0. A dedicated workflow builds pinned FFmpeg and LAME sources, with GPL and nonfree components disabled and LAME's GPL decoder excluded. Only the encoder library is built. Opul launches separate executables; its Rust binary does not link to FFmpeg. The selected codecs support merging without video re-encoding, MP3, AAC/M4A, subtitle conversion and embedding, and JPG thumbnails.

The Intel build is cross-compiled using Apple's SDK on Apple silicon. The tools workflow requires Rosetta to execute both Intel programs and run every media job. A missing translation environment fails the build instead of recording a compile-only pass. Local verification also executes this same suite through Rosetta. This establishes Intel execution, while deployment-target inspection establishes the macOS 13 build contract; it is not a runtime test on macOS 13.

The workflow publishes immutable `ffmpeg-<n>` pre-releases with both binary archives, exact source tarballs, recipe, notices, checksums and smoke-test logs. Tools releases never become Latest. Normal CI, local Tauri builds, Nightly and release packaging download only their target architecture and verify committed archive hashes. They never rebuild FFmpeg. The fetch hook uses Tauri's target triple, so cross-packaging cannot accidentally select the host architecture.

See [the build and update procedure](../built-in-tools.md).
