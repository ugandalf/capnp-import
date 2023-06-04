//! Download and/or build official Cap-n-Proto compiler (capnp) release for the current OS and architecture

use anyhow::bail;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use std::str::FromStr;
use std::{
    env, fs,
    io::Read,
    path::{Component, Path, PathBuf},
};
use syn::parse::Parser;
use syn::{parse_macro_input, Expr, ExprArray, LitStr, Token};
use tempfile::NamedTempFile;
use tempfile::{tempdir, TempDir};
use walkdir::WalkDir;
use wax::{BuildError, Glob, Pattern};

include!(concat!(env!("OUT_DIR"), "/binary_decision.rs"));

#[proc_macro]
pub fn capnp_import(input: TokenStream) -> TokenStream {
    // let paths = parse_macro_input!(input as ExprArray);
    // let x: Vec<String> = paths
    //     .elems
    //     .iter()
    //     .map(|path| {
    //         path.into_token_stream()
    //             .to_string()
    //             .trim_matches('\"')
    //             .to_string()
    //     })
    //     .collect();

    // considered alternative
    //let paths = parse_macro_input!(input as syn::Expr);
    //let x: syn::punctuated::IntoIter<syn::Expr> = match paths {
    //    syn::Expr::Array(ExprArray { elems, .. }) => elems.into_iter(),
    //    _ => panic!("Couldn't parse capnp_import contents for {}", input),
    //};

    // Another alternative
    let parser = syn::punctuated::Punctuated::<LitStr, Token![,]>::parse_separated_nonempty;
    let paths = parser.parse(input).unwrap();
    let x: Vec<String> = paths.into_iter().map(|item| item.value()).collect();
    let result = process_inner(&x).unwrap();
    result.into()
}

fn process_inner<T: AsRef<str>>(path_patterns: &[T]) -> anyhow::Result<TokenStream2> {
    // TODO Make it IntoIter
    let cmdpath = CAPNP_BIN_PATH;
    let mut helperfile = TokenStream2::new();
    let output_dir = tempdir()?;

    let mut cmd = capnpc::CompilerCommand::new();
    cmd.capnp_executable(cmdpath);
    cmd.output_path(&output_dir);
    // any() wants to borrow the list of strings we give it, but we can't pass in path_patterns
    // because the borrw checker doesn't like it. We also can't pass in Vec<String> because
    // TryInto<Pattern isn't implemented for String. So, we turn the strings into owned Globs
    // (which clones the string internally)
    let globs: Result<Vec<Glob<'static>>, BuildError<'static>> = path_patterns
        .iter()
        .map(|s| {
            Glob::new(s.as_ref())
                .map_err(BuildError::into_owned)
                .map(Glob::into_owned)
        })
        .collect();
    let combined_globs = wax::any::<Glob, _>(globs?)?;

    for entry_result in WalkDir::new(".") {
        let entry = entry_result?;
        let path = normalize_path(entry.path()); // Remove the current directory indicator

        println!("Processing path: {:?}", path.to_str());
        if path.is_file() && combined_globs.is_match(path.as_path()) {
            println!("Processing {:?}", path);
            cmd.file(path);
            //helperfile.extend(append_path(&mut cmd, &path));
        }
    }

    if let Err(e) = cmd.run() {
        bail!(e.to_string());
    }

    for entry in WalkDir::new(output_dir.path()) {
        let e = entry.unwrap().into_path();
        if e.is_file() {
            println!("File created: {:?}", e);
            let contents = TokenStream2::from_str(&fs::read_to_string(&e).unwrap()).unwrap();
            let module_name = format_ident!("{}", e.file_stem().unwrap().to_str().unwrap());
            let w = quote! {
                mod #module_name {
                    #contents
                }
            };
            helperfile.extend(w);
        }
    }

    return Ok(helperfile);
}

// fn process(path_patterns: &[&str]) -> anyhow::Result<()> {
//     let target_dir = env::var("OUT_DIR").unwrap();
//     fs::write(
//         target_dir + "/capnp_include.rs",
//         process_inner(path_patterns)?,
//     )?;
//     Ok(())
// }

fn append_path(
    cmd: &mut capnpc::CompilerCommand,
    file_path: &Path,
) -> anyhow::Result<TokenStream2> {
    cmd.file(file_path);

    let file_stem = file_path.file_stem().unwrap().to_str().unwrap();
    let file_extension = file_path.extension().unwrap().to_str().unwrap();
    let module_name = format_ident!("{}_{}", file_stem, file_extension);
    let rust_module_path = file_path
        .with_file_name(format!("{}.rs", module_name))
        .to_string_lossy()
        .replace('\\', "/");

    let helperfile = quote! {
        mod #module_name {
            include!(concat!(env!("CARGO_MANIFEST_DIR"), #rust_module_path));
        }
    };
    Ok(helperfile)
}

fn normalize_path(path: &Path) -> PathBuf {
    path.components()
        .filter(|x| match x {
            Component::Normal(_) => true,
            _ => false,
        })
        .collect()
}

// #[test]
// fn basic_file_test() -> anyhow::Result<()> {
//     println!("{:?}", std::env::current_dir().unwrap());
//     let file = NamedTempFile::new().unwrap();
//     let path = file.into_temp_path();
//     assert_eq!(
//         process_inner(&["tests/example.capnp"], path)?,
//         "// This file is autogenerated by capnp-fetch\n\n\nmod example_capnp {\ninclude!(concat!(env!(\"OUT_DIR\"), \"/tests/example_capnp.rs\"));\n}"
//     );
//     Ok(())
// }

// #[test]
// fn glob_test() -> anyhow::Result<()> {
//     println!("{:?}", std::env::current_dir().unwrap());
//     let file = NamedTempFile::new().unwrap();
//     let path = file.into_temp_path();

//     assert_eq!(
//         process_inner(&["tests/**/*.capnp"], path)?,
//         "// This file is autogenerated by capnp-fetch\n\n\nmod example_capnp {\ninclude!(concat!(env!(\"OUT_DIR\"), \"/tests/example_capnp.rs\"));\n}\n\nmod example_capnp {\ninclude!(concat!(env!(\"OUT_DIR\"), \"/tests/folder-test/example_capnp.rs\"));\n}"
//     );
//     Ok(())
// }
