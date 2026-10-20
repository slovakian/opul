// swift-tools-version: 6.2
import PackageDescription

let package = Package(
    name: "Opul",
    platforms: [
        .macOS(.v14),
    ],
    dependencies: [
        .package(url: "https://github.com/sparkle-project/Sparkle", from: "2.10.0"),
    ],
    targets: [
        .target(
            name: "opul_coreFFI",
            path: "Sources/opul_coreFFI",
            publicHeadersPath: "include",
            linkerSettings: [
                .linkedLibrary("opul_core"),
                .unsafeFlags(["-L", "build/lib"]),
            ]
        ),
        .target(
            name: "OpulCore",
            dependencies: ["opul_coreFFI"],
            path: "Sources/OpulCore"
        ),
        .target(
            name: "OpulClipboard",
            path: "Sources/OpulClipboard"
        ),
        .executableTarget(
            name: "Opul",
            dependencies: [
                "OpulClipboard",
                "OpulCore",
                .product(name: "Sparkle", package: "Sparkle"),
            ],
            path: "Sources/Opul",
            resources: [
                .copy("Resources/FallbackArtwork"),
                .copy("Resources/Sounds"),
            ],
            swiftSettings: [
                .defaultIsolation(MainActor.self),
            ]
        ),
        .executableTarget(
            name: "OpulPasteChecks",
            dependencies: ["OpulClipboard"],
            path: "Sources/OpulPasteChecks"
        ),
        .executableTarget(
            name: "OpulCoreChecks",
            dependencies: ["OpulCore"],
            path: "Sources/OpulCoreChecks"
        ),
    ]
)
