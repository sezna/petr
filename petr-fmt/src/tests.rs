use expect_test::{expect, Expect};
use petr_utils::render_error;

use crate::{
    config::{FormatterConfig, FormatterConfigBuilder as FCB},
    Formattable, FormatterContext,
};

fn check(
    config: FormatterConfig,
    input: impl Into<String>,
    expect: Expect,
) {
    let input = input.into();
    let parser = petr_parse::Parser::new(vec![("test", input)]);
    let (ast, errs, interner, source_map) = parser.into_result();
    if !errs.is_empty() {
        errs.into_iter().for_each(|err| eprintln!("{:?}", render_error(&source_map, err)));
        panic!("fmt failed: code didn't parse");
    }
    let mut ctx = FormatterContext::from_interner(interner).with_config(config);
    let result = ast.line_length_aware_format(&mut ctx).render();
    expect.assert_eq(&result);
}

#[test]
fn basic_func_decl() {
    check(
        Default::default(),
        "function foo(a in 'int, b in 'int) returns 'int + 2 3",
        expect![[r#"
            function foo(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3
        "#]],
    );
}

#[test]
fn func_decl_params_same_line() {
    check(
        FCB::default().put_fn_params_on_new_lines(false).build(),
        "function foo(a in 'int, b in 'int) returns 'int + 2 3",
        expect![[r#"
            function foo(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3
        "#]],
    );
}

#[test]
fn commented_fn_decl() {
    check(
        Default::default(),
        "{- this function does stuff -} function foo(a in 'int, b in 'int) returns 'int + 2 3",
        expect![[r#"
            {- this function does stuff -}
            function foo(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3
        "#]],
    );
}

#[test]
fn multiple_comments_before_fn() {
    check(
        Default::default(),
        "{- comment one -} {- comment two -} function foo(a in 'int, b in 'int) returns 'int + 2 3",
        expect![[r#"
            {- comment one
               comment two -}
            function foo(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3
        "#]],
    );
}
#[test]
fn multiple_comments_before_fn_no_join() {
    check(
        FCB::default().join_comments(false).build(),
        "{- comment one -} {- comment two -} function foo(a in 'int, b in 'int) returns 'int + 2 3",
        expect![[r#"
            {- comment one -}
            {- comment two -}
            function foo(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3
        "#]],
    );
}

#[test]
fn extract_comments_from_within_decl() {
    check(
        FormatterConfig::default(),
        "function {- this comment should get moved to a more normal location -} foo(a in 'int, b in 'int) returns 'int + 2 3",
        expect![[r#"
            {- this comment should get moved to a more normal location -}
            function foo(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3
        "#]],
    );
}

#[test]
fn multiple_functions() {
    check(
        Default::default(),
        r#"function foo(a in 'int, b in 'int) returns 'int + 2 3
        function bar(a in 'int, b in 'int) returns 'int + 2 3"#,
        expect![[r#"
            function foo(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3

            function bar(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3
        "#]],
    );
}

#[test]
fn multiple_functions_with_comments() {
    check(
        Default::default(),
        r#"

        function {- this function is called foo -} foo(a in 'int, b in 'int) returns 'int + 2 3

        function{- this function is called bar -} bar(a in 'int, b in 'int) returns 'int + 2 3"#,
        expect![[r#"
            {- this function is called foo -}
            function foo(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3

            {- this function is called bar -}
            function bar(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3
        "#]],
    );
}

#[test]
fn multiple_functions_more_newlines_between_functions() {
    check(
        FCB::default().newlines_between_items(2).build(),
        r#"

        function {- this function is called foo -} foo(a in 'int, b in 'int) returns 'int + 2 3

        function{- this function is called bar -} bar(a in 'int, b in 'int) returns 'int + 2 3"#,
        expect![[r#"
            {- this function is called foo -}
            function foo(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3


            {- this function is called bar -}
            function bar(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3
        "#]],
    );
}

#[test]
fn multiple_functions_newlines_between_comment_and_item() {
    check(
        FCB::default().newlines_between_comment_and_item(1).build(),
        r#"

        function {- this function is called foo -} foo(a in 'int, b in 'int) returns 'int + 2 3

        {- bar should look like this -}
        function{- this function is called bar -} bar(a in 'int, b in 'int) returns 'int + 2 3"#,
        expect![[r#"
            {- this function is called foo -}

            function foo(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3

            {- bar should look like this
               this function is called bar -}

            function bar(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3
        "#]],
    );
}

#[test]
fn multiple_functions_newlines_between_comment_and_item_unjoined() {
    check(
        FCB::default().newlines_between_comment_and_item(1).join_comments(false).build(),
        r#"

        function {- this function is called foo -} foo(a in 'int, b in 'int) returns 'int + 2 3

        {- bar should look like this -}
        function{- this function is called bar -} bar(a in 'int, b in 'int) returns 'int + 2 3"#,
        expect![[r#"
            {- this function is called foo -}

            function foo(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3

            {- bar should look like this -}
            {- this function is called bar -}

            function bar(
              a ∈ 'int,
              b ∈ 'int,
            ) returns 'int
              + 2 3
        "#]],
    );
}

#[test]
fn ty_decl_basic() {
    check(
        Default::default(),
        "type foo  = a | b",
        expect![[r#"
        type foo = a
                 | b
    "#]],
    );
}

#[test]
fn ty_decl_basic_more_variants() {
    check(
        Default::default(),
        "type foo  = a | b | c | d | e",
        expect![[r#"
            type foo = a
                     | b
                     | c
                     | d
                     | e
        "#]],
    );
}
#[test]
fn ty_decl_basic_same_line() {
    check(
        FCB::default().put_variants_on_new_lines(false).build(),
        "type foo  = a | b | c | d | e",
        expect![[r#"
            type foo = a | b | c | d | e
        "#]],
    );
}

#[test]
fn ty_decl_no_variants() {
    check(
        Default::default(),
        "type foo",
        expect![[r#"
        type foo;
    "#]],
    );
}

#[test]
fn ty_decl_one_variant() {
    check(
        Default::default(),
        "type foo = a",
        expect![[r#"
            type foo = a

        "#]],
    );
}

#[test]
fn ty_decl_one_variant_fields() {
    check(
        Default::default(),
        "type foo = a 'int 'string",
        expect![[r#"
            type foo = a 'int 'string

        "#]],
    );
}

#[test]
fn ty_decl_multi_variant_fields() {
    check(
        Default::default(),
        "type foo = a 'int 'string | b 'bool 'bool",
        expect![[r#"
            type foo = a 'int 'string
                     | b 'bool 'bool
        "#]],
    );
}

#[test]
fn format_list() {
    check(
        Default::default(),
        "function returns_list() returns 'list [1, 2, 3]",
        expect![[r#"
            function returns_list() returns 'list
              [1, 2, 3]
        "#]],
    );
}

#[test]
fn long_line_forces_newlines() {
    check(
        Default::default(),
        "function returns_list() returns 'list [1, 2, 3]

        function returns_list(a in 'a, b in 'b, c in 'c, d in 'd, e in 'e, f in 'f) returns 'list [1, 2, 3, 4, 5]

        function returns_list() returns 'list [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22]
        ",
        expect![[r#"
            function returns_list() returns 'list
              [
                1,
                2,
                3,
              ]

            function returns_list(
              a ∈ 'a,
              b ∈ 'b,
              c ∈ 'c,
              d ∈ 'd,
              e ∈ 'e,
              f ∈ 'f,
            ) returns 'list
              [
                1,
                2,
                3,
                4,
                5,
              ]

            function returns_list() returns 'list
              [
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                9,
                10,
                11,
                12,
                13,
                14,
                15,
                16,
                17,
                18,
                19,
                20,
                21,
                22,
              ]
        "#]],
    );
}

#[test]
fn put_only_params_on_newlines_if_necessary() {
    check(
        Default::default(),
        "function returns_list(a in 'a, b in 'b, c in 'c, d in 'd, e in 'e, f in 'f) returns 'list [1, 2, 3, 4]",
        expect![[r#"
            function returns_list(
              a ∈ 'a,
              b ∈ 'b,
              c ∈ 'c,
              d ∈ 'd,
              e ∈ 'e,
              f ∈ 'f,
            ) returns 'list
              [1, 2, 3, 4]
        "#]],
    );
}

#[test]
fn put_only_params_and_body_on_newlines_if_necessary() {
    check(
        FCB::default().max_line_length(54).build(),
        "function returns_list(a in 'a) returns 'aaaaaaaaaaaaaaaa [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]",
        expect![[r#"
            function returns_list(
              a ∈ 'a,
            ) returns 'aaaaaaaaaaaaaaaa
              [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        "#]],
    );
}

#[test]
fn no_matter_what_the_line_is_too_long_use_shortest_config_best_attempt() {
    check(
        FCB::default().max_line_length(1).build(),
        "function returns_list(a in 'a) returns 'aaaaaaaaaaaaaaaaaaaaaaaaaaaa [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]",
        expect![[r#"
            function returns_list(
              a ∈ 'a,
            ) returns 'aaaaaaaaaaaaaaaaaaaaaaaaaaaa
              [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        "#]],
    );
}

#[test]
fn exported_func() {
    check(
        Default::default(),
        "Function foo() returns 'int + 1 2",
        expect![[r#"
            Function foo() returns 'int
              + 1 2
        "#]],
    )
}

#[test]
fn func_application_in_list() {
    check(
        Default::default(),
        "function foo() returns 'int [
            ~foo 1,2,3,4
        ]",
        expect![[r#"
            function foo() returns 'int
              [~foo 1, 2, 3, 4]
        "#]],
    )
}

#[test]
fn func_application_in_list_with_parens_resolve_ambiguity() {
    check(
        Default::default(),
        "function foo() returns 'int [
            ~foo(1,2),3,4
         ]",
        expect![[r#"
            function foo() returns 'int
              [~foo(1, 2), 3, 4]
        "#]],
    )
}

#[test]
fn func_application() {
    check(
        Default::default(),
        "function foo() returns 'int ~foo 1,2,3,4",
        expect![[r#"
            function foo() returns 'int
              ~foo 1, 2, 3, 4
        "#]],
    )
}

#[test]
fn string_in_list() {
    check(
        Default::default(),
        "function returns_list() returns 'list [\"one\", \"two\", \"three\"]",
        expect![[r#"
            function returns_list() returns 'list
              ["one", "two", "three"]
        "#]],
    );
}

#[test]
fn string_in_operators() {
    check(
        Default::default(),
        "function concat_strings() returns 'string + \"Hello, \" \"world!\"",
        expect![[r#"
            function concat_strings() returns 'string
              + "Hello, " "world!"
        "#]],
    );
}

#[test]
fn string_literals() {
    check(
        Default::default(),
        "function string_literals() returns 'string \"This is a string literal.\"",
        expect![[r#"
            function string_literals() returns 'string
              "This is a string literal."
        "#]],
    );
}

#[test]
fn intrinsic() {
    check(
        Default::default(),
        r#"
        function string_literals() returns 'string "This is a string literal."

        function my_func() returns 'unit @puts(~string_literal)"#,
        expect![[r#"
            function string_literals() returns 'string
              "This is a string literal."

            function my_func() returns 'unit
              @puts(~string_literal)
        "#]],
    );
}

#[test]
fn intrinsic_2() {
    check(
        Default::default(),
        r#"
                  function my_func() returns 'unit @puts("hello, world!")"#,
        expect![[r#"
            function my_func() returns 'unit
              @puts("hello, world!")
        "#]],
    );
}

#[test]
fn let_bindings_trailing_comma() {
    check(
        FCB::default().put_trailing_commas_on_let_bindings(true).build(),
        "function makes_function_call(c in 'int) returns 'int                      let a = 1, b = 20, ~fn_call a, b, let z = 10 c

            function fn_call(a in 'int, b in 'int, c in 'int) returns 'int + a + b c
                     ",
        expect![[r#"
            function makes_function_call(
              c ∈ 'int,
            ) returns 'int
              let a = 1,
                  b = 20,
              ~fn_call a, b, let z = 10, c

            function fn_call(
              a ∈ 'int,
              b ∈ 'int,
              c ∈ 'int,
            ) returns 'int
              + a + b c
        "#]],
    )
}

#[test]
fn let_bindings_no_trailing_comma() {
    check(
        FCB::default().put_trailing_commas_on_let_bindings(false).build(),
        "function makes_function_call(c in 'int) returns 'int                      let a = 1, b = 20, ~fn_call a, b, let z = 10 c

            function fn_call(a in 'int, b in 'int, c in 'int) returns 'int + a + b c
                     ",
        expect![[r#"
            function makes_function_call(
              c ∈ 'int,
            ) returns 'int
              let a = 1,
                  b = 20
              ~fn_call a, b, let z = 10 c

            function fn_call(
              a ∈ 'int,
              b ∈ 'int,
              c ∈ 'int,
            ) returns 'int
              + a + b c
        "#]],
    )
}