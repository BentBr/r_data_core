use super::safe_field;
use crate::dsl::from::EntityFilter;
use crate::dsl::transform::validate_transform;
use crate::dsl::transform::{
    ArithmeticOp, ArithmeticTransform, ConcatTransform, Operand, StringOperand, Transform,
};

fn arithmetic(target: &str, left: Operand, right: Operand) -> Transform {
    Transform::Arithmetic(ArithmeticTransform {
        target: target.to_string(),
        left,
        op: ArithmeticOp::Add,
        right,
    })
}

#[test]
fn valid_arithmetic_field_operands_ok() {
    let t = arithmetic(
        "total",
        Operand::Field {
            field: "a".to_string(),
        },
        Operand::Field {
            field: "b".to_string(),
        },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

#[test]
fn arithmetic_unsafe_target_fails() {
    let t = arithmetic(
        "bad target!",
        Operand::Const { value: 1.0 },
        Operand::Const { value: 2.0 },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn arithmetic_unsafe_left_field_fails() {
    let t = arithmetic(
        "total",
        Operand::Field {
            field: "bad field!".to_string(),
        },
        Operand::Const { value: 2.0 },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn arithmetic_unsafe_right_field_fails() {
    let t = arithmetic(
        "total",
        Operand::Const { value: 2.0 },
        Operand::Field {
            field: "bad field!".to_string(),
        },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn arithmetic_const_operands_ok() {
    let t = arithmetic(
        "total",
        Operand::Const { value: 1.5 },
        Operand::Const { value: 2.5 },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

fn external_operand(entity_definition: &str, filter: EntityFilter, field: &str) -> Operand {
    Operand::ExternalEntityField {
        entity_definition: entity_definition.to_string(),
        filter,
        field: field.to_string(),
    }
}

fn ok_filter() -> EntityFilter {
    EntityFilter {
        field: "sku".to_string(),
        operator: "eq".to_string(),
        value: "abc".to_string(),
    }
}

#[test]
fn arithmetic_external_entity_field_valid_ok() {
    let t = arithmetic(
        "total",
        external_operand("product", ok_filter(), "price"),
        Operand::Const { value: 1.0 },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

#[test]
fn arithmetic_external_entity_field_empty_entity_definition_fails() {
    let t = arithmetic(
        "total",
        external_operand("", ok_filter(), "price"),
        Operand::Const { value: 1.0 },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn arithmetic_external_entity_field_empty_filter_value_fails() {
    let filter = EntityFilter {
        field: "sku".to_string(),
        operator: "eq".to_string(),
        value: String::new(),
    };
    let t = arithmetic(
        "total",
        external_operand("product", filter, "price"),
        Operand::Const { value: 1.0 },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn arithmetic_external_entity_field_unsafe_field_fails() {
    let t = arithmetic(
        "total",
        external_operand("product", ok_filter(), "bad field!"),
        Operand::Const { value: 1.0 },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

fn concat(target: &str, left: StringOperand, right: StringOperand) -> Transform {
    Transform::Concat(ConcatTransform {
        target: target.to_string(),
        left,
        separator: Some(" ".to_string()),
        right,
    })
}

#[test]
fn valid_concat_ok() {
    let t = concat(
        "full_name",
        StringOperand::Field {
            field: "first".to_string(),
        },
        StringOperand::Field {
            field: "last".to_string(),
        },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

#[test]
fn concat_unsafe_target_fails() {
    let t = concat(
        "bad target!",
        StringOperand::ConstString {
            value: "a".to_string(),
        },
        StringOperand::ConstString {
            value: "b".to_string(),
        },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn concat_unsafe_left_fails() {
    let t = concat(
        "full_name",
        StringOperand::Field {
            field: "bad field!".to_string(),
        },
        StringOperand::ConstString {
            value: "b".to_string(),
        },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn concat_unsafe_right_fails() {
    let t = concat(
        "full_name",
        StringOperand::ConstString {
            value: "a".to_string(),
        },
        StringOperand::Field {
            field: "bad field!".to_string(),
        },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn concat_const_operands_ok() {
    let t = concat(
        "full_name",
        StringOperand::ConstString {
            value: "a".to_string(),
        },
        StringOperand::ConstString {
            value: "b".to_string(),
        },
    );
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}
