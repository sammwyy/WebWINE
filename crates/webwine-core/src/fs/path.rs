use crate::error::{Result, VmError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestPath {
    pub drive: char,
    pub components: Vec<String>,
}

impl GuestPath {
    pub fn parse(raw: &str) -> Result<Self> {
        let normalized = raw.replace('/', "\\");
        let s = normalized.trim_end_matches('\\');

        // Drive-rooted path: C:\...
        if s.len() >= 2 && s.chars().nth(1) == Some(':') {
            let drive = s
                .chars()
                .next()
                .unwrap()
                .to_ascii_uppercase();

            if !drive.is_ascii_alphabetic() {
                return Err(VmError::Path(format!("invalid drive letter in '{raw}'")));
            }

            let rest = if s.len() > 3 {
                &s[3..]
            } else if s.len() == 3 {
                ""
            } else if s.len() == 2 {
                ""
            } else {
                ""
            };

            let components = Self::split_components(rest);
            return Ok(GuestPath { drive, components });
        }

        Err(VmError::Path(format!(
            "relative paths not supported at this layer: '{raw}'"
        )))
    }

    fn split_components(rest: &str) -> Vec<String> {
        if rest.is_empty() {
            return vec![];
        }
        rest.split('\\')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    pub fn display(&self) -> String {
        if self.components.is_empty() {
            return format!("{}:\\", self.drive);
        }
        format!("{}:\\{}", self.drive, self.components.join("\\"))
    }

    pub fn file_name(&self) -> Option<&str> {
        self.components.last().map(|s| s.as_str())
    }

    pub fn parent(&self) -> Option<GuestPath> {
        if self.components.is_empty() {
            return None;
        }
        let mut p = self.clone();
        p.components.pop();
        Some(p)
    }

    pub fn join(&self, name: &str) -> GuestPath {
        let mut p = self.clone();
        p.components.push(name.to_string());
        p
    }
}

impl std::fmt::Display for GuestPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_path() {
        let p = GuestPath::parse("C:\\Users\\guest\\Desktop\\demo.exe").unwrap();
        assert_eq!(p.drive, 'C');
        assert_eq!(p.components, ["Users", "guest", "Desktop", "demo.exe"]);
    }

    #[test]
    fn normalizes_forward_slashes() {
        let p = GuestPath::parse("C:/Users/guest/Desktop/demo.exe").unwrap();
        assert_eq!(p.drive, 'C');
        assert_eq!(p.components, ["Users", "guest", "Desktop", "demo.exe"]);
    }

    #[test]
    fn parses_drive_root() {
        let p = GuestPath::parse("C:\\").unwrap();
        assert_eq!(p.drive, 'C');
        assert!(p.components.is_empty());
    }
}
