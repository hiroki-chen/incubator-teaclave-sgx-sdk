/*
 * Copyright (C) 2011-2021 Intel Corporation. All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 *   * Redistributions of source code must retain the above copyright
 *     notice, this list of conditions and the following disclaimer.
 *   * Redistributions in binary form must reproduce the above copyright
 *     notice, this list of conditions and the following disclaimer in
 *     the documentation and/or other materials provided with the
 *     distribution.
 *   * Neither the name of Intel Corporation nor the names of its
 *     contributors may be used to endorse or promote products derived
 *     from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
 * A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
 * OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
 * LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 */

#include "ipp_wrapper.h"
#include "errno.h"

IppStatus sgx_ipp_newBN(const Ipp32u *p_data, int size_in_bytes, IppsBigNumState **p_new_BN)
{
    IppsBigNumState *pBN = 0;
    int bn_size = 0;

    if (p_new_BN == NULL || (size_in_bytes <= 0) || ((size_in_bytes % sizeof(Ipp32u)) != 0))
        return ippStsBadArgErr;

    // Get the size of the IppsBigNumState context in bytes
    IppStatus error_code = ippsBigNumGetSize(size_in_bytes/(int)sizeof(Ipp32u), &bn_size);
    if (error_code != ippStsNoErr)
    {
        *p_new_BN = 0;
        return error_code;
    }
    pBN = (IppsBigNumState *)malloc(bn_size);
    if (!pBN)
    {
        error_code = ippStsMemAllocErr;
        *p_new_BN = 0;
        return error_code;
    }
    // Initialize context and partition allocated buffer
    error_code = ippsBigNumInit(size_in_bytes/(int)sizeof(Ipp32u), pBN);
    if (error_code != ippStsNoErr)
    {
        free(pBN);
        *p_new_BN = 0;
        return error_code;
    }
    if (p_data)
    {
        error_code = ippsSet_BN(IppsBigNumPOS, size_in_bytes/(int)sizeof(Ipp32u), p_data, pBN);
        if (error_code != ippStsNoErr)
        {
            *p_new_BN = 0;
            free(pBN);
            return error_code;
        }
    }


    *p_new_BN = pBN;
    return error_code;
}

void sgx_ipp_secure_free_BN(IppsBigNumState *pBN, int size_in_bytes)
{
    if (pBN == NULL || size_in_bytes <= 0 || ((size_in_bytes % sizeof(Ipp32u)) != 0))
    {
        if (pBN)
        {
            free(pBN);
        }
        return;
    }
    int bn_size = 0;

    // Get the size of the IppsBigNumState context in bytes
    // Since we have checked the size_in_bytes before and the &bn_size is not NULL, ippsBigNumGetSize never returns failure
    IppStatus error_code = ippsBigNumGetSize(size_in_bytes/(int)sizeof(Ipp32u), &bn_size);
    if (error_code != ippStsNoErr)
    {
        free(pBN);
        return;
    }
    // Clear the buffer before free.
    memset_s(pBN, bn_size, 0, bn_size);
    free(pBN);
    return;
}

IppStatus IPP_STDCALL sgx_ipp_DRNGen(Ipp32u* pRandBNU, int nBits, void* pCtx)
{
    sgx_status_t sgx_ret;
    UNUSED(pCtx);

    if (0 != nBits % 8)
    {
        // Must be byte aligned
        return ippStsSizeErr;
    }

    if (!pRandBNU)
    {
        return ippStsNullPtrErr;
    }
    sgx_ret = sgx_read_rand((uint8_t*)pRandBNU, (uint32_t)nBits / 8);
    if (SGX_SUCCESS != sgx_ret)
    {
        return ippStsErr;
    }
    return ippStsNoErr;
}


IppStatus sgx_ipp_newPrimeGen(int nMaxBits, IppsPrimeState ** pPrimeG)
{
    if (pPrimeG == NULL || nMaxBits <= 0) {
        return ippStsBadArgErr;
    }
    int ctxSize = 0;
    IppStatus error_code = ippsPrimeGetSize(nMaxBits, &ctxSize);
    if (error_code != ippStsNoErr) {
        return error_code;
    }
    IppsPrimeState* pCtx = (IppsPrimeState *)malloc(ctxSize);
    if (pCtx == NULL) {
        return ippStsMemAllocErr;
    }

    error_code = ippsPrimeInit(nMaxBits, pCtx);
    if (error_code != ippStsNoErr) {
        free(pCtx);
        return error_code;
    }

    *pPrimeG = pCtx;
    return error_code;
}

#ifdef _UCRYPTO_
/*
 * __memset_vp is a volatile pointer to a function.
 * It is initialised to point to memset, and should never be changed.
 */
static void * (* const volatile __memset_vp)(void *, int, size_t)
    = (memset);

#undef memset_s /* in case it was defined as a macro */

int memset_s(void *s, size_t smax, int c, size_t n)
{
    int err = 0;

    if (s == NULL) {
        err = EINVAL;
        goto out;
    }

    if (n > smax) {
        err = EOVERFLOW;
        n = smax;
    }

    /* Calling through a volatile pointer should never be optimised away. */
    (*__memset_vp)(s, c, n);

    out:
    if (err == 0)
        return 0;
    else {
        errno = err;
        /* XXX call runtime-constraint handler */
        return err;
    }
}

int consttime_memequal(const void *b1, const void *b2, size_t len)
{
	const unsigned char *c1 =  (const unsigned char *)b1, *c2 =  (const unsigned char *)b2;
	unsigned int res = 0;

	while (len--)
		res |= *c1++ ^ *c2++;

	/*
	 * Map 0 to 1 and [1, 256) to 0 using only constant-time
	 * arithmetic.
	 *
	 * This is not simply `!res' because although many CPUs support
	 * branchless conditional moves and many compilers will take
	 * advantage of them, certain compilers generate branches on
	 * certain CPUs for `!res'.
	 */
	return (1 & ((res - 1) >> 8));
}

sgx_status_t sgx_read_rand(unsigned char *rand, size_t length_in_bytes)
{
    // check parameters
    if (!rand || !length_in_bytes) {
        return SGX_ERROR_INVALID_PARAMETER;
    }

    int ctxSize = 0;
    int length_in_bits = length_in_bytes * 8;
	IppsPRNGState* pPRNG = NULL;
	IppStatus ipp_ret = ippStsNoErr;

    do {
        ipp_ret = ippsPRNGGetSize(&ctxSize);
        ERROR_BREAK(ipp_ret);

        pPRNG = (IppsPRNGState*)(malloc(ctxSize));
        if (!pPRNG) {
            ipp_ret = ippStsNoMemErr;
            break;
        }

        ipp_ret = ippsPRNGInit(length_in_bits, pPRNG);
        ERROR_BREAK(ipp_ret);

        ipp_ret = ippsPRNGen((Ipp32u *)rand, length_in_bits, pPRNG);
        ERROR_BREAK(ipp_ret);
    }  while (0);

    CLEAR_FREE_MEM(pPRNG, ctxSize);

    switch (ipp_ret)
    {
    case ippStsNoErr: return SGX_SUCCESS;
    case ippStsNoMemErr:
    case ippStsMemAllocErr: return SGX_ERROR_OUT_OF_MEMORY;
    case ippStsNullPtrErr:
    case ippStsLengthErr:
    case ippStsOutOfRangeErr:
    case ippStsSizeErr:
    case ippStsBadArgErr: return SGX_ERROR_INVALID_PARAMETER;
    default: return SGX_ERROR_UNEXPECTED;
    }
}

#endif
