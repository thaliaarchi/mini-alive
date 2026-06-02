//! Function building.

use std::{
    collections::{HashMap, hash_map::Entry},
    mem,
};

use crate::{
    arena::{Arena, Id},
    syntax::{
        ast::{LocalVar, ResolvedVar, Var},
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
    kind: VarKind,
    /// Whether the variable has been defined or only referenced.
    defined: bool,
    /// The span of the definition or first reference.
    span: Span,
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
        self.define_var(var, span, VarKind::BBlock)
    }

    /// Resolves the variable for a basic block reference.
    pub(super) fn use_bblock(
        &mut self,
        var: Var<'s>,
        span: Span,
    ) -> Result<LocalVar<'s>, Error<'s>> {
        self.use_var(var, span, VarKind::BBlock)
    }

    /// Resolves the variable for a value definition.
    pub(super) fn define_value(
        &mut self,
        var: Var<'s>,
        span: Span,
    ) -> Result<LocalVar<'s>, Error<'s>> {
        self.define_var(var, span, VarKind::Value)
    }

    /// Resolves the variable for a value reference.
    pub(super) fn use_value(
        &mut self,
        var: Var<'s>,
        span: Span,
    ) -> Result<LocalVar<'s>, Error<'s>> {
        self.use_var(var, span, VarKind::Value)
    }

    /// Resolves the variable for a definition.
    fn define_var(
        &mut self,
        var: Var<'s>,
        span: Span,
        kind: VarKind,
    ) -> Result<LocalVar<'s>, Error<'s>> {
        let resolved = match var {
            Var::Name(name) => ResolvedVar::Name(name),
            Var::Numeric(n) => {
                let resolved = ResolvedVar::Numeric(n);
                if n < self.next_num {
                    let err = VarError {
                        var: resolved,
                        kind: VarErrorKind::NonIncreasingNumeric { min: self.next_num },
                    };
                    return Err(Error {
                        detail: err.into(),
                        span,
                        src: self.src,
                    });
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
                    let err = VarError {
                        var: resolved,
                        kind: VarErrorKind::Redefined {
                            first_span: def.span,
                        },
                    };
                    return Err(Error {
                        detail: err.into(),
                        span,
                        src: self.src,
                    });
                }
                if def.kind != kind {
                    let err = VarError {
                        var: resolved,
                        kind: VarErrorKind::KindMismatch {
                            kind,
                            def_kind: def.kind,
                            def_span: def.span,
                        },
                    };
                    return Err(Error {
                        detail: err.into(),
                        span,
                        src: self.src,
                    });
                }
                self.undef_count -= 1;
                def.defined = true;
                def.span = span;
                Ok(LocalVar(resolved))
            }
            Entry::Vacant(entry) => {
                entry.insert(Def {
                    kind,
                    defined: true,
                    span,
                });
                Ok(LocalVar(resolved))
            }
        }
    }

    /// Resolves the variable for a reference.
    fn use_var(
        &mut self,
        var: Var<'s>,
        span: Span,
        kind: VarKind,
    ) -> Result<LocalVar<'s>, Error<'s>> {
        let resolved = match var {
            Var::Name(name) => ResolvedVar::Name(name),
            Var::Numeric(n) => ResolvedVar::Numeric(n),
            Var::Unnamed => unreachable!("references are always explicit"),
        };
        match self.names.entry(resolved) {
            Entry::Occupied(entry) => {
                let def = entry.into_mut();
                if def.kind != kind {
                    let err = VarError {
                        var: resolved,
                        kind: VarErrorKind::KindMismatch {
                            kind,
                            def_kind: def.kind,
                            def_span: def.span,
                        },
                    };
                    return Err(Error {
                        detail: err.into(),
                        span,
                        src: self.src,
                    });
                }
                Ok(LocalVar(resolved))
            }
            Entry::Vacant(entry) => {
                self.undef_count += 1;
                entry.insert(Def {
                    kind,
                    defined: false,
                    span,
                });
                Ok(LocalVar(resolved))
            }
        }
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
        let err = VarError {
            var,
            kind: VarErrorKind::Undefined { kind: def.kind },
        };
        Err(Error {
            detail: err.into(),
            span: def.span,
            src: self.src,
        })
    }

    /// Resets the builder, to start building another function.
    pub(super) fn reset(&mut self) {
        self.arena.clear();
        self.names.clear();
        self.next_num = 0;
        self.undef_count = 0;
    }
}
