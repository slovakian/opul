import Foundation

extension Bundle {
    /// SwiftPM's `Bundle.module` only looks beside Opul.app and in the build folder, and traps when neither exists.
    /// Packaged builds carry the resource bundle in Contents/Resources, so look there first.
    static let opulResources: Bundle = {
        if let url = Bundle.main.url(forResource: "Opul_Opul", withExtension: "bundle"),
           let bundle = Bundle(url: url) {
            return bundle
        }
        return .module
    }()
}
