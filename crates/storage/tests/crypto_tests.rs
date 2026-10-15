use storage::crypto::{
    derive_kek, normalize_recovery, open_sealed, random_bytes, recovery_code, seal, KdfParams,
};

const FAST: KdfParams = KdfParams::fast_for_tests();

#[test]
fn derivation_is_deterministic_and_depends_on_every_input() {
    let salt = [7u8; 16];
    let a = derive_kek(b"1234", &salt, FAST).unwrap();
    let b = derive_kek(b"1234", &salt, FAST).unwrap();
    assert_eq!(*a, *b);
    assert_ne!(*a, *derive_kek(b"1235", &salt, FAST).unwrap(), "інший PIN");
    assert_ne!(
        *a,
        *derive_kek(b"1234", &[8u8; 16], FAST).unwrap(),
        "інша сіль"
    );
    let other = KdfParams { t: 2, ..FAST };
    assert_ne!(
        *a,
        *derive_kek(b"1234", &salt, other).unwrap(),
        "інші параметри"
    );
}

#[test]
fn sealing_round_trips_and_detects_any_tampering() {
    let key = [1u8; 32];
    let (nonce, ct) = seal(&key, b"secret dek", b"header").unwrap();
    let back = open_sealed(&key, &nonce, &ct, b"header").unwrap();
    assert_eq!(&*back, b"secret dek");
    assert!(
        open_sealed(&[2u8; 32], &nonce, &ct, b"header").is_err(),
        "інший ключ"
    );
    assert!(
        open_sealed(&key, &nonce, &ct, b"other header").is_err(),
        "інші дані заголовка"
    );
    let mut bad = ct.clone();
    bad[0] ^= 1;
    assert!(
        open_sealed(&key, &nonce, &bad, b"header").is_err(),
        "змінений шифротекст"
    );
    let mut bad_nonce = nonce;
    bad_nonce[0] ^= 1;
    assert!(
        open_sealed(&key, &bad_nonce, &ct, b"header").is_err(),
        "змінений nonce"
    );
}

#[test]
fn every_seal_uses_a_fresh_nonce() {
    let key = [1u8; 32];
    let (n1, c1) = seal(&key, b"x", b"").unwrap();
    let (n2, c2) = seal(&key, b"x", b"").unwrap();
    assert_ne!(n1, n2);
    assert_ne!(c1, c2);
}

#[test]
fn random_bytes_differ_between_calls() {
    let a: [u8; 32] = random_bytes().unwrap();
    let b: [u8; 32] = random_bytes().unwrap();
    assert_ne!(a, b);
    assert_ne!(a, [0u8; 32]);
}

#[test]
fn recovery_codes_have_a_readable_format_and_are_unique() {
    let a = recovery_code().unwrap();
    let b = recovery_code().unwrap();
    assert_ne!(a, b);
    assert_eq!(a.len(), 23, "20 символів у 4 групах по 5 через дефіс: {a}");
    assert_eq!(a.matches('-').count(), 3);
    assert!(a
        .chars()
        .all(|c| c == '-' || c.is_ascii_digit() || c.is_ascii_uppercase()));
    assert!(
        !a.contains(['I', 'L', 'O', 'U']),
        "без символів, які плутаються: {a}"
    );
}

#[test]
fn typed_recovery_codes_are_forgiving_but_strict_about_length() {
    let code = recovery_code().unwrap();
    let canon = normalize_recovery(&code).unwrap();
    assert_eq!(canon.len(), 20);
    assert_eq!(
        normalize_recovery(&code.to_lowercase()).as_deref(),
        Some(canon.as_str())
    );
    assert_eq!(
        normalize_recovery(&code.replace('-', " ")).as_deref(),
        Some(canon.as_str())
    );
    assert_eq!(
        normalize_recovery(&format!("  {code}  ")).as_deref(),
        Some(canon.as_str())
    );
    assert!(normalize_recovery("TOO-SHORT").is_none());
    assert!(normalize_recovery(&format!("{code}X")).is_none());
    assert!(normalize_recovery("").is_none());
    // Схожі символи: O читається як 0, I і L як 1.
    assert_eq!(
        normalize_recovery("OOOOO-OOOOO-IIIII-LLLLL").unwrap(),
        "00000000001111111111"
    );
}

#[test]
fn kdf_parameters_are_bounded_so_a_tampered_file_cannot_exhaust_memory() {
    assert!(KdfParams::production().validate().is_ok());
    assert!(FAST.validate().is_ok());
    for bad in [
        KdfParams { m_kib: 0, ..FAST },
        KdfParams {
            m_kib: 4 * 1024 * 1024,
            ..FAST
        },
        KdfParams { t: 0, ..FAST },
        KdfParams { t: 1000, ..FAST },
        KdfParams { p: 0, ..FAST },
        KdfParams { p: 64, ..FAST },
    ] {
        assert!(bad.validate().is_err(), "{bad:?}");
        assert!(derive_kek(b"1", &[0u8; 16], bad).is_err());
    }
}
