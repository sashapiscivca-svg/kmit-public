mod common;

use common::TestDir;
use storage::crypto::KdfParams;
use storage::profiles::{ProfileStore, LOCK_AFTER, MAX_LOCK_SECS};
use storage::StorageError;

const FAST: KdfParams = KdfParams::fast_for_tests();

fn setup() -> (TestDir, ProfileStore, u8) {
    let d = TestDir::new();
    let store = ProfileStore::new(d.path());
    let (slot, v, _code) = store.create("4821", 1, FAST, None).unwrap();
    drop(v);
    (d, store, slot)
}

fn wrong(store: &ProfileStore, slot: u8, now: u64) -> StorageError {
    store.open_at(slot, "0000", None, now).err().unwrap()
}

#[test]
fn a_few_wrong_pins_are_free_then_attempts_are_delayed() {
    let (_d, store, slot) = setup();
    for i in 0..LOCK_AFTER {
        assert_eq!(
            wrong(&store, slot, 1_000 + u64::from(i)),
            StorageError::WrongPin,
            "спроба {i}"
        );
    }
    // Після п'ятої хибної: пауза, навіть правильний PIN у цей час не пробується.
    let e = store.open_at(slot, "4821", None, 1_005).err().unwrap();
    assert!(
        matches!(e, StorageError::Locked(s) if (1..=30).contains(&s)),
        "{e:?}"
    );
    // Коли пауза минула, правильний PIN відкриває й скидає лічильник.
    assert!(store.open_at(slot, "4821", None, 1_100).is_ok());
    for _ in 0..LOCK_AFTER - 1 {
        assert_eq!(
            wrong(&store, slot, 2_000),
            StorageError::WrongPin,
            "лічильник скинуто"
        );
    }
}

#[test]
fn the_delay_grows_with_each_failure_and_is_capped() {
    let (_d, store, slot) = setup();
    let mut now = 10_000u64;
    let mut last = 0u32;
    for n in 0..14 {
        // Дочекатися кінця паузи й помилитися знову.
        let e = loop {
            match store.open_at(slot, "0000", None, now) {
                Err(StorageError::Locked(s)) => now += u64::from(s),
                Err(e) => break e,
                Ok(_) => panic!("хибний PIN відкрив профіль"),
            }
        };
        assert_eq!(e, StorageError::WrongPin);
        // Пробу правильним PIN робимо лише коли пауза вже має бути (інакше вона б відкрила профіль і скинула лічильник).
        let secs = if n >= LOCK_AFTER - 1 {
            match store.open_at(slot, "4821", None, now) {
                Err(StorageError::Locked(s)) => s,
                _ => 0,
            }
        } else {
            0
        };
        if n >= LOCK_AFTER - 1 {
            assert!(
                secs >= last.min(MAX_LOCK_SECS),
                "пауза не зменшується: {secs} після {last}"
            );
            assert!(secs <= MAX_LOCK_SECS, "пауза не довша за межу: {secs}");
        }
        last = secs;
        now += 1;
    }
    assert_eq!(last, MAX_LOCK_SECS, "доходить до межі");
}

#[test]
fn moving_the_clock_back_cannot_extend_or_skip_the_lock_beyond_the_cap() {
    let (_d, store, slot) = setup();
    for _ in 0..LOCK_AFTER {
        wrong(&store, slot, 50_000);
    }
    // Годинник відкотили далеко назад: пауза не довша за межу (профіль не блокується назавжди).
    let e = store.open_at(slot, "4821", None, 10).err().unwrap();
    assert!(
        matches!(e, StorageError::Locked(s) if s <= MAX_LOCK_SECS),
        "{e:?}"
    );
}

#[test]
fn the_recovery_code_shares_the_same_limit() {
    let (_d, store, slot) = setup();
    for _ in 0..LOCK_AFTER {
        assert_eq!(
            store
                .recover_at(slot, "AAAA-AAAA-AAAA-AAAA-AAAA", "9999", None, 100)
                .err()
                .unwrap(),
            StorageError::WrongRecoveryCode
        );
    }
    assert!(matches!(
        store
            .recover_at(slot, "AAAA-AAAA-AAAA-AAAA-AAAA", "9999", None, 101)
            .err()
            .unwrap(),
        StorageError::Locked(_)
    ));
    assert!(matches!(
        store.open_at(slot, "4821", None, 102).err().unwrap(),
        StorageError::Locked(_)
    ));
}
