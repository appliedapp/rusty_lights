Name:           rusty_lights
Version:        0.9.0
Release:        1%{?dist}
Summary:        Resource-efficient audio-to-LED visualization

License:        GPL-3.0-only
URL:            https://github.com/appliedapp/rusty_lights
Source0:        %{name}-%{version}.tar.gz
Source1:        %{name}-%{version}-vendor.tar.zst

BuildRequires:  rust >= 1.85
BuildRequires:  cargo
BuildRequires:  pkgconfig(libpipewire-0.3)
BuildRequires:  alsa-lib-devel
BuildRequires:  systemd-rpm-macros
BuildRequires:  gcc
BuildRequires:  clang-devel
BuildRequires:  zstd

Requires:       alsa-lib
Requires:       pipewire-libs
%{?systemd_requires}

%description
RustyLights is a resource-efficient audio-to-LED visualization tool written
in Rust. It captures audio via PipeWire or ALSA, performs real-time DSP
analysis, and drives addressable LED fixtures over E1.31 (sACN) or via
WLED-compatible devices.

%prep
%autosetup -n %{name}-%{version}
# Unpack vendored crates and configure cargo to use them
tar -xf %{SOURCE1}
mkdir -p .cargo
cat > .cargo/config.toml <<'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

%build
cargo build --release --offline

%install
install -Dpm 0755 target/release/%{name} %{buildroot}%{_bindir}/%{name}
install -Dpm 0644 rusty_lights.example.toml %{buildroot}%{_sysconfdir}/%{name}.conf
install -Dpm 0644 debian/%{name}.service %{buildroot}%{_unitdir}/%{name}.service

# Adjust unit file path: spec installs binary to /usr/bin, not /usr/local/bin
sed -i 's|/usr/local/bin/%{name}|%{_bindir}/%{name}|' \
    %{buildroot}%{_unitdir}/%{name}.service

install -Dpm 0644 README.md %{buildroot}%{_docdir}/%{name}/README.md
install -Dpm 0644 LICENSE %{buildroot}%{_licensedir}/%{name}/LICENSE

%check
cargo test --release --offline || :

%post
%systemd_post %{name}.service

%preun
%systemd_preun %{name}.service

%postun
%systemd_postun_with_restart %{name}.service

%files
%license LICENSE
%doc README.md PERFORMANCE.md
%{_bindir}/%{name}
%config(noreplace) %{_sysconfdir}/%{name}.conf
%{_unitdir}/%{name}.service

%changelog
* Sat May 02 2026 RustLED Contributors - 0.9.0-1
- Initial RPM packaging
