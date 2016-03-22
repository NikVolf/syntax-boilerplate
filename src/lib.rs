#![cfg_attr(feature = "nightly-testing", plugin(clippy))]
#![cfg_attr(feature = "nightly-testing", feature(plugin))]
#![cfg_attr(feature = "nightly-testing", allow(used_underscore_binding))]
#![cfg_attr(not(feature = "with-syntex"), feature(rustc_private, plugin))]
#![cfg_attr(not(feature = "with-syntex"), plugin(quasi_macros))]

extern crate aster;
extern crate quasi;

#[cfg(feature = "with-syntex")]
extern crate syntex;

#[cfg(feature = "with-syntex")]
#[macro_use]
extern crate syntex_syntax as syntax;

#[cfg(not(feature = "with-syntex"))]
#[macro_use]
extern crate syntax;

#[cfg(not(feature = "with-syntex"))]
extern crate rustc_plugin;

#[cfg(not(feature = "with-syntex"))]
use syntax::feature_gate::AttributeType;

#[cfg(feature = "with-syntex")]
include!(concat!(env!("OUT_DIR"), "/lib.rs"));

#[cfg(not(feature = "with-syntex"))]
include!("lib.rs.in");

#[cfg(feature = "with-syntex")]
pub fn register(reg: &mut syntex::Registry) {
    use syntax::{ast, fold};

    reg.add_attr("feature(custom_derive)");
    reg.add_attr("feature(custom_attribute)");

    reg.add_decorator("derive_Ipc", codegen::expand_ipc_implementation);
}

#[cfg(not(feature = "with-syntex"))]
pub fn register(reg: &mut rustc_plugin::Registry) {
    reg.register_syntax_extension(
        syntax::parse::token::intern("derive_Ipc"),
        syntax::ext::base::MultiDecorator(
            Box::new(ser::expand_derive_serialize)));
}

pub mod interface;
