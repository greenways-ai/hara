//! Publisher identity management for the Hara CLI.
//!
//! Private keys remain behind `HARA_SIGNER`.  This client exchanges only
//! canonical enrollment bytes, detached signatures and public key material.

use crate::tap;
use std::env;
use std::process::Command;

const DEFAULT_IDENTITY_ENDPOINT: &str = "https://id.hara-lang.org";

pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None | Some("--help" | "-h") => {
            usage();
            Ok(())
        }
        Some("login") => {
            println!("{}/login/github", endpoint().trim_end_matches('/'));
            Ok(())
        }
        Some("enroll") => enroll(&args[1..]),
        Some("status") => get("/v1/status", &args[1..]),
        Some("namespace") => get("/v1/namespaces", &args[1..]),
        Some("key") => key_command(&args[1..]),
        Some(command) => Err(format!("unknown id command: {command}")),
    }
}

fn enroll(args: &[String]) -> Result<(), String> {
    let owner = required_option(args, "--owner")?;
    let tap_name = optional_option(args, "--tap").unwrap_or_else(|| "hara".into());
    let tap_name = if tap_name == "official" {
        "hara".to_owned()
    } else {
        tap_name
    };
    let public_key = env::var("HARA_SIGNER_PUBLIC_KEY")
        .map_err(|_| "id enroll requires HARA_SIGNER_PUBLIC_KEY".to_owned())?;
    validate_hex(&public_key, 32, "HARA_SIGNER_PUBLIC_KEY")?;
    let challenge = if let Some(challenge) = optional_option(args, "--challenge") {
        challenge
    } else {
        fetch_challenge(&owner)?
    };
    let request = canonical_enrollment(&tap_name, &owner, &public_key, &challenge);
    let (key_id, signature) = tap::sign(request.as_bytes())?;
    if args.iter().any(|arg| arg == "--dry-run") {
        print!("{request}");
        println!("key-id={key_id} signature={signature}");
        return Ok(());
    }
    let envelope = format!(
        "{{:enrollment/request {} :enrollment/key-id {} :enrollment/signature {}}}\n",
        edn_string(&request),
        edn_string(&key_id),
        edn_string(&signature)
    );
    post("/v1/enrollments", &envelope)
}

fn key_command(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("list") => get("/v1/keys", &args[1..]),
        Some("rotate") => post("/v1/keys/rotate", "{}\n"),
        Some("revoke") => {
            let key_id = args.get(1).ok_or("id key revoke requires KEY_ID")?;
            post(
                &format!("/v1/keys/{key_id}/revocations"),
                "{:revocation/reason :publisher-request}\n",
            )
        }
        _ => Err("usage: hara id key <list|rotate|revoke KEY_ID>".into()),
    }
}

pub fn canonical_enrollment(tap: &str, owner: &str, public_key: &str, challenge: &str) -> String {
    format!(
        "{{:enrollment/format 1 :enrollment/tap {} :enrollment/provider :github :enrollment/owner {} :enrollment/public-key {} :enrollment/challenge {}}}\n",
        edn_string(tap),
        edn_string(owner),
        edn_string(public_key),
        edn_string(challenge)
    )
}

fn fetch_challenge(owner: &str) -> Result<String, String> {
    let url = format!(
        "{}/v1/enrollments/challenge?owner={owner}",
        endpoint().trim_end_matches('/')
    );
    let output = Command::new("curl")
        .args(["--fail-with-body", "--silent", "--show-error", &url])
        .output()
        .map_err(|error| format!("cannot start identity client: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "identity challenge failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let challenge = String::from_utf8(output.stdout)
        .map_err(|_| "identity challenge must be UTF-8")?
        .trim()
        .to_owned();
    if challenge.is_empty() {
        return Err("identity service returned an empty challenge".into());
    }
    Ok(challenge)
}

fn get(path: &str, _args: &[String]) -> Result<(), String> {
    request("GET", path, None)
}

fn post(path: &str, body: &str) -> Result<(), String> {
    request("POST", path, Some(body))
}

fn request(method: &str, path: &str, body: Option<&str>) -> Result<(), String> {
    let url = format!("{}{}", endpoint().trim_end_matches('/'), path);
    let mut command = Command::new("curl");
    command.args([
        "--fail-with-body",
        "--silent",
        "--show-error",
        "-X",
        method,
        "-H",
        "accept: application/edn",
    ]);
    if let Some(body) = body {
        command.args(["-H", "content-type: application/edn", "--data-binary", body]);
    }
    let output = command
        .arg(url)
        .output()
        .map_err(|error| format!("cannot start identity client: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "identity request failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    print!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}

fn endpoint() -> String {
    env::var("HARA_ID_ENDPOINT").unwrap_or_else(|_| DEFAULT_IDENTITY_ENDPOINT.into())
}

fn required_option(args: &[String], name: &str) -> Result<String, String> {
    optional_option(args, name).ok_or_else(|| format!("id enroll requires {name}"))
}

fn optional_option(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|value| value == name)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn validate_hex(value: &str, bytes: usize, label: &str) -> Result<(), String> {
    if value.len() != bytes * 2
        || !value
            .bytes()
            .all(|value| value.is_ascii_hexdigit() && !value.is_ascii_uppercase())
    {
        return Err(format!("{label} must be lowercase {}-byte hex", bytes));
    }
    Ok(())
}

fn edn_string(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    )
}

fn usage() {
    println!("hara id login");
    println!("hara id enroll --owner OWNER [--tap hara] [--dry-run]");
    println!("hara id status");
    println!("hara id namespace");
    println!("hara id key <list|rotate|revoke KEY_ID>");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enrollment_bytes_are_stable_and_bind_the_public_key() {
        assert_eq!(
            canonical_enrollment(
                "hara",
                "alice",
                &"ab".repeat(32),
                "challenge-1"
            ),
            format!(
                "{{:enrollment/format 1 :enrollment/tap \"hara\" :enrollment/provider :github :enrollment/owner \"alice\" :enrollment/public-key \"{}\" :enrollment/challenge \"challenge-1\"}}\n",
                "ab".repeat(32)
            )
        );
    }
}
