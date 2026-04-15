// watch_p256_auth_bdd.rs — BDD harness for P256 ECDSA watch authentication
//
// Tests the server-side verifier (verify_p256_signature) and the full
// register_device round-trip using real P256 keys, proving the
// Secure Enclave key type is handled correctly.

use cucumber::{given, then, when, World};
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use tempfile::TempDir;
use vibecli_cli::watch_auth::{WatchAuthManager, WatchRegisterRequest};

#[derive(Debug, Default, World)]
pub struct P256World {
    manager:     Option<WatchAuthManager>,
    _tmp:        Option<TempDir>,
    signing_key: Option<SigningKey>,
    nonce:       String,
    issued_at:   u64,
    last_req:    Option<WatchRegisterRequest>,
    last_result: Option<Result<String, String>>, // device_id or error message
}

impl P256World {
    fn mgr(&mut self) -> &mut WatchAuthManager {
        self.manager.as_mut().expect("manager not initialised")
    }

    fn build_req(&self, pk_bytes: &[u8], sig_bytes: &[u8]) -> WatchRegisterRequest {
        WatchRegisterRequest {
            device_id:          "testdevice00000000000000000000001".into(),
            name:               "BDD Watch".into(),
            os_version:         "11.0".into(),
            model:              "Watch7,1".into(),
            public_key_b64:     B64.encode(pk_bytes),
            signature_b64:      B64.encode(sig_bytes),
            nonce:              self.nonce.clone(),
            device_check_token: None,
        }
    }
}

// ── Given ────────────────────────────────────────────────────────────────────

#[given("a fresh WatchAuthManager")]
fn a_fresh_manager(world: &mut P256World) {
    let tmp = tempfile::tempdir().expect("tmpdir");
    let mgr = WatchAuthManager::new_with_path(tmp.path(), &[42u8; 32])
        .expect("WatchAuthManager::new_with_path");
    world._tmp    = Some(tmp);
    world.manager = Some(mgr);
}

#[given("a P256 signing key is generated")]
fn generate_p256_key(world: &mut P256World) {
    world.signing_key = Some(SigningKey::random(&mut rand::thread_rng()));
}

#[given("a registration challenge is issued")]
fn issue_challenge(world: &mut P256World) {
    let ch = world.mgr().issue_challenge().expect("issue_challenge");
    world.nonce     = ch.nonce;
    world.issued_at = ch.issued_at;
}

// ── When ─────────────────────────────────────────────────────────────────────

#[when("the watch signs the challenge with the P256 key")]
fn sign_challenge(world: &mut P256World) {
    let sk = world.signing_key.as_ref().expect("no signing key");
    let vk = sk.verifying_key();

    let mut msg = world.nonce.as_bytes().to_vec();
    msg.extend_from_slice(b"testdevice00000000000000000000001");
    msg.extend_from_slice(&world.issued_at.to_be_bytes());

    let sig: Signature = sk.sign(&msg);
    let pk_uncompressed = vk.to_encoded_point(false);
    let pk_bytes = &pk_uncompressed.as_bytes()[1..]; // strip 0x04

    let req = world.build_req(pk_bytes, &sig.to_bytes());
    world.last_req = Some(req);
}

#[when("register_device is called with a 32-byte public key")]
fn call_with_short_key(world: &mut P256World) {
    let pk  = vec![0u8; 32];
    let sig = vec![0u8; 64];
    let req = world.build_req(&pk, &sig);
    let result = world.mgr().register_device(&req)
        .map(|d| d.device_id)
        .map_err(|e| e.to_string());
    world.last_result = Some(result);
}

#[when("register_device is called with zeroed signature bytes")]
fn call_with_zero_sig(world: &mut P256World) {
    let sk = world.signing_key.as_ref().expect("no signing key");
    let vk = sk.verifying_key();
    let pk_uncompressed = vk.to_encoded_point(false);
    let pk_bytes = &pk_uncompressed.as_bytes()[1..];
    let sig = vec![0u8; 64];
    let req = world.build_req(pk_bytes, &sig);
    let result = world.mgr().register_device(&req)
        .map(|d| d.device_id)
        .map_err(|e| e.to_string());
    world.last_result = Some(result);
}

#[when("the watch signs a tampered message with the P256 key")]
fn sign_tampered_message(world: &mut P256World) {
    let sk = world.signing_key.as_ref().expect("no signing key");
    let vk = sk.verifying_key();
    let pk_uncompressed = vk.to_encoded_point(false);
    let pk_bytes = &pk_uncompressed.as_bytes()[1..];

    // Sign the wrong message
    let wrong_msg = b"this is not the nonce+device_id+timestamp";
    let sig: Signature = sk.sign(wrong_msg.as_ref());
    let req = world.build_req(pk_bytes, &sig.to_bytes());
    let result = world.mgr().register_device(&req)
        .map(|d| d.device_id)
        .map_err(|e| e.to_string());
    world.last_result = Some(result);
}

#[when("register_device succeeds")]
fn do_register(world: &mut P256World) {
    let req = world.last_req.clone().expect("no pending request");
    let result = world.mgr().register_device(&req)
        .map(|d| d.device_id)
        .map_err(|e| e.to_string());
    world.last_result = Some(result);
}

#[when("the same request is replayed")]
fn replay_register(world: &mut P256World) {
    // last_req still has the original nonce which is now consumed
    let req = world.last_req.clone().expect("no request to replay");
    let result = world.mgr().register_device(&req)
        .map(|d| d.device_id)
        .map_err(|e| e.to_string());
    world.last_result = Some(result);
}

// ── Then ─────────────────────────────────────────────────────────────────────

#[then("register_device succeeds")]
fn assert_success(world: &mut P256World) {
    let req = world.last_req.clone().expect("no pending request");
    let result = world.mgr().register_device(&req)
        .map(|d| d.device_id)
        .map_err(|e| e.to_string());
    world.last_result = Some(result.clone());
    assert!(result.is_ok(), "expected success, got: {:?}", result.err());
}

#[then("the returned device_id matches the request")]
fn assert_device_id(world: &mut P256World) {
    let result = world.last_result.as_ref().expect("no result");
    let device_id = result.as_ref().expect("expected Ok(device_id)");
    assert_eq!(device_id, "testdevice00000000000000000000001");
}

#[then(expr = "register_device fails with {string}")]
fn assert_fails_with(world: &mut P256World, needle: String) {
    let result = world.last_result.as_ref().expect("no result");
    let err = result.as_ref().expect_err("expected Err");
    assert!(
        err.to_lowercase().contains(&needle.to_lowercase()),
        "expected error containing {:?}, got: {:?}", needle, err
    );
}

// ── Runner ────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    P256World::run("tests/features/watch_p256_auth.feature").await;
}
