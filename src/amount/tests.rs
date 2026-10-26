use super::*;

#[test]
fn parse_table() {
    let cases: &[(&str, &str, Result<i64, AmountError>)] = &[
        ("whole", "1", Ok(10000)),
        ("one_decimal", "1.0", Ok(10000)),
        ("four_decimals", "1.5000", Ok(15000)),
        ("smallest", "0.0001", Ok(1)),
        ("zero_plain", "0", Ok(0)),
        ("zero_padded", "0.0000", Ok(0)),
        ("trailing_dot", "1.", Ok(10000)),
        ("leading_dot", ".5", Ok(5000)),
        ("leading_zeros", "000000000000000000000000000001", Ok(10000)),
        ("max_value", "922337203685477.5807", Ok(i64::MAX)),
        ("empty", "", Err(AmountError::Empty)),
        ("dot_only", ".", Err(AmountError::Malformed)),
        ("plus_sign", "+1", Err(AmountError::Malformed)),
        ("plus_zero", "+0", Err(AmountError::Malformed)),
        ("minus_sign", "-1", Err(AmountError::Malformed)),
        ("minus_zero", "-0", Err(AmountError::Malformed)),
        ("exponent", "1e3", Err(AmountError::Malformed)),
        ("comma_decimal", "1,5", Err(AmountError::Malformed)),
        ("nan", "NaN", Err(AmountError::Malformed)),
        ("inf", "inf", Err(AmountError::Malformed)),
        ("letters", "abc", Err(AmountError::Malformed)),
        ("two_dots", "1.2.3", Err(AmountError::Malformed)),
        ("underscore", "1_000", Err(AmountError::Malformed)),
        ("leading_space", " 2.5", Err(AmountError::Malformed)),
        ("trailing_space", "2.5 ", Err(AmountError::Malformed)),
        ("inner_space", "1 .5", Err(AmountError::Malformed)),
        ("inner_tab", "1\t5", Err(AmountError::Malformed)),
        ("nbsp", "1\u{a0}5", Err(AmountError::Malformed)),
        ("five_decimals", "0.00001", Err(AmountError::TooManyDecimals)),
        ("trailing_zero_fifth", "1.50000", Err(AmountError::TooManyDecimals)),
        ("bad_char_beats_decimals", "1.23456x", Err(AmountError::Malformed)),
        ("one_over_max", "922337203685477.5808", Err(AmountError::Overflow)),
        ("whole_overflow", "922337203685478", Err(AmountError::Overflow)),
        ("huge", "99999999999999999999999999", Err(AmountError::Overflow)),
    ];

    let mut failures = Vec::new();
    for (name, input, expected) in cases {
        let got = input.parse::<Amount>().map(Amount::units);
        if got != *expected {
            failures.push(format!("{name}: input {input:?}: got {got:?}, want {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn display_table() {
    let cases: &[(&str, i64, &str)] = &[
        ("zero", 0, "0.0000"),
        ("one_unit", 1, "0.0001"),
        ("five_units", 5, "0.0005"),
        ("minus_one_unit", -1, "-0.0001"),
        ("minus_half_hundredth", -500, "-0.0500"),
        ("minus_eight", -80000, "-8.0000"),
        ("one", 10000, "1.0000"),
        ("one_and_half", 15000, "1.5000"),
        ("just_below_ten", 99999, "9.9999"),
        ("ten", 100000, "10.0000"),
        ("max", i64::MAX, "922337203685477.5807"),
        ("min", i64::MIN, "-922337203685477.5808"),
    ];

    let mut failures = Vec::new();
    for (name, units, expected) in cases {
        let got = Amount::from_units(*units).to_string();
        if got != *expected {
            failures.push(format!("{name}: units {units}: got {got:?}, want {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn checked_ops_table() {
    let cases: &[(&str, &str, i64, i64, Option<i64>)] = &[
        ("add_normal", "add", 10000, 5000, Some(15000)),
        ("add_zero", "add", i64::MAX, 0, Some(i64::MAX)),
        ("add_overflow", "add", i64::MAX, 1, None),
        ("add_to_negative", "add", i64::MIN, i64::MAX, Some(-1)),
        ("sub_normal", "sub", 15000, 5000, Some(10000)),
        ("sub_goes_negative", "sub", 1, 2, Some(-1)),
        ("sub_to_min", "sub", -i64::MAX, 1, Some(i64::MIN)),
        ("sub_underflow", "sub", i64::MIN, 1, None),
        ("sub_overflow_positive", "sub", i64::MAX, -1, None),
    ];

    let mut failures = Vec::new();
    for (name, op, a, b, expected) in cases {
        let lhs = Amount::from_units(*a);
        let rhs = Amount::from_units(*b);
        let got = match *op {
            "add" => lhs.checked_add(rhs),
            _ => lhs.checked_sub(rhs),
        }
        .map(Amount::units);
        if got != *expected {
            failures.push(format!("{name}: {op} {a} and {b}: got {got:?}, want {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn round_trip_table() {
    let cases: &[(&str, i64)] = &[
        ("zero", 0),
        ("one_unit", 1),
        ("just_below_one", 9999),
        ("one", 10000),
        ("one_and_half", 15000),
        ("max", i64::MAX),
    ];

    let mut failures = Vec::new();
    for (name, units) in cases {
        let amount = Amount::from_units(*units);
        let text = amount.to_string();
        let got = text.parse::<Amount>();
        if got != Ok(amount) {
            failures.push(format!("{name}: text {text:?}: got {got:?}, want Ok({amount:?})"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
