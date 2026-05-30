use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::ast;
use crate::lexer::Lexer;
use crate::parser::Parser;

#[derive(Debug)]
pub enum LoadError {
    Io { path: PathBuf, error: String },
    Parse { path: PathBuf, message: String },
    Cycle { path: PathBuf },
    MissingImport { from: PathBuf, target: PathBuf, segments: Vec<String> },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io { path, error } =>
                write!(f, "io error reading {}: {}", path.display(), error),
            LoadError::Parse { path, message } =>
                write!(f, "parse error in {}:\n{}", path.display(), message),
            LoadError::Cycle { path } =>
                write!(f, "import cycle detected at {}", path.display()),
            LoadError::MissingImport { from, target, segments } =>
                write!(f, "cannot resolve `import {}` from {}: missing {}",
                    segments.join("."), from.display(), target.display()),
        }
    }
}

impl std::error::Error for LoadError {}

#[derive(Debug)]
pub struct LoadedProgram {
    pub decls: Vec<ast::Decl>,
    pub sources: Vec<(PathBuf, String)>,
    pub entry_source: String,
}

pub fn load_program(entry: &Path) -> Result<LoadedProgram, LoadError> {
    let root = entry.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let mut out = LoadedProgram {
        decls: Vec::new(),
        sources: Vec::new(),
        entry_source: String::new(),
    };
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut in_progress: HashSet<PathBuf> = HashSet::new();
    load_recursive(entry, &root, &[], &mut out, &mut visited, &mut in_progress, true)?;
    Ok(out)
}

fn load_recursive(
    file: &Path,
    root: &Path,
    module_path: &[String],
    out: &mut LoadedProgram,
    visited: &mut HashSet<PathBuf>,
    in_progress: &mut HashSet<PathBuf>,
    is_entry: bool,
) -> Result<(), LoadError> {
    let canon = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    if visited.contains(&canon) { return Ok(()); }
    if in_progress.contains(&canon) {
        return Err(LoadError::Cycle { path: canon });
    }
    in_progress.insert(canon.clone());

    let source = std::fs::read_to_string(file).map_err(|e| LoadError::Io {
        path: file.to_path_buf(),
        error: e.to_string(),
    })?;
    let mut parser = Parser::new(Lexer::new(&source)).with_source(source.clone());
    let mut decls = parser.parse_program();
    if !parser.errors.is_empty() {
        return Err(LoadError::Parse {
            path: file.to_path_buf(),
            message: parser.pretty_print_errors(),
        });
    }

    for decl in &mut decls {
        if let ast::Decl::Use { path, .. } = decl {
            let base = file.parent().unwrap_or(root);
            let dep_path = resolve_import(base, root, path);
            if !dep_path.exists() {
                return Err(LoadError::MissingImport {
                    from: file.to_path_buf(),
                    target: dep_path,
                    segments: path.clone(),
                });
            }
            let canonical = module_path_from_file(root, &dep_path);
            *path = canonical.clone();
            load_recursive(&dep_path, root, &canonical, out, visited, in_progress, false)?;
        }
    }

    in_progress.remove(&canon);
    visited.insert(canon);
    if is_entry { out.entry_source = source.clone(); }
    out.sources.push((file.to_path_buf(), source));
    if is_entry {
        out.decls.extend(decls);
    } else {
        out.decls.push(ast::Decl::ModEnter(module_path.to_vec()));
        out.decls.extend(decls);
        out.decls.push(ast::Decl::ModExit);
    }
    Ok(())
}

fn resolve_import(base: &Path, root: &Path, path: &[String]) -> PathBuf {
    let relative = join_import(base, path);
    if relative.exists() {
        return relative;
    }
    let from_root = join_import(root, path);
    if from_root.exists() {
        return from_root;
    }
    relative
}

fn join_import(base: &Path, path: &[String]) -> PathBuf {
    let mut p = base.to_path_buf();
    let n = path.len();
    for (i, seg) in path.iter().enumerate() {
        if i + 1 == n {
            p.push(format!("{}.abe", seg));
        } else {
            p.push(seg);
        }
    }
    p
}

fn module_path_from_file(root: &Path, file: &Path) -> Vec<String> {
    let rel = file.strip_prefix(root).unwrap_or(file);
    let mut segs: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if let Some(last) = segs.last_mut() {
        if let Some(stripped) = last.strip_suffix(".abe") {
            *last = stripped.to_string();
        }
    }
    segs
}
