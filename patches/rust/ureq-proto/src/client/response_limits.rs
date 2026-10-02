//! Per-hop finite accounting at complete and pending response-header boundaries.

use crate::Error;

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_INFORMATIONAL: u32 = 32;

#[derive(Debug, Default)]
pub(crate) struct ResponseLimits {
    bytes: usize,
    informational: u32,
}

impl ResponseLimits {
    pub(crate) fn pending(&self, bytes: usize) -> Result<(), Error> {
        if bytes > MAX_HEADER_BYTES.saturating_sub(self.bytes) {
            Err(Error::ResponseHeaderLimit)
        } else {
            Ok(())
        }
    }

    pub(crate) fn complete(&mut self, bytes: usize, informational: bool) -> Result<(), Error> {
        self.pending(bytes)?;
        self.bytes += bytes;
        if informational {
            if self.informational == MAX_INFORMATIONAL {
                return Err(Error::InformationalResponseLimit);
            }
            self.informational += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_retries_do_not_double_charge_consumed_headers() {
        let mut limits = ResponseLimits::default();
        limits.complete(1024, true).unwrap();
        for _ in 0..3 {
            limits.pending(65_536 - 1024).unwrap();
        }
        assert_eq!(
            limits.pending(65_536 - 1023),
            Err(Error::ResponseHeaderLimit)
        );
        limits.complete(65_536 - 1024, false).unwrap();
        assert_eq!(limits.complete(1, false), Err(Error::ResponseHeaderLimit));
    }

    #[test]
    fn repeated_informational_responses_refuse_the_first_excess() {
        let mut limits = ResponseLimits::default();
        for _ in 0..32 {
            limits.complete(20, true).unwrap();
        }
        assert_eq!(
            limits.complete(20, true),
            Err(Error::InformationalResponseLimit)
        );
        assert_eq!(limits.informational, 32);
    }
}
