//! Function building.

use std::{
    collections::{HashMap, hash_map::Entry},
    mem,
};

use crate::{
    arena::{Arena, Id},
    syntax::{
        ast::{LocalVar, ResolvedVar, Type, Var},
        error::{Error, VarError, VarErrorKind, VarKind},
        inst::InstData,
        source::{SourceFile, Span},
    },
};

/// Function-local name resolution and numeric ID assignment.
pub(super) struct FuncBuilder<'s> {
    /// The instructions in the function, not associated with basic blocks.
    arena: Arena<InstData<'s>>,
    /// A mapping from variables to definitions.
    names: HashMap<ResolvedVar<'s>, Def>,
    /// The next ID to use for numeric variables.
    next_num: u32,
    /// The number of undefined variables.
    undef_count: usize,
    /// The source file of the function.
    src: &'s SourceFile,
}

/// The definition or first reference of a variable.
struct Def {
    /// The kind of the definition.
    kind: DefKind,
    /// Whether the variable has been defined or only referenced.
    defined: bool,
    /// The span of the definition or first reference.
    span: Span,
}

/// The kind of a definition.
enum DefKind {
    /// Basic block.
    BBlock,
    /// Value (instruction or parameter).
    Value { ty: Type },
}

impl<'s> FuncBuilder<'s> {
    /// Constructs an empty name map.
    pub(super) fn new(src: &'s SourceFile) -> Self {
        FuncBuilder {
            arena: Arena::new(),
            names: HashMap::new(),
            next_num: 0,
            undef_count: 0,
            src,
        }
    }

    /// Inserts an instruction into the arena, not associated with a basic
    /// block.
    pub(super) fn insert_inst(&mut self, inst: InstData<'s>) -> Id<InstData<'s>> {
        self.arena.insert(inst)
    }

    /// Resolves the variable for a basic block definition.
    pub(super) fn define_bblock(
        &mut self,
        var: Var<'s>,
        span: Span,
    ) -> Result<LocalVar<'s>, Error<'s>> {
        self.define_var(var, span, DefKind::BBlock)
    }

    /// Resolves the variable for a basic block reference.
    pub(super) fn use_bblock(
        &mut self,
        var: Var<'s>,
        span: Span,
    ) -> Result<LocalVar<'s>, Error<'s>> {
        self.use_var(var, span, DefKind::BBlock)
    }

    /// Resolves the variable for a value definition.
    pub(super) fn define_value(
        &mut self,
        var: Var<'s>,
        ty: &Type,
        span: Span,
    ) -> Result<LocalVar<'s>, Error<'s>> {
        self.define_var(var, span, DefKind::Value { ty: ty.clone() })
    }

    /// Resolves the variable for a value reference.
    pub(super) fn use_value(
        &mut self,
        var: Var<'s>,
        ty: &Type,
        span: Span,
    ) -> Result<LocalVar<'s>, Error<'s>> {
        self.use_var(var, span, DefKind::Value { ty: ty.clone() })
    }

    /// Resolves the variable for a definition.
    fn define_var(
        &mut self,
        var: Var<'s>,
        span: Span,
        kind: DefKind,
    ) -> Result<LocalVar<'s>, Error<'s>> {
        let resolved = match var {
            Var::Name(name) => ResolvedVar::Name(name),
            Var::Numeric(n) => {
                let resolved = ResolvedVar::Numeric(n);
                if n < self.next_num {
                    let err = VarErrorKind::NonIncreasingNumeric { min: self.next_num };
                    return Err(self.err(resolved, err, span));
                }
                self.next_num = n + 1;
                resolved
            }
            Var::Unnamed => {
                self.next_num += 1;
                ResolvedVar::Numeric(self.next_num - 1)
            }
        };
        match self.names.entry(resolved) {
            Entry::Occupied(entry) => {
                let def = entry.into_mut();
                if def.defined {
                    let err = VarErrorKind::Redefined {
                        first_span: def.span,
                    };
                    return Err(self.err(resolved, err, span));
                }
                if let Err(err) = def.check_kind(kind) {
                    return Err(self.err(resolved, err, span));
                }
                self.undef_count -= 1;
                def.defined = true;
                def.span = span;
            }
            Entry::Vacant(entry) => {
                entry.insert(Def {
                    kind,
                    defined: true,
                    span,
                });
            }
        }
        Ok(LocalVar(resolved))
    }

    /// Resolves the variable for a reference.
    fn use_var(
        &mut self,
        var: Var<'s>,
        span: Span,
        kind: DefKind,
    ) -> Result<LocalVar<'s>, Error<'s>> {
        let resolved = match var {
            Var::Name(name) => ResolvedVar::Name(name),
            Var::Numeric(n) => ResolvedVar::Numeric(n),
            Var::Unnamed => unreachable!("references are always explicit"),
        };
        match self.names.entry(resolved) {
            Entry::Occupied(entry) => {
                let def = &*entry.into_mut();
                if let Err(err) = def.check_kind(kind) {
                    return Err(self.err(resolved, err, span));
                }
            }
            Entry::Vacant(entry) => {
                self.undef_count += 1;
                entry.insert(Def {
                    kind,
                    defined: false,
                    span,
                });
            }
        }
        Ok(LocalVar(resolved))
    }

    /// Finishes building the function and emits any remaining errors.
    pub(super) fn finish(&mut self) -> Result<Arena<InstData<'s>>, Error<'s>> {
        if self.undef_count == 0 {
            return Ok(mem::take(&mut self.arena));
        }
        let Some((&var, def)) = self
            .names
            .iter()
            .filter(|(_, def)| !def.defined)
            .min_by_key(|(_, def)| def.span.start.offset)
        else {
            unreachable!();
        };
        let err = VarErrorKind::Undefined {
            kind: def.kind.var_kind(),
        };
        Err(self.err(var, err, def.span))
    }

    /// Resets the builder, to start building another function.
    pub(super) fn reset(&mut self) {
        self.arena.clear();
        self.names.clear();
        self.next_num = 0;
        self.undef_count = 0;
    }

    fn err(&mut self, var: ResolvedVar<'s>, err: VarErrorKind, span: Span) -> Error<'s> {
        Error {
            detail: VarError { var, kind: err }.into(),
            span,
            src: self.src,
        }
    }
}

impl DefKind {
    /// Gets the tag for this kind.
    fn var_kind(&self) -> VarKind {
        match self {
            DefKind::BBlock => VarKind::BBlock,
            DefKind::Value { .. } => VarKind::Value,
        }
    }
}

impl Def {
    /// Validates that a use or definition matches the stored kind.
    fn check_kind(&self, kind: DefKind) -> Result<(), VarErrorKind> {
        match (&self.kind, kind) {
            (DefKind::Value { ty: def_ty }, DefKind::Value { ty }) => {
                if def_ty == &ty {
                    Ok(())
                } else {
                    Err(VarErrorKind::TypeMismatch {
                        ty,
                        def_ty: def_ty.clone(),
                        def_span: self.span,
                    })
                }
            }
            (DefKind::BBlock, DefKind::BBlock) => Ok(()),
            (_, kind) => Err(VarErrorKind::KindMismatch {
                kind: kind.var_kind(),
                def_kind: self.kind.var_kind(),
                def_span: self.span,
            }),
        }
    }
}
