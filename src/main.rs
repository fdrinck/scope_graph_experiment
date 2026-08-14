mod ast;
mod parser;
mod scope_graph;

use ast::SourceFile;
use parser::parse;
use scope_graph::{RefId, ScopeGraph};
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-source-file>", args[0]);
        process::exit(1);
    }

    let file_path = &args[1];
    let code = fs::read_to_string(file_path).unwrap_or_else(|err| {
        eprintln!("Error reading file '{file_path}': {err}");
        process::exit(1);
    });

    let root = parse(&code);
    let source_file = SourceFile::cast(root);
    let graph = ScopeGraph::build(&source_file);

    println!("Scopes constructed: {}", graph.scope_count());
    println!("Declarations: {}", graph.declaration_count());
    println!("References: {}", graph.reference_count());
    println!("Imports: {}", graph.import_count());

    for (i, reference) in graph.references().iter().enumerate() {
        let ref_id = RefId(i as u32);
        let resolved = graph.resolve(ref_id);
        let ref_node = &reference.node;

        match resolved {
            Some(decl_id) => {
                let decl = graph.declaration(decl_id).unwrap();
                println!(
                    "Resolved ref '{}' at {:?} (Scope {}) -> decl '{}' at {:?} (Scope {})",
                    ref_node.text(),
                    ref_node.text_range(),
                    reference.scope.0,
                    decl.node.text(),
                    decl.node.text_range(),
                    decl.scope.0
                );
            }
            None => println!(
                "Unresolved ref '{}' at {:?} (Scope {})",
                ref_node.text(),
                ref_node.text_range(),
                reference.scope.0
            ),
        }
    }
}
