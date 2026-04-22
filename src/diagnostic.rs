//! Spanned value to `miette::Report` bridge.
//!
//! Converts validation errors on [`Spanned<T>`](crate::Spanned) values
//! into rich [`miette::Report`] diagnostics with source spans, so that
//! CLI tools get underlined error output for free.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use crate::spanned::Spanned;
use core::fmt;

/// A diagnostic error tied to a source span.
///
/// Implements [`miette::Diagnostic`] so it renders with a highlighted
/// source region in terminals that support it.
#[derive(Debug)]
struct SpannedDiagnostic {
    message: String,
    span: miette::SourceSpan,
    source_code: String,
}

impl fmt::Display for SpannedDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SpannedDiagnostic {}

impl miette::Diagnostic for SpannedDiagnostic {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new("noyalib::validation"))
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.source_code)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        Some(Box::new(core::iter::once(miette::LabeledSpan::new(
            Some(self.message.clone()),
            self.span.offset(),
            self.span.len(),
        ))))
    }
}

/// Create a [`miette::Report`] pointing at the source span of a
/// [`Spanned<T>`](crate::Spanned) value.
///
/// This bridges noyalib's source-location tracking with miette's rich
/// terminal diagnostic output, letting users validate deserialized
/// config and report errors that point back to the exact YAML source.
///
/// # Example
///
/// ```rust,no_run
/// use noyalib::{from_str, Spanned};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Cfg { port: Spanned<u16> }
///
/// let yaml = "port: 80";
/// let cfg: Cfg = from_str(yaml).unwrap();
/// if cfg.port.value < 1024 {
///     let report = noyalib::diagnostic::spanned_error(
///         yaml, &cfg.port, "port must be >= 1024",
///     );
///     eprintln!("{report:?}");
/// }
/// ```
pub fn spanned_error<T, M: fmt::Display>(
    source: &str,
    span: &Spanned<T>,
    message: M,
) -> miette::Report {
    let start = span.start.index();
    let end = span.end.index();
    let len = if end > start { end - start } else { 1 };

    miette::Report::new(SpannedDiagnostic {
        message: message.to_string(),
        span: miette::SourceSpan::from(start..start + len),
        source_code: source.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Spanned;

    #[test]
    fn spanned_error_creates_report() {
        let yaml = "port: 80\n";
        #[derive(serde::Deserialize)]
        struct Cfg {
            port: Spanned<u16>,
        }
        let cfg: Cfg = crate::from_str(yaml).unwrap();
        let report = spanned_error(yaml, &cfg.port, "port must be >= 1024");
        let msg = format!("{report}");
        assert!(msg.contains("port must be >= 1024"));
    }

    #[test]
    fn spanned_error_diagnostic_has_labels() {
        use miette::Diagnostic;

        let yaml = "value: 42\n";
        #[derive(serde::Deserialize)]
        struct Cfg {
            value: Spanned<i32>,
        }
        let cfg: Cfg = crate::from_str(yaml).unwrap();
        let report = spanned_error(yaml, &cfg.value, "too small");

        // The underlying diagnostic should have labels.
        let diag: &dyn Diagnostic = report.as_ref();
        assert!(diag.labels().is_some());
        assert!(diag.code().is_some());
        assert!(diag.source_code().is_some());
    }
}
