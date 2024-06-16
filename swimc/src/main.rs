use std::{fs, path::PathBuf};

use clap::Parser as ClapParser;
use swim_ir::Lowerer;
use swim_parse::Parser;
use swim_utils::{IndexMap, SourceId, SpannedItem};
use swim_vm::Vm;

#[derive(ClapParser)]
#[command(version = "0.0", author = "Alex H <alex@alex-hansen.com>")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(ClapParser)]
enum Commands {
    #[command(about = "run the program")]
    Run {
        #[arg(short, long, help = "target to run on", value_parser = ["vm", "native"], default_value = "vm")]
        target:   String,
        #[arg(short = 'i', long, help = "print the IR to stdout")]
        print_ir: bool,
    },
    #[command(about = "format all sources in the project")]
    Fmt,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { target, print_ir } => {
            let (lockfile, buf) = load_project_and_dependencies();
            let lockfile_toml = toml::to_string(&lockfile).expect("Failed to serialize lockfile to TOML");
            fs::write("swim.lock", lockfile_toml).expect("Failed to write lockfile");

            // convert pathbufs into strings for the parser
            let buf = buf
                .into_iter()
                .map(|(pathbuf, s)| (pathbuf.to_string_lossy().to_string(), s))
                .collect::<Vec<_>>();

            // parse
            let parser = Parser::new(buf);
            let (ast, parse_errs, interner, source_map) = parser.into_result();
            render_errors(parse_errs, &source_map);
            // errs.append(&mut parse_errs);
            // resolve symbols
            let (resolution_errs, resolved) = swim_resolve::resolve_symbols(ast, interner);
            // TODO impl diagnostic for resolution errors
            if !resolution_errs.is_empty() {
                dbg!(&resolution_errs);
            }
            // errs.append(&mut resolution_errs);

            // type check
            let (type_errs, type_checker) = swim_typecheck::type_check(resolved);

            // TODO impl diagnostic for type errors
            if !type_errs.is_empty() {
                dbg!(&type_errs);
            }
            // errs.append(&mut type_errs);

            let lowerer: Lowerer = Lowerer::new(type_checker);
            if print_ir {
                println!("{}", lowerer.pretty_print());
            }

            let (data, instructions) = lowerer.finalize();

            match target.to_lowercase().as_str() {
                "vm" => {
                    let mut vm = Vm::new(instructions, data);
                    vm.run().expect("Failed to run vm");
                },
                "native" => todo!(),
                _ => {
                    eprintln!("Invalid target: {}", target);
                },
            }
        },
        Commands::Fmt => {
            let manifest = swim_pkg::manifest::find_manifest().expect("Failed to find manifest");
            let files = load_files();
            swim_fmt::format_sources(files, manifest.formatter).expect("TODO errs");
        },
    }
}

fn load_project_and_dependencies() -> (swim_pkg::Lockfile, Vec<(PathBuf, String)>) {
    let manifest = swim_pkg::manifest::find_manifest().expect("Failed to find manifest");
    let dependencies = manifest.dependencies;
    let (lockfile, _) = swim_pkg::load_dependencies(dependencies);

    let files = load_files();
    (lockfile, files)
}

fn load_files() -> Vec<(PathBuf, String)> {
    let files = fs::read_dir("./src")
        .expect("Failed to read src directory")
        .filter_map(|entry| {
            let entry = entry.expect("Failed to read directory entry");
            if entry.path().extension().and_then(|s| s.to_str()) == Some("swim") {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut buf = Vec::with_capacity(files.len());
    for file in files {
        let source = fs::read_to_string(&file).expect("Failed to read file");
        buf.push((file, source));
    }
    buf
}

fn render_errors<T>(
    errs: Vec<SpannedItem<T>>,
    sources: &IndexMap<SourceId, (&'static str, &'static str)>,
) where
    T: miette::Diagnostic + Send + Sync + 'static,
{
    for err in errs {
        let rendered = swim_utils::render_error(sources, err);
        eprintln!("{:?}", rendered);
    }
}
