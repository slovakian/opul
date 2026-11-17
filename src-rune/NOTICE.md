# Owned Rune controls

`src/components.rs` adapts the button, text input and progress construction in Rune-kit's `actions.rs`, `text_input.rs` and `feedback.rs` at upstream revision `cca5df6fafb4721d96fe105f126ebfd2ec351bb6`.

The owned menu surface also adapts `menu.rs` at that revision: modal focus, roving items, fitted anchoring, dismissal and trigger restoration. The sidebar disclosure uses Rune's hover-group/focus styling and keyboard handlers. Product navigation and preference ordering are implemented here; no Rune-kit runtime dependency is introduced.

Copyright 2026 Anomaly, MIT. The full notice is in [LICENSE-Rune](LICENSE-Rune). The reduced controls use Opul's semantic tokens and core capabilities. The folder outline is drawn here. The production package does not depend on `rune-kit`.

The shared Rune dependency and all five workspace patches use the same audited revision as #100. The disposable prototypes retain their independent lockfiles.
