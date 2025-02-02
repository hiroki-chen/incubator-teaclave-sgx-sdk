// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License..

use crate::os::unix::prelude::*;

use crate::error::Error as StdError;
use crate::ffi::{CStr, CString, OsStr, OsString};
use crate::fmt;
use crate::io;
use crate::iter;
#[cfg(feature = "env")]
use crate::path;
use crate::path::PathBuf;
use crate::slice;
use crate::str;
use crate::sys::cvt_ocall;
use crate::sys::memchr;
use crate::vec;

use sgx_oc::{c_char, c_int, size_t};
use sgx_trts::error as trts_error;
use sgx_types::metadata::SE_PAGE_SIZE;


const TMPBUF_SZ: usize = 128;
const PATH_SEPARATOR: u8 = b':';

#[inline]
pub fn errno() -> i32 {
    trts_error::errno()
}

#[inline]
pub fn set_errno(e: i32) {
    trts_error::set_errno(e)
}

pub fn error_string(errno: i32) -> String {
    extern "C" {
        fn strerror_r(errnum: c_int, buf: *mut c_char, buflen: size_t) -> c_int;
    }

    let mut buf = [0_i8; TMPBUF_SZ];

    let p = buf.as_mut_ptr();
    unsafe {
        assert!(strerror_r(errno as c_int, p, buf.len()) >= 0, "strerror_r failure");

        let p = p as *const _;
        str::from_utf8(CStr::from_ptr(p).to_bytes()).unwrap().to_owned()
    }
}

#[cfg(feature = "env")]
#[inline]
pub fn getcwd() -> io::Result<PathBuf> {
    _getcwd()
}

pub(crate) fn _getcwd() -> io::Result<PathBuf> {
    let s = cvt_ocall(unsafe { libc::getcwd() })?;
    Ok(PathBuf::from(OsString::from_vec(s.into_bytes())))
}

#[cfg(feature = "env")]
pub fn chdir(p: &path::Path) -> io::Result<()> {
    let p: &OsStr = p.as_ref();
    let p = CString::new(p.as_bytes())?;
    unsafe { cvt_ocall(libc::chdir(&p)) }
}

#[allow(clippy::type_complexity)]
pub struct SplitPaths<'a> {
    iter: iter::Map<slice::Split<'a, u8, fn(&u8) -> bool>, fn(&'a [u8]) -> PathBuf>,
}

pub fn split_paths(unparsed: &OsStr) -> SplitPaths<'_> {
    fn bytes_to_path(b: &[u8]) -> PathBuf {
        PathBuf::from(<OsStr as OsStrExt>::from_bytes(b))
    }
    fn is_separator(b: &u8) -> bool {
        *b == PATH_SEPARATOR
    }
    let unparsed = unparsed.as_bytes();
    SplitPaths {
        iter: unparsed
            .split(is_separator as fn(&u8) -> bool)
            .map(bytes_to_path as fn(&[u8]) -> PathBuf),
    }
}

impl<'a> Iterator for SplitPaths<'a> {
    type Item = PathBuf;
    fn next(&mut self) -> Option<PathBuf> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

#[derive(Debug)]
pub struct JoinPathsError;

pub fn join_paths<I, T>(paths: I) -> Result<OsString, JoinPathsError>
where
    I: Iterator<Item = T>,
    T: AsRef<OsStr>,
{
    let mut joined = Vec::new();

    for (i, path) in paths.enumerate() {
        let path = path.as_ref().as_bytes();
        if i > 0 {
            joined.push(PATH_SEPARATOR)
        }
        if path.contains(&PATH_SEPARATOR) {
            return Err(JoinPathsError);
        }
        joined.extend_from_slice(path);
    }
    Ok(OsStringExt::from_vec(joined))
}

impl fmt::Display for JoinPathsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "path segment contains separator `{}`", char::from(PATH_SEPARATOR))
    }
}

impl StdError for JoinPathsError {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        "failed to join paths"
    }
}

#[cfg(feature = "env")]
pub fn current_exe() -> io::Result<PathBuf> {
    match crate::fs::read_link("/proc/self/exe") {
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => Err(io::const_io_error!(
            io::ErrorKind::Uncategorized,
            "no /proc/self/exe available. Is /proc mounted?",
        )),
        other => other,
    }
}

pub struct Env {
    iter: vec::IntoIter<(OsString, OsString)>,
}

impl !Send for Env {}
impl !Sync for Env {}

impl Iterator for Env {
    type Item = (OsString, OsString);
    fn next(&mut self) -> Option<(OsString, OsString)> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub unsafe fn environ() -> Vec<CString> {
    libc::env().unwrap_or_default()
}

/// Returns a vector of (variable, value) byte-vector pairs for all the
/// environment variables of the current process.
pub fn env() -> Env {
    unsafe {
        let environ = environ();
        let mut result = Vec::new();

        for var in environ {
            if let Some(key_value) = parse(var.as_bytes()) {
                result.push(key_value);
            }
        }
        return Env { iter: result.into_iter() };
    }

    fn parse(input: &[u8]) -> Option<(OsString, OsString)> {
        // Strategy (copied from glibc): Variable name and value are separated
        // by an ASCII equals sign '='. Since a variable name must not be
        // empty, allow variable names starting with an equals sign. Skip all
        // malformed lines.
        if input.is_empty() {
            return None;
        }
        let pos = memchr::memchr(b'=', &input[1..]).map(|p| p + 1);
        pos.map(|p| {
            (
                OsStringExt::from_vec(input[..p].to_vec()),
                OsStringExt::from_vec(input[p + 1..].to_vec()),
            )
        })
    }
}

pub fn getenv(k: &OsStr) -> io::Result<Option<OsString>> {
    // environment variables with a nul byte can't be set, so their value is
    // always None as well
    let k = CString::new(k.as_bytes())?;
    unsafe {
        let result_opt = cvt_ocall(libc::getenv(&k));
        result_opt.map(|opt| opt.map(|v| OsString::from_vec(v.into_bytes())))
    }
}

pub fn setenv(k: &OsStr, v: &OsStr) -> io::Result<()> {
    let k = CString::new(k.as_bytes())?;
    let v = CString::new(v.as_bytes())?;

    unsafe {
        cvt_ocall(libc::setenv(&k, &v, 1))
    }
}

pub fn unsetenv(n: &OsStr) -> io::Result<()> {
    let nbuf = CString::new(n.as_bytes())?;

    unsafe {
        cvt_ocall(libc::unsetenv(&nbuf))
    }
}

pub fn page_size() -> usize {
    SE_PAGE_SIZE
}

pub fn temp_dir() -> PathBuf {
    crate::env::var_os("TMPDIR").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("/tmp")
    })
}

pub fn home_dir() -> Option<PathBuf> {
    crate::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(feature = "unsupported_process")]
pub fn getpid() -> io::Result<u32> {
    unsafe { cvt_ocall(libc::getpid()).map(|pid| pid as u32) }
}

mod libc {
    #[cfg(feature = "env")]
    pub use sgx_oc::ocall::chdir;
    #[cfg(feature = "unsupported_process")]
    pub use sgx_oc::ocall::getpid;
    pub use sgx_oc::ocall::{env, getenv, setenv, unsetenv, getcwd};
    pub use sgx_oc::*;
}
