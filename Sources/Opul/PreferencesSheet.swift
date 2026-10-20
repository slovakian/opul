import SwiftUI

struct PreferencesSheet: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject private var updater: AppUpdater
    let library: DownloadLibrary

    @State private var selectedTab: PrefTab = .general

    enum PrefTab: String, CaseIterable, Identifiable {
        case general = "General"
        case sounds = "Sounds"
        case media = "Media"
        case torrents = "Torrenting"
        case bandwidth = "Bandwidth"

        var id: String { rawValue }

        var symbolName: String {
            switch self {
            case .general: "gearshape"
            case .sounds: "speaker.wave.2"
            case .media: "film.stack"
            case .torrents: "arrow.down.circle"
            case .bandwidth: "speedometer"
            }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            tabPicker
            Divider()
            Group {
                switch selectedTab {
                case .general:
                    generalView
                case .sounds:
                    soundsView
                case .media:
                    mediaView
                case .torrents:
                    torrentsView
                case .bandwidth:
                    bandwidthView
                }
            }
            .frame(minWidth: 560, minHeight: 380)
            Divider()
            footerView
        }
        .frame(minWidth: 580, minHeight: 440)
    }

    private var generalView: some View {
        Form {
            Section("File Organization") {
                Picker("Default Organization", selection: Binding(
                    get: { library.preferences.fileOrganization },
                    set: { library.preferences.fileOrganization = $0 }
                )) {
                    ForEach(DownloadFileOrganization.allCases) { organization in
                        Text(organization.title).tag(organization)
                    }
                }
                Text(library.preferences.fileOrganization.explanation)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }

            Section(footer: Text("Workspaces use this default unless they have their own File Organization setting.")) {
                EmptyView()
            }

            Section("Updates") {
                Toggle("Automatically install updates", isOn: $updater.automaticallyInstallsUpdates)
                Button("Check for Updates…", action: updater.checkForUpdates)
                    .disabled(!updater.canCheckForUpdates)
            }
        }
        .formStyle(.grouped)
    }

    private var mediaView: some View {
        Form {
            Section("Download Engine") {
                LabeledContent("Status") {
                    HStack(spacing: 6) {
                        Image(systemName: library.mediaToolStatus?.ready == true ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                            .foregroundStyle(library.mediaToolStatus?.ready == true ? Color.green : Color.orange)
                        Text(library.mediaToolStatus?.message ?? "Not initialized")
                            .multilineTextAlignment(.trailing)
                    }
                }
                if let version = library.mediaToolStatus?.ytDlpVersion {
                    LabeledContent("yt-dlp", value: version)
                }
                if let version = library.mediaToolStatus?.ffmpegVersion {
                    LabeledContent("FFmpeg", value: version)
                }
                TextField("yt-dlp Path", text: Binding(
                    get: { library.preferences.ytDlpPath },
                    set: { library.preferences.ytDlpPath = $0 }
                ), prompt: Text("Automatic"))
                TextField("FFmpeg Path", text: Binding(
                    get: { library.preferences.ffmpegPath },
                    set: { library.preferences.ffmpegPath = $0 }
                ), prompt: Text("Automatic"))
                Stepper(
                    "Concurrent downloads: \(library.preferences.maxConcurrentMediaDownloads)",
                    value: Binding(
                        get: { library.preferences.maxConcurrentMediaDownloads },
                        set: { library.preferences.maxConcurrentMediaDownloads = $0 }
                    ),
                    in: 1...12
                )
            }

            Section("Subtitles & Metadata") {
                Toggle("Download subtitles and automatic captions", isOn: Binding(
                    get: { library.preferences.downloadSubtitles },
                    set: { library.preferences.downloadSubtitles = $0 }
                ))
                TextField("Subtitle Languages", text: Binding(
                    get: { library.preferences.subtitleLanguages },
                    set: { library.preferences.subtitleLanguages = $0 }
                ), prompt: Text("en.*,en"))
                    .disabled(!library.preferences.downloadSubtitles)
                Toggle("Embed subtitles when possible", isOn: Binding(
                    get: { library.preferences.embedSubtitles },
                    set: { library.preferences.embedSubtitles = $0 }
                ))
                    .disabled(!library.preferences.downloadSubtitles)
                Toggle("Write thumbnail", isOn: Binding(
                    get: { library.preferences.writeThumbnail },
                    set: { library.preferences.writeThumbnail = $0 }
                ))
                Toggle("Write metadata JSON", isOn: Binding(
                    get: { library.preferences.writeMetadata },
                    set: { library.preferences.writeMetadata = $0 }
                ))
            }

            Section("File Naming") {
                TextField("Template", text: Binding(
                    get: { library.preferences.filenameTemplate },
                    set: { library.preferences.filenameTemplate = $0 }
                ))
                Text("Uses yt-dlp output-template fields. Tool path changes apply after reopening Opul.")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
    }

    private var soundsView: some View {
        Form {
            Section {
                Toggle("Play interaction sounds", isOn: Binding(
                    get: { library.preferences.interactionSounds.isEnabled },
                    set: { library.preferences.interactionSounds.isEnabled = $0 }
                ))

                LabeledContent("Volume") {
                    HStack(spacing: 10) {
                        Slider(
                            value: Binding(
                                get: { library.preferences.interactionSounds.volume },
                                set: { library.preferences.interactionSounds.volume = $0 }
                            ),
                            in: 0...1,
                            step: 0.05
                        ) {
                            Text("Sound volume")
                        }
                        .frame(minWidth: 180)
                        .accessibilityValue(soundVolumeLabel)

                        Text(soundVolumeLabel)
                            .foregroundStyle(.secondary)
                            .monospacedDigit()
                            .frame(width: 42, alignment: .trailing)
                    }
                }
                .disabled(!library.preferences.interactionSounds.isEnabled)
            } header: {
                Text("Playback")
            } footer: {
                Text("Sounds use your Mac's current audio output and system volume.")
            }

            Section {
                ForEach(InteractionSoundCategory.allCases) { category in
                    soundCategoryRow(category)
                }
            } header: {
                Text("Categories")
            } footer: {
                Text("Sound effects accompany the visual status in Opul. They never replace it.")
            }
        }
        .formStyle(.grouped)
    }

    private func soundCategoryRow(_ category: InteractionSoundCategory) -> some View {
        HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(category.title)
                Text(category.explanation)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            Button("Play sample", systemImage: "play.fill") {
                library.previewSound(category)
            }
            .labelStyle(.iconOnly)
            .help("Play a sample \(category.title.lowercased()) sound")
            .accessibilityLabel("Play a sample \(category.title.lowercased()) sound")
            .disabled(!library.preferences.interactionSounds.isEnabled(category))

            Toggle("\(category.title) sounds", isOn: Binding(
                get: { library.preferences.interactionSounds.enabledCategories.contains(category) },
                set: { library.preferences.interactionSounds.setEnabled($0, for: category) }
            ))
            .labelsHidden()
            .accessibilityLabel("\(category.title) sounds")
        }
        .disabled(!library.preferences.interactionSounds.isEnabled)
    }

    private var soundVolumeLabel: String {
        library.preferences.interactionSounds.volume.formatted(
            .percent.precision(.fractionLength(0))
        )
    }

    private var tabPicker: some View {
        Picker("Preferences Tab", selection: $selectedTab) {
            ForEach(PrefTab.allCases) { tab in
                Label(tab.rawValue, systemImage: tab.symbolName).tag(tab)
            }
        }
        .pickerStyle(.segmented)
        .padding(16)
    }

    private var torrentsView: some View {
        Form {
            Section("Seeding") {
                Picker("Default Seed Ratio", selection: Binding(
                    get: { library.preferences.defaultSeedRatio },
                    set: { library.preferences.defaultSeedRatio = $0 }
                )) {
                    Text("No limit").tag(0.0)
                    Text("0.5 (50%)").tag(0.5)
                    Text("1.0 (100%)").tag(1.0)
                    Text("1.5 (150%)").tag(1.5)
                    Text("2.0 (200%)").tag(2.0)
                }

                Toggle("Stop seeding automatically when target ratio is reached", isOn: Binding(
                    get: { library.preferences.autoStopSeeding },
                    set: { library.preferences.autoStopSeeding = $0 }
                ))
            }

            Section("Network") {
                HStack {
                    Text("Listen Port")
                    Spacer()
                    TextField("Port", value: Binding(
                        get: { library.preferences.listenPort },
                        set: { library.preferences.listenPort = $0 }
                    ), format: .number)
                    .frame(width: 80)
                    .textFieldStyle(.roundedBorder)
                }

                Toggle("Use random port at launch", isOn: Binding(
                    get: { library.preferences.randomizePort },
                    set: { library.preferences.randomizePort = $0 }
                ))

                Toggle("Enable DHT (Distributed Hash Table)", isOn: Binding(
                    get: { library.preferences.enableDHT },
                    set: { library.preferences.enableDHT = $0 }
                ))

                HStack {
                    Text("Maximum Connections")
                    Spacer()
                    TextField("Connections", value: Binding(
                        get: { library.preferences.maxConnections },
                        set: { library.preferences.maxConnections = $0 }
                    ), format: .number)
                    .frame(width: 80)
                    .textFieldStyle(.roundedBorder)
                }
            }
        }
        .formStyle(.grouped)
    }

    private var bandwidthView: some View {
        Form {
            Section("Global Speed Limits") {
                HStack {
                    Text("Max Download Rate")
                    Spacer()
                    TextField("KB/s", value: Binding(
                        get: { library.preferences.maxDownloadRateKBps },
                        set: { library.preferences.maxDownloadRateKBps = $0 }
                    ), format: .number)
                    .frame(width: 80)
                    .textFieldStyle(.roundedBorder)
                    Text(library.preferences.maxDownloadRateKBps == 0 ? "Unlimited" : "KB/s")
                        .foregroundStyle(.secondary)
                        .frame(width: 60, alignment: .leading)
                }

                HStack {
                    Text("Max Upload Rate")
                    Spacer()
                    TextField("KB/s", value: Binding(
                        get: { library.preferences.maxUploadRateKBps },
                        set: { library.preferences.maxUploadRateKBps = $0 }
                    ), format: .number)
                    .frame(width: 80)
                    .textFieldStyle(.roundedBorder)
                    Text(library.preferences.maxUploadRateKBps == 0 ? "Unlimited" : "KB/s")
                        .foregroundStyle(.secondary)
                        .frame(width: 60, alignment: .leading)
                }
            }

            Section(footer: Text("Setting 0 means no speed limit. The listen port, DHT, and connection cap apply the next time Opul opens. Torrent limits apply immediately; the media limit applies when a download starts.")) {
                EmptyView()
            }
        }
        .formStyle(.grouped)
    }

    private var footerView: some View {
        HStack {
            Spacer()
            Button("Done") {
                dismiss()
            }
            .buttonStyle(.borderedProminent)
            .keyboardShortcut(.defaultAction)
        }
        .padding(14)
    }
}
