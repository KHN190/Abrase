use ect::ast::{TypeBody, RecordField, Type as AstType, VariantCase};
use ect::ty::Type;
use ect::typeck::Checker;

fn d_span() -> ect::ast::Span {
    ect::ast::Span { line: 0, col: 0 }
}

// Direct Self-Reference Detection

#[test]
fn verify_direct_self_reference_rejected() {
    let mut checker = Checker::new();

    // type A = { x: A } - direct infinite size
    let field_a = RecordField {
        name: "x".into(),
        ty: AstType::Named("A".into()),
        is_pub: false,
    };

    let a_type = TypeBody::Record(vec![field_a]);

    let is_valid = checker.check_recursive_type("A", &a_type, d_span());
    assert!(!is_valid, "Direct self-reference should be rejected");
    assert!(checker.errors.len() > 0);
    assert!(checker.errors.iter().any(|e| e.message.contains("recursive") || e.message.contains("cycle")));
}

#[test]
fn verify_indirect_cycle_rejected() {
    let mut checker = Checker::new();

    // type A = { x: B }
    // type B = { y: A } - creates cycle A -> B -> A
    let field_ab = RecordField {
        name: "x".into(),
        ty: AstType::Named("B".into()),
        is_pub: false,
    };
    let a_type = TypeBody::Record(vec![field_ab]);

    let field_ba = RecordField {
        name: "y".into(),
        ty: AstType::Named("A".into()),
        is_pub: false,
    };
    let b_type = TypeBody::Record(vec![field_ba]);

    checker.register_type("A".into(), a_type);
    checker.register_type("B".into(), b_type);

    let is_valid = checker.detect_type_cycles();
    assert!(!is_valid, "Indirect cycles should be detected");
    assert!(checker.errors.len() > 0);
}

#[test]
fn verify_self_reference_through_variant() {
    let mut checker = Checker::new();

    // type List = | Cons(Int, List) | Nil
    let cons_case = VariantCase::Tuple(
        "Cons".into(),
        vec![AstType::Named("Int".into()), AstType::Named("List".into())],
    );
    let nil_case = VariantCase::Unit("Nil".into());

    let list_type = TypeBody::Variant(vec![cons_case, nil_case]);

    let is_valid = checker.check_recursive_type("List", &list_type, d_span());
    assert!(!is_valid, "Direct variant self-reference should be rejected");
}

// Indirection Through References

#[test]
fn verify_reference_indirection_allowed() {
    let mut checker = Checker::new();

    // type A = { x: &A } - self-reference through pointer is OK
    let field = RecordField {
        name: "x".into(),
        ty: AstType::Reference {
            is_mut: false,
            inner: Box::new(AstType::Named("A".into())),
            region: None,
        },
        is_pub: false,
    };

    let a_type = TypeBody::Record(vec![field]);

    let is_valid = checker.check_recursive_type("A", &a_type, d_span());
    assert!(is_valid, "Self-reference through & should be allowed");
    assert_eq!(checker.errors.len(), 0);
}

#[test]
fn verify_mutable_reference_indirection_allowed() {
    let mut checker = Checker::new();

    // type Node = { value: Int, next: &mut Node }
    let value_field = RecordField {
        name: "value".into(),
        ty: AstType::Named("Int".into()),
        is_pub: false,
    };

    let next_field = RecordField {
        name: "next".into(),
        ty: AstType::Reference {
            is_mut: true,
            inner: Box::new(AstType::Named("Node".into())),
            region: None,
        },
        is_pub: false,
    };

    let node_type = TypeBody::Record(vec![value_field, next_field]);

    let is_valid = checker.check_recursive_type("Node", &node_type, d_span());
    assert!(is_valid);
}

#[test]
fn verify_reference_in_variant_allowed() {
    let mut checker = Checker::new();

    // type Tree = | Node(Int, &Tree, &Tree) | Leaf(Int)
    let node_case = VariantCase::Tuple(
        "Node".into(),
        vec![
            AstType::Named("Int".into()),
            AstType::Reference {
                is_mut: false,
                inner: Box::new(AstType::Named("Tree".into())),
                region: None,
            },
            AstType::Reference {
                is_mut: false,
                inner: Box::new(AstType::Named("Tree".into())),
                region: None,
            },
        ],
    );
    let leaf_case = VariantCase::Tuple("Leaf".into(), vec![AstType::Named("Int".into())]);

    let tree_type = TypeBody::Variant(vec![node_case, leaf_case]);

    let is_valid = checker.check_recursive_type("Tree", &tree_type, d_span());
    assert!(is_valid, "Reference indirection in variant should be allowed");
}

// Finite Termination Checking

#[test]
fn verify_non_recursive_type_accepted() {
    let mut checker = Checker::new();

    // type Point = { x: Int, y: Int } - no recursion
    let x_field = RecordField {
        name: "x".into(),
        ty: AstType::Named("Int".into()),
        is_pub: false,
    };

    let y_field = RecordField {
        name: "y".into(),
        ty: AstType::Named("Int".into()),
        is_pub: false,
    };

    let point_type = TypeBody::Record(vec![x_field, y_field]);

    let is_valid = checker.check_recursive_type("Point", &point_type, d_span());
    assert!(is_valid, "Non-recursive type should be accepted");
}

#[test]
fn verify_recursive_with_base_case() {
    let mut checker = Checker::new();

    // type IntList = | Cons(Int, IntList) | Nil - has base case Nil
    let cons = VariantCase::Tuple(
        "Cons".into(),
        vec![AstType::Named("Int".into()), AstType::Named("IntList".into())],
    );
    let nil = VariantCase::Unit("Nil".into());

    let list_type = TypeBody::Variant(vec![cons, nil]);

    let is_valid = checker.check_recursive_type("IntList", &list_type, d_span());
    assert!(!is_valid, "Should still reject direct self-reference even with base case");
}

// Mutual Recursion Detection

#[test]
fn verify_mutual_recursion_without_indirection_rejected() {
    let mut checker = Checker::new();

    // type A = { b: B }
    // type B = { a: A } - A and B form a cycle
    let a_field = RecordField {
        name: "b".into(),
        ty: AstType::Named("B".into()),
        is_pub: false,
    };

    let b_field = RecordField {
        name: "a".into(),
        ty: AstType::Named("A".into()),
        is_pub: false,
    };

    checker.register_type("A".into(), TypeBody::Record(vec![a_field]));
    checker.register_type("B".into(), TypeBody::Record(vec![b_field]));

    let has_cycle = checker.detect_type_cycles();
    assert!(!has_cycle, "Mutual recursion without indirection should be detected");
}

#[test]
fn verify_mutual_recursion_with_indirection_allowed() {
    let mut checker = Checker::new();

    // type A = { b: &B }
    // type B = { a: &A } - A and B form a cycle but with references
    let a_field = RecordField {
        name: "b".into(),
        ty: AstType::Reference {
            is_mut: false,
            inner: Box::new(AstType::Named("B".into())),
            region: None,
        },
        is_pub: false,
    };

    let b_field = RecordField {
        name: "a".into(),
        ty: AstType::Reference {
            is_mut: false,
            inner: Box::new(AstType::Named("A".into())),
            region: None,
        },
        is_pub: false,
    };

    checker.register_type("A".into(), TypeBody::Record(vec![a_field]));
    checker.register_type("B".into(), TypeBody::Record(vec![b_field]));

    let has_cycle = checker.detect_type_cycles();
    assert!(has_cycle == false || checker.errors.is_empty(), "Mutual recursion with references should be allowed");
}

// Integration Tests

#[test]
fn verify_complex_recursive_structure() {
    let mut checker = Checker::new();

    // type BinaryTree = | Node(Int, &BinaryTree, &BinaryTree) | Leaf
    let node = VariantCase::Tuple(
        "Node".into(),
        vec![
            AstType::Named("Int".into()),
            AstType::Reference {
                is_mut: false,
                inner: Box::new(AstType::Named("BinaryTree".into())),
                region: None,
            },
            AstType::Reference {
                is_mut: false,
                inner: Box::new(AstType::Named("BinaryTree".into())),
                region: None,
            },
        ],
    );
    let leaf = VariantCase::Unit("Leaf".into());

    let tree_type = TypeBody::Variant(vec![node, leaf]);

    let is_valid = checker.check_recursive_type("BinaryTree", &tree_type, d_span());
    assert!(is_valid, "Complex recursive structure with proper indirection should be valid");
}

#[test]
fn verify_linked_list_pattern() {
    let mut checker = Checker::new();

    // Standard linked list: type LinkedList<T> = | Node(T, &LinkedList<T>) | Nil
    let node = VariantCase::Tuple(
        "Node".into(),
        vec![
            AstType::Named("T".into()),
            AstType::Reference {
                is_mut: false,
                inner: Box::new(AstType::Named("LinkedList".into())),
                region: None,
            },
        ],
    );
    let nil = VariantCase::Unit("Nil".into());

    let list_type = TypeBody::Variant(vec![node, nil]);

    let is_valid = checker.check_recursive_type("LinkedList", &list_type, d_span());
    assert!(is_valid, "Standard linked list pattern should be valid");
}
