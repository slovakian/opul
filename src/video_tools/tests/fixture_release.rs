//! Changes to what the fixture server publishes after it starts.

use super::*;
use ed25519_dalek::{Signer, SigningKey};

impl Fixture {
    pub(super) fn minimum_opul_version(&self, version: &str) {
        let mut routes = self.routes.lock().unwrap();
        let mut pointer: serde_json::Value =
            serde_json::from_slice(&routes["/pointer"].bytes).unwrap();
        pointer["minimum_opul_version"] = version.into();
        let bytes = serde_json::to_vec(&pointer).unwrap();
        routes.get_mut("/pointer.sig").unwrap().bytes = SigningKey::from_bytes(&[9; 32])
            .sign(&bytes)
            .to_bytes()
            .to_vec();
        routes.get_mut("/pointer").unwrap().bytes = bytes;
    }
    pub(super) fn publish_nightly(&self, number: u64, version: &str, health: &str) {
        let stable_routes: Vec<_> = self
            .routes
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _)| path.starts_with("/yt/"))
            .map(|(path, response)| (path.clone(), response.clone()))
            .collect();
        self.publish(number, version, &health.replace("2026.10.26", version));
        let mut routes = self.routes.lock().unwrap();
        let nightly_routes: Vec<_> = routes
            .iter()
            .filter(|(path, _)| path.starts_with("/yt/"))
            .map(|(path, response)| {
                let mut response = response.clone();
                response.redirect = response
                    .redirect
                    .map(|url| url.replace("/yt/", "/nightly/"));
                (path.replacen("/yt/", "/nightly/", 1), response)
            })
            .collect();
        routes.retain(|path, _| !path.starts_with("/yt/"));
        routes.extend(stable_routes);
        routes.extend(nightly_routes);
    }
}
