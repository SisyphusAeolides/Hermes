%global debug_package %{nil}

Name:           hermes
Version:        0.1.0
Release:        2%{?dist}
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
install -Dm755 target/release/hermes-settings-gui %{buildroot}%{_bindir}/hermes-settings-gui
install -Dm755 target/release/nvidia-smi %{buildroot}%{_bindir}/nvidia-smi
install -Dm755 target/release/nvidia-settings %{buildroot}%{_bindir}/nvidia-settings
install -Dm755 target/release/nvidia-persistenced %{buildroot}%{_bindir}/nvidia-persistenced
install -Dm755 target/release/nvidia-modprobe %{buildroot}%{_bindir}/nvidia-modprobe
install -Dm755 target/release/nvidia-debugdump %{buildroot}%{_bindir}/nvidia-debugdump
install -Dm755 target/release/nvidia-cuda-mps-control %{buildroot}%{_bindir}/nvidia-cuda-mps-control
install -Dm755 target/release/libnvidia_ml.so %{buildroot}%{_libdir}/libnvidia-ml.so.1
ln -sf libnvidia-ml.so.1 %{buildroot}%{_libdir}/libnvidia-ml.so
install -Dm644 packaging/hermes-settings-gui.desktop %{buildroot}%{_datadir}/applications/hermes-settings-gui.desktop
install -Dm644 assets/icons/hermes-gpu-settings.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/hermes-gpu-settings.svg

%files
%license LICENSE
%doc README.md
%{_bindir}/hermes-ctl
%{_bindir}/hermes-settings-gui
%{_bindir}/nvidia-smi
%{_bindir}/nvidia-settings
%{_bindir}/nvidia-persistenced
%{_bindir}/nvidia-modprobe
%{_bindir}/nvidia-debugdump
%{_bindir}/nvidia-cuda-mps-control
%{_libdir}/libnvidia-ml.so*
%{_datadir}/applications/hermes-settings-gui.desktop
%{_datadir}/icons/hicolor/scalable/apps/hermes-gpu-settings.svg

%changelog
* Sat Nov 07 2026 Kenny Glauner <SisyphusAeolides@pm.me> - 0.1.0-2
- Add MacBook Pro Kepler NVKM, Intel Crystal Well Gen7.5, live sysfs/applesmc telemetry, and hermes-settings-gui packaging

* Sun Sep 13 2026 Kenny Glauner <SisyphusAeolides@pm.me> - 0.1.0-1
- Initial RPM packaging
