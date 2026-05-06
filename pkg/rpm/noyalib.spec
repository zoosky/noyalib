# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# RPM spec for the noyalib CLI package (`noyafmt`, `noyavalidate`).
# Used by Fedora / RHEL / openSUSE packagers either directly via
# `rpmbuild -ba pkg/rpm/noyalib.spec` or via the
# `cargo-generate-rpm` helper invoked from `release-binaries.yml`
# on Linux gnu legs.

Name:           noyalib
Version:        @@VERSION@@
Release:        1%{?dist}
Summary:        Pure-Rust YAML 1.2 parser, formatter, and validator
License:        MIT OR Apache-2.0
URL:            https://github.com/sebastienrousseau/noyalib
Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz
BuildRequires:  rust >= 1.75
BuildRequires:  cargo
BuildRequires:  asciidoctor

%description
noyalib provides the noyafmt and noyavalidate command-line tools
backed by a pure-Rust YAML 1.2 implementation with full serde
integration and zero `unsafe` code.

noyafmt rewrites YAML files through a lossless CST formatter that
preserves comments, anchor positions, and document structure
byte-for-byte while normalising whitespace and quoting.

noyavalidate checks YAML syntax and (optionally) enforces a
JSON Schema 2020-12 contract against parsed documents.

%package debuginfo
Summary:        Debug information for noyalib
Requires:       %{name}%{?_isa} = %{version}-%{release}

%description debuginfo
Debug symbols for the noyafmt and noyavalidate binaries shipped
by the `noyalib` package.

%prep
%autosetup -n noyalib-%{version}

%build
# Reproducible-build flags. SOURCE_DATE_EPOCH is set by rpmbuild
# from the spec's last changelog entry; we honour it.
export RUSTFLAGS='--remap-path-prefix=%{_builddir}=. --remap-path-prefix=%{_topdir}=. -C strip=none'
export NOYA_GEN_ASSETS=1
cargo build --release --locked \
    --manifest-path crates/noya-cli/Cargo.toml \
    --bin noyafmt --bin noyavalidate

%install
install -Dm755 target/release/noyafmt        %{buildroot}%{_bindir}/noyafmt
install -Dm755 target/release/noyavalidate   %{buildroot}%{_bindir}/noyavalidate
install -Dm644 doc/noyafmt.1                 %{buildroot}%{_mandir}/man1/noyafmt.1
install -Dm644 doc/noyavalidate.1            %{buildroot}%{_mandir}/man1/noyavalidate.1
install -Dm644 complete/noyafmt.bash         %{buildroot}%{_datadir}/bash-completion/completions/noyafmt
install -Dm644 complete/noyavalidate.bash    %{buildroot}%{_datadir}/bash-completion/completions/noyavalidate
install -Dm644 complete/noyafmt.fish         %{buildroot}%{_datadir}/fish/vendor_completions.d/noyafmt.fish
install -Dm644 complete/noyavalidate.fish    %{buildroot}%{_datadir}/fish/vendor_completions.d/noyavalidate.fish
install -Dm644 complete/_noyafmt             %{buildroot}%{_datadir}/zsh/site-functions/_noyafmt
install -Dm644 complete/_noyavalidate        %{buildroot}%{_datadir}/zsh/site-functions/_noyavalidate

%files
%license LICENSE-MIT LICENSE-APACHE
%doc README.md CHANGELOG.md
%{_bindir}/noyafmt
%{_bindir}/noyavalidate
%{_mandir}/man1/noyafmt.1*
%{_mandir}/man1/noyavalidate.1*
%{_datadir}/bash-completion/completions/noya*
%{_datadir}/fish/vendor_completions.d/noya*
%{_datadir}/zsh/site-functions/_noya*

%files debuginfo
%defattr(-,root,root,-)

%changelog
* @@DATE@@ Sebastien Rousseau <sebastian.rousseau@gmail.com> - @@VERSION@@-1
- Release @@VERSION@@. See upstream CHANGELOG.md for details.
