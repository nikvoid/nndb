//! This script will compile typescript files to js
//! Most of the code is taken from:
//!  
//! https://github.com/swc-project/swc/blob/main/crates/swc_ecma_transforms_typescript/examples/ts_to_js.rs

use swc_common::{
    self,
    comments::SingleThreadedComments,
    errors::{ColorConfig, Handler},
    sync::Lrc,
    Globals, Mark, SourceMap, GLOBALS,
};
use swc_ecma_codegen::{text_writer::JsWriter, Emitter};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};
use swc_ecma_transforms_base::{fixer::fixer, hygiene::hygiene, resolver};
use swc_ecma_transforms_typescript::strip;
use swc_ecma_visit::FoldWith;

const TS_FOLDER: &str = "ts";
const OUT_FILE: &str = "static/script.js";

fn main() {
    let cm: Lrc<SourceMap> = Default::default();

    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let mut buf = vec![];
    for file in std::fs::read_dir(TS_FOLDER).expect("failed to read ts dir") {
        let input = file
            .expect("failed to read entry")
            .path();
        
        let fm = cm
            .load_file(&input)
            .expect("failed to load input typescript file");

        let comments = SingleThreadedComments::default();

        let lexer = Lexer::new(
            Syntax::Typescript(TsConfig {
                tsx: input.ends_with(".tsx"),
                ..Default::default()
            }),
            Default::default(),
            StringInput::from(&*fm),
            Some(&comments),
        );

        let mut parser = Parser::new_from(lexer);

        for e in parser.take_errors() {
            e.into_diagnostic(&handler).emit();
        }

        let module = parser
            .parse_module()
            .map_err(|e| e.into_diagnostic(&handler).emit())
            .expect("failed to parse module.");

        let globals = Globals::default();
        GLOBALS.set(&globals, || {
            let unresolved_mark = Mark::new();
            let top_level_mark = Mark::new();

            // Optionally transforms decorators here before the resolver pass
            // as it might produce runtime declarations.

            // Conduct identifier scope analysis
            let module = module.fold_with(&mut resolver(unresolved_mark, top_level_mark, true));

            // Remove typescript types
            let module = module.fold_with(&mut strip(top_level_mark));

            // Fix up any identifiers with the same name, but different contexts
            let module = module.fold_with(&mut hygiene());

            // Ensure that we have enough parenthesis.
            let module = module.fold_with(&mut fixer(Some(&comments)));

            {
                let mut emitter = Emitter {
                    cfg: swc_ecma_codegen::Config {
                        minify: false,
                        ..Default::default()
                    },
                    cm: cm.clone(),
                    comments: Some(&comments),
                    wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
                };

                emitter.emit_module(&module).unwrap();
            }

        });
    }
    std::fs::write(OUT_FILE, buf)
        .expect("Failed to write output js");
}