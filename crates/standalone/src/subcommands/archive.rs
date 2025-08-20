use std::{fmt, str::FromStr};

/// Destination for archiving: `s3://<bucket>/<optional-prefix>/`
/// If non-empty, prefix ends with "/".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveUrl {
    pub bucket: String,
    pub prefix: String,
}

impl fmt::Display for ArchiveUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.prefix.is_empty() {
            write!(f, "s3://{}", self.bucket)
        } else {
            write!(f, "s3://{}/{}", self.bucket, self.prefix)
        }
    }
}

impl FromStr for ArchiveUrl {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let rest = raw
            .strip_prefix("s3://")
            .ok_or_else(|| "expected URL starting with s3://".to_string())?;

        let (bucket, raw_prefix) = match rest.split_once('/') {
            Some((b, p)) => (b, p),
            None => (rest, ""),
        };

        validate_bucket(bucket)?;

        let mut prefix = raw_prefix.trim_start_matches('/').to_string();
        if !prefix.is_empty() && !prefix.ends_with('/') {
            prefix.push('/');
        }

        Ok(ArchiveUrl {
            bucket: bucket.to_string(),
            prefix,
        })
    }
}

fn validate_bucket(b: &str) -> Result<(), String> {
    let len = b.len();
    if !(3..=63).contains(&len) {
        return Err("bucket name must be 3..=63 characters".to_string());
    }
    let bytes = b.as_bytes();
    let first = *bytes.first().unwrap();
    let last = *bytes.last().unwrap();
    let is_alnum = |c: u8| c.is_ascii_digit() || c.is_ascii_lowercase();
    if !is_alnum(first) || !is_alnum(last) {
        return Err("bucket must start and end with a letter or digit".to_string());
    }
    for &c in bytes {
        let ok = is_alnum(c) || c == b'-' || c == b'.';
        if !ok {
            return Err("bucket may contain only lowercase letters, digits, '.' or '-'".to_string());
        }
    }
    if b.contains("..") {
        return Err("bucket may not contain consecutive '.' characters".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{error::ErrorKind, value_parser, Arg, Command};

    #[test]
    fn parses_bucket_only() {
        let v: ArchiveUrl = "s3://my-bucket".parse().unwrap();
        assert_eq!(v.bucket, "my-bucket");
        assert_eq!(v.prefix, "");
        assert_eq!(v.to_string(), "s3://my-bucket");
    }

    #[test]
    fn normalizes_prefix() {
        let v1: ArchiveUrl = "s3://bkt/prod".parse().unwrap();
        assert_eq!(v1.prefix, "prod/");
        let v2: ArchiveUrl = "s3://bkt/prod/cluster".parse().unwrap();
        assert_eq!(v2.prefix, "prod/cluster/");
        let v3: ArchiveUrl = "s3://bkt/prod/cluster/".parse().unwrap();
        assert_eq!(v3.prefix, "prod/cluster/");
    }

    #[test]
    fn rejects_bad_scheme_and_bucket() {
        for bad in [
            "http://bkt/foo",
            "s3:/bkt/foo",
            "s3://",
            "s3://Bad",
            "s3://my_bucket",
            "s3://-badstart",
            "s3://badend-",
            "s3://a..b",
            "s3://a",
        ] {
            assert!(bad.parse::<ArchiveUrl>().is_err(), "should fail: {bad}");
        }
    }

    #[test]
    fn clap_value_parser_reports_errors() {
        let cmd = Command::new("test").arg(
            Arg::new("archive-url")
                .long("archive-url")
                .value_parser(value_parser!(ArchiveUrl)),
        );

        let ok = cmd
            .clone()
            .try_get_matches_from(["test", "--archive-url", "s3://ok-bkt/path"])
            .unwrap();
        let parsed = ok.get_one::<ArchiveUrl>("archive-url").unwrap();
        assert_eq!(parsed.bucket, "ok-bkt");
        assert_eq!(parsed.prefix, "path/");

        let err = cmd
            .try_get_matches_from(["test", "--archive-url", "s3://Bad/"])
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ValueValidation);
    }
}
