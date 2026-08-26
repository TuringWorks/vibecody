//! The `webrtc-rs` answering peer — the daemon's side of the transport test.
//!
//! It relays every RTP packet it receives straight back out on an outbound
//! track, so a single run proves both directions: webview→daemon (what
//! streaming ASR needs) and daemon→webview (what streaming TTS needs).

use std::sync::{Arc, Mutex};
use std::time::Instant;

use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtp_transceiver::rtp_codec::{RTCRtpCodecCapability, RTPCodecType};
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::{TrackLocal, TrackLocalWriter};

#[derive(Default)]
pub struct Obs {
    pub ice_states: Vec<String>,
    pub packets_in: u64,
    pub bytes_in: u64,
    pub first_packet_ms: Option<u128>,
    pub relayed_out: u64,
    pub remote_candidates: Vec<String>,
    pub connected_ms: Option<u128>,
    pub error: Option<String>,
}

pub type Shared = Arc<Mutex<Obs>>;

pub fn snapshot(o: &Obs) -> serde_json::Value {
    serde_json::json!({
        "ice_states": o.ice_states,
        "packets_in": o.packets_in,
        "bytes_in": o.bytes_in,
        "first_packet_ms": o.first_packet_ms,
        "relayed_out": o.relayed_out,
        "remote_candidates": o.remote_candidates,
        "connected_ms": o.connected_ms,
        "error": o.error,
    })
}

pub async fn answer(offer_sdp: String, obs: Shared, t0: Instant) -> anyhow::Result<String> {
    let mut m = MediaEngine::default();
    m.register_default_codecs()?;
    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut m)?;
    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    // No STUN. If this connects it connected on host + peer-reflexive
    // candidates alone, which is exactly the property under test.
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer::default()],
        ..Default::default()
    };
    let pc = Arc::new(api.new_peer_connection(config).await?);

    let out = Arc::new(TrackLocalStaticRTP::new(
        RTCRtpCodecCapability { mime_type: "audio/opus".to_owned(), ..Default::default() },
        "relay".to_owned(),
        "gv1c".to_owned(),
    ));
    pc.add_track(Arc::clone(&out) as Arc<dyn TrackLocal + Send + Sync>).await?;
    pc.add_transceiver_from_kind(RTPCodecType::Audio, None).await?;

    {
        let obs = Arc::clone(&obs);
        pc.on_ice_connection_state_change(Box::new(move |s: RTCIceConnectionState| {
            let obs = Arc::clone(&obs);
            let ms = t0.elapsed().as_millis();
            Box::pin(async move {
                if let Ok(mut o) = obs.lock() {
                    o.ice_states.push(format!("{s}@{ms}ms"));
                    if s == RTCIceConnectionState::Connected && o.connected_ms.is_none() {
                        o.connected_ms = Some(ms);
                    }
                }
            })
        }));
    }
    {
        let obs = Arc::clone(&obs);
        let out = Arc::clone(&out);
        pc.on_track(Box::new(move |track, _recv, _trans| {
            let obs = Arc::clone(&obs);
            let out = Arc::clone(&out);
            Box::pin(async move {
                tokio::spawn(async move {
                    while let Ok((pkt, _)) = track.read_rtp().await {
                        let n = pkt.payload.len() as u64;
                        if let Ok(mut o) = obs.lock() {
                            o.packets_in += 1;
                            o.bytes_in += n;
                            if o.first_packet_ms.is_none() {
                                o.first_packet_ms = Some(t0.elapsed().as_millis());
                            }
                        }
                        if out.write_rtp(&pkt).await.is_ok() {
                            if let Ok(mut o) = obs.lock() { o.relayed_out += 1; }
                        }
                    }
                });
            })
        }));
    }

    let offer = RTCSessionDescription::offer(offer_sdp)?;
    if let Ok(mut o) = obs.lock() {
        for line in offer.sdp.lines().filter(|l| l.starts_with("a=candidate")) {
            o.remote_candidates.push(line.to_string());
        }
    }
    pc.set_remote_description(offer).await?;
    let a = pc.create_answer(None).await?;
    let mut gather = pc.gathering_complete_promise().await;
    pc.set_local_description(a).await?;
    let _ = gather.recv().await;
    let local = pc
        .local_description()
        .await
        .ok_or_else(|| anyhow::anyhow!("no local description"))?;

    // The connection must outlive this request handler.
    std::mem::forget(pc);
    Ok(local.sdp)
}
