#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(line: usize, col: usize) -> Self { Self { line, col } }
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
    Mod(String),
    Import { path: Vec<String>, items: Vec<ImportItem> },
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
        is_mut: bool,
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
