//! Preserve current transport errors and typed TLS causes through the I/O boundary.
use super::*;

fn is_wrapped_ureq_error(error: &io::Error) -> bool {
    error.get_ref().is_some_and(|cause| cause.is::<Error>())
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        if is_wrapped_ureq_error(&error) {
            // Exact type identity proves the consuming downcast of this immutable error.
            return *error.into_inner().unwrap().downcast::<Error>().unwrap();
        }
        #[cfg(feature = "_rustls")]
        if error
            .get_ref()
            .is_some_and(|cause| cause.is::<rustls::Error>())
        {
            return Error::Rustls(
                *error
                    .into_inner()
                    .unwrap()
                    .downcast::<rustls::Error>()
                    .unwrap(),
            );
        }
        Error::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_and_unrelated_io_causes_keep_their_real_kind() {
        for source in [
            io::Error::from(io::ErrorKind::PermissionDenied),
            io::Error::new(io::ErrorKind::InvalidData, "synthetic non-TLS data failure"),
        ] {
            let expected = source.kind();
            let converted = Error::from(source);
            assert_eq!(converted.into_io().kind(), expected);
        }
    }

    #[test]
    fn wrapped_current_protocol_error_preserves_its_variant_and_value() {
        let source = io::Error::other(Error::Protocol(ureq_proto::Error::HttpParseFail(
            "synthetic framing".into(),
        )));
        let converted = Error::from(source);
        let expected =
            Error::Protocol(ureq_proto::Error::HttpParseFail("synthetic framing".into()));
        assert_eq!(format!("{converted:?}"), format!("{expected:?}"));
    }

    #[cfg(feature = "_rustls")]
    #[test]
    fn certificate_failure_inside_io_remains_a_typed_tls_error() {
        let source = io::Error::new(
            io::ErrorKind::InvalidData,
            rustls::Error::InvalidCertificate(rustls::CertificateError::NotValidForName),
        );
        let converted = Error::from(source);
        assert!(matches!(
            converted,
            Error::Rustls(rustls::Error::InvalidCertificate(
                rustls::CertificateError::NotValidForName
            ))
        ));
    }
}
