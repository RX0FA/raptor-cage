use anyhow::Context;
use std::env;

/// Represents a value as used by the DISPLAY environment variable.
/// The format is: `hostname:display_number.screen_number`.
#[derive(Debug, PartialEq)]
pub struct Display {
  /// Includes unparsed hostname or IP address part of the DISPLAY value. The value is not being
  /// parsed as IpAddr because it's value can be a hostname which would require additional steps to
  /// resolve the hostname.
  pub hostname: Option<String>,
  /// The display number refers to a particular X server instance on the host, e.g. for a given
  /// `DISPLAY=:0` the display number is `0`.
  pub display_number: u32,
  /// Each display can have multiple physical or virtual screens attached to it. The screen number
  /// identifies which screen within the display should be used.
  pub screen_number: Option<u32>,
}

impl Display {
  pub fn from_str(display: &str) -> anyhow::Result<Self> {
    let parts: Vec<&str> = display.split(':').collect();
    if parts.len() != 2 {
      anyhow::bail!("invalid DISPLAY format: {}", display);
    }
    let hostname: Option<String> = if !parts[0].is_empty() {
      Some(parts[0].into())
    } else {
      None
    };
    let display_screen: Vec<&str> = parts[1].split('.').collect();
    let display_number = display_screen[0]
      .parse::<u32>()
      .with_context(|| format!("invalid display number: {}", display_screen[0]))?;
    let screen_number = if display_screen.len() > 1 {
      Some(
        display_screen[1]
          .parse::<u32>()
          .with_context(|| format!("invalid screen number: {}", display_screen[1]))?,
      )
    } else {
      None
    };
    Ok(Self {
      hostname,
      display_number,
      screen_number,
    })
  }

  pub fn from_env() -> anyhow::Result<Self> {
    match env::var("DISPLAY") {
      Ok(display) => Self::from_str(&display),
      Err(_) => anyhow::bail!("DISPLAY environment variable is not set"),
    }
  }

  /// Calculate the X11 socket path based on the display number. The display number is usually `0`
  /// when running under X11, and `1` when running under Wayland.
  pub fn get_socket_path(&self) -> String {
    format!("/tmp/.X11-unix/X{}", self.display_number)
  }

  pub fn to_display_address(&self) -> String {
    let hostname = self.hostname.as_deref().unwrap_or("");
    let screen_part = match self.screen_number {
      Some(screen) => format!(".{}", screen),
      None => String::new(),
    };
    format!("{}:{}{}", hostname, self.display_number, screen_part)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_valid_display_with_hostname() {
    let display_str = "localhost:0.0";
    let display = Display::from_str(display_str).expect("failed to parse display");
    assert_eq!(display.hostname, Some("localhost".to_string()));
    assert_eq!(display.display_number, 0);
    assert_eq!(display.screen_number, Some(0));
    assert_eq!(display.get_socket_path(), "/tmp/.X11-unix/X0");
  }

  #[test]
  fn test_valid_display_without_hostname() {
    let display_str = ":1.2";
    let display = Display::from_str(display_str).expect("failed to parse display");
    assert_eq!(display.hostname, None);
    assert_eq!(display.display_number, 1);
    assert_eq!(display.screen_number, Some(2));
    assert_eq!(display.get_socket_path(), "/tmp/.X11-unix/X1");
  }

  #[test]
  fn test_display_without_screen_number() {
    let display_str = ":2";
    let display = Display::from_str(display_str).expect("failed to parse display");
    assert_eq!(display.hostname, None);
    assert_eq!(display.display_number, 2);
    assert_eq!(display.screen_number, None);
    assert_eq!(display.get_socket_path(), "/tmp/.X11-unix/X2");
  }

  #[test]
  fn test_invalid_display_format() {
    let display_str = "invalid_format";
    let result = Display::from_str(display_str);
    assert!(result.is_err());
  }

  #[test]
  fn test_invalid_display_number() {
    let display_str = ":abc";
    let result = Display::from_str(display_str);
    assert!(result.is_err());
  }

  #[test]
  fn test_invalid_screen_number() {
    let display_str = ":0.xyz";
    let result = Display::from_str(display_str);
    assert!(result.is_err());
  }

  #[test]
  fn test_no_display_variable() {
    env::remove_var("DISPLAY");
    let result = Display::from_env();
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_local_display() {
    let display_str = ":0.0";
    let expected = Display {
      hostname: None,
      display_number: 0,
      screen_number: Some(0),
    };
    let result = Display::from_str(display_str).unwrap();
    assert_eq!(result, expected);
  }

  #[test]
  fn test_parse_remote_display() {
    let display_str = "192.168.1.100:1.1";
    let expected = Display {
      hostname: Some("192.168.1.100".to_string()),
      display_number: 1,
      screen_number: Some(1),
    };
    let result = Display::from_str(display_str).unwrap();
    assert_eq!(result, expected);
  }

  #[test]
  fn test_get_socket_path_display_0() {
    let display = Display {
      hostname: None,
      display_number: 0,
      screen_number: Some(0),
    };
    assert_eq!(display.get_socket_path(), "/tmp/.X11-unix/X0");
  }

  #[test]
  fn test_get_socket_path_display_1() {
    let display = Display {
      hostname: None,
      display_number: 1,
      screen_number: Some(0),
    };
    assert_eq!(display.get_socket_path(), "/tmp/.X11-unix/X1");
  }

  #[test]
  fn test_with_hostname_and_screen() {
    let display = Display {
      hostname: Some("localhost".to_string()),
      display_number: 0,
      screen_number: Some(0),
    };
    assert_eq!(display.to_display_address(), "localhost:0.0");
  }

  #[test]
  fn test_with_hostname_without_screen() {
    let display = Display {
      hostname: Some("localhost".to_string()),
      display_number: 1,
      screen_number: None,
    };
    assert_eq!(display.to_display_address(), "localhost:1");
  }

  #[test]
  fn test_without_hostname_with_screen() {
    let display = Display {
      hostname: None,
      display_number: 2,
      screen_number: Some(1),
    };
    assert_eq!(display.to_display_address(), ":2.1");
  }

  #[test]
  fn test_without_hostname_and_screen() {
    let display = Display {
      hostname: None,
      display_number: 3,
      screen_number: None,
    };
    assert_eq!(display.to_display_address(), ":3");
  }
}
