//! given bindings, fully resolve an AST
//! This crate's job is to tee up the type checker for the next stage of compilation.

pub use petr_ast::{Intrinsic as IntrinsicName, Literal, Ty};
pub use petr_bind::Dependency;
use petr_utils::{SpannedItem, SymbolInterner};
pub use resolved::QueryableResolvedItems;
use resolver::Resolver;
pub use resolver::{Expr, ExprKind, Function, FunctionCall, Intrinsic, ResolutionError, Type};

mod resolved;
mod resolver;

pub fn resolve_symbols(
    ast: petr_ast::Ast,
    interner: SymbolInterner,
    dependencies: Vec<Dependency>,
) -> (Vec<SpannedItem<ResolutionError>>, QueryableResolvedItems) {
    let resolver = Resolver::new(ast, interner, dependencies);
    resolver.into_queryable()
}
