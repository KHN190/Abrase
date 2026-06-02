use crate::ast;
use crate::bytecode::{Chunk, NativeChunk};
use crate::ty::Type as TyType;
use super::Compiler;

impl Compiler {
    pub(super) fn register_builtins(&mut self) {
        let s = TyType::String;
        let i = TyType::Int;
        let f = TyType::Float;
        let u = TyType::Unit;
        let b = TyType::Bool;
        let c = TyType::Char;

        let cid = self.register_native_chunk("__concat", 2);
        self.concat_fn_id = Some(cid);
        let tid = self.register_native_chunk("__to_str", 1);
        self.to_str_fn_id = Some(tid);
        // Console
        self.register_typed_native("print",    vec![s.clone()],            u.clone(), 1);
        self.register_typed_native("println",  vec![s.clone()],            u.clone(), 1);
        // Float-only math
        self.register_typed_native("ceil",     vec![f.clone()],            f.clone(), 1);
        self.register_typed_native("flr",      vec![f.clone()],            f.clone(), 1);
        self.register_typed_native("cos",      vec![f.clone()],            f.clone(), 1);
        self.register_typed_native("sin",      vec![f.clone()],            f.clone(), 1);
        self.register_typed_native("sqrt",     vec![f.clone()],            f.clone(), 1);
        // Method-body native chunks for built-in traits
        self.register_typed_native("__int_max",   vec![i.clone(), i.clone()], i.clone(), 2);
        self.register_typed_native("__int_min",   vec![i.clone(), i.clone()], i.clone(), 2);
        self.register_typed_native("__int_abs",   vec![i.clone()],            i.clone(), 1);
        self.register_typed_native("__float_max", vec![f.clone(), f.clone()], f.clone(), 2);
        self.register_typed_native("__float_min", vec![f.clone(), f.clone()], f.clone(), 2);
        self.register_typed_native("__float_abs", vec![f.clone()],            f.clone(), 1);
        // Type conversions
        let conv: &[(&str, TyType, TyType)] = &[
            ("__int_to_f",    i.clone(), f.clone()),
            ("__char_to_f",   c.clone(), f.clone()),
            ("__bool_to_f",   b.clone(), f.clone()),
            ("__float_to_i",  f.clone(), i.clone()),
            ("__char_to_i",   c.clone(), i.clone()),
            ("__bool_to_i",   b.clone(), i.clone()),
            ("__int_to_c",    i.clone(), c.clone()),
            ("__int_to_s",    i.clone(), s.clone()),
            ("__float_to_s",  f.clone(), s.clone()),
            ("__bool_to_s",   b.clone(), s.clone()),
            ("__char_to_s",   c.clone(), s.clone()),
            ("__string_to_s", s.clone(), s.clone()),
            ("__unit_to_s",   u.clone(), s.clone()),
        ];
        for (name, p, r) in conv {
            self.register_typed_native(name, vec![p.clone()], r.clone(), 1);
        }
        // System
        self.register_typed_native("halt",  vec![i.clone()], u.clone(), 1);
        self.register_typed_native("abort", vec![s.clone()], u.clone(), 1);
    }

    pub(super) fn register_frame_present_native(&mut self) {
        self.register_typed_native("__frame_present", vec![], crate::ty::Type::Unit, 0);
    }

    fn register_native_chunk(&mut self, name: &str, param_count: usize) -> usize {
        self.register_native_chunk_aliased(name, name, param_count)
    }

    fn register_native_chunk_aliased(
        &mut self,
        user_name: &str,
        chunk_name: &str,
        param_count: usize,
    ) -> usize {
        let id = self.functions.len();
        self.func_map.insert(user_name.into(), id);
        self.functions.push(Chunk::Native(NativeChunk {
            name: chunk_name.into(),
            param_count,
        }));
        id
    }

    fn register_typed_native(
        &mut self,
        name: &str,
        params: Vec<TyType>,
        ret: TyType,
        param_count: usize,
    ) {
        let id = self.register_native_chunk(name, param_count);
        self.builtin_types.insert(name.into(), (params.clone(), ret.clone()));
        self.fn_signatures.insert(id, (params, ret));
    }

    pub(super) fn register_builtins_to_checker(&self, checker: &mut crate::typeck::Checker) {
        let device_in_ty = TyType::Function {
            params: vec![TyType::Int, TyType::Int],
            effects: vec![],
            ret: Box::new(TyType::Unit),
        };
        checker.insert_var("device_in".into(), device_in_ty, false, ast::Span { line: 0, col: 0 });
        let device_out_ty = TyType::Function {
            params: vec![TyType::Int],
            effects: vec![],
            ret: Box::new(TyType::Int),
        };
        checker.insert_var("device_out".into(), device_out_ty, false, ast::Span { line: 0, col: 0 });

        self.register_builtin_effects(checker);

        for decl in self.host_fns.values() {
            let fn_ty = TyType::Function {
                params: decl.params.clone(),
                effects: checker.convert_effect_items(&decl.effects),
                ret: Box::new(decl.ret.clone()),
            };
            checker.insert_var(decl.name.clone(), fn_ty, false, ast::Span { line: 0, col: 0 });
            if !decl.effects.is_empty() {
                checker.register_function_effects(decl.name.clone(), decl.effects.clone());
            }
        }
        for (name, (params, ret)) in &self.builtin_types {
            let fn_ty = TyType::Function {
                params: params.clone(),
                effects: vec![],
                ret: Box::new(ret.clone()),
            };
            checker.insert_var(name.clone(), fn_ty, false, ast::Span { line: 0, col: 0 });
        }
        self.register_builtin_traits(checker);
    }

    fn register_builtin_traits(&self, checker: &mut crate::typeck::Checker) {
        let self_ty = TyType::Named("Self".into());
        let i = TyType::Int;
        let f = TyType::Float;
        let c = TyType::Char;
        let s = TyType::String;

        checker.register_trait("Ord".into(), vec!["max".into(), "min".into()]);
        checker.register_trait("Abs".into(), vec!["abs".into()]);
        checker.register_trait("ToF".into(), vec!["to_f".into()]);
        checker.register_trait("ToI".into(), vec!["to_i".into()]);
        checker.register_trait("ToC".into(), vec!["to_c".into()]);
        checker.register_trait("ToS".into(), vec!["to_s".into()]);

        checker.register_trait_method_sig("Ord", "max", vec![self_ty.clone(), self_ty.clone()], self_ty.clone());
        checker.register_trait_method_sig("Ord", "min", vec![self_ty.clone(), self_ty.clone()], self_ty.clone());
        checker.register_trait_method_sig("Abs", "abs", vec![self_ty.clone()], self_ty.clone());
        checker.register_trait_method_sig("ToF", "to_f", vec![self_ty.clone()], f.clone());
        checker.register_trait_method_sig("ToI", "to_i", vec![self_ty.clone()], i.clone());
        checker.register_trait_method_sig("ToC", "to_c", vec![self_ty.clone()], c.clone());
        checker.register_trait_method_sig("ToS", "to_s", vec![self_ty.clone()], s.clone());

        for &(ty, mx, mn, ab) in &[
            ("Int",   "__int_max",   "__int_min",   "__int_abs"),
            ("Float", "__float_max", "__float_min", "__float_abs"),
        ] {
            checker.register_impl_method("Ord", ty, "max", mx.into());
            checker.register_impl_method("Ord", ty, "min", mn.into());
            checker.register_impl_method("Abs", ty, "abs", ab.into());
            checker.register_impl(ty, "Ord");
            checker.register_impl(ty, "Abs");
        }
        for &(ty, mangled) in &[
            ("Int",   "__int_to_f"),
            ("Char",  "__char_to_f"),
            ("Bool",  "__bool_to_f"),
        ] {
            checker.register_impl_method("ToF", ty, "to_f", mangled.into());
            checker.register_impl(ty, "ToF");
        }
        for &(ty, mangled) in &[
            ("Float", "__float_to_i"),
            ("Char",  "__char_to_i"),
            ("Bool",  "__bool_to_i"),
        ] {
            checker.register_impl_method("ToI", ty, "to_i", mangled.into());
            checker.register_impl(ty, "ToI");
        }
        checker.register_impl_method("ToC", "Int", "to_c", "__int_to_c".into());
        checker.register_impl("Int", "ToC");
        for &(ty, mangled) in &[
            ("Int",    "__int_to_s"),
            ("Float",  "__float_to_s"),
            ("Bool",   "__bool_to_s"),
            ("Char",   "__char_to_s"),
            ("String", "__string_to_s"),
            ("Unit",   "__unit_to_s"),
        ] {
            checker.register_impl_method("ToS", ty, "to_s", mangled.into());
            checker.register_impl(ty, "ToS");
        }
    }

    fn register_builtin_effects(&self, checker: &mut crate::typeck::Checker) {
        let io = vec![ast::EffectItem { name: vec!["IO".into()], arg: None }];
        let nondet = vec![ast::EffectItem { name: vec!["nondet".into()], arg: None }];
        for name in &["now", "sleep_ms"] {
            checker.register_function_effects(name.to_string(), io.clone());
        }
        for name in &["rand", "srand"] {
            checker.register_function_effects(name.to_string(), nondet.clone());
        }
        // Graphics: a built-in user effect for screen/draw natives, kept
        // separate from <IO> so a cart can declare "draws but no file/net".
        checker.register_effect("Graphics".into(), vec![]);
        checker.register_effect("frame".into(), vec!["present".into()]);
        checker.register_effect_op(
            "frame::present".into(),
            crate::ty::Type::Function {
                params: vec![],
                ret: Box::new(crate::ty::Type::Unit),
                effects: vec![],
            },
        );
    }

    pub(super) fn seed_builtin_method_dispatch(
        dispatch: &mut std::collections::HashMap<(String, String), String>,
    ) {
        let entries: &[(&str, &str, &str)] = &[
            ("Int",    "max",  "__int_max"),
            ("Int",    "min",  "__int_min"),
            ("Int",    "abs",  "__int_abs"),
            ("Float",  "max",  "__float_max"),
            ("Float",  "min",  "__float_min"),
            ("Float",  "abs",  "__float_abs"),
            ("Int",    "to_f", "__int_to_f"),
            ("Char",   "to_f", "__char_to_f"),
            ("Bool",   "to_f", "__bool_to_f"),
            ("Float",  "to_i", "__float_to_i"),
            ("Char",   "to_i", "__char_to_i"),
            ("Bool",   "to_i", "__bool_to_i"),
            ("Int",    "to_c", "__int_to_c"),
            ("Int",    "to_s", "__int_to_s"),
            ("Float",  "to_s", "__float_to_s"),
            ("Bool",   "to_s", "__bool_to_s"),
            ("Char",   "to_s", "__char_to_s"),
            ("String", "to_s", "__string_to_s"),
            ("Unit",   "to_s", "__unit_to_s"),
        ];
        for &(ty, m, mangled) in entries {
            dispatch.insert((ty.into(), m.into()), mangled.into());
        }
    }
}
