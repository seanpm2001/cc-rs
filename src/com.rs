// Copyright © 2017 winapi-rs developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option.
// All files in the project carrying such notice may not be copied, modified, or distributed
// except according to those terms.

#![allow(unused)]

use crate::winapi::{IUnknown, Interface};
use std::{
    ffi::{OsStr, OsString},
    mem::ManuallyDrop,
    ops::Deref,
    os::windows::ffi::{OsStrExt, OsStringExt},
    ptr::{null, null_mut},
    slice::from_raw_parts,
};
use windows_sys::{
    core::{BSTR, HRESULT},
    Win32::{
        Foundation::{SysFreeString, SysStringLen, S_FALSE, S_OK},
        System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
    },
};

pub fn initialize() -> Result<(), HRESULT> {
    let err = unsafe { CoInitializeEx(null(), COINIT_MULTITHREADED) };
    if err != S_OK && err != S_FALSE {
        // S_FALSE just means COM is already initialized
        Err(err)
    } else {
        Ok(())
    }
}

pub struct ComPtr<T>(*mut T)
where
    T: Interface;
impl<T> ComPtr<T>
where
    T: Interface,
{
    /// Creates a `ComPtr` to wrap a raw pointer.
    /// It takes ownership over the pointer which means it does __not__ call `AddRef`.
    /// `T` __must__ be a COM interface that inherits from `IUnknown`.
    pub unsafe fn from_raw(ptr: *mut T) -> ComPtr<T> {
        assert!(!ptr.is_null());
        ComPtr(ptr)
    }
    /// Casts up the inheritance chain
    pub fn up<U>(self) -> ComPtr<U>
    where
        T: Deref<Target = U>,
        U: Interface,
    {
        ComPtr(self.into_raw() as *mut U)
    }
    /// Extracts the raw pointer.
    /// You are now responsible for releasing it yourself.
    pub fn into_raw(self) -> *mut T {
        ManuallyDrop::new(self).0
    }
    /// For internal use only.
    fn as_unknown(&self) -> &IUnknown {
        unsafe { &*(self.0 as *mut IUnknown) }
    }
    /// Performs QueryInterface fun.
    pub fn cast<U>(&self) -> Result<ComPtr<U>, i32>
    where
        U: Interface,
    {
        let mut obj = null_mut();
        let err = unsafe { self.as_unknown().QueryInterface(&U::uuidof(), &mut obj) };
        if err < 0 {
            return Err(err);
        }
        Ok(unsafe { ComPtr::from_raw(obj as *mut U) })
    }
}
impl<T> Deref for ComPtr<T>
where
    T: Interface,
{
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.0 }
    }
}
impl<T> Clone for ComPtr<T>
where
    T: Interface,
{
    fn clone(&self) -> Self {
        unsafe {
            self.as_unknown().AddRef();
            ComPtr::from_raw(self.0)
        }
    }
}
impl<T> Drop for ComPtr<T>
where
    T: Interface,
{
    fn drop(&mut self) {
        unsafe {
            self.as_unknown().Release();
        }
    }
}
pub struct BStr(BSTR);
impl BStr {
    pub unsafe fn from_raw(s: BSTR) -> BStr {
        BStr(s)
    }
    pub fn to_osstring(&self) -> OsString {
        let len = unsafe { SysStringLen(self.0) };
        let slice = unsafe { from_raw_parts(self.0, len as usize) };
        OsStringExt::from_wide(slice)
    }
}
impl Drop for BStr {
    fn drop(&mut self) {
        unsafe { SysFreeString(self.0) };
    }
}

pub trait ToWide {
    fn to_wide(&self) -> Vec<u16>;
    fn to_wide_null(&self) -> Vec<u16>;
}
impl<T> ToWide for T
where
    T: AsRef<OsStr>,
{
    fn to_wide(&self) -> Vec<u16> {
        self.as_ref().encode_wide().collect()
    }
    fn to_wide_null(&self) -> Vec<u16> {
        self.as_ref().encode_wide().chain(Some(0)).collect()
    }
}
pub trait FromWide
where
    Self: Sized,
{
    fn from_wide(wide: &[u16]) -> Self;
    fn from_wide_null(wide: &[u16]) -> Self {
        let len = wide.iter().take_while(|&&c| c != 0).count();
        Self::from_wide(&wide[..len])
    }
}
impl FromWide for OsString {
    fn from_wide(wide: &[u16]) -> OsString {
        OsStringExt::from_wide(wide)
    }
}
