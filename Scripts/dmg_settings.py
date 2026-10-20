# dmgbuild layout for the drag-to-Applications window. Run through Scripts/package_dmg.sh.
import os.path

application = defines["app"]  # noqa: F821 (provided by dmgbuild)
appname = os.path.basename(application)

format = "ULFO"
files = [application]
symlinks = {"Applications": "/Applications"}
icon = os.path.join(application, "Contents", "Resources", "Icon.icns")
hide_extension = [appname]

window_rect = ((200, 120), (540, 360))
icon_size = 128
icon_locations = {appname: (140, 170), "Applications": (400, 170)}
