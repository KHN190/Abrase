#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExprId(pub u32);

impl ExprId {
    pub const NONE: ExprId = ExprId(0);
}

#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub line: usize,
    pub col: usize,
    pub id: ExprId,
}

impl Span {
    pub fn new(line: usize, col: usize) -> Self { Self { line, col, id: ExprId::NONE } }
    pub fn with_id(mut self, id: ExprId) -> Self { self.id = id; self }
}

impl PartialEq for Span {
    fn eq(&self, other: &Self) -> bool { self.line == other.line && self.col == other.col }
}
impl Eq for Span {}
impl std::hash::Hash for Span {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.line.hash(state);
        self.col.hash(state);
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

#[derive(Debug, PartialEq, Clone)]
pub enum StringPart {
    Literal(String),
    Interp(Vec<String>), // path segments: ["user", "name"] for {user.name}
}

#[derive(Debug, PartialEq, Clone)]
pub struct EffectItem {
    pub name: Vec<String>,        // qualified name, e.g. ["gpu", "device"]
    pub arg: Option<Box<Type>>,   // for parameterised effects like exn<E>
}

#[derive(Debug, PartialEq, Clone)]
pub struct GenericParam {
    pub name: String, // type or effect variable
}

#[derive(Debug, PartialEq, Clone)]
pub struct WhereBound {
    pub ty: Type,
    pub bounds: Vec<Vec<String>>, // list of trait names (each may be qualified)
}

#[derive(Debug, PartialEq, Clone)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<AttrArg>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AttrArg {
    Ident(String),
    Lit(Literal),
    Named(String, Literal),
}

#[derive(Debug, PartialEq, Clone)]
pub enum OwnershipAttr { Copy, Move, Share }

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    Named(String),                                       // Int, String, User
    Qualified(Vec<String>),                              // io.IoError
    Generic { name: String, args: Vec<Type> },           // List<Int>, Result<T, E>
    Array { elem: Box<Type>, size: usize },              // [Int; 16]
    Tuple(Vec<Type>),                                    // (), (Int, Bool), (Int,)
    Reference { is_mut: bool, inner: Box<Type>, region: Option<String> }, // &T, &mut T in r
    Function { params: Vec<Type>, effects: Vec<EffectItem>, ret: Box<Type> },
}


#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
    StringInterp(Vec<StringPart>),
    Unit,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinaryOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Gt, Lte, Gte,
    And, Or,
    BitAnd, BitOr, BitXor, Shl, Shr,
    Assign, AddAssign, SubAssign, MulAssign, DivAssign, ModAssign,
}

#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOp { Not, Neg, Deref, Ref, RefMut }

#[derive(Debug, PartialEq, Clone)]
pub enum Pattern {
    Wildcard,
    Literal(Literal),

    Range { start: Option<Literal>, end: Option<Literal>, inclusive: bool },

    Bind(String),
    Tuple(Vec<Spanned<Pattern>>),
    Array(Vec<Spanned<Pattern>>),
    Record { ty: Vec<String>, fields: Vec<FieldPattern>, rest: bool },
    Variant { ty: Vec<String>, args: Vec<Spanned<Pattern>> },

    Rest,

    Ref(Box<Spanned<Pattern>>),
    Or(Vec<Spanned<Pattern>>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct FieldPattern {
    pub name: String,
    pub is_mut: bool,
    pub pattern: Option<Spanned<Pattern>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MatchArm {
    pub pattern: Spanned<Pattern>,
    pub guard: Option<Spanned<Expr>>,
    pub body: Spanned<Expr>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct HandleArm {
    pub kind: HandleArmKind,
    pub pattern: Option<Spanned<Pattern>>,
    pub body: Spanned<Expr>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum HandleArmKind {
    Return,
    Exn,
    Effect(Vec<String>), // qualified effect name, e.g. ["logger", "log"]
}

#[derive(Debug, PartialEq, Clone)]
pub struct FieldInit {
    pub name: String,

    pub value: Option<Spanned<Expr>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ClosureParam {
    pub pattern: Spanned<Pattern>,
    pub ty: Option<Type>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    pub stmts: Vec<Spanned<Stmt>>,
    pub ret: Option<Box<Spanned<Expr>>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Literal(Literal),
    Identifier(String),
    Binary   { op: BinaryOp, left: Box<Spanned<Expr>>, right: Box<Spanned<Expr>> },
    Unary    { op: UnaryOp,  right: Box<Spanned<Expr>> },
    Call     { callee: Box<Spanned<Expr>>, args: Vec<Spanned<Expr>> },
    Index    { base: Box<Spanned<Expr>>,   index: Box<Spanned<Expr>> },
    Block(Block),
    If       { condition: Box<Spanned<Expr>>, consequence: Box<Spanned<Expr>>,
               alternative: Option<Box<Spanned<Expr>>> },
    Match    { scrutinee: Box<Spanned<Expr>>, arms: Vec<MatchArm> },
    For      { pattern: Spanned<Pattern>, iter: Box<Spanned<Expr>>, body: Block },
    While    { condition: Box<Spanned<Expr>>, body: Block },
    Loop     { body: Block },
    Break    (Option<Box<Spanned<Expr>>>),
    Continue,
    Return   (Option<Box<Spanned<Expr>>>),
    Throw    (Box<Spanned<Expr>>),

    Question (Box<Spanned<Expr>>),
    Paren    (Box<Spanned<Expr>>),
    Tuple    (Vec<Spanned<Expr>>),
    Array    (Vec<Spanned<Expr>>),

    ArrayRepeat { elem: Box<Spanned<Expr>>, count: Box<Spanned<Expr>> },
    Record   { ty: Vec<String>, fields: Vec<FieldInit> },
    Variant  { ty: Vec<String>, args: Vec<Spanned<Expr>> },
    FieldAccess { base: Box<Spanned<Expr>>, field: String },
    Closure  {
        is_move: bool,
        params: Vec<ClosureParam>,
        effects: Vec<EffectItem>,
        return_type: Option<Type>,
        body: Box<Spanned<Expr>>,
    },

    Range    { start: Option<Box<Spanned<Expr>>>, end: Option<Box<Spanned<Expr>>>, inclusive: bool },
    Region   { label: Option<String>, body: Block },
    Handle   { expr: Box<Spanned<Expr>>, arms: Vec<HandleArm> },
    Resume   (Option<Box<Spanned<Expr>>>),
    Error,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {

    Let { pattern: Spanned<Pattern>, is_mut: bool, ty: Option<Type>, value: Spanned<Expr> },
    Expr(Spanned<Expr>),
    Empty,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Param {
    Named { pattern: Spanned<Pattern>, ty: Type },
    SelfVal,
    SelfRef { is_mut: bool },
}

#[derive(Debug, PartialEq, Clone)]
pub struct RecordField {
    pub is_pub: bool,
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, PartialEq, Clone)]
pub enum VariantCase {
    Unit(String),
    Tuple(String, Vec<Type>),
    Record(String, Vec<RecordField>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum TypeBody {
    Record(Vec<RecordField>),
    Variant(Vec<VariantCase>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct FnSignature {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub effects: Vec<EffectItem>,
    pub return_type: Option<Type>,
    pub where_clause: Vec<WhereBound>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FnDecl {
    pub attrs: Vec<Attribute>,
    pub is_pub: bool,
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub effects: Vec<EffectItem>,
    pub return_type: Option<Type>,
    pub where_clause: Vec<WhereBound>,
    pub body: Block,
}


#[derive(Debug, PartialEq, Clone)]
pub enum TraitItem {
    Required(FnSignature),
    Default(FnDecl),
}


#[derive(Debug, PartialEq, Clone)]
pub struct ImportItem {
    pub name: String,
    pub alias: Option<String>,
}


#[derive(Debug, PartialEq, Clone)]
pub enum Decl {
    Fn(FnDecl),
    ModEnter(Vec<String>),
    ModExit,
    Use { path: Vec<String>, items: Vec<ImportItem> },
    Type {
        attrs: Vec<Attribute>,
        is_pub: bool,
        ownership: Option<OwnershipAttr>,
        name: String,
        generics: Vec<GenericParam>,
        body: TypeBody,
    },
    TypeAlias {
        is_pub: bool,
        name: String,
        generics: Vec<GenericParam>,
        ty: Type,
    },
    Trait {
        is_pub: bool,
        name: String,
        generics: Vec<GenericParam>,
        where_clause: Vec<WhereBound>,
        items: Vec<TraitItem>,
    },
    Impl {
        generics: Vec<GenericParam>,
        trait_name: Option<Vec<String>>,
        for_type: Type,
        where_clause: Vec<WhereBound>,
        methods: Vec<FnDecl>,
    },
    Const {
        is_pub: bool,
        is_fn: bool,
        name: String,
        generics: Vec<GenericParam>,
        params: Vec<Param>,
        ty: Type,
        value: Spanned<Expr>,
    },
    Static {
        is_pub: bool,
        name: String,
        ty: Type,
        value: Spanned<Expr>,
    },
    Effect {
        is_pub: bool,
        name: String,
        ops: Vec<FnSignature>,
    },
    EffectAlias {
        is_pub: bool,
        name: String,
        effects: Vec<EffectItem>,
    },
}

pub fn stamp_expr_ids(decls: &mut [Decl]) {
    let mut n: u32 = 0;
    for d in decls.iter_mut() {
        stamp_decl(d, &mut n);
    }
}

fn stamp_decl(d: &mut Decl, n: &mut u32) {
    match d {
        Decl::Fn(fd) => stamp_block(&mut fd.body, n),
        Decl::Const { value, .. } | Decl::Static { value, .. } => stamp_expr(value, n),
        Decl::Trait { items, .. } => {
            for it in items.iter_mut() {
                if let TraitItem::Default(fd) = it { stamp_block(&mut fd.body, n); }
            }
        }
        Decl::Impl { methods, .. } => {
            for fd in methods.iter_mut() { stamp_block(&mut fd.body, n); }
        }
        _ => {}
    }
}

fn stamp_block(b: &mut Block, n: &mut u32) {
    for s in b.stmts.iter_mut() {
        match &mut s.node {
            Stmt::Let { value, .. } => stamp_expr(value, n),
            Stmt::Expr(e) => stamp_expr(e, n),
            Stmt::Empty => {}
        }
    }
    if let Some(r) = b.ret.as_mut() { stamp_expr(r, n); }
}

fn stamp_expr(e: &mut Spanned<Expr>, n: &mut u32) {
    *n += 1;
    e.span.id = ExprId(*n);
    match &mut e.node {
        Expr::Binary { left, right, .. } => { stamp_expr(left, n); stamp_expr(right, n); }
        Expr::Unary { right, .. } => stamp_expr(right, n),
        Expr::Call { callee, args } => { stamp_expr(callee, n); for a in args { stamp_expr(a, n); } }
        Expr::Index { base, index } => { stamp_expr(base, n); stamp_expr(index, n); }
        Expr::Block(b) => stamp_block(b, n),
        Expr::If { condition, consequence, alternative } => {
            stamp_expr(condition, n);
            stamp_expr(consequence, n);
            if let Some(a) = alternative.as_mut() { stamp_expr(a, n); }
        }
        Expr::Match { scrutinee, arms } => {
            stamp_expr(scrutinee, n);
            for arm in arms.iter_mut() {
                if let Some(g) = arm.guard.as_mut() { stamp_expr(g, n); }
                stamp_expr(&mut arm.body, n);
            }
        }
        Expr::For { iter, body, .. } => { stamp_expr(iter, n); stamp_block(body, n); }
        Expr::While { condition, body } => { stamp_expr(condition, n); stamp_block(body, n); }
        Expr::Loop { body } | Expr::Region { body, .. } => stamp_block(body, n),
        Expr::Break(o) | Expr::Return(o) | Expr::Resume(o) => {
            if let Some(x) = o.as_mut() { stamp_expr(x, n); }
        }
        Expr::Throw(x) | Expr::Question(x) | Expr::Paren(x) => stamp_expr(x, n),
        Expr::Tuple(xs) | Expr::Array(xs) | Expr::Variant { args: xs, .. } => {
            for x in xs.iter_mut() { stamp_expr(x, n); }
        }
        Expr::ArrayRepeat { elem, count } => { stamp_expr(elem, n); stamp_expr(count, n); }
        Expr::Record { fields, .. } => {
            for f in fields.iter_mut() {
                if let Some(v) = f.value.as_mut() { stamp_expr(v, n); }
            }
        }
        Expr::FieldAccess { base, .. } => stamp_expr(base, n),
        Expr::Closure { body, .. } => stamp_expr(body, n),
        Expr::Range { start, end, .. } => {
            if let Some(s) = start.as_mut() { stamp_expr(s, n); }
            if let Some(en) = end.as_mut() { stamp_expr(en, n); }
        }
        Expr::Handle { expr, arms } => {
            stamp_expr(expr, n);
            for arm in arms.iter_mut() { stamp_expr(&mut arm.body, n); }
        }
        Expr::Literal(_) | Expr::Identifier(_) | Expr::Continue | Expr::Error => {}
    }
}
