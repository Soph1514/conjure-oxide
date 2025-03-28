use conjure_core::ast::{Expression as Expr, SymbolTable};
use conjure_core::rule_engine::{
    register_rule, ApplicationError::RuleNotApplicable, ApplicationResult, Reduction,
};

use crate::into_matrix_expr;

#[register_rule(("Base", 1000))]
fn expand_comprehension(expr: &Expr, _: &SymbolTable) -> ApplicationResult {
    let Expr::Comprehension(_, comprehension) = expr else {
        return Err(RuleNotApplicable);
    };

    // TODO: check what kind of error this throws and maybe panic

    let results = comprehension
        .as_ref()
        .clone()
        .solve_with_minion()
        .or(Err(RuleNotApplicable))?;

    Ok(Reduction::pure(into_matrix_expr!(results)))
}
