## BNF Grammar

- `'literal'` - 终结符
- `<rule>` - 非终结符
- `?` - 可选
- `*` - 零次或多次
- `+` - 一次或多次
- `|` - 或
- `(...)` - 分组

### 1 Top

```bnf
<source-file>     ::= <mod-decl>? <top-decl>*

<mod-decl>        ::= 'mod' <mod-path>

<mod-path>        ::= <identifier> ('.' <identifier>)*

<top-decl>        ::= <import>
                    | <type-decl>
                    | <type-alias>
                    | <trait-decl>
                    | <impl-decl>
                    | <fn-decl>
                    | <const-decl>
                    | <effect-alias>
                    | <effect-decl>

<import>          ::= 'import' <mod-path> <import-list>? ';'

<import-list>     ::= '.' '{' <import-item> (',' <import-item>)* ','? '}'

<import-item>     ::= <identifier> ('as' <identifier>)?
                    | <type-name> ('as' <type-name>)?
```

### 2 Types

```bnf
<type-decl>       ::= <attribute>* <ownership-attr>? 'pub'? 'type' <type-name> <generic-params>?
                      '=' <type-body>

<type-body>       ::= <record-body>
                    | <variant-body>

<record-body>     ::= '{' <record-field> (',' <record-field>)* ','? '}'

<record-field>    ::= 'pub'? <identifier> ':' <type>

<variant-body>    ::= '|'? <variant-case> ('|' <variant-case>)*

<variant-case>    ::= <type-name>
                    | <type-name> '(' <type> (',' <type>)* ')'
                    | <type-name> <record-body>

<type-alias>      ::= 'pub'? 'type' 'alias' <type-name> <generic-params>? '=' <type>

<generic-params>  ::= '<' <generic-param> (',' <generic-param>)* '>'

<generic-param>   ::= <type-name>
                    | <identifier>                          /* effect 变量 */

<ownership-attr>  ::= '@copy' | '@move' | '@share'
```

### 3 Type Expressions

```bnf
<type>            ::= <function-type>
                    | <non-fn-type>

<function-type>   ::= '(' <type-list>? ')' '->' <effect-set>? <type>

<non-fn-type>     ::= <base-type>
                    | <tuple-type>
                    | <array-type>
                    | <reference-type>
                    | <generic-instance>

<base-type>       ::= <qualified-name>

<tuple-type>      ::= '(' ')'                               /* unit */
                    | '(' <type> ',' ')'                    /* 单元素元组 */
                    | '(' <type> (',' <type>)+ ','? ')'

<array-type>      ::= '[' <type> ';' <int-literal> ']'

<reference-type>  ::= '&' <type> ('in' <identifier>)?
                    | '&' 'mut' <type> ('in' <identifier>)?

<generic-instance>::= <qualified-name> '<' <type-arg-list> '>'

<type-arg-list>   ::= <type> (',' <type>)*

<type-list>       ::= <type> (',' <type>)*

<qualified-name>  ::= <identifier> ('.' <identifier>)* ('.' <type-name>)?
                    | <type-name>
```

### 4 Effect

```bnf
<effect-set>      ::= '<' <effect-item> (',' <effect-item>)* '>'

<effect-item>     ::= <effect-name>
                    | <effect-name> '<' <type> '>'          /* 参数化 effect 如 exn<E> */
                    | <qualified-name>                       /* 模块限定 effect */
                    | <identifier>                           /* effect 变量 */

<effect-alias>    ::= 'pub'? 'effect' 'alias' <identifier> '=' <effect-set>

<effect-decl>     ::= 'pub'? 'effect' <identifier> '{' <effect-op>* '}'

<effect-op>       ::= 'fn' <identifier> '(' <param-list>? ')' '->' <type>
```

### 5 Functions

```bnf
<fn-decl>         ::= <attribute>* 'pub'? 'async'? 'fn' <identifier> <generic-params>?
                      '(' <param-list>? ')'
                      ('->' <effect-set>? <type>)?
                      <where-clause>?
                      <block>

<param-list>      ::= <param> (',' <param>)*

<param>           ::= <pattern> ':' <type>
                    | 'self' ':' <self-type>
                    | 'self'
                    | '&' 'self'
                    | '&' 'mut' 'self'

<self-type>       ::= 'Self' | '&' 'Self' | '&' 'mut' 'Self'

<where-clause>    ::= 'where' <where-bound> (',' <where-bound>)*

<where-bound>     ::= <type> ':' <trait-bound>

<trait-bound>     ::= <qualified-name> ('+' <qualified-name>)*

<const-decl>      ::= 'pub'? 'const' ('fn')? <identifier>
                      <generic-params>? ('(' <param-list>? ')')?
                      ':' <type>
                      ('->' <effect-set>? <type>)?
                      '=' <expr> ';'
```

### 6 Trait & Impl

```bnf
<trait-decl>      ::= 'pub'? 'trait' <type-name> <generic-params>?
                      <where-clause>?
                      '{' <trait-item>* '}'

<trait-item>      ::= <fn-decl>                             /* 可有默认实现 */
                    | <fn-signature>

<fn-signature>    ::= 'async'? 'fn' <identifier> <generic-params>?
                      '(' <param-list>? ')'
                      ('->' <effect-set>? <type>)?
                      <where-clause>?

<impl-decl>       ::= 'impl' <generic-params>? <qualified-name> ('for' <type>)?
                      <where-clause>?
                      '{' <fn-decl>* '}'
```

### 7 Statement & Block

```bnf
<block>           ::= '{' <stmt>* <expr>? '}'

<stmt>            ::= <let-stmt> ';'
                    | <expr> ';'
                    | <expr-with-block>
                    | ';'

<let-stmt>        ::= 'let' 'mut'? <pattern> (':' <type>)? '=' <expr>

<expr-with-block> ::= <if-expr>
                    | <match-expr>
                    | <for-expr>
                    | <while-expr>
                    | <loop-expr>
                    | <scope-expr>
                    | <region-expr>
                    | <handle-expr>
                    | <block>
```

注: `<expr-with-block>` 末尾不需要 `;`,因为它本身以 `}` 结束。

### 8 Expressions

```bnf
<expr>            ::= <assign-expr>

<assign-expr>     ::= <or-expr> (<assign-op> <or-expr>)?

<assign-op>       ::= '=' | '+=' | '-=' | '*=' | '/=' | '%='

<or-expr>         ::= <and-expr> ('||' <and-expr>)*

<and-expr>        ::= <compare-expr> ('&&' <compare-expr>)*

<compare-expr>    ::= <range-expr> (<compare-op> <range-expr>)?

<compare-op>      ::= '==' | '!=' | '<' | '>' | '<=' | '>='

<range-expr>      ::= <add-expr> (('..' | '..=') <add-expr>?)?
                    | ('..' | '..=') <add-expr>?

<add-expr>        ::= <mul-expr> (('+' | '-') <mul-expr>)*
<mul-expr>        ::= <unary-expr> (('*' | '/' | '%') <unary-expr>)*

<unary-expr>      ::= ('!' | '-' | '&' | '&' 'mut' | '*') <unary-expr>
                    | <postfix-expr>

<postfix-expr>    ::= <primary-expr> <postfix-op>*

<postfix-op>      ::= '.' <identifier>                      /* 字段/方法访问 */
                    | '.' 'await'                           /* await */
                    | '(' <arg-list>? ')'                   /* 函数调用 */
                    | '[' <expr> ']'                        /* 索引 */
                    | '?'                                   /* 错误传播 */

<arg-list>        ::= <expr> (',' <expr>)*

<primary-expr>    ::= <literal>
                    | <qualified-name>
                    | <tuple-expr>
                    | <array-expr>
                    | <record-expr>
                    | <variant-expr>
                    | <closure-expr>
                    | <if-expr>
                    | <match-expr>
                    | <block>
                    | <return-expr>
                    | <break-expr>
                    | <continue-expr>
                    | <throw-expr>
                    | <scope-expr>
                    | <region-expr>
                    | <handle-expr>
                    | <thread-expr>
                    | <for-expr>
                    | <while-expr>
                    | <loop-expr>
                    | '(' <expr> ')'

<tuple-expr>      ::= '(' ')'
                    | '(' <expr> ',' ')'
                    | '(' <expr> (',' <expr>)+ ','? ')'

<array-expr>      ::= '[' <expr-list>? ']'
                    | '[' <expr> ';' <expr> ']'

<expr-list>       ::= <expr> (',' <expr>)*

<record-expr>     ::= <qualified-name> '{' <field-init> (',' <field-init>)* ','? '}'

<field-init>      ::= <identifier> ':' <expr>
                    | <identifier>                          /* 简写 */

<variant-expr>    ::= <qualified-name> '(' <expr-list>? ')'

<closure-expr>    ::= 'move'? '|' <closure-params>? '|'
                      ('->' <effect-set>? <type>)?
                      <closure-body>

<closure-params>  ::= <closure-param> (',' <closure-param>)*

<closure-param>   ::= <pattern> (':' <type>)?

<closure-body>    ::= <expr>
                    | <block>
```

### 9 Control Flow

```bnf
<if-expr>         ::= 'if' <expr> <block> ('else' (<if-expr> | <block>))?

<match-expr>      ::= 'match' <expr> '{' <match-arm> (',' <match-arm>)* ','? '}'

<match-arm>       ::= <pattern> ('if' <expr>)? '=>' <match-body>

<match-body>      ::= <expr>
                    | <block>

<for-expr>        ::= 'for' <pattern> 'in' <expr> <block>

<while-expr>      ::= 'while' <expr> <block>

<loop-expr>       ::= 'loop' <block>

<return-expr>     ::= 'return' <expr>?

<break-expr>      ::= 'break' <expr>?

<continue-expr>   ::= 'continue'

<throw-expr>      ::= 'throw' <expr>
```

### 10 Scope

```bnf
<scope-expr>      ::= 'scope' <identifier>? <scope-options>? <block>

<scope-options>   ::= 'with' <expr>

<region-expr>     ::= 'region' <identifier>? <block>

<handle-expr>     ::= 'handle' <expr> '{' <handle-arm> (',' <handle-arm>)* ','? '}'

<handle-arm>      ::= 'return' <pattern> '=>' <match-body>
                    | 'exn' <pattern> '=>' <match-body>
                    | <qualified-name> <pattern>? '=>' <match-body>

<thread-expr>     ::= <identifier> '.' 'thread' '(' <expr> ')'
                    /* 在 scope 内调用,如 s.thread(...) */
```

### 11 Pattern

```bnf
<pattern>         ::= <pattern-alt>

<pattern-alt>     ::= <pattern-primary> ('|' <pattern-primary>)*

<pattern-primary> ::= '_'
                    | <literal-pattern>
                    | <range-pattern>
                    | <identifier>                          /* 绑定或变量 */
                    | <wildcard-pattern>
                    | <tuple-pattern>
                    | <array-pattern>
                    | <record-pattern>
                    | <variant-pattern>
                    | '&' <pattern>
                    | '(' <pattern> ')'

<literal-pattern> ::= <literal>

<range-pattern>   ::= <literal> '..' <literal>
                    | <literal> '..=' <literal>
                    | <literal> '..'                        /* 开区间 */
                    | '..=' <literal>

<wildcard-pattern>::= '_'

<tuple-pattern>   ::= '(' <pattern> (',' <pattern>)* ','? ')'

<array-pattern>   ::= '[' <pattern> (',' <pattern>)* ','? ']'

<record-pattern>  ::= <qualified-name> '{' <field-pattern> (',' <field-pattern>)* (',' '..')? ','? '}'

<field-pattern>   ::= <identifier> ':' <pattern>
                    | <identifier>                          /* 简写 */

<variant-pattern> ::= <qualified-name> ('(' <pattern> (',' <pattern>)* ')')?
                    | <qualified-name> <record-body>
```

### 12 Literals

```bnf
<literal>         ::= <int-literal>
                    | <float-literal>
                    | <bool-literal>
                    | <char-literal>
                    | <string-literal>
                    | <unit-literal>

<int-literal>     ::= <digit>+

<float-literal>   ::= <digit>+ '.' <digit>+ <exponent>?
                    | <digit>+ <exponent>

<exponent>        ::= ('e' | 'E') ('+' | '-')? <digit>+

<bool-literal>    ::= 'true' | 'false'

<char-literal>    ::= "'" <char-content> "'"

<string-literal>  ::= '"' <string-content>* '"'

<unit-literal>    ::= '(' ')'

<digit>           ::= '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9'

<char-content>    ::= <printable-char-except-quote-backslash>
                    | <escape-sequence>

<string-content>  ::= <printable-char-except-quote-backslash>
                    | <escape-sequence>
                    | <interpolation>

<interpolation>   ::= '{' <identifier> ('.' <identifier>)* '}'

<escape-sequence> ::= '\\' ('n' | 't' | 'r' | '\\' | '"' | "'" | '0'
                            | 'u' '{' <hex-digit>+ '}')

<hex-digit>       ::= <digit> | 'a'..'f' | 'A'..'F'
```

### 13 Attributes

```bnf
<attribute>       ::= '@' <identifier> ('(' <attr-args>? ')')?

<attr-args>       ::= <attr-arg> (',' <attr-arg>)*

<attr-arg>        ::= <identifier>
                    | <literal>
                    | <identifier> '=' <literal>
```

属性可前置于声明:
```
@derive(Eq, Ord)
@export
pub fn handler(req: Request) -> Response { ... }
```