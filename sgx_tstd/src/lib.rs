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

//! # The Rust SGX SDK Standard Library
//!
//! The Rust SGX standard library is the foundation of portable Rust SGX SDK,
//! a set of minimal and battle-tested shared abstractions for the Rust SGX
//! ecosystem. Similar to Rust's libstd, it offers core types, like [`Vec<T>`] and
//! [`Option<T>`], library-defined [operations on language
//! primitives](#primitives), [standard macros](#macros), [I/O] and
//! [multithreading], among [many other things][other].
//!
//! `std` is available to all Rust crates by default, just as if each one
//! contained an `extern crate sgx_tstd as std;` import at the [crate root]. Therefore the
//! standard library can be accessed in [`use`] statements through the path
//! `std`, as in [`use std::env`], or in expressions through the absolute path
//! `::std`, as in [`::std::env::args`].

#![no_std]
#![cfg_attr(target_vendor = "teaclave", feature(rustc_private))]

#![needs_panic_runtime]
#![allow(non_camel_case_types)]
#![allow(unused_must_use)]
#![allow(dead_code)]
#![allow(deprecated)]
#![allow(incomplete_features)]
#![allow(unused_assignments)]

#![allow(clippy::declare_interior_mutable_const)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::new_without_default)]
#![allow(clippy::transmute_ptr_to_ptr)]
#![allow(clippy::wrong_self_convention)]

#![feature(rustc_allow_const_fn_unstable)]
#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![feature(allocator_internals)]
#![feature(allow_internal_unsafe)]
#![feature(allow_internal_unstable)]
#![feature(array_error_internals)]
#![feature(assert_matches)]
#![feature(async_iterator)]
#![feature(bench_black_box)]
#![feature(bool_to_option)]
#![feature(box_syntax)]
#![feature(c_unwind)]
#![feature(c_variadic)]
#![feature(cfg_accessible)]
#![feature(cfg_eval)]
#![feature(cfg_target_has_atomic)]
#![feature(char_error_internals)]
#![feature(char_internals)]
#![feature(concat_bytes)]
#![feature(concat_idents)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_fn_trait_bound)]
#![feature(const_format_args)]
#![feature(const_mut_refs)]
#![feature(const_trait_impl)]
#![feature(core_intrinsics)]
#![feature(core_panic)]
#![feature(custom_test_frameworks)]
#![feature(decl_macro)]
#![feature(dropck_eyepatch)]
#![feature(duration_checked_float)]
#![feature(duration_constants)]
#![feature(edition_panic)]
#![feature(exact_size_is_empty)]
#![feature(extend_one)]
#![feature(fn_traits)]
#![feature(float_minimum_maximum)]
#![feature(format_args_nl)]
#![feature(gen_future)]
#![feature(get_mut_unchecked)]
#![feature(hashmap_internals)]
#![feature(int_error_internals)]
#![feature(into_future)]
#![feature(lang_items)]
#![feature(log_syntax)]
#![feature(map_try_insert)]
#![feature(maybe_uninit_slice)]
#![feature(maybe_uninit_write_slice)]
#![feature(mixed_integer_ops)]
#![feature(must_not_suspend)]
#![feature(needs_panic_runtime)]
#![feature(negative_impls)]
#![feature(never_type)]
#![feature(new_uninit)]
#![feature(once_cell)]
#![feature(panic_info_message)]
#![feature(panic_internals)]
#![feature(panic_can_unwind)]
#![feature(panic_unwind)]
#![feature(prelude_import)]
#![feature(ptr_as_uninit)]
#![feature(ptr_internals)]
#![feature(rustc_attrs)]
#![feature(slice_internals)]
#![feature(specialization)]
#![feature(std_internals)]
#![feature(str_internals)]
#![feature(test)]
#![feature(thread_local)]
#![feature(toowned_clone_into)]
#![feature(total_cmp)]
#![feature(trace_macros)]
#![feature(try_blocks)]
#![feature(try_reserve_kind)]
#![feature(unboxed_closures)]
#![default_lib_allocator]

// Explicitly import the prelude. The compiler uses this same unstable attribute
// to import the prelude implicitly when building crates that depend on std.
#[prelude_import]
#[allow(unused)]
use prelude::v1::*;

extern crate hashbrown;

#[allow(unused_imports)] // macros from `alloc` are not used on all platforms
#[macro_use]
extern crate alloc as alloc_crate;

// We always need an unwinder currently for backtraces
#[allow(unused_extern_crates)]
extern crate sgx_unwind;
#[cfg(feature = "backtrace")]
extern crate sgx_backtrace_sys;
#[cfg(feature = "backtrace")]
extern crate sgx_demangle;
extern crate sgx_alloc;

#[macro_use]
extern crate sgx_types;
pub use sgx_types::cfg_if;

extern crate sgx_ffi;
extern crate sgx_oc;
extern crate sgx_rsrvmm;
extern crate sgx_sync;

extern crate sgx_trts;

// The standard macros that are not built-in to the compiler.
#[macro_use]
mod macros;

// The Rust prelude
pub mod prelude;

// Public module declarations and re-exports
pub use alloc_crate::borrow;
pub use alloc_crate::boxed;
pub use alloc_crate::fmt;
pub use alloc_crate::format;
pub use alloc_crate::rc;
pub use alloc_crate::slice;
pub use alloc_crate::str;
pub use alloc_crate::string;
pub use alloc_crate::vec;
pub use core::any;
pub use core::array;
pub use core::async_iter;
pub use core::cell;
pub use core::char;
pub use core::clone;
pub use core::cmp;
pub use core::convert;
pub use core::default;
pub use core::future;
pub use core::hash;
pub use core::hint;
pub use core::i128;
pub use core::i16;
pub use core::i32;
pub use core::i64;
pub use core::i8;
pub use core::intrinsics;
pub use core::isize;
pub use core::iter;
pub use core::marker;
pub use core::mem;
pub use core::ops;
pub use core::option;
pub use core::pin;
pub use core::ptr;
pub use core::result;
pub use core::u128;
pub use core::u16;
pub use core::u32;
pub use core::u64;
pub use core::u8;
pub use core::usize;

// The runtime entry point and a few unstable public functions used by the
// compiler
#[macro_use]
pub mod rt;

pub mod f32;
pub mod f64;

#[macro_use]
pub mod thread;
pub mod ascii;
pub mod collections;
pub mod env;
pub mod error;
pub mod ffi;
#[cfg(feature = "untrusted_fs")]
pub mod fs;
pub mod io;
pub mod net;
pub mod num;
pub mod os;
pub mod panic;
pub mod path;
#[cfg(feature = "unsupported_process")]
pub mod process;
pub mod sync;
pub mod time;
pub mod enclave;
pub mod untrusted;

pub mod lazy;

pub mod task {
    //! Types and Traits for working with asynchronous tasks.

    #[doc(inline)]
    pub use core::task::*;

    #[doc(inline)]
    pub use alloc_crate::task::*;
}

pub mod arch {
    // The `no_inline`-attribute is required to make the documentation of all
    // targets available.
    // See https://github.com/rust-lang/rust/pull/57808#issuecomment-457390549 for
    // more information.
    #[doc(no_inline)] // Note (#82861): required for correct documentation
    pub use core::arch::*;

    pub use sgx_trts::macros::is_x86_feature_detected;
}

pub use sgx_trts::macros::is_x86_feature_detected;

// Platform-abstraction modules
mod sys;
mod sys_common;

pub mod alloc;

// Private support modules
mod panicking;

#[cfg(not(feature = "untrusted_fs"))]
mod fs;

#[cfg(feature = "backtrace")]
pub mod backtrace;

// Re-export macros defined in libcore.
#[allow(deprecated, deprecated_in_future)]
pub use core::{
    assert_eq, assert_ne, debug_assert, debug_assert_eq, debug_assert_ne, matches, r#try, todo,
    unimplemented, unreachable, write, writeln,
};

// Re-export built-in macros defined through libcore.
#[allow(deprecated)]
pub use core::{
    assert, assert_matches, cfg, column, compile_error, concat, concat_idents, const_format_args,
    env, file, format_args, format_args_nl, include, include_bytes, include_str, line, log_syntax,
    module_path, option_env, stringify, trace_macros,
};

pub use core::concat_bytes;

pub use core::primitive;

mod sealed {
    /// This trait being unreachable from outside the crate
    /// prevents outside implementations of our extension traits.
    /// This allows adding more trait methods in the future.
    pub trait Sealed {}
}
