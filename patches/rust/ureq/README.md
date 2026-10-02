# HTMLCut maintained HTTP transport

Upstream base: ureq3.4.2, MIT OR Apache-2.0. License texts remain here.

The response receiver calls the current complete-header protocol API. It has no partial-redirect option or synthesized incomplete response. Normal finite redirect policy, transport pooling, TLS verification and timeout handling retain their upstream responsibilities. The changed response receiver lives in src/run/response.rs and is owned by HTMLCut assurance.

Response representation bytes and encoding/length headers stay intact. Content decompression and charset conversion are caller-owned; no transport feature performs them. Unit fixtures compile only under cfg(test), never through a production feature.
