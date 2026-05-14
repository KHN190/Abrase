#[test]
fn debug_error_message() {
    let mut checker = typeck::Checker::new();
    
    let identifiers = vec!["name".into()];
    let span = typeck::d_span();
    checker.validate_string_interpolation(&identifiers, span);
    
    if checker.errors.len() > 0 {
        println!("Error message: '{}'", checker.errors[0].message);
    } else {
        println!("No errors generated");
    }
}
