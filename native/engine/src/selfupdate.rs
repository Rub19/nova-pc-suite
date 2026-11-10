use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use std::path::PathBuf;
use std::time::Duration;

const PUBKEY: &str = include_str!("../../update.pub");
pub const MANIFEST: &str = "https://github.com/Rub19/nova-pc-suite/releases/latest/download/latest.json";
const ALLOWED: &str = "https://github.com/Rub19/nova-pc-suite/releases/download/";
const LIMIT: u64 = 300 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct Available {
    pub version: String,
    pub notes: String,
    pub url: String,
    pub signature: String,
}

pub fn configured() -> bool {
    !PUBKEY.trim().is_empty()
}

fn parts(version: &str) -> Vec<u64> {
    version.trim_start_matches('v').split('.').map(|part| part.split(['-', '+']).next().unwrap_or("").parse().unwrap_or(0)).collect()
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    parts(candidate) > parts(current)
}

fn verify_with(key: &str, data: &[u8], signature: &str) -> bool {
    let decode = |text: &str| base64::engine::general_purpose::STANDARD.decode(text.trim()).ok().and_then(|bytes| String::from_utf8(bytes).ok());
    let (Some(key), Some(sig)) = (decode(key), decode(signature)) else { return false };
    let (Ok(key), Ok(sig)) = (PublicKey::decode(&key), Signature::decode(&sig)) else { return false };
    key.verify(data, &sig, true).is_ok()
}

pub fn verify(data: &[u8], signature: &str) -> bool {
    configured() && verify_with(PUBKEY, data, signature)
}

pub fn parse(manifest: &str, current: &str) -> Result<Option<Available>, String> {
    let value: serde_json::Value = serde_json::from_str(manifest).map_err(|_| "Réponse de mise à jour illisible.".to_string())?;
    let version = value["version"].as_str().ok_or("Version absente.")?.trim_start_matches('v').to_string();
    if !is_newer(&version, current) {
        return Ok(None);
    }
    let platform = &value["platforms"]["windows-x86_64"];
    let url = platform["url"].as_str().ok_or("Adresse absente.")?.to_string();
    if !url.starts_with(ALLOWED) {
        return Err("Adresse de mise à jour refusée.".into());
    }
    Ok(Some(Available { version, notes: value["notes"].as_str().unwrap_or_default().to_string(), url, signature: platform["signature"].as_str().ok_or("Signature absente.")?.to_string() }))
}

fn agent() -> ureq::Agent {
    use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
    ureq::Agent::config_builder()
        .max_redirects(5)
        .https_only(true)
        .user_agent(concat!("NovaPCSuite/", env!("CARGO_PKG_VERSION")))
        .timeout_global(Some(Duration::from_secs(120)))
        .tls_config(TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build())
        .build()
        .new_agent()
}

fn get(url: &str) -> Result<Vec<u8>, String> {
    let mut response = agent().get(url).call().map_err(|error| match error {
        ureq::Error::StatusCode(404) => "Aucune version publiée.".to_string(),
        ureq::Error::StatusCode(code) => format!("réponse {code}"),
        _ => "Pas de réseau.".to_string(),
    })?;
    response.body_mut().with_config().limit(LIMIT).read_to_vec().map_err(|_| "Téléchargement interrompu.".to_string())
}

pub fn check(current: &str) -> Result<Option<Available>, String> {
    if !configured() {
        return Err("Mises à jour automatiques pas encore configurées.".into());
    }
    let manifest = String::from_utf8(get(MANIFEST)?).map_err(|_| "Réponse de mise à jour illisible.".to_string())?;
    parse(&manifest, current)
}

pub fn download(update: &Available) -> Result<PathBuf, String> {
    let data = get(&update.url)?;
    if !verify(&data, &update.signature) {
        return Err("Signature de la mise à jour invalide : rien n'est installé.".into());
    }
    let path = std::env::temp_dir().join(format!("Nova-PC-Suite-{}-setup.exe", update.version));
    std::fs::write(&path, data).map_err(|error| error.to_string())?;
    Ok(path)
}

pub fn launch(installer: &std::path::Path) -> Result<(), String> {
    std::process::Command::new(installer).args(["/P", "/UPDATE", "/R"]).spawn().map(|_| ()).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTERIOR_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEUxRkJEM0ZDNEI4NTczMUYKUldRZmM0VkwvTlA3NFp1TTJmWkttQjVDazllVmU4TXBWMHpkZGxEZEUzWnkva0J0MGJnUHBrQnMK";

    #[test]
    fn versions_compare_by_number() {
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("v0.4.1", "0.4.0"));
        assert!(!is_newer("0.4.0", "0.4.0"));
        assert!(!is_newer("0.2.14-preview", "0.4.0"));
    }

    #[test]
    fn manifests_are_checked() {
        let manifest = |version: &str, url: &str| format!(r#"{{"version":"{version}","notes":"n","platforms":{{"windows-x86_64":{{"signature":"s","url":"{url}"}}}}}}"#);
        let ours = "https://github.com/Rub19/nova-pc-suite/releases/download/v0.5.0/Nova.exe";
        assert_eq!(parse(&manifest("0.3.0", ours), "0.4.0"), Ok(None));
        assert_eq!(parse(&manifest("0.5.0", ours), "0.4.0").unwrap().unwrap().version, "0.5.0");
        assert!(parse(&manifest("0.5.0", "https://github.com/Rub19/autre/releases/download/a.exe"), "0.4.0").is_err());
        assert!(parse(&manifest("0.5.0", "http://github.com/Rub19/nova-pc-suite/releases/download/a.exe"), "0.4.0").is_err());
        assert!(parse("pas du json", "0.4.0").is_err());
    }

    #[test]
    fn signatures_are_checked_against_the_key() {
        assert!(!verify_with(INTERIOR_KEY, b"data", "AAAA"));
        assert!(!verify_with(INTERIOR_KEY, b"data", "pas du base64 !"));
        assert!(!verify_with("", b"data", "AAAA"));
        if !configured() {
            assert!(!verify(b"data", "AAAA"));
            assert!(check("0.4.0").is_err());
        }
    }
}
