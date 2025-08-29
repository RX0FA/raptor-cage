use rand::Rng;
use std::fmt;
use std::str::FromStr;

// Minimum id to use when generating a random UID/GID; usually system accounts use from 0 to 999,
// normal users start from 1000 and so on, and other applications like Docker use ranges up to
// 296608 (231072 + 65536), so a safe bet is to use values >= 300_000.
const MIN_ID: u32 = 300_000;
// On many Linux systems, even though the maximum UID/GID is an u32, we are only allowed to use ids
// up to 2147483647 (the max value of i32), otherwise namespace creation may fail.
// This cast simply performs a conversion without checking for overflow, and for this particular
// scenario is safe.
const MAX_ID: u32 = i32::MAX as u32;

#[derive(Debug)]
pub enum UserMappingError {
  InvalidFormat(String),
  InvalidId(String),
  OutOfRangeId(u32),
}

// Yeah, yeah, thiserror could be used, it just doesn't feel good to add an extra crate to avoid
// writing ~8 lines; thiserror can be used if we start implementing lots of custom errors.
impl fmt::Display for UserMappingError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      UserMappingError::InvalidFormat(s) => write!(f, "Expected UID:GID format, but got: {}", s),
      UserMappingError::InvalidId(s) => write!(f, "Input is not a valid ID: {}", s),
      UserMappingError::OutOfRangeId(uid) => write!(
        f,
        "Value must be between {} and {}, but got {}",
        MIN_ID, MAX_ID, uid
      ),
    }
  }
}

impl std::error::Error for UserMappingError {}

fn parse_id(id: &str) -> Result<u32, UserMappingError> {
  let value = id
    .parse::<u32>()
    .map_err(|_| UserMappingError::InvalidId(id.to_string()))?;
  if value < MIN_ID || value > MAX_ID {
    return Err(UserMappingError::OutOfRangeId(value));
  }
  Ok(value)
}

/// Represents a user mapping configuration.
#[derive(Debug, Clone, Copy)]
pub enum UserMapping {
  /// Use random user and group ids.
  Random,
  /// Explicitly specify user and group ids.
  Custom(u32, u32),
}

impl UserMapping {
  pub fn get_uid_gid(&self) -> (u32, u32) {
    match self {
      UserMapping::Random => {
        let mut rng = rand::thread_rng();
        let random_uid = rng.gen_range(MIN_ID..=MAX_ID);
        let random_gid = rng.gen_range(MIN_ID..=MAX_ID);
        (random_uid, random_gid)
      }
      UserMapping::Custom(uid, gid) => (*uid, *gid),
    }
  }
  pub fn get_uid_gid_string(&self) -> (String, String) {
    let (uid, gid) = self.get_uid_gid();
    (uid.to_string(), gid.to_string())
  }
}

impl FromStr for UserMapping {
  type Err = UserMappingError;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if s.eq_ignore_ascii_case("random") {
      return Ok(UserMapping::Random);
    }
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
      return Err(UserMappingError::InvalidFormat(s.to_string()));
    }
    let uid = parse_id(parts[0])?;
    let gid = parse_id(parts[1])?;
    Ok(UserMapping::Custom(uid, gid))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_random_user_mapping() {
    let mapping = UserMapping::Random;
    let (uid, gid) = mapping.get_uid_gid();
    assert!(uid >= MIN_ID && uid <= MAX_ID);
    assert!(gid >= MIN_ID && gid <= MAX_ID);
  }

  #[test]
  fn test_custom_user_mapping() {
    let mapping = UserMapping::Custom(500_000, 600_000);
    let (uid, gid) = mapping.get_uid_gid();
    assert_eq!(uid, 500_000);
    assert_eq!(gid, 600_000);
  }

  #[test]
  fn test_user_mapping_from_str_random() {
    let mapping: UserMapping = "random".parse().unwrap();
    assert!(matches!(mapping, UserMapping::Random));
  }

  #[test]
  fn test_user_mapping_from_str_custom() {
    let mapping: UserMapping = "500000:600000".parse().unwrap();
    if let UserMapping::Custom(uid, gid) = mapping {
      assert_eq!(uid, 500_000);
      assert_eq!(gid, 600_000);
    } else {
      panic!("Expected custom mapping");
    }
  }
}
