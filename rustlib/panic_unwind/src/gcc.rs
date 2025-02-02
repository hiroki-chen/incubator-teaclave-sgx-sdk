//! Implementation of panics backed by libgcc/libunwind (in some form).
//!
//! For background on exception handling and stack unwinding please see
//! "Exception Handling in LLVM" (llvm.org/docs/ExceptionHandling.html) and
//! documents linked from it.
//! These are also good reads:
//!  * <https://itanium-cxx-abi.github.io/cxx-abi/abi-eh.html>
//!  * <https://monoinfinito.wordpress.com/series/exception-handling-in-c/>
//!  * <https://www.airs.com/blog/index.php?s=exception+frames>
//!
//! ## A brief summary
//!
//! Exception handling happens in two phases: a search phase and a cleanup
//! phase.
//!
//! In both phases the unwinder walks stack frames from top to bottom using
//! information from the stack frame unwind sections of the current process's
//! modules ("module" here refers to an OS module, i.e., an executable or a
//! dynamic library).
//!
//! For each stack frame, it invokes the associated "personality routine", whose
//! address is also stored in the unwind info section.
//!
//! In the search phase, the job of a personality routine is to examine
//! exception object being thrown, and to decide whether it should be caught at
//! that stack frame. Once the handler frame has been identified, cleanup phase
//! begins.
//!
//! In the cleanup phase, the unwinder invokes each personality routine again.
//! This time it decides which (if any) cleanup code needs to be run for
//! the current stack frame. If so, the control is transferred to a special
//! branch in the function body, the "landing pad", which invokes destructors,
//! frees memory, etc. At the end of the landing pad, control is transferred
//! back to the unwinder and unwinding resumes.
//!
//! Once stack has been unwound down to the handler frame level, unwinding stops
//! and the last personality routine transfers control to the catch block.

use alloc::boxed::Box;
use core::any::Any;

use crate::dwarf::eh::{self, EHAction, EHContext};
use sgx_types::types::{c_int, uintptr_t};
use sgx_unwind as uw;

#[repr(C)]
struct Exception {
    _uwe: uw::_Unwind_Exception,
    cause: Box<dyn Any + Send>,
}

pub unsafe fn panic(data: Box<dyn Any + Send>) -> u32 {
    let exception = Box::new(Exception {
        _uwe: uw::_Unwind_Exception {
            exception_class: rust_exception_class(),
            exception_cleanup,
            private: [0; uw::unwinder_private_data_size],
        },
        cause: data,
    });
    let exception_param = Box::into_raw(exception) as *mut uw::_Unwind_Exception;
    return uw::_Unwind_RaiseException(exception_param) as u32;

    extern "C" fn exception_cleanup(
        _unwind_code: uw::_Unwind_Reason_Code,
        exception: *mut uw::_Unwind_Exception,
    ) {
        unsafe {
            let _: Box<Exception> = Box::from_raw(exception as *mut Exception);
            super::__rust_drop_panic();
        }
    }
}

pub unsafe fn cleanup(ptr: *mut u8) -> Box<dyn Any + Send> {
    let exception = ptr as *mut uw::_Unwind_Exception;
    if (*exception).exception_class != rust_exception_class() {
        uw::_Unwind_DeleteException(exception);
        super::__rust_foreign_exception();
    } else {
        let exception = Box::from_raw(exception as *mut Exception);
        exception.cause
    }
}

// Rust's exception class identifier.  This is used by personality routines to
// determine whether the exception was thrown by their own runtime.
fn rust_exception_class() -> uw::_Unwind_Exception_Class {
    // M O Z \0  R U S T -- vendor, language
    0x4d4f_5a00_5255_5354
}

// Register ids were lifted from LLVM's TargetLowering::getExceptionPointerRegister()
// and TargetLowering::getExceptionSelectorRegister() for each architecture,
// then mapped to DWARF register numbers via register definition tables
// (typically <arch>RegisterInfo.td, search for "DwarfRegNum").
// See also https://llvm.org/docs/WritingAnLLVMBackend.html#defining-a-register.

#[cfg(target_arch = "x86")]
const UNWIND_DATA_REG: (i32, i32) = (0, 2); // EAX, EDX

#[cfg(target_arch = "x86_64")]
const UNWIND_DATA_REG: (i32, i32) = (0, 1); // RAX, RDX

// The following code is based on GCC's C and C++ personality routines.  For reference, see:
// https://github.com/gcc-mirror/gcc/blob/master/libstdc++-v3/libsupc++/eh_personality.cc
// https://github.com/gcc-mirror/gcc/blob/trunk/libgcc/unwind-c.c

unsafe extern "C" fn rust_eh_personality_impl(
    version: c_int,
    actions: uw::_Unwind_Action,
    _exception_class: uw::_Unwind_Exception_Class,
    exception_object: *mut uw::_Unwind_Exception,
    context: *mut uw::_Unwind_Context,
) -> uw::_Unwind_Reason_Code {
    if version != 1 {
        return uw::_URC_FATAL_PHASE1_ERROR;
    }
    let eh_action = match find_eh_action(context) {
        Ok(action) => action,
        Err(_) => return uw::_URC_FATAL_PHASE1_ERROR,
    };
    if actions as i32 & uw::_UA_SEARCH_PHASE as i32 != 0 {
        match eh_action {
            EHAction::None | EHAction::Cleanup(_) => uw::_URC_CONTINUE_UNWIND,
            EHAction::Catch(_) => uw::_URC_HANDLER_FOUND,
            EHAction::Terminate => uw::_URC_FATAL_PHASE1_ERROR,
        }
    } else {
        match eh_action {
            EHAction::None => uw::_URC_CONTINUE_UNWIND,
            EHAction::Cleanup(lpad) | EHAction::Catch(lpad) => {
                uw::_Unwind_SetGR(context, UNWIND_DATA_REG.0, exception_object as uintptr_t);
                uw::_Unwind_SetGR(context, UNWIND_DATA_REG.1, 0);
                uw::_Unwind_SetIP(context, lpad);
                uw::_URC_INSTALL_CONTEXT
            }
            EHAction::Terminate => uw::_URC_FATAL_PHASE2_ERROR,
        }
    }
}

#[lang = "eh_personality"]
unsafe extern "C" fn rust_eh_personality(
    version: c_int,
    actions: uw::_Unwind_Action,
    exception_class: uw::_Unwind_Exception_Class,
    exception_object: *mut uw::_Unwind_Exception,
    context: *mut uw::_Unwind_Context,
) -> uw::_Unwind_Reason_Code {
    rust_eh_personality_impl(version, actions, exception_class, exception_object, context)
}

unsafe fn find_eh_action(context: *mut uw::_Unwind_Context) -> Result<EHAction, ()> {
    let lsda = uw::_Unwind_GetLanguageSpecificData(context) as *const u8;
    let mut ip_before_instr: c_int = 0;
    let ip = uw::_Unwind_GetIPInfo(context, &mut ip_before_instr);
    let eh_context = EHContext {
        // The return address points 1 byte past the call instruction,
        // which could be in the next IP range in LSDA range table.
        ip: if ip_before_instr != 0 { ip } else { ip - 1 },
        func_start: uw::_Unwind_GetRegionStart(context),
        get_text_start: &|| uw::_Unwind_GetTextRelBase(context),
        get_data_start: &|| uw::_Unwind_GetDataRelBase(context),
    };
    eh::find_eh_action(lsda, &eh_context)
}
