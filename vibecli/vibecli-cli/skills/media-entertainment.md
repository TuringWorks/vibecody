---
triggers: ["media", "streaming", "content management", "CMS", "DAM", "digital asset", "video transcoding", "CDN", "DRM", "ad tech", "programmatic advertising", "OTT"]
tools_allowed: ["read_file", "write_file", "bash"]
category: media
---

# Media & Entertainment Systems

When working with media, streaming, and entertainment platforms:

1. Build video transcoding pipelines using FFmpeg or managed services (AWS MediaConvert, GCP Transcoder API) that accept mezzanine-quality source files, produce multiple output renditions (resolutions from 360p to 4K, bitrate ladders per codec), and package into adaptive streaming formats; use a job queue (SQS, RabbitMQ) to manage transcoding workloads, implement progress callbacks, and store outputs in object storage with a predictable key structure keyed by content ID and profile.

2. Implement adaptive bitrate streaming using HLS (HTTP Live Streaming) and DASH (Dynamic Adaptive Streaming over HTTP) by generating multi-variant playlists with per-rendition bandwidth declarations; segment media into 2-6 second chunks for low-latency startup, include I-frame-only playlists for trick-play (scrubbing/thumbnails), and set appropriate `EXT-X-TARGETDURATION` and `EXT-X-MEDIA-SEQUENCE` values for live streaming use cases.

3. Configure CDN (Content Delivery Network) distribution with cache-control headers optimized for media workloads: long TTLs on immutable segment files, short TTLs on live manifests, signed URLs or signed cookies for access control; implement cache invalidation via API when content is unpublished or replaced, use origin shield to reduce load on storage backends, and monitor cache hit ratios per content tier.

4. Implement DRM (Digital Rights Management) with multi-DRM support covering Widevine (Chrome, Android), FairPlay (Safari, iOS), and PlayReady (Edge, Xbox); use a centralized key server (or a DRM-as-a-service provider) to issue content encryption keys, encrypt content using CENC (Common Encryption) for cross-DRM compatibility, configure license policies (rental periods, offline playback windows, output protection levels), and handle license renewal for long-running sessions.

5. Architect CMS (Content Management System) and DAM (Digital Asset Management) systems with a metadata-first approach: define a rich content schema (title, synopsis, cast, genres, ratings, territories, rights windows), support hierarchical content models (series > season > episode), store binary assets in object storage with DAM managing renditions and derivatives, and implement a workflow engine for editorial review, approval, and publication stages.

6. Build content recommendation engines using collaborative filtering (user-item interaction matrices), content-based features (genre, cast, director, tags), and hybrid approaches; train models on implicit signals (watch time, completion rate, replay) rather than just explicit ratings, implement cold-start strategies for new users (popularity-based) and new content (content-feature-based), and serve recommendations with sub-100ms latency via a feature store and pre-computed candidate lists.

7. Implement server-side ad insertion (SSAI) by stitching ad segments into the streaming manifest at the CDN edge or origin, using VAST/VMAP ad responses from the ad decision server; maintain session state to track ad pod positions, enforce frequency capping, handle mid-roll ad breaks at scene boundaries using SCTE-35 markers, and report ad impressions and quartile completion events back to the ad server for billing reconciliation.

8. Integrate programmatic advertising systems using the OpenRTB (Real-Time Bidding) protocol: build or integrate a Supply-Side Platform (SSP) that sends bid requests with inventory metadata (content genre, viewer demographics, device type), evaluate bid responses within the auction timeout (typically 100-200ms), apply floor prices and advertiser block lists, and implement header bidding for client-side demand competition alongside server-side exchange calls.

9. Build audience analytics pipelines that capture playback events (play, pause, seek, buffer, quality-switch, error) from client-side players, ingest them via a streaming platform (Kafka/Kinesis), sessionize events by viewer and content, and compute engagement metrics (average watch duration, completion rate, concurrent viewers, churn indicators); materialize dashboards for content performance, audience segmentation, and A/B test analysis.

10. Manage content rights by modeling rights windows with territory, platform, date-range, and exclusivity dimensions; enforce rights checks at playback time (geo-IP + platform detection), automate content availability and takedown based on window start/end dates, integrate with rights management databases for contract-level tracking, and generate royalty reports based on actual viewership per territory and window.

11. Design live streaming infrastructure using RTMP or SRT ingest from encoders to a media server cluster, transcode into adaptive bitrate ladders in real time, and distribute via CDN with low-latency configurations (LL-HLS with partial segments, or CMAF with chunked transfer encoding); implement redundant ingest paths with automatic failover, DVR/timeshift capability using a rolling segment window, and real-time stream health monitoring (bitrate stability, keyframe interval, audio sync).

12. Build playlist and catalog management systems that support editorial curation (featured rows, themed collections, continue-watching rails), algorithmic ranking within rows, and personalized ordering per user; implement a catalog API that supports faceted search, filtering by availability and rights, and efficient pagination for browse experiences; cache catalog responses per user segment to balance personalization with CDN efficiency.

13. Optimize client-side video player performance by implementing buffer management strategies (target buffer length, rebuffer recovery), preloading initial segments before playback start, selecting an appropriate initial bitrate based on connection speed estimation, and instrumenting quality-of-experience (QoE) metrics (time-to-first-frame, rebuffer ratio, average bitrate) that feed into server-side analytics for continuous optimization.

14. Handle content moderation and compliance by integrating automated content analysis (nudity detection, profanity filtering, copyright fingerprinting via Content ID or Audible Magic) into the ingest pipeline, routing flagged content to human review queues, applying regional content ratings (MPAA, BBFC, FSK), and enforcing parental controls with PIN-protected maturity filters in the player application.
