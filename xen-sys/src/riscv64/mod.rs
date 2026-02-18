/*
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

mod hypercall;
pub use hypercall::*;

mod bindgen {
    #![allow(
        non_upper_case_globals,
        non_camel_case_types,
        non_snake_case,
        deref_nullptr,
        clippy::redundant_static_lifetimes
    )]
    include!("./bindgen.rs");
}
pub use bindgen::*;
