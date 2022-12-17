use std::{fs, io};

use nom::{error::convert_error, Finish};
use parser::{ExportType, Expr, Func, Module};

use crate::parser::module;

mod parser;

#[allow(unused)]
#[repr(u8)]
enum Section {
    Custom = 0,
    Type,
    Import,
    Func,
    Table,
    Memory,
    Global,
    Export,
    Start,
    Elem,
    Code,
    Data,
    DataCount,
}

fn main() {
    let path = std::env::args().nth(1).expect("must specify input file");
    let mut out_file_path = path.split_once("wat").unwrap().0.to_owned();
    out_file_path.push_str("wasm");
    let input = fs::read_to_string(&path).expect("invalid path");
    match module(&input).finish() {
        Ok((_, module)) => {
            let wasm = compile(&module).unwrap();
            fs::write(&out_file_path, wasm).unwrap();
        }
        Err(e) => eprintln!("{}", convert_error(input.as_str(), e)),
    }
}

fn compile(module: &Module) -> io::Result<Vec<u8>> {
    let mut export_sec = Vec::new();
    for export in &module.exports {
        let idx = match export.ty {
            ExportType::Func => {
                let idx = module
                    .funcs
                    .iter()
                    .position(|f| f.name == export.ident)
                    .expect("unknown function");

                idx
            }
        };

        write_export(&mut export_sec, &export.ident, export.ty, idx)?;
    }
    into_wasm_vec(&mut export_sec, module.exports.len())?;

    let mut func_sec = Vec::new();
    let mut type_sec = Vec::new();
    let mut code_sec = Vec::new();
    for (idx, func) in module.funcs.iter().enumerate() {
        leb128::write::unsigned(&mut func_sec, idx as u64)?;
        write_type(&mut type_sec, func)?;
        write_code(&mut code_sec, func)?;
    }
    let funcs_len = module.funcs.len();
    into_wasm_vec(&mut func_sec, funcs_len)?;
    into_wasm_vec(&mut type_sec, funcs_len)?;
    into_wasm_vec(&mut code_sec, funcs_len)?;

    let mut wasm = Vec::new();
    write_magic_and_version(&mut wasm);
    write_section(&mut wasm, Section::Type, &type_sec)?;
    write_section(&mut wasm, Section::Func, &func_sec)?;
    write_section(&mut wasm, Section::Export, &export_sec)?;
    write_section(&mut wasm, Section::Code, &code_sec)?;

    Ok(wasm)
}

fn into_wasm_vec(vec: &mut Vec<u8>, len: usize) -> io::Result<()> {
    let mut len_bytes = Vec::new();
    leb128::write::unsigned(&mut len_bytes, len as u64)?;
    vec.splice(0..0, len_bytes);

    Ok(())
}

fn write_magic_and_version(vec: &mut Vec<u8>) {
    let magic = b"\0asm";
    let version = 1_u32;
    vec.extend_from_slice(magic);
    vec.extend_from_slice(&version.to_le_bytes());
}

fn write_export(vec: &mut Vec<u8>, name: &str, ty: ExportType, idx: usize) -> io::Result<()> {
    let mut name_bytes = Vec::from(name.as_bytes());
    into_wasm_vec(&mut name_bytes, name.len())?;
    vec.extend_from_slice(&name_bytes);
    vec.push(ty as u8);
    leb128::write::unsigned(vec, idx as u64)?;

    Ok(())
}

fn write_type(vec: &mut Vec<u8>, func: &Func) -> io::Result<()> {
    vec.push(0x60);
    let params_len = func.params.len();
    let mut types = Vec::new();
    for (_, param) in &func.params {
        leb128::write::unsigned(&mut types, *param as u64)?;
    }
    into_wasm_vec(&mut types, params_len)?;
    vec.extend_from_slice(&types);

    if let Some(result) = func.result {
        let mut results = Vec::new();
        leb128::write::unsigned(&mut results, result as u64)?;
        into_wasm_vec(&mut results, 1)?;
        vec.extend_from_slice(&results);
    }

    Ok(())
}

fn write_code(vec: &mut Vec<u8>, func: &Func) -> io::Result<()> {
    // TODO: Properly handle declaration of locals
    let mut locals = Vec::new();
    // create empty `locals` vec
    into_wasm_vec(&mut locals, 0)?;

    let mut code = Vec::new();
    for expr in &func.body {
        match expr {
            Expr::Instr(instr) => code.push(*instr as u8),
            Expr::Ident(ident) => {
                let idx = func
                    .params
                    .iter()
                    .position(|p| &p.0 == ident)
                    .expect("unknown ident");

                leb128::write::unsigned(&mut code, idx as u64)?;
            }
            _ => unimplemented!(),
        }
    }
    // end
    code.push(0x0b);

    let size = locals.len() + code.len();
    leb128::write::unsigned(vec, size as u64)?;
    vec.extend_from_slice(&locals);
    vec.extend_from_slice(&code);

    Ok(())
}

fn write_section(vec: &mut Vec<u8>, ty: Section, bytes: &[u8]) -> io::Result<()> {
    vec.push(ty as u8);
    leb128::write::unsigned(vec, bytes.len() as u64)?;
    vec.extend_from_slice(bytes);

    Ok(())
}
