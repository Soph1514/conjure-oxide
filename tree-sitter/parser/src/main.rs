// use conjure_core::parse;
// use std::collections::btree_map::Entry;
use std::fs;
// use conjure_core::solver::SolverFamily;
use tree_sitter::{Node, Parser, Tree};
use tree_sitter_essence::LANGUAGE;
use std::sync::{Arc, RwLock};

use conjure_core::ast::{Constant, DecisionVariable, Domain, Expression, Name, Range, SymbolTable};
//use conjure_core::rule_engine::Rule;
// use conjure_core::bug;
use conjure_core::context::Context;
// use conjure_core::error::{Error, Result};
use conjure_core::metadata::Metadata;
use conjure_core::Model;
use std::collections::HashMap;
use std::collections::HashSet;

fn main() {
    let source_code =
        fs::read_to_string("./test_code.txt").expect("Failed to read the source code file");
    let tree = get_tree(&source_code);
    let root_node = tree.root_node();
    print_tree(root_node, 0);

    //let target_solver_family = SolverFamily::Minion;
    //let extra_rule_set_names = vec![String::from("rule")];
    //let context = Context::new_ptr(target_solver_family, extra_rule_set_names, rules, rule_sets);

    //let model = create_model(context, tree, &source_code);
}


fn get_tree(source_code: &str) -> Tree {
    let mut parser = Parser::new();
    parser.set_language(&LANGUAGE.into()).unwrap();

    return parser.parse(source_code, None).expect("Failed to parse");
}

fn print_tree(node: tree_sitter::Node, indent: usize) {
    let prefix = "  ".repeat(indent);
    let kind = node.kind();
    let start_byte = node.start_byte();
    let end_byte = node.end_byte();
    //let content = &source[start_byte..end_byte];

    println!("{}{}: {} - {}", prefix, kind, start_byte, end_byte);

    for child in node.children(&mut node.walk()) {
        print_tree(child, indent + 1);
    }
}

fn create_model(context: Arc<RwLock<Context<'static>>>, tree: Tree, source_code: &str) -> Model {
    let mut model = Model::new_empty(context);

    let root_node = tree.root_node();
    let mut cursor = root_node.walk();
    for statement in root_node.named_children(&mut cursor) {
        match statement.kind() {
            "find_statement" => {
                let var_hashmap = parse_find_statement(statement, source_code);
                for (name, decision_variable) in var_hashmap {
                    model.add_variable(name, decision_variable);
                }
            }
            "constraint" => {
                let expression = parse_constraint(statement, source_code);
                model.add_constraint(expression);
            }
            _ => panic!("Error"),
        }
    }
    return model
}

fn parse_find_statement(root_node: Node, source_code: &str) -> HashMap<Name, DecisionVariable> {
    let mut symbol_table = SymbolTable::new();
    let mut temp_symbols = HashSet::new();
    let mut domain: Option<Domain> = None;

    let mut cursor = root_node.walk();
    for node in root_node.named_children(&mut cursor) {
        match node.kind() {
            "variable_list" => {
                let mut cursor = node.walk();
                for variable in node.named_children(&mut cursor) {
                    let variable_name = &source_code[variable.start_byte()..variable.end_byte()];
                    temp_symbols.insert(variable_name);
                }
            }
            "domain" => {
                domain = Some(parse_domain(node, source_code));
            }
            _ => panic!("issue"), 
            // int(1..7)
            // int(1,5..7, 9..11)
        }
    }
    let domain = domain.expect("No domain found");
    
    for name in temp_symbols {
        let decision_variable = DecisionVariable::new(domain.clone());
        symbol_table.insert(Name::UserName(String::from(name)), decision_variable);
    }
    return symbol_table;
}

fn parse_domain(root_node: Node, source_code: &str) -> Domain {
    let range = root_node
        .child_by_field_name("range")
        .unwrap();
    let lower_bound_node = range
        .child_by_field_id(0)
        .unwrap();
    let upper_bound_node = range
        .child_by_field_id(1)
        .unwrap();
    let lower_bound = &source_code
        [lower_bound_node.start_byte()..lower_bound_node.end_byte()]
        .parse::<i32>()
        .unwrap();
    let upper_bound = &source_code
        [upper_bound_node.start_byte()..upper_bound_node.end_byte()]
        .parse::<i32>()
        .unwrap();
    return Domain::IntDomain(vec![Range::Bounded(*lower_bound, *upper_bound)])
}

fn parse_constraint(root_node: Node, source_code: &str) -> Expression {
    match root_node.kind() {
        "constraint" => {
            //TODO: when grammar is changed to allow multiple expressions, make this a for loop thing
            let mut cursor = root_node.walk();
            cursor.goto_first_child();
            return parse_constraint(cursor.node(), source_code)
        }
        "expression" => {
            if has_mult_children(root_node) {
                let mut cursor = root_node.walk();
                let mut vec_exprs = Vec::new();
                for conjunction in root_node.named_children(&mut cursor) {
                    vec_exprs.push(parse_constraint(conjunction, source_code));
                }
                return Expression::Or(Metadata::new(), vec_exprs)
            }
            //TODO: this can be made into a method getChild
            let mut cursor = root_node.walk();
            cursor.goto_first_child();
            return parse_constraint(cursor.node(), source_code)
        }
        "conjunction" => {
            if has_mult_children(root_node) {
                let mut cursor = root_node.walk();
                let mut vec_exprs = Vec::new();
                for comparison in root_node.named_children(&mut cursor) {
                    vec_exprs.push(parse_constraint(comparison, source_code));
                }
                return Expression::And(Metadata::new(), vec_exprs)
            }
            let mut cursor = root_node.walk();
            cursor.goto_first_child();
            return parse_constraint(cursor.node(), source_code)
        }
        "comparison" => {
            //TODO: right now assuming thers only two but really could be any number, change
            if has_mult_children(root_node) {
                let expr1 = parse_constraint(root_node.child_by_field_id(0).unwrap(), source_code);
                let expr2 = parse_constraint(root_node.child_by_field_id(2).unwrap(), source_code);
                
                let comp_op = root_node.child_by_field_name("comp_op")
                    .unwrap();
                let op_type = &source_code[comp_op.start_byte()..comp_op.end_byte()];
                match op_type {
                    "=" => {
                        return Expression::Eq(Metadata::new(), Box::new(expr1), Box::new(expr2));
                    }
                    "!=" => {
                        return Expression::Neq(Metadata::new(), Box::new(expr1), Box::new(expr2));
                    }
                    "<=" => {
                        return Expression::Leq(Metadata::new(), Box::new(expr1), Box::new(expr2));
                    }
                    ">=" => {
                        return Expression::Geq(Metadata::new(), Box::new(expr1), Box::new(expr2));
                    }
                    "<" => {
                        return Expression::Lt(Metadata::new(), Box::new(expr1), Box::new(expr2));
                    }
                    ">" => {
                        return Expression::Gt(Metadata::new(), Box::new(expr1), Box::new(expr2));
                    }
                    _ => panic!("error!")
                }
            }
            let mut cursor = root_node.walk();
            cursor.goto_first_child();
            return parse_constraint(cursor.node(), source_code)
        }
        "addition" => {
            //TODO: right now assuming its a "+", add for if its a "-"
            //TODO: right now assuming its only two terms, really could be any number 
                //(pos use goto_last_child because its vec and then Box)
            if has_mult_children(root_node) {
                let term1 = parse_constraint(root_node.child_by_field_id(0).unwrap(), source_code);
                let term2 = parse_constraint(root_node.child_by_field_id(2).unwrap(), source_code);
                return Expression::SumEq(Metadata::new(), vec![term1], Box::new(term2))
            }
            let mut cursor = root_node.walk();
            cursor.goto_first_child();
            return parse_constraint(cursor.node(), source_code)
        }
        "term" => {
            //TODO: right now assuming its a "/", add for if its a "*"
            //TODO: right now assuming its safe, could be unsafe
            //TODO: right now assuming its only two terms, really could be any number
            if has_mult_children(root_node) {
                let factor1 = parse_constraint(root_node.child_by_field_id(0).unwrap(), source_code);
                let factor2 = parse_constraint(root_node.child_by_field_id(2).unwrap(), source_code);
                return Expression::SafeDiv(Metadata::new(), Box::new(factor1), Box::new(factor2))
            }
            let mut cursor = root_node.walk();
            cursor.goto_first_child();
            return parse_constraint(cursor.node(), source_code)
        }
        "factor" => {
            let mut cursor = root_node.walk();
            cursor.goto_first_child();
            if has_mult_children(root_node) {
                cursor.goto_next_sibling();
            }
            return parse_constraint(cursor.node(), source_code)
        }
        "constant" => {
            //once the grammar is changed, this will be more complicated
            let mut cursor = root_node.walk();
            cursor.goto_first_child();
            let constant_value = &source_code
                [cursor.node().start_byte()..cursor.node().end_byte()]
                .parse::<i32>()
                .unwrap();
            return Expression::Constant(Metadata::new(), Constant::Int(*constant_value))
        }
        "variable" => {
            let mut cursor = root_node.walk();
            cursor.goto_first_child();
            let variable_name = String::from(&source_code[cursor.node().start_byte()..cursor.node().end_byte()]);
            return Expression::Reference(Metadata::new(), Name::UserName(variable_name))
        }
        _ => {
            return Expression::Nothing
        }

    }
}

fn has_mult_children(root_node: Node) -> bool {
    let mut cursor = root_node.walk();
    cursor.goto_first_child();
    return cursor.goto_next_sibling();
}