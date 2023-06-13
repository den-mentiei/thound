// We mimick the original type names from Windows headers.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
// IDL macro generates "dead" code, but is required to make a proper vtable.
#![allow(dead_code)]

use std::ffi::{self, OsString};
use std::ops::{Deref, DerefMut};
use std::os::windows::prelude::*;
use std::slice;
use std::path::PathBuf;
use std::ptr::null_mut;

pub type wchar_t    = u16;
pub type WCHAR      = wchar_t;
pub type HKEY       = isize;
pub type LPCSTR     = *const i8;
pub type LPCWSTR    = *const WCHAR;
pub type BSTR       = *mut WCHAR;
pub type DWORD      = u32;
pub type LPDWORD    = *mut DWORD;
pub type REGSAM     = u32;
pub type PHKEY      = *mut HKEY;
pub type LSTATUS    = u32;
pub type LPBYTE     = *mut u8;
pub type LPVOID     = *mut ffi::c_void;
pub type HRESULT    = i32;
pub type IID        = GUID;
pub type REFIID     = *const IID;
pub type REFCLSID   = *const IID;
pub type LPUNKNOWN  = *mut IUnknown;
pub type UINT       = u32;
pub type ULONG      = u32;
pub type LCID       = ULONG;
pub type LPFILETIME = *mut FILETIME;
pub type LPCOLESTR  = *const WCHAR;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct FILETIME {
	pub dwLowDateTime: DWORD,
	pub dwHighDateTime: DWORD,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct GUID {
	pub data1: u32,
	pub data2: u16,
	pub data3: u16,
	pub data4: [u8; 8],
}

pub const ERROR_SUCCESS: LSTATUS = 0;

pub const HKEY_LOCAL_MACHINE:    HKEY = 0x80000002 as HKEY;
pub const KEY_QUERY_VALUE:        u32 = 0x0001;
pub const KEY_ENUMERATE_SUB_KEYS: u32 = 0x0008;
pub const KEY_WOW64_32KEY:        u32 = 0x0200;
pub const REG_SZ:                 u32 = 1;

pub const S_OK:                 HRESULT = 0;
pub const S_FALSE:              HRESULT = 1;
pub const COINIT_MULTITHREADED: u32     = 0x0;
pub const CLSCTX_INPROC_SERVER: u32     = 0x1;

macro_rules! guid {
	(
		$name:ident, $d1:expr, $d2:expr, $d3:expr,
		$b1:expr, $b2:expr, $b3:expr, $b4:expr, $b5:expr, $b6:expr, $b7:expr, $b8:expr
	) => {
		pub const $name: GUID = GUID {
			data1: $d1,
			data2: $d2,
			data3: $d3,
			data4: [$b1, $b2, $b3, $b4, $b5, $b6, $b7, $b8],
		};
	}
}

guid!(IID_ISetupConfiguration,  0x42843719, 0xDB4C, 0x46C2, 0x8E, 0x7C, 0x64, 0xF1, 0x81, 0x6E, 0xFD, 0x5B);
guid!(CLSID_SetupConfiguration, 0x177F0C4A, 0x1CD3, 0x4DE7, 0xA3, 0x2C, 0x71, 0xDB, 0xBB, 0x9F, 0xA3, 0x6D);

#[link(name = "Advapi32")]
extern "stdcall" {
	pub fn RegOpenKeyExA(
		hKey: HKEY,
		lpSubKey: LPCSTR,
		ulOptions: DWORD,
		samDesired: REGSAM,
		pnkResult: PHKEY,
	) -> LSTATUS;

	pub fn RegCloseKey(hKey: HKEY) -> LSTATUS;

	pub fn RegQueryValueExW(
		hKey: HKEY,
		lpValueName: LPCWSTR,
		lpReserved: LPDWORD,
		lpType: LPDWORD,
		lpData: LPBYTE,
		lpcbData: LPDWORD,
	) -> LSTATUS;
}

#[link(name = "Ole32")]
extern "stdcall" {
	pub fn CoInitializeEx(pvReserved: LPVOID, dwCoInit: DWORD) -> HRESULT;
	pub fn CoCreateInstance(
		rclsid: REFCLSID,
		pUnkOuter: LPUNKNOWN,
		dwClsContext: DWORD,
		riid: REFIID,
		ppv: *mut LPVOID,
	) -> HRESULT;
}

#[link(name = "OleAut32")]
extern "stdcall" {
	pub fn SysFreeString(bstrString: BSTR);
	pub fn SysStringLen(pbstr: BSTR) -> UINT;
}

// @Cleanup "Compress" this by common parts reuse.
macro_rules! IDL {
	(interface $interface:ident($vtbl:ident) {
		$(fn $method:ident($($param:ident: $param_ty:ty,)*) -> $result:ty;)+
	}) => (
		IDL! { @vtbl $vtbl $interface {$(fn $method($($param: $param_ty,)*) -> $result;)*} }
		IDL! { @impl $interface $vtbl {$(fn $method($($param: $param_ty,)*) -> $result;)*} }
	);

	(interface $interface:ident($vtbl:ident): $parent_interface:ident($parent_vtbl:ident) {
		$(fn $method:ident($($param:ident: $param_ty:ty,)*) -> $result:ty;)+
	}) => (
		IDL! { @vtbl $vtbl $interface {$(fn $method($($param: $param_ty,)*) -> $result;)*} pub parent: $parent_vtbl, }
		IDL! { @impl $interface $vtbl {$(fn $method($($param: $param_ty,)*) -> $result;)*} }

		impl ::core::ops::Deref for $interface {
			type Target = $parent_interface;

			#[inline]
			fn deref(&self) -> &Self::Target {
				unsafe { &*(self as *const _ as *const _) }
			}
		}
	);

	(@vtbl $vtbl:ident $interface:ident
		{ $(fn $method:ident($($param:ident: $param_ty:ty,)*) -> $result:ty;)+ }
		$($fields:tt)*
	) => (
		#[repr(C)]
		pub struct $vtbl {
			$($fields)*
			$(
				pub $method: unsafe extern "stdcall" fn(
						this: *mut $interface,
						$($param: $param_ty,)*
					) -> $result,
			)*
		}
	);

	(@impl $interface:ident $vtbl:ident {
		$(
			fn $method:ident($($param:ident: $param_ty:ty,)*) -> $result:ty;
		)+
	}) => (
		#[repr(C)]
		pub struct $interface {
			pub lpVtbl: *const $vtbl,
		}

		impl $interface {
			$(
				#[inline]
				pub unsafe fn $method(&self, $($param: $param_ty,)*) -> $result {
					((*self.lpVtbl).$method)(self as *const _ as *mut _, $($param,)*)
				}
			)*
		}
	);

}

IDL! {
	interface IUnknown(IUnknownVtbl) {
		fn QueryInterface(
			riid: REFIID,
			ppvObject: *mut *mut ffi::c_void,
		) -> HRESULT;
		fn AddRef() -> ULONG;
		fn Release() -> ULONG;
	}
}

// B41463C3-8866-43B5-BC33-2B0676F7F42E
IDL! {
	interface ISetupInstance(ISetupInstanceVtbl): IUnknown(IUnknownVtbl) {
		fn GetInstanceId(
			pbstrInstanceId: *mut BSTR,
		) -> HRESULT;
		fn GetInstallDate(
			pInstallDate: LPFILETIME,
		) -> HRESULT;
		fn GetInstallationName(
			pbstrInstallationName: *mut BSTR,
		) -> HRESULT;
		fn GetInstallationPath(
			pbstrInstallationPath: *mut BSTR,
		) -> HRESULT;
		fn GetInstallationVersion(
			pbstrInstallationVersion: *mut BSTR,
		) -> HRESULT;
		fn GetDisplayName(
			lcid: LCID,
			pbstrDisplayName: *mut BSTR,
		) -> HRESULT;
		fn GetDescription(
			lcid: LCID,
			pbstrDescription: *mut BSTR,
		) -> HRESULT;
		fn ResolvePath(
			pwszRelativePath: LPCOLESTR,
			pbstrAbsolutePath: *mut BSTR,
		) -> HRESULT;
	}
}

// {6380BCFF-41D3-4B2E-8B2E-BF8A6810C848}
IDL! {
	interface IEnumSetupInstances(IEnumSetupInstancesVtbl): IUnknown(IUnknownVtbl) {
		fn Next(
			celt: ULONG,
			rgelt: *mut *mut ISetupInstance,
			pceltFetched: *mut ULONG,
		) -> HRESULT;
		fn Skip(
			celt: ULONG,
		) -> HRESULT;
		fn Reset() -> HRESULT;
		fn Clone(
			ppenum: *mut *mut IEnumSetupInstances,
		) -> HRESULT;
	}
}

// {42843719-DB4C-46C2-8E7C-64F1816EFD5B}
IDL! {
	interface ISetupConfiguration(ISetupConfigurationVtbl): IUnknown(IUnknownVtbl) {
		fn EnumInstances(
			ppEnumInstances: *mut *mut IEnumSetupInstances,
		) -> HRESULT;
		fn GetInstanceForCurrentProcess(
			ppInstance: *mut *mut ISetupInstance,
		) -> HRESULT;
		fn GetInstanceForPath(
			wzPath: LPCWSTR,
			ppInstance: *mut *mut ISetupInstance,
		) -> HRESULT;
	}
}

pub(crate) struct ComPtr<T>(*mut T);

impl<T> ComPtr<T> {
	pub unsafe fn from_raw(ptr: *mut T) -> Self {
		debug_assert!(!ptr.is_null());
		Self(ptr)
	}

	pub fn as_unknown(&self) -> &IUnknown {
        unsafe { &*(self.0 as *mut IUnknown) }
    }
}

impl<T> Deref for ComPtr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0 }
    }
}

impl<T> Drop for ComPtr<T> {
	fn drop(&mut self) {
		unsafe { self.as_unknown().Release() };
	}
}

pub(crate) struct BStr(BSTR);

impl BStr {
	pub unsafe fn from_raw(s: BSTR) -> Self {
		debug_assert!(!s.is_null());
		Self(s)
	}

	pub fn to_osstring(&self) -> OsString {
		let n = unsafe { SysStringLen(self.0) };
		let s = unsafe { slice::from_raw_parts(self.0, n as usize) };
		OsStringExt::from_wide(s)
	}
}

impl Drop for BStr {
	fn drop(&mut self) {
		unsafe { SysFreeString(self.0) }
	}
}

pub(crate) struct RegKey(HKEY);

impl RegKey {
	pub fn zeroed() -> Self {
		Self(0)
	}
}

impl Deref for RegKey {
	type Target = HKEY;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for RegKey {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl Drop for RegKey {
    fn drop(&mut self) {
		unsafe { RegCloseKey(self.0) };
    }
}

// Given a key to an already opened registry entry,
// get the value stored under the sub-key.
pub(crate) fn reg_query_string_value(key: &RegKey, sub_key: &str) -> Option<PathBuf> {
	// @Speed As we do not have wide-literals support, we are forced
	// to allocate. This can be solved by rolling out a macro/const-fn
	// to do the widening during compile-time or at least collecting
	// the results into a stack array. Not sure if worth it at this,
	// moment.
	let wide_sub_key = sub_key.encode_utf16().chain(Some(0)).collect::<Vec<_>>();

	let mut kind:           DWORD = 0;
	let mut required_bytes: DWORD = 0;
	let err = unsafe {
		RegQueryValueExW(
			**key,
			wide_sub_key.as_ptr(),
			null_mut(),
			&mut kind,
			null_mut(),
			&mut required_bytes,
		)
	};
	if err != ERROR_SUCCESS || kind != REG_SZ {
		return None;
	}

	assert!(required_bytes % 2 == 0, "invalid wide string byte size: {}", required_bytes);
	let len = (required_bytes / 2) as usize; // UCS-2.

	// @Cleanup Just do `std::alloc::alloc()`.
	let mut value = vec![0u16; len];

	let err = unsafe {
		RegQueryValueExW(
			**key,
			wide_sub_key.as_ptr() as LPCWSTR,
			null_mut(),
			null_mut(),
			value.as_mut_ptr() as LPBYTE,
			&mut required_bytes as LPDWORD,
		)
	};
	if err != ERROR_SUCCESS {
		return None;
	}

	assert!(required_bytes % 2 == 0, "invalid wide string byte size: {}", required_bytes);
	let actual_len = (required_bytes / 2) as usize; // UCS-2.
	assert!(actual_len <= value.len());
	value.truncate(actual_len);

	// Registry keys may have or have no terminating nul character,
	// but as `OsString::from_wide` handles the nul for us, we chop it
	// off if it's there.
	if !value.is_empty() && value[actual_len - 1] == 0 {
		value.pop();
	}

	let root = OsString::from_wide(&value[..]);
	Some(root.into())
}
