//! Hello, sailor!
//!
//! The purpose of this is to find the folders that contain libraries
//! you may need to link against, on Windows, if you are linking with
//! any compiled C or C++ code. This will be necessary for many
//! non-C++ programming language environments that want to provide
//! compatibility.
//!
//! We find the following:
//! - place where the Visual Studio libraries live (e.g. `libvcruntime.lib`)
//! - where the linker and compiler executables live (e.g. `link.exe`)
//! - where the Windows SDK libraries reside (e.g. `kernel32.lib`, `libucrt.lib`)
//!
//! We all with you didn't have to worry about so many weird
//! dependencies, but we don't erally have a choice about this, sadly.
//!
//! ## Notes
//!
//! One other shortcut I took is that this is hardcoded to return the
//! folders for x64 libraries. If enough people want x86 or arm, it
//! can be worked in here.
//!
//! ## Unsafe usage
//!
//! This crate contains `unsafe` code as we speak to the Windows APIs
//! & COM, which is unsafe by definition.
//!
//! ## Supported platforms
//!
//! We support Windows & Visual Studio only, as those are the main
//! reasons for this crate to exist.
//!
//! If your need is bigger than this, you might go check the
//! [cc](https://crates.io/crates/cc) crate.

#![deny(missing_docs)]

#[cfg(not(target_os = "windows"))]
compile_error!("This crate only supports Windows.");

use std::ptr::null_mut;
use std::fs;
use std::path::{Path, PathBuf};

use winapi::*;

mod winapi;

/// Information about Windows SDK and VC toolchain.
#[derive(Debug)]
pub struct Info {
	/// Windows SDK information, if it is available.
	pub sdk: Option<SdkInfo>,
	/// VC toolchain, if it is available.
	pub toolchain: Option<ToolchainInfo>,
}

/// Information about Windows SDK.
#[derive(Debug)]
pub struct SdkInfo {
	/// Major version.
	///
	/// E.g. `10`.
	pub major_version: u8,
	/// Root folder of the SDK.
	///
	/// E.g. `C:\Program Files (x86)\Windows Kits\10\Lib\10.0.22000.0`.
	pub root: PathBuf,
	/// Path to the folder, which contains platform libraries.
	///
	/// E.g. `C:\Program Files (x86)\Windows Kits\10\Lib\10.0.22000.0\um\x64`.
	pub um_lib_path: PathBuf,
	/// Path to the folder, which contains CRT (default) libraries.
	///
	/// E.g. `C:\Program Files (x86)\Windows Kits\10\Lib\10.0.22000.0\ucrt\x64`.
	pub ucrt_lib_path: PathBuf,
}


/// Information about VC toolchain.
///
/// **IMPORTANT**: If you have multiple versions of Visual Studio
/// installed, we return the first one, rather than the newest. This
/// solves the basic problem for now, but can be easily expanded in
/// the future.
#[derive(Debug)]
pub struct ToolchainInfo {
	/// Path to the folder, which contains `cl.exe`, `link.exe` and friends.
	///
	/// E.g. `C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.36.32532\bin\Hostx64\x64`.
	pub exe_path: PathBuf,
	/// Path to the folder, which contains VC runtime libraries.
	///
	/// E.g. `C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.36.32532\lib\x64`.
	pub lib_path: PathBuf,
}

/// Tries to detect the available VC toolchain and latest Windows SDK, returns the result as a [`Info`].
///
/// # Examples
///
/// In this example, we just use this function to get the info and output it.
///
/// ```no_run
/// fn main() {
///     let info = thound::find_vc_and_windows_sdk();
///     println!("{info:#?}");
/// }
/// ```
pub fn find_vc_and_windows_sdk() -> Option<Info> {
	let wsdk_root = find_windows_kit();

	let sdk = wsdk_root.map(|sdk| SdkInfo {
		major_version: sdk.0,
		root:          sdk.1.clone(),
		um_lib_path:   sdk.1.join(r"um\x64"),
		ucrt_lib_path: sdk.1.join(r"ucrt\x64"),
	});

	let toolchain = find_vc_toolchain();

	Some(Info { sdk, toolchain })
}

fn find_vc_toolchain() -> Option<ToolchainInfo> {
	let _com = ComScope::start()?;

	let mut config = null_mut();
	let status = unsafe {
		CoCreateInstance(
			&CLSID_SetupConfiguration,
			null_mut(),
			CLSCTX_INPROC_SERVER,
			&IID_ISetupConfiguration,
			&mut config,
		)
	};
	if status < 0 {
		return None;
	}
	let config = unsafe { ComPtr::from_raw(config as *mut ISetupConfiguration) };

	let mut instances = null_mut();
	let status = unsafe { config.EnumInstances(&mut instances) };
	if status < 0 {
		return None;
	}
	let instances = unsafe { ComPtr::from_raw(instances) };

	loop {
		let mut found: ULONG = 0;
		let mut instance = null_mut();
		let hr = unsafe { instances.Next(1, &mut instance, &mut found) };
		if hr != S_OK {
			break;
		}
		let instance = unsafe { ComPtr::from_raw(instance) };

		let mut install_path = null_mut();
		let hr = unsafe { instance.GetInstallationPath(&mut install_path) };
		if hr != S_OK {
			break;
		}
		let install_path = unsafe { BStr::from_raw(install_path) };
		let install_path = install_path.to_osstring();
		let install_path = Path::new(&install_path);

		let tools_version = install_path.join(r"VC\Auxiliary\Build\Microsoft.VCToolsVersion.default.txt");
		let tools_version = fs::read_to_string(tools_version).ok()?;
		let tools_version = tools_version.trim();

		let mut lib_path = install_path.join(r"VC\Tools\MSVC");
		lib_path.push(&tools_version);
		lib_path.push(r"lib\x64");

		let lib_file = lib_path.join("vcruntime.lib");

		if lib_file.exists() {
			let mut exe_path = install_path.join(r"VC\Tools\MSVC");
			exe_path.push(&tools_version);
			exe_path.push(r"bin\Hostx64\x64");

			let info = ToolchainInfo { exe_path, lib_path };
			return Some(info);
		}

		// Ryan Saunderson said:
		// "Clang uses the 'SetupInstance->GetInstallationVersion' and
		// ISetupHelper->ParseVersion to find the newest version and
		// then reads the tools file to define the tools path - which
		// is definitely better than what i did."

		// So... @Incomplete: Should probably pick the newest version...
	}

	// If we got here, we didn't find Visual Studio 2017 or newer. Try
	// earlier versions.

	let info = find_old_vc_toolchain();
	if info.is_some() {
		return info;
	}

	// If we get here, we failed to find anything.
	None
}

fn find_old_vc_toolchain() -> Option<ToolchainInfo> {
	let mut vs7_key = RegKey::zeroed();
	let err = unsafe {
		RegOpenKeyExA(
			HKEY_LOCAL_MACHINE,
			"SOFTWARE\\Microsoft\\VisualStudio\\SxS\\VS7\0".as_ptr() as LPCSTR,
			0,
			KEY_QUERY_VALUE | KEY_WOW64_32KEY,
			&mut *vs7_key,
		)
	};
	if err != ERROR_SUCCESS {
		return None;
	}

	// Hardcoded search for 4 prior Visual Studio versions. Is there something better to do here?
	const OLD_VERSIONS: [&str; 4] = ["14.0", "12.0", "11.0", "10.0"];

	for version in OLD_VERSIONS {
		if let Some(root) = reg_query_string_value(&vs7_key, version) {
			let lib_path = root.join(r"VC\Lib\amd64");
			let lib_file = lib_path.join("vcruntime.lib");

			if lib_file.exists() {
				let exe_path = root.join(r"VC\bin");
				let info     = ToolchainInfo { exe_path, lib_path };
				return Some(info);
			}
		}
	}

	None
}

type Version = (u32, u32, u32, u32);

const SDK_VERSIONS: [(u8, &str, fn(&Path) -> Option<Version>); 2] = [
	(10, "KitsRoot10", parse_w10_version),
	( 8, "KitsRoot81", parse_w8_version),
];

fn find_windows_kit() -> Option<(u8, PathBuf)> {
	// Information about the Windows 10 and Windows 8 development kits
	// is stored in the same place in the registry. We open a key to
	// that place, first checking preferentially for a Windows 10 kit,
	// then, if that's not found, a Windows 8 kit.
	let mut main_key = RegKey::zeroed();
	let err = unsafe {
		RegOpenKeyExA(
			HKEY_LOCAL_MACHINE,
			"SOFTWARE\\Microsoft\\Windows Kits\\Installed Roots\0".as_ptr() as LPCSTR,
			0,
			KEY_QUERY_VALUE | KEY_WOW64_32KEY | KEY_ENUMERATE_SUB_KEYS,
			&mut *main_key,
		)
	};
	if err != ERROR_SUCCESS {
		return None;
	}

	for (major_version, sub_key, version_parser) in SDK_VERSIONS {
		let root = reg_query_string_value(&main_key, sub_key);
		if let Some(mut root) = root {
			root.push("Lib");
			let best = find_latest_version(root, version_parser);
			if let Some(best) = best {
				return Some((major_version, best))
			}
		}
	}

	None
}

fn find_latest_version<P>(root: PathBuf, version_parser: P) -> Option<PathBuf>
where
	P: Fn(&Path) -> Option<Version>,
{
	let mut latest = None;
	let mut folder = None;

	for entry in fs::read_dir(&root).ok()? {
		let entry = entry.ok()?;
		let path  = entry.path();
		if path.is_dir() {
			let version = version_parser(&path);
			if version > latest {
				latest = version;
				folder = Some(path);
			}
		}
	}

	folder
}

fn parse_w10_version(path: &Path) -> Option<Version> {
	let name = path.file_name()?.to_str()?;

	let mut parts = name.split('.');
	let v0 = parts.next()?.parse().ok()?;
	let v1 = parts.next()?.parse().ok()?;
	let v2 = parts.next()?.parse().ok()?;
	let v3 = parts.next()?.parse().ok()?;
	if parts.next().is_some() {
		return None;
	}

	Some((v0, v1, v2, v3))
}

fn parse_w8_version(path: &Path) -> Option<Version> {
	let name = path.file_name()?.to_str()?;

	let mut parts = name.strip_prefix("winv")?.split('.');
	let v0 = parts.next()?.parse().ok()?;
	let v1 = parts.next()?.parse().ok()?;
	if parts.next().is_some() {
		return None;
	}

	Some((v0, v1, 0, 0))
}
