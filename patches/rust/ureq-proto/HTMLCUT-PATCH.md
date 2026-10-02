# Bounded HTTP response protocol

Based on upstream ureq-proto 0.6.4, MIT/Apache-2.0. The canonical package name is retained because Cargo transitive patch resolution requires it; the local package is unpublished and carries build provenance metadata. Own complete informational-header consumption and finite response-header accounting. TLS, socket I/O and routing remain in ureq. Application diagnostics never serialize dependency error chains.
