# Opul

Opul is a Mac download manager. People give it a link, a video page, or a torrent, and the files land in the right folder.

## Links

**Direct file**:
A link that points straight at a file, so Opul downloads it as is.
_Avoid_: Direct link, plain download

**Video page**:
A web page whose video or audio Opul finds and downloads with yt-dlp.
_Avoid_: Media page, page link

## Tools

**Tools**:
Everything Opul runs next to itself to do its work: the built-in tools and the video tools.
_Avoid_: Dependencies, helpers, plugins

**Built-in tools**:
Tools that ship inside Opul and are always there, such as FFmpeg.
_Avoid_: Bundled tools, media tools, helpers

**Video tools**:
The optional tools that let Opul download video pages: the Python runtime and yt-dlp. A person can use Opul without them.
_Avoid_: Media tools, add-on, video pack, using "yt-dlp" to mean both

**yt-dlp**:
The third-party program in the video tools that reads a video page and downloads its media.
_Avoid_: Downloader, extractor

**Python runtime**:
The part of the video tools that yt-dlp runs on.
_Avoid_: Runtime pack, Python pack

**Setup**:
The screen shown on first launch that introduces Opul and offers to download the video tools.
_Avoid_: Onboarding, welcome sheet, wizard

## Video tools states

**Not set up**:
The video tools aren't installed, and the person hasn't asked Opul to stop offering them.
_Avoid_: Missing, uninstalled

**Turned off**:
The video tools aren't installed, and the person asked Opul not to offer them again.
_Avoid_: Declined, disabled, opted out

**Setting up**:
The video tools are being installed for the first time.
_Avoid_: Installing, downloading

**Ready**:
The video tools work. A Ready state can also have an update available or an update in progress.
_Avoid_: Installed, enabled

**Not working**:
The video tools should work but don't, because a download failed, a check failed, or they stopped running.
_Avoid_: Broken, failed, error
