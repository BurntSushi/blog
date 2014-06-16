// This attribute specifies that the Rust compiler will output a dynamic 
// library when this file is compiled.
// Generally, many Rust libraries will *also* have a `#![crate_type = "rlib"]`
// attribute set, which means the Rust compiler will produce a static library.
// However, libraries which provide syntax extensions must be dynamically 
// linked with `libsyntax`, so we elide the `rlib` and only produce a dynamic 
// library.
#![crate_type = "dylib"]

// Enable the `plugin_registrar` feature (which is the compiler hook).
#![feature(plugin_registrar)]

extern crate rustc;
extern crate syntax;

use std::gc::{Gc, GC};
use syntax::ast;
use syntax::codemap;
use syntax::ext::base::{ExtCtxt, MacResult, MacExpr};
use rustc::plugin::Registry;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("factorial", expand)
}

fn expand(_: &mut ExtCtxt, sp: codemap::Span, _: &[ast::TokenTree]) -> Box<MacResult> {
    let answer = factorial(5 as u64);
    MacExpr::new(uint_literal(sp, answer))
}

fn factorial(n: u64) -> u64 {
    use std::iter::MultiplicativeIterator;
    range(2, n+1).product()
}

fn uint_literal(sp: codemap::Span, n: u64) -> Gc<ast::Expr> {
    let lit = ast::LitUint(n as u64, ast::TyU64);
    let spanned = box(GC) codemap::respan(sp, lit);
    dummy_expr(sp, ast::ExprLit(spanned))
}

fn dummy_expr(sp: codemap::Span, e: ast::Expr_) -> Gc<ast::Expr> {
    box(GC) ast::Expr {
        id: ast::DUMMY_NODE_ID,
        node: e,
        span: sp,
    }
}
