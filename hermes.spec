%global debug_package %{nil}

Name:           hermes
Version:        0.1.0
Release:        1%{?dist}
Summary:        Open-source GPU System Processor and Firmware Host for NVIDIA, AMD, and Intel GPUs

License:        MIT
URL:            https://github.com/SisyphusAeolides/Hermes
Source0:        %{url}/archive/main/%{name}-main.tar.gz

BuildRequires:  cargo
BuildRequires:  clang
BuildRequires:  gcc
BuildRequires:  gcc-gfortran
BuildRequires:  make
BuildRequires:  rust

%description
Hermes is an open-source, evidence-driven, universal GPU System Processor (GSP)
and Firmware Host for NVIDIA, AMD, and Intel GPUs. It supports NVIDIA Turing+,
AMD RDNA/CDNA, and Intel Xe/Arc families through OpenRM, SMU, and GuC firmware
paths respectively.

%prep
%autosetup -n Hermes-main

%build
cargo build --release \
    -p hermes-settings \
    -p hermes-ctl \
    -p hermes-nvml

%install
install -Dm755 target/release/hermes-ctl %{buildroot}%{_bindir}/hermes-ctl
install -Dm755 target/release/nvidia-smi %{buildroot}%{_bindir}/nvidia-smi
install -Dm755 target/release/nvidia-settings %{buildroot}%{_bindir}/nvidia-settings

%files
%license LICENSE
%doc README.md
%{_bindir}/hermes-ctl
%{_bindir}/nvidia-smi
%{_bindir}/nvidia-settings

%changelog
* Sun Sep 13 2026 Kenny Glauner <SisyphusAeolides@pm.me> - 0.1.0-1
- Initial RPM packaging
