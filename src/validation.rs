use crate::announcements::Announcement;
use crate::vrps::ValidatedRoaPayload;
use std::fmt::Display;
use std::fmt;


//------------ ValidationState ----------------------------------------------

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum ValidationState {
    Valid,
    InvalidAsn,
    InvalidLength,
    NotFound
}

impl Display for ValidationState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            ValidationState::Valid => "valid",
            ValidationState::InvalidAsn => "invalid asn",
            ValidationState::InvalidLength => "invalid length",
            ValidationState::NotFound => "not found"
        };

        write!(f, "{}", msg)
    }
}


//------------ ValidatedAnnouncement -----------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct ValidatedAnnouncement {
    announcement: Announcement,
    state: ValidationState
}

impl ValidatedAnnouncement {
    pub fn state(&self) -> &ValidationState {
        &self.state
    }

    fn derive_state(
        ann: &Announcement,
        vrps: &[&ValidatedRoaPayload]
    ) -> ValidationState {
        let mut state = ValidationState::NotFound;

        for vrp in vrps {
            if vrp.contains(ann.as_ref()) {
                if vrp.asn() != ann.asn() {
                    if state != ValidationState::InvalidLength {
                        state = ValidationState::InvalidAsn;
                    }
                    continue
                }

                if ann.prefix().length() > vrp.max_length() {
                    state = ValidationState::InvalidLength;
                    continue
                }

                return ValidationState::Valid
            }
        }

        state
    }

    /// Creates a validated announcement for the referenced announcement, and
    /// validated roa prefixes. Takes references because this stuff is kept
    /// in immutable IntervalTree structures.
    pub fn create(ann: &Announcement, vrps: &[&ValidatedRoaPayload]) -> Self {
        let state = Self::derive_state(ann, vrps);

        ValidatedAnnouncement {
            announcement: ann.clone(),
            state
        }
    }
}

impl Display for ValidatedAnnouncement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.announcement.asn(),
            self.announcement.prefix(),
            self.state
        )
    }
}


//------------ RoaImpact -----------------------------------------------------

pub struct VrpImpact {
    unseen: bool
}

impl VrpImpact {
    pub fn evaluate(vrp: &ValidatedRoaPayload, anns: &[&Announcement]) -> Self {
        for ann in anns {
            if vrp.asn() == ann.asn()
               && vrp.contains(ann.prefix().as_ref())
               && vrp.max_length() >= ann.prefix().length() {
                return VrpImpact { unseen: false }
            }
        }
        VrpImpact { unseen: true }
    }

    pub fn is_unseen(&self) -> bool {
        self.unseen
    }
}


//------------ Tests --------------------------------------------------------

#[cfg(test)]
mod tests {

    use super::*;
    use std::str::FromStr;

    fn vrp(s: &str) -> ValidatedRoaPayload {
        ValidatedRoaPayload::from_str(s).unwrap()
    }

    fn ann(s: &str) -> Announcement {
        Announcement::from_str(s).unwrap()
    }


    #[test]
    fn should_validate_announcement() {
        let ann = ann("65000, 192.168.0.0/20");

        let vrp_valid   = vrp("AS65000, 192.168.0.0/20, 20");
        let vrp_inv_len = vrp("AS65000, 192.168.0.0/16, 16");
        let vrp_inv_asn = vrp("AS65001, 192.168.0.0/16, 20");
        let vrp_not_fnd = vrp("AS65000, 192.168.0.0/24, 24");

        {
            // not found
            let validated = ValidatedAnnouncement::create(
                &ann,
                &[&vrp_not_fnd]
            );
            assert_eq!(&ValidationState::NotFound, validated.state());
        }

        {
            // invalid_len
            let validated = ValidatedAnnouncement::create(
                &ann,
                &[&vrp_inv_len, &vrp_inv_asn, &vrp_not_fnd]
            );
            assert_eq!(&ValidationState::InvalidLength, validated.state());
        }

        {
            // invalid asn
            let validated = ValidatedAnnouncement::create(
                &ann,
                &[&vrp_inv_asn, &vrp_not_fnd]
            );
            assert_eq!(&ValidationState::InvalidAsn, validated.state());
        }

        {
            // valid
            let validated = ValidatedAnnouncement::create(
                &ann,
                &[&vrp_inv_len, &vrp_inv_asn, &vrp_not_fnd, &vrp_valid]
            );
            assert_eq!(&ValidationState::Valid, validated.state());
        }
    }

    #[test]
    fn should_detect_staleness() {
        let vrp_current = vrp("AS65000, 192.168.0.0/20, 20");
        let vrp_stale   = vrp("AS65000, 192.168.16.0/20, 20");

        let ann1 = ann("65000, 192.168.0.0/20");
        let ann2 = ann("65000, 192.168.16.0/24");

        assert!(!VrpImpact::evaluate(&vrp_current, &[&ann1, &ann2]).is_unseen());
        assert!(VrpImpact::evaluate(&vrp_stale, &[&ann1, &ann2]).is_unseen());
    }
}