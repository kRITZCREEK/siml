use crate::expr::{Expr, Literal};
use crate::utils::parens_if;
use std::collections::HashSet;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Type {
    Int,
    Bool,
    Var(String),
    Existential(String),
    Poly { vars: Vec<String>, ty: Box<Type> },
    Fun { arg: Box<Type>, result: Box<Type> },
}

impl Type {
    pub fn print(&self) -> String {
        self.print_inner(0)
    }

    fn print_inner(&self, depth: u32) -> String {
        match self {
            Type::Int => "Int".to_string(),
            Type::Bool => "Bool".to_string(),
            Type::Var(s) => s.clone(),
            Type::Existential(e) => format!("{{{}}}", e.clone()),
            Type::Poly { vars, ty } => {
                let vars_printed: String = vars
                    .iter()
                    .fold("".to_string(), |acc, var| format!("{} {}", acc, var));
                format!("∀{}. {}", vars_printed, ty.print())
            }
            Type::Fun { arg, result } => parens_if(
                depth > 0,
                format!(
                    "{} -> {}",
                    arg.print_inner(depth + 1),
                    result.print_inner(0)
                ),
            ),
        }
    }

    pub fn is_mono(&self) -> bool {
        match self {
            Type::Var(_) | Type::Existential(_) | Type::Int | Type::Bool => true,
            Type::Poly { .. } => false,
            Type::Fun { arg, result } => arg.is_mono() && result.is_mono(),
        }
    }

    pub fn free_vars(&self) -> HashSet<String> {
        let mut res = HashSet::new();
        match self {
            Type::Var(x) => {
                res.insert(x.clone());
            }
            Type::Fun { arg, result } => {
                res.extend(arg.free_vars());
                res.extend(result.free_vars());
            }
            Type::Existential(v) => {
                res.insert(v.clone());
            }
            Type::Poly { vars, ty } => {
                let mut free_in_ty = ty.free_vars();
                vars.iter().for_each(|var| {
                    free_in_ty.remove(var);
                });
                res.extend(free_in_ty);
            }
            Type::Int | Type::Bool => {}
        }
        res
    }

    pub fn subst(&self, var: &String, replacement: &Type) -> Type {
        match self {
            Type::Bool => Type::Bool,
            Type::Int => Type::Int,
            Type::Var(v) | Type::Existential(v) => {
                if v == var {
                    replacement.clone()
                } else {
                    self.clone()
                }
            }
            Type::Poly { vars, ty } => {
                if vars.contains(var) {
                    self.clone()
                } else {
                    Type::Poly {
                        vars: vars.clone(),
                        ty: Box::new(ty.subst(var, replacement)),
                    }
                }
            }
            Type::Fun { arg, result } => Type::Fun {
                arg: Box::new(arg.subst(var, replacement)),
                result: Box::new(result.subst(var, replacement)),
            },
        }
    }
    pub fn subst_mut(&mut self, var: &String, replacement: &Type) {
        match self {
            Type::Bool | Type::Int => {}
            Type::Var(v) | Type::Existential(v) => {
                if v == var {
                    *self = replacement.clone();
                }
            }
            Type::Poly { vars, ty } => {
                if !vars.contains(var) {
                    ty.subst_mut(var, replacement);
                }
            }
            Type::Fun { arg, result } => {
                arg.subst_mut(var, replacement);
                result.subst_mut(var, replacement);
            }
        }
    }
}

// Smart constructors
impl Type {
    fn var(str: &str) -> Self {
        Type::Var(str.to_string())
    }

    fn ex(str: &str) -> Self {
        Type::Existential(str.to_string())
    }

    fn fun(arg: Type, result: Type) -> Self {
        Type::Fun {
            arg: Box::new(arg),
            result: Box::new(result),
        }
    }

    fn poly(vars: Vec<&str>, ty: Type) -> Self {
        Type::Poly {
            vars: vars.into_iter().map(|s| s.to_string()).collect(),
            ty: Box::new(ty),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Context {
    elems: Vec<ContextElem>,
}

impl Context {
    pub fn new(elems: Vec<ContextElem>) -> Context {
        Context { elems }
    }

    pub fn print(&self) -> String {
        let mut res = "{\n".to_string();
        self.elems.iter().for_each(|ce| {
            res += "  ";
            res += &ce.print();
            res += ",\n";
        });
        res += "}";
        res
    }

    fn split_at(&self, elem: &ContextElem) -> Option<(&[ContextElem], &[ContextElem])> {
        let pos = self.elems.iter().position(|x| x == elem);
        pos.map(|pos| self.elems.split_at(pos))
    }

    /// Splits the context at the introduction of ExVar(ex)
    fn split_at_ex(&self, ex: &String) -> Option<(&[ContextElem], &ContextElem, &[ContextElem])> {
        let pos = self.elems.iter().position(|e| match e {
            ContextElem::ExVar(e) if e == ex => true,
            _ => false,
        });
        pos.map(|pos| {
            let (before, after) = self.elems.split_at(pos);
            let (ex, after) = after.split_first().unwrap();
            (before, ex, after)
        })
    }

    fn elem(&self, elem: &ContextElem) -> bool {
        self.split_at(elem).is_some()
    }

    fn push(&mut self, elem: ContextElem) {
        self.elems.push(elem)
    }

    fn push_elems(&mut self, elems: Vec<ContextElem>) {
        self.elems.extend(elems.into_iter())
    }

    fn insert_at_ex(&self, ex: &String, elems: Vec<ContextElem>) -> Context {
        match self.split_at_ex(ex) {
            // TODO: Could accept a function that could use the ContextElem here, instead of discarding it
            Some((before, _, after)) => {
                let mut new_elems = vec![];
                new_elems.extend_from_slice(before);
                new_elems.extend(elems.into_iter());
                new_elems.extend_from_slice(after);
                Context::new(new_elems)
            }
            None => unreachable!(),
        }
    }

    fn drop_marker(&mut self, marker: ContextElem) {
        let (before_marker, _) = self
            .split_at(&marker)
            .expect("dropping non-existent marker");
        self.elems = before_marker.to_vec();
    }

    fn break_marker(&self, marker: ContextElem) -> (Vec<ContextElem>, Vec<ContextElem>) {
        let (before_marker, after_marker) = self
            .split_at(&marker)
            .expect("breaking non-existent marker");
        (
            before_marker.to_vec(),
            after_marker.split_first().unwrap().1.to_vec(),
        )
    }

    fn markers(&self) -> Vec<String> {
        self.elems
            .iter()
            .filter_map(|x| match x {
                ContextElem::Marker(m) => Some(m.clone()),
                _ => None,
            })
            .collect()
    }
    fn vars(&self) -> Vec<String> {
        self.elems
            .iter()
            .filter_map(|x| match x {
                ContextElem::Anno(v, _) => Some(v.clone()),
                _ => None,
            })
            .collect()
    }
    fn foralls(&self) -> Vec<String> {
        self.elems
            .iter()
            .filter_map(|x| match x {
                ContextElem::Universal(v) => Some(v.clone()),
                _ => None,
            })
            .collect()
    }
    fn existentials(&self) -> Vec<String> {
        self.elems
            .iter()
            .filter_map(|x| match x {
                ContextElem::ExVar(v) => Some(v.clone()),
                ContextElem::ExVarSolved(v, _) => Some(v.clone()),
                _ => None,
            })
            .collect()
    }

    fn find_solved(&self, ex: &String) -> Option<&Type> {
        self.elems.iter().find_map(|e| match e {
            ContextElem::ExVarSolved(var, ty) if var == ex => Some(ty),
            _ => None,
        })
    }

    fn find_var(&self, var: &String) -> Option<&Type> {
        self.elems.iter().find_map(|e| match e {
            ContextElem::Anno(v, ty) => {
                if var == v {
                    Some(ty)
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    /// solve (ΓL,α^,ΓR) α τ = (ΓL,α = τ,ΓR)
    fn solve(&self, ex: &String, ty: Type) -> Option<Context> {
        let (gamma_l, gamma_r) = self.break_marker(ContextElem::ExVar(ex.clone()));
        let mut ctx = Context::new(gamma_l);
        if ctx.wf_type(&ty) {
            ctx.push(ContextElem::ExVarSolved(ex.clone(), ty));
            ctx.push_elems(gamma_r);
            Some(ctx)
        } else {
            None
        }
    }

    /// existentials_ordered Γ α β = True <=> Γ[α^][β^]
    fn existentials_ordered(&self, ex1: &String, ex2: &String) -> bool {
        let (gamma_l, _) = self.break_marker(ContextElem::ExVar(ex2.to_string()));
        gamma_l.contains(&ContextElem::ExVar(ex1.to_string()))
    }

    fn u_var_wf(&self, var: &String) -> bool {
        self.elem(&ContextElem::Universal(var.clone()))
    }

    fn arrow_wf(&self, a: &Type, b: &Type) -> bool {
        self.wf_type(a) && self.wf_type(b)
    }

    fn forall_wf(&self, vars: &Vec<String>, ty: &Type) -> bool {
        let mut tmp_elems = self.elems.clone();
        tmp_elems.extend(vars.iter().map(|v| ContextElem::Universal(v.clone())));
        let tmp_ctx = Context { elems: tmp_elems };

        tmp_ctx.wf_type(ty)
    }

    fn evar_wf(&self, evar: &String) -> bool {
        self.elem(&ContextElem::ExVar(evar.clone()))
    }
    fn solved_evar_wf(&self, evar: &String) -> bool {
        self.elems
            .iter()
            .find(|el| match el {
                ContextElem::ExVarSolved(var, _) => var == evar,
                _ => false,
            })
            .is_some()
    }

    fn wf_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Bool => true,
            Type::Int => true,
            Type::Poly { vars, ty } => self.forall_wf(vars, ty),
            Type::Fun { arg, result } => self.arrow_wf(arg, result),
            Type::Var(var) => self.u_var_wf(var),
            Type::Existential(var) => self.evar_wf(var) || self.solved_evar_wf(var),
        }
    }

    fn wf(&self) -> bool {
        self.clone().wf_mut()
    }

    fn wf_mut(&mut self) -> bool {
        if let Some(el) = self.elems.pop() {
            match el {
                // UvarCtx
                ContextElem::Universal(v) => !self.foralls().contains(&v),
                // VarCtx
                ContextElem::Anno(v, ty) => !self.vars().contains(&v) && self.wf_type(&ty),
                //EvarCtx
                ContextElem::ExVar(ex) => !self.existentials().contains(&ex),
                //SolvedEvarCtx
                ContextElem::ExVarSolved(ex, ty) => {
                    !self.existentials().contains(&ex) && self.wf_type(&ty)
                }
                // MarkerCtx
                ContextElem::Marker(m) => {
                    !self.markers().contains(&m) && !self.existentials().contains(&m)
                }
            }
        } else {
            true
        }
    }

    fn apply(&self, ty: &Type) -> Type {
        match ty {
            Type::Bool => Type::Bool,
            Type::Int => Type::Int,
            Type::Var(v) => Type::Var(v.clone()),
            Type::Existential(ex) => self
                .find_solved(&ex)
                .map_or_else(|| ty.clone(), |ty| self.apply(ty)),
            Type::Fun { arg, result } => Type::Fun {
                arg: Box::new(self.apply(arg)),
                result: Box::new(self.apply(result)),
            },
            Type::Poly { vars, ty } => Type::Poly {
                vars: vars.clone(),
                ty: Box::new(self.apply(ty)),
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ContextElem {
    Universal(String),
    ExVar(String),
    ExVarSolved(String, Type),
    Marker(String),
    Anno(String, Type),
}

impl ContextElem {
    pub fn print(&self) -> String {
        match self {
            ContextElem::Universal(u) => u.clone(),
            ContextElem::ExVar(ex) => format!("{{{}}}", ex),
            ContextElem::ExVarSolved(ex, ty) => format!("{{{}}} = {}", ex, ty.print()),
            ContextElem::Marker(marker) => format!("|> {}", marker),
            ContextElem::Anno(var, ty) => format!("{} : {}", var, ty.print()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TypeError {
    Subtype(Type, Type),
    UnknownVar(String),
    InvalidAnnotation(Type),
    IsNotAFunction(Type),
    ExistentialEscaped(Context, Type, String),
}

impl TypeError {
    pub fn print(&self) -> String {
        match self {
            TypeError::Subtype(ty1, ty2) => format!(
                "Can't figure out subtyping between: {} <: {}",
                ty1.print(),
                ty2.print()
            ),
            TypeError::UnknownVar(var) => format!("Unknown variable: {}", var),
            TypeError::InvalidAnnotation(ty) => {
                format!("{} is not a valid annotation here.", ty.print())
            }
            TypeError::IsNotAFunction(ty) => format!("{} is not a function", ty.print()),
            TypeError::ExistentialEscaped(ctx, ty, ex) => format!(
                "An existential escaped, go get it! {} {} {}",
                ctx.print(),
                ty.print(),
                ex
            ),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeChecker {
    name_gen: NameGen,
}

#[derive(Debug, PartialEq, Eq)]
struct NameGen {
    ty_gen: u32,
    ex_gen: u32,
}

impl NameGen {
    pub fn new() -> NameGen {
        NameGen {
            ty_gen: 0,
            ex_gen: 0,
        }
    }

    pub fn fresh_var(&mut self) -> String {
        self.ty_gen = self.ty_gen + 1;
        format!("{}v", self.ty_gen)
    }
}

impl TypeChecker {
    pub fn new() -> TypeChecker {
        TypeChecker {
            name_gen: NameGen::new(),
        }
    }

    /// Instantiates all bound type variables for a Polytype with fresh vars,
    /// and returns the renamed type as well as the freshly generated vars
    fn rename_poly<F>(&mut self, vars: &Vec<String>, ty: &Type, f: F) -> (Type, Vec<String>)
    where
        F: Fn(String) -> Type,
    {
        let fresh_vars: Vec<(String, String)> = vars
            .iter()
            .map(|v| (v.clone(), self.name_gen.fresh_var()))
            .collect();
        let renamed_ty = {
            let mut tmp_ty = ty.clone();
            for (old, new) in &fresh_vars {
                tmp_ty.subst_mut(old, &f(new.to_string()));
            }
            tmp_ty
        };
        (renamed_ty, fresh_vars.into_iter().map(|x| x.1).collect())
    }

    pub fn subtype<'a>(
        &mut self,
        ctx: Context,
        ty1: &Type,
        ty2: &Type,
    ) -> Result<Context, TypeError> {
        debug!(
            "[subtype] \n{} ({}) ({})",
            ctx.print(),
            ty1.print(),
            ty2.print()
        );
        assert!(ctx.wf_type(ty1));
        assert!(ctx.wf_type(ty2));

        match (ty1, ty2) {
            (Type::Int, Type::Int) => Ok(ctx),
            (Type::Bool, Type::Bool) => Ok(ctx),
            (Type::Var(v1), Type::Var(v2)) if v1 == v2 => Ok(ctx),
            (Type::Existential(e1), Type::Existential(e2)) if e1 == e2 => Ok(ctx),
            (
                Type::Fun {
                    arg: arg1,
                    result: result1,
                },
                Type::Fun {
                    arg: arg2,
                    result: result2,
                },
            ) => {
                let tmp_ctx = self.subtype(ctx, arg2, arg1)?;
                let res1 = tmp_ctx.apply(result1);
                let res2 = tmp_ctx.apply(result2);
                self.subtype(tmp_ctx, &res1, &res2)
            }
            (ty1, Type::Poly { vars, ty: ty2 }) => {
                let (renamed_ty, fresh_vars) = self.rename_poly(vars, ty2, |v| Type::Var(v));

                let marker_var = fresh_vars[0].clone();
                let mut tmp_ctx = ctx.clone();
                tmp_ctx.push_elems(
                    fresh_vars
                        .into_iter()
                        .map(|v| ContextElem::Universal(v))
                        .collect(),
                );

                let mut res = self.subtype(tmp_ctx, ty1, &renamed_ty)?;
                res.drop_marker(ContextElem::Universal(marker_var));
                Ok(res)
            }
            (Type::Poly { vars, ty: ty1 }, ty2) => {
                let (renamed_ty, fresh_vars) =
                    self.rename_poly(vars, ty1, |v| Type::Existential(v));

                let marker_var = fresh_vars[0].clone();
                let mut tmp_ctx = ctx.clone();
                tmp_ctx.push_elems(
                    fresh_vars
                        .into_iter()
                        .flat_map(|v| vec![ContextElem::Marker(v.clone()), ContextElem::ExVar(v)])
                        .collect(),
                );
                let mut res = self.subtype(tmp_ctx, &renamed_ty, ty2)?;
                res.drop_marker(ContextElem::Marker(marker_var));
                Ok(res)
            }
            (Type::Existential(ex), ty) if !ty.free_vars().contains(ex) => {
                self.instantiate_l(ctx, ex, ty)
            }
            (ty, Type::Existential(ex)) if !ty.free_vars().contains(ex) => {
                self.instantiate_r(ctx, ty, ex)
            }
            _ => Err(TypeError::Subtype(ty1.clone(), ty2.clone())),
        }
    }

    fn instantiate_l(
        &mut self,
        ctx: Context,
        ex: &String,
        ty: &Type,
    ) -> Result<Context, TypeError> {
        debug!(
            "[instantiate_l]\n{} ({}) <=: ({})",
            ctx.print(),
            ex,
            ty.print()
        );
        match ty {
            Type::Existential(ex2) if ctx.existentials_ordered(ex, ex2) => {
                // InstLReac
                let new_ctx = ctx
                    .solve(ex2, Type::Existential(ex.clone()))
                    .expect("InstLReach, attempted to solve non-existent existential.");
                Ok(new_ctx)
            }
            Type::Fun { arg, result } => {
                // InstLArr
                let a2 = self.name_gen.fresh_var();
                let a1 = self.name_gen.fresh_var();
                let tmp_ctx = ctx.insert_at_ex(
                    ex,
                    vec![
                        ContextElem::ExVar(a2.clone()),
                        ContextElem::ExVar(a1.clone()),
                        ContextElem::ExVarSolved(
                            ex.clone(),
                            Type::Fun {
                                arg: Box::new(Type::Existential(a1.clone())),
                                result: Box::new(Type::Existential(a2.clone())),
                            },
                        ),
                    ],
                );
                let omega_ctx = self.instantiate_r(tmp_ctx, arg, &a1)?;
                let applied_res = omega_ctx.apply(result);
                self.instantiate_l(omega_ctx, &a2, &applied_res)
            }
            Type::Poly { vars, ty } => {
                //InstLAIIR
                let mut new_ctx = ctx;
                let (renamed_ty, fresh_vars) = self.rename_poly(vars, ty, |v| Type::Existential(v));
                new_ctx.push_elems(
                    fresh_vars
                        .clone()
                        .into_iter()
                        .map(|v| ContextElem::Universal(v))
                        .collect(),
                );
                let mut res_ctx = self.instantiate_r(new_ctx, &renamed_ty, ex)?;
                res_ctx.drop_marker(ContextElem::Marker(fresh_vars[0].clone()));
                Ok(res_ctx)
            }
            ty if ty.is_mono() => {
                // InstLSolve
                let mut tmp_ctx = ctx.clone();
                tmp_ctx.drop_marker(ContextElem::ExVar(ex.to_string()));
                if tmp_ctx.wf_type(ty) {
                    Ok(ctx.solve(ex, ty.clone()).unwrap())
                } else {
                    Err(TypeError::ExistentialEscaped(
                        tmp_ctx,
                        ty.clone(),
                        ex.clone(),
                    ))
                }
            }
            _ => unreachable!("InstLSolve, How did we get here?"),
        }
    }
    fn instantiate_r(
        &mut self,
        ctx: Context,
        ty: &Type,
        ex: &String,
    ) -> Result<Context, TypeError> {
        debug!(
            "[instantiate_r] \n{} ({}) <=: ({})",
            ctx.print(),
            ty.print(),
            ex
        );
        match ty {
            Type::Existential(ex2) if ctx.existentials_ordered(ex, ex2) => {
                // InstRReach
                Ok(ctx.solve(ex2, Type::Existential(ex.clone())).unwrap())
            }
            Type::Fun { arg, result } => {
                // InstRArr
                let a1 = self.name_gen.fresh_var();
                let a2 = self.name_gen.fresh_var();
                let tmp_ctx = ctx.insert_at_ex(
                    ex,
                    vec![
                        ContextElem::ExVar(a2.clone()),
                        ContextElem::ExVar(a1.clone()),
                        ContextElem::ExVarSolved(
                            ex.clone(),
                            Type::Fun {
                                arg: Box::new(Type::Existential(a1.clone())),
                                result: Box::new(Type::Existential(a2.clone())),
                            },
                        ),
                    ],
                );
                let omega_ctx = self.instantiate_l(tmp_ctx, &a1, arg)?;
                let applied_res = omega_ctx.apply(result);
                self.instantiate_r(omega_ctx, &applied_res, &a2)
            }
            Type::Poly { vars, ty } => {
                //InstRAIIL
                let (renamed_ty, fresh_vars) =
                    self.rename_poly(vars, ty, |v| Type::Existential(v.clone()));
                let mut new_ctx = ctx;
                let marker = ContextElem::Marker(fresh_vars[0].clone());
                new_ctx.push_elems(
                    fresh_vars
                        .into_iter()
                        .flat_map(|v| vec![ContextElem::Marker(v.clone()), ContextElem::ExVar(v)])
                        .collect(),
                );
                let mut res_ctx = self.instantiate_r(new_ctx, &renamed_ty, ex)?;
                res_ctx.drop_marker(marker);
                Ok(res_ctx)
            }
            ty if ty.is_mono() => {
                // InstRSolve
                debug!("[InstRSolve] {} := {}", ex, ty.print());
                let mut tmp_ctx = ctx.clone();
                tmp_ctx.drop_marker(ContextElem::ExVar(ex.to_string()));
                if tmp_ctx.wf_type(ty) {
                    Ok(ctx.solve(ex, ty.clone()).unwrap())
                } else {
                    Err(TypeError::ExistentialEscaped(
                        tmp_ctx.clone(),
                        ty.clone(),
                        ex.clone(),
                    ))
                }
            }
            _ => unreachable!("InstRSolve, how does this happen?"),
        }
    }

    fn check(&mut self, ctx: Context, expr: &Expr, ty: &Type) -> Result<Context, TypeError> {
        match (expr, ty) {
            (Expr::Literal(Literal::Int(_)), Type::Int) => Ok(ctx),
            (Expr::Literal(Literal::Bool(_)), Type::Bool) => Ok(ctx),
            (Expr::Lambda { binder, body }, Type::Fun { arg, result }) => {
                // ->l
                let mut new_ctx = ctx;
                let anno_elem = ContextElem::Anno(binder.clone(), *arg.clone());
                new_ctx.push(anno_elem.clone());
                let mut res_ctx = self.check(new_ctx, body, result)?;
                res_ctx.drop_marker(anno_elem);
                Ok(res_ctx)
            }
            (_, Type::Poly { vars, ty }) => {
                //forall_l
                let mut tmp_ctx = ctx;
                let (renamed_ty, fresh_vars) = self.rename_poly(vars, ty, |v| Type::Var(v.clone()));
                let marker = ContextElem::Universal(fresh_vars[0].clone());
                tmp_ctx.push_elems(
                    fresh_vars
                        .into_iter()
                        .map(|v| ContextElem::Universal(v))
                        .collect(),
                );
                let mut res_ctx = self.check(tmp_ctx, expr, &renamed_ty)?;
                res_ctx.drop_marker(marker);
                Ok(res_ctx)
            }
            _ => {
                // Sub
                let (ctx, inferred) = self.infer(ctx, expr)?;
                let inferred = ctx.apply(&inferred);
                let ty = ctx.apply(ty);
                self.subtype(ctx, &inferred, &ty)
            }
        }
    }

    fn infer(&mut self, ctx: Context, expr: &Expr) -> Result<(Context, Type), TypeError> {
        match expr {
            Expr::Literal(Literal::Int(_)) => Ok((ctx, Type::Int)),
            Expr::Literal(Literal::Bool(_)) => Ok((ctx, Type::Bool)),
            Expr::Var(var) => {
                // Var
                let res = match ctx.find_var(var) {
                    Some(ty) => Ok(ty.clone()),
                    None => Err(TypeError::UnknownVar(var.clone())),
                };
                res.map(|ty| (ctx, ty))
            }
            Expr::Ann { expr, ty } => {
                // Anno
                if ctx.wf_type(ty) {
                    let new_ctx = self.check(ctx, expr, ty)?;
                    Ok((new_ctx, ty.clone()))
                } else {
                    Err(TypeError::InvalidAnnotation(ty.clone()))
                }
            }
            Expr::Lambda { binder, body } => {
                // ->l=>
                let mut tmp_ctx = ctx;
                let binder_fresh = self.name_gen.fresh_var();
                let a = self.name_gen.fresh_var();
                let b = self.name_gen.fresh_var();
                let marker = ContextElem::Anno(binder_fresh.clone(), Type::Existential(a.clone()));
                tmp_ctx.push_elems(vec![
                    ContextElem::ExVar(a.clone()),
                    ContextElem::ExVar(b.clone()),
                    marker.clone(),
                ]);

                let mut res_ctx = self.check(
                    tmp_ctx,
                    &body.subst(binder, &Expr::Var(binder_fresh)),
                    &Type::Existential(b.clone()),
                )?;
                res_ctx.drop_marker(marker);
                Ok((res_ctx, Type::fun(Type::ex(&a), Type::ex(&b))))
            }
            Expr::App { func, arg } => {
                let (ctx, func_ty) = self.infer(ctx, func)?;
                let applied_func_ty = ctx.apply(&func_ty);
                self.infer_application(ctx, &applied_func_ty, arg)
            }
        }
    }

    fn infer_application(
        &mut self,
        ctx: Context,
        ty: &Type,
        expr: &Expr,
    ) -> Result<(Context, Type), TypeError> {
        match ty {
            Type::Poly { vars, ty: ty1 } => {
                // forall App
                debug!(
                    "[forall App] {} {} . {}",
                    ctx.print(),
                    ty.print(),
                    expr.print()
                );
                let (renamed_ty, fresh_vars) =
                    self.rename_poly(vars, ty1, |v| Type::Existential(v.clone()));
                let mut new_ctx = ctx;
                new_ctx.push_elems(
                    fresh_vars
                        .into_iter()
                        .map(|v| ContextElem::ExVar(v))
                        .collect(),
                );
                self.infer_application(new_ctx, &renamed_ty, expr)
            }
            Type::Existential(ex) => {
                // alpha^app
                let a1 = self.name_gen.fresh_var();
                let a2 = self.name_gen.fresh_var();
                let new_ctx = ctx.insert_at_ex(
                    ex,
                    vec![
                        ContextElem::ExVar(a2.clone()),
                        ContextElem::ExVar(a1.clone()),
                        ContextElem::ExVarSolved(
                            ex.clone(),
                            Type::Fun {
                                arg: Box::new(Type::Existential(a1.clone())),
                                result: Box::new(Type::Existential(a2.clone())),
                            },
                        ),
                    ],
                );
                let res_ctx = self.check(new_ctx, expr, &Type::Existential(a1))?;
                Ok((res_ctx, Type::Existential(a2)))
            }
            Type::Fun { arg, result } => {
                // ->App
                debug!("[->App] {} . {}", ty.print(), expr.print());
                let res_ctx = self.check(ctx, expr, arg)?;
                let applied_res = res_ctx.apply(result);
                Ok((res_ctx, applied_res))
            }
            ty => Err(TypeError::IsNotAFunction(ty.clone())),
        }
    }

    pub fn synth(&mut self, expr: &Expr) -> Result<Type, TypeError> {
        self.infer(Context::new(vec![]), expr).map(|x| {
            debug!("synth_ctx: {:?}", x.0);
            x.0.apply(&x.1)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subst_mut() {
        let mut ty = Type::var("x");
        ty.subst_mut(&"x".to_string(), &Type::Int);
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn subst_mut_fun() {
        let mut ty = Type::fun(Type::var("a"), Type::var("b"));
        ty.subst_mut(&"a".to_string(), &Type::Int);
        assert_eq!(ty, Type::fun(Type::Int, Type::var("b")));
    }

    #[test]
    fn solve_ex() {
        let ctx = Context::new(vec![
            ContextElem::Universal("x".to_string()),
            ContextElem::ExVar("alpha".to_string()),
            ContextElem::Anno("var".to_string(), Type::Int),
        ]);
        let expected = Context::new(vec![
            ContextElem::Universal("x".to_string()),
            ContextElem::ExVarSolved("alpha".to_string(), Type::Var("x".to_string())),
            ContextElem::Anno("var".to_string(), Type::Int),
        ]);
        let new_ctx = ctx.solve(&"alpha".to_string(), Type::Var("x".to_string()));
        assert_eq!(new_ctx, Some(expected));
    }

    #[test]
    fn subty() {
        let mut tc = TypeChecker::new();
        let ctx = Context::new(vec![]);
        let a = Type::poly(vec!["a"], Type::var("a"));
        let b = Type::Int;
        // (forall a. a) <: Int
        let res = tc.subtype(ctx, &a, &b);
        assert_eq!(res, Ok(Context::new(vec![])));
    }
}
