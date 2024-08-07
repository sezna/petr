use expect_test::expect;

use crate::run_snippet_inner;
#[ignore]
#[test]
fn test_run_snippet() {
    fn retrieve_output_content(s: &str) {
        expect![[r#"
            <div class="errors"><div class="error">  × Expected one of tokens identifier, [, ~, true, false, string, integer, @intrinsic, or let; found ,
               ╭─[snippet:2:1]
             2 │ fn main() returns 'unit
             3 │             let a = ~std.io.print("yo"),
               ·                                        ┬
               ·                                        ╰── Expected one of tokens identifier, [, ~, true, false, string, integer, @intrinsic, or let; found ,
             4 │             ~std.io.print "Hello, World!"
               ╰────
              help: while parsing function declaration
                      ↪ while parsing expression
                        ↪ expected expression
            </div></div>"#]]
        .assert_eq(s);
    }

    run_snippet_inner(
        r#"
fn main() returns 'unit
            let a = ~std.io.print("yo"),
            ~std.io.print "Hello, World!"
"#,
        retrieve_output_content,
    );
}

#[ignore]
#[test]
fn repro_of_wasm_panic() {
    fn retrieve_output_content(s: &str) {
        expect!["Logs:<br>	yo      <br>Result: <br>	5"].assert_eq(s);
    }
    run_snippet_inner(
        r#"        fn main() returns 'int
  let boo = ~std.io.print "yo"
  5"#,
        retrieve_output_content,
    );
}

#[ignore]
#[test]
fn trailing_comma_repro() {
    fn retrieve_output_content(s: &str) {
        expect![[r#"
            <div class="errors"><div class="error">  × Expected one of tokens identifier, [, ~, true, false, string, integer, @intrinsic, or let; found EOF
               ╭─[snippet:3:1]
             3 │   let a = ~std.io.print "yo",
             4 │   ~std.io.print "Hello, World!"
               ╰────
              help: while parsing function declaration
                      ↪ while parsing expression
                        ↪ expected expression
            </div></div>"#]].assert_eq(s);
    }
    run_snippet_inner(
        r#"
        fn main() returns 'unit 
  let a = ~std.io.print "yo",
  ~std.io.print "Hello, World!""#,
        retrieve_output_content,
    );
}
