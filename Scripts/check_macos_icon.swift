// Compare the actual development and Finder icons with the pre-PR native appearance.
// Run with: swift Scripts/check_macos_icon.swift [Opul.app]
import AppKit

func fail(_ message: String) -> Never {
  fputs("ERROR: \(message)\n", stderr)
  exit(1)
}

let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
let app =
  CommandLine.arguments.count > 1
  ? URL(fileURLWithPath: CommandLine.arguments[1]) : root.appendingPathComponent("Opul.app")
let artifacts = root.appendingPathComponent("test-results/macos-icon")
try FileManager.default.createDirectory(at: artifacts, withIntermediateDirectories: true)
let temporary = FileManager.default.temporaryDirectory.appendingPathComponent(
  "opul-icons-" + UUID().uuidString)
try FileManager.default.createDirectory(at: temporary, withIntermediateDirectories: true)
defer { try? FileManager.default.removeItem(at: temporary) }

func load(_ url: URL) -> NSImage {
  guard let image = NSImage(contentsOf: url) else { fail("Cannot read \(url.path)") }
  return image
}

func render(_ image: NSImage, size: Int) -> NSBitmapImageRep {
  guard
    let bitmap = NSBitmapImageRep(
      bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
      bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
      isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0
    ), let context = NSGraphicsContext(bitmapImageRep: bitmap)
  else {
    fail("Cannot allocate icon bitmap")
  }
  NSGraphicsContext.saveGraphicsState()
  NSGraphicsContext.current = context
  image.draw(
    in: NSRect(x: 0, y: 0, width: size, height: size),
    from: .zero, operation: .copy, fraction: 1)
  NSGraphicsContext.restoreGraphicsState()
  return bitmap
}

let reference = load(root.appendingPathComponent("assets/AppIcon-macOS.png"))
var failed = false

func check(_ image: NSImage, name: String, against reference: NSImage = reference) throws {
  for size in [32, 128, 256, 512] {
    let actual = render(image, size: size)
    let expected = render(reference, size: size)
    var difference = 0.0
    for y in 0..<size {
      for x in 0..<size {
        let a = actual.colorAt(x: x, y: y)!.usingColorSpace(.deviceRGB)!
        let e = expected.colorAt(x: x, y: y)!.usingColorSpace(.deviceRGB)!
        // Compare premultiplied colors so invisible RGB values do not count.
        difference += abs(a.redComponent * a.alphaComponent - e.redComponent * e.alphaComponent)
        difference += abs(a.greenComponent * a.alphaComponent - e.greenComponent * e.alphaComponent)
        difference += abs(a.blueComponent * a.alphaComponent - e.blueComponent * e.alphaComponent)
        difference += abs(a.alphaComponent - e.alphaComponent)
      }
    }
    difference /= Double(size * size * 4)
    // Allow resampling differences between ICNS resolutions and OS versions;
    // small system icons also receive stronger optical adjustments.
    // Checks across these sizes catch both the opaque square and nested tile.
    let passed = difference < (size == 32 ? 0.04 : 0.02)
    failed = failed || !passed
    print(
      "\(passed ? "PASS" : "FAIL") \(name) \(size)px: \(String(format: "%.2f", difference * 100))% pixel difference"
    )
    if size == 256 {
      try actual.representation(using: .png, properties: [:])!.write(
        to: artifacts.appendingPathComponent(name + ".png")
      )
    }
  }
}

try check(load(root.appendingPathComponent("src-tauri/icons/icon.png")), name: "development-png")
try check(load(root.appendingPathComponent("src-tauri/icons/icon.icns")), name: "development-icns")

let plistURL = app.appendingPathComponent("Contents/Info.plist")
let plist = try PropertyListSerialization.propertyList(
  from: Data(contentsOf: plistURL), format: nil)
guard let info = plist as? [String: Any], let iconName = info["CFBundleIconFile"] as? String else {
  fail("\(plistURL.path) has no CFBundleIconFile")
}
let iconFile = (iconName as NSString).pathExtension.isEmpty ? iconName + ".icns" : iconName
let bundled = load(app.appendingPathComponent("Contents/Resources/" + iconFile))
try check(bundled, name: "bundled-icns")

// Finder caches icons across rebuilds. A fresh copy exercises the shipped bundle
// without reusing an older icon cached for Opul.app's path.
let freshApp = temporary.appendingPathComponent("Opul.app")
try FileManager.default.copyItem(at: app, to: freshApp)
var finderReference = reference
if #available(macOS 26, *) {
  // Compare with the original artwork through the same system renderer at each
  // size, including its optical adjustments for small icons.
  let legacy = temporary.appendingPathComponent("Original.app")
  try FileManager.default.createDirectory(
    at: legacy.appendingPathComponent("Contents/Resources"), withIntermediateDirectories: true)
  try FileManager.default.createDirectory(
    at: legacy.appendingPathComponent("Contents/MacOS"), withIntermediateDirectories: true)
  try FileManager.default.copyItem(
    atPath: "/usr/bin/true", toPath: legacy.appendingPathComponent("Contents/MacOS/Original").path)
  let build = Process()
  build.executableURL = URL(fileURLWithPath: "/bin/bash")
  build.arguments = [
    root.appendingPathComponent("Scripts/build_icon.sh").path,
    root.appendingPathComponent("assets/AppIcon.png").path,
    legacy.appendingPathComponent("Contents/Resources/Icon.icns").path,
  ]
  try build.run()
  build.waitUntilExit()
  guard build.terminationStatus == 0 else { fail("Cannot build the original reference icon") }
  let legacyInfo = [
    "CFBundleName": "Original",
    "CFBundleIdentifier": "dev.opul.icon-reference." + UUID().uuidString,
    "CFBundleExecutable": "Original", "CFBundlePackageType": "APPL",
    "CFBundleIconFile": "Icon.icns",
  ]
  try PropertyListSerialization.data(fromPropertyList: legacyInfo, format: .xml, options: 0)
    .write(to: legacy.appendingPathComponent("Contents/Info.plist"))
  finderReference = NSWorkspace.shared.icon(forFile: legacy.path)
}
try check(NSWorkspace.shared.icon(forFile: freshApp.path), name: "finder", against: finderReference)
print("Icon previews: \(artifacts.path)")
if failed { fail("macOS icon differs from the original native appearance") }
