Name:           xwlm
Version:        0.1.2
Release:        1%{?dist}
Summary:        A TUI for managing Wayland monitor configurations

License:        MIT
URL:            https://github.com/x34-dzt/xwlm
Source0:        %{url}/archive/v%{version}/xwlm-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  wayland-devel

%description
A terminal user interface for managing Wayland monitor configurations.
Supports Hyprland, Sway, and River compositors.

%prep
%autosetup -n xwlm-%{version}

%build
cargo build --release

%install
install -Dm755 target/release/xwlm %{buildroot}%{_bindir}/xwlm
install -Dm644 LICENSE %{buildroot}%{_datadir}/licenses/%{name}/LICENSE

%files
%license LICENSE
%{_bindir}/xwlm
