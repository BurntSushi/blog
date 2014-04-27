// This attribute specifies that the Rust compiler will output a dynamic 
// library when this file is compiled.
// Generally, many Rust libraries will *also* have a `#![crate_type = "rlib"]`
// attribute set, which means the Rust compiler will produce a static library.
// However, libraries which provide syntax extensions must be dynamically 
// linked with `libsyntax`, so we elide the `rlib` and only produce a dynamic 
// library.
#![crate_type = "dylib"]

// Enable the `macro_registrar` feature (which is the compiler hook) and the
// `managed_boxes` feature. At some point, the `managed_boxes` feature will 
// probably be removed when the `@` sigil is removed.
#![feature(macro_registrar, managed_boxes)]

extern crate syntax;

use syntax::ast;
use syntax::codemap;
use syntax::ext::base::{
    SyntaxExtension, ExtCtxt, MacResult, MacExpr,
    NormalTT, BasicMacroExpander,
};
use syntax::parse::token;

#[macro_registrar]
pub fn macro_registrar(register: |ast::Name, SyntaxExtension|) {
    let expander = ~BasicMacroExpander { expander: expand, span: None };
    register(token::intern("factorial"), NormalTT(expander, None))
}

fn expand(_: &mut ExtCtxt, sp: codemap::Span, _: &[ast::TokenTree]) -> ~MacResult {
    let answer = factorial(5 as u64);
    MacExpr::new(int_literal(sp, answer))
}

fn factorial(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => n * factorial(n - 1),
    }
}

fn int_literal(sp: codemap::Span, n: u64) -> @ast::Expr {
    let lit = ast::LitUint(n as u64, ast::TyU64);
    let spanned = @codemap::Spanned { span: sp, node: lit };
    dummy_expr(sp, ast::ExprLit(spanned))
}

fn dummy_expr(sp: codemap::Span, e: ast::Expr_) -> @ast::Expr {
    @ast::Expr {
        id: ast::DUMMY_NODE_ID,
        node: e,
        span: sp,
    }
}
