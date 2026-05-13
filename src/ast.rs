#[derive(Debug, PartialEq, Clone, Copy)]
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
pub enum Type {
    Named(String),
    Tuple(Vec<Type>),
    Reference { is_mut: bool, inner: Box<Type> },
    Function { params: Vec<Type>, ret: Box<Type> },
}

#[derive(Debug, PartialEq, Clone)]
pub enum Literal { Int(i64), Float(f64), Bool(bool), Char(char), String(String), Unit }

#[derive(Debug, PartialEq, Clone)]
pub enum BinaryOp { Add, Sub, Mul, Div, Mod, Eq, Neq, Lt, Gt, Lte, Gte, And, Or, Assign }

#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOp { Not, Neg, Deref, Ref, RefMut }

#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    pub stmts: Vec<Spanned<Stmt>>,
    pub ret: Option<Box<Spanned<Expr>>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Literal(Literal),
    Identifier(String),
    Binary { op: BinaryOp, left: Box<Spanned<Expr>>, right: Box<Spanned<Expr>> },
    Unary { op: UnaryOp, right: Box<Spanned<Expr>> },
    Call { callee: Box<Spanned<Expr>>, args: Vec<Spanned<Expr>> },
    Block(Block),
    If { condition: Box<Spanned<Expr>>, consequence: Box<Spanned<Expr>>, alternative: Option<Box<Spanned<Expr>>> },
    Tuple(Vec<Spanned<Expr>>),
    FieldAccess { base: Box<Spanned<Expr>>, field: String },
    Await(Box<Spanned<Expr>>),
    Return(Option<Box<Spanned<Expr>>>),
    Error,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    Let { name: String, is_mut: bool, ty: Option<Type>, value: Spanned<Expr> },
    Expr(Spanned<Expr>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Decl {
    Fn {
        name: String,
        is_async: bool,
        is_pub: bool,
        params: Vec<Param>,
        return_type: Option<Type>,
        body: Block,
    },
    Mod(String),
    Import(Vec<String>),
}