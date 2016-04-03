// fastpow2() and fastexp() adapted from C++ source code 
// located at http://fastapprox.googlecode.com/svn/trunk/fastapprox/src/fastonebigheader.h
// The original code was accompanied by the following license: 


/*=====================================================================*
 *                   Copyright (C) 2011 Paul Mineiro                   *
 * All rights reserved.                                                *
 *                                                                     *
 * Redistribution and use in source and binary forms, with             *
 * or without modification, are permitted provided that the            *
 * following conditions are met:                                       *
 *                                                                     *
 *     * Redistributions of source code must retain the                *
 *     above copyright notice, this list of conditions and             *
 *     the following disclaimer.                                       *
 *                                                                     *
 *     * Redistributions in binary form must reproduce the             *
 *     above copyright notice, this list of conditions and             *
 *     the following disclaimer in the documentation and/or            *
 *     other materials provided with the distribution.                 *
 *                                                                     *
 *     * Neither the name of Paul Mineiro nor the names                *
 *     of other contributors may be used to endorse or promote         *
 *     products derived from this software without specific            *
 *     prior written permission.                                       *
 *                                                                     *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND              *
 * CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,         *
 * INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES               *
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE             *
 * ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER               *
 * OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,                 *
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES            *
 * (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE           *
 * GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR                *
 * BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF          *
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT           *
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY              *
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE             *
 * POSSIBILITY OF SUCH DAMAGE.                                         *
 *                                                                     *
 * Contact: Paul Mineiro <paul@mineiro.com>                            *
 *=====================================================================*/


#[inline]
pub fn fastpow2(p: f32) -> f32 {
    let offset = if p < 0. { 1.0 } else { 0.0 };
    let clipp = if p < -126. { -126.0 } else { p };
    let w = clipp as i32;
    let z: f32 = clipp - (w as f32) + offset;
  
    let v : u32 = ( (1 << 23) as f32 * (clipp + 121.2740575 + 27.7280233 / (4.84252568 - z) - 1.49012907 * z) ) as u32;
    let f : &f32 = unsafe {
    	::std::mem::transmute(&v)
	};

    *f
}

#[inline]
pub fn fastexp(p: f32) -> f32 {
    let z = fastpow2 (1.442695040 * p);
//	    let err = ((z - p.exp()) / z).abs();
//	    if err > 0.000072 {
//	  	    println!("shite: p={}, real={}, fast={}, error={}", p, p.exp(), z, err)
//	    }
    z
}
