use std::task::Poll;

use futures::{pin_mut, poll};

use super::*;

fn name(value: &str) -> DeviceName {
    DeviceName::new(value).unwrap()
}

#[test]
fn given_name_boundaries_when_validated_then_only_exact_safe_utf8_is_accepted() {
    for value in [
        "Laptop",
        "Рабочий ПК",
        "é",
        &"a".repeat(237),
        &"é".repeat(118),
        "机 A",
    ] {
        let parsed = DeviceName::new(value).unwrap();
        assert_eq!(parsed.as_str(), value);
        assert_eq!(
            serde_json::from_str::<DeviceName>(&serde_json::to_string(&parsed).unwrap()).unwrap(),
            parsed
        );
    }
    for value in [
        "",
        " ",
        " a",
        "a ",
        "\tname",
        "n\0ame",
        "a\n",
        "x\u{7f}",
        "a\u{202e}b",
        "a\u{2066}b",
        &"x".repeat(238),
        &"é".repeat(119),
    ] {
        assert_eq!(DeviceName::new(value), Err(IdentityError::InvalidName));
        assert!(
            serde_json::from_str::<DeviceName>(&serde_json::to_string(value).unwrap()).is_err()
        );
    }
    for code in (0..=31)
        .chain(127..=159)
        .chain(0x202a..=0x202e)
        .chain(0x2066..=0x2069)
    {
        let value = format!("a{}b", char::from_u32(code).unwrap());
        assert_eq!(DeviceName::new(&value), Err(IdentityError::InvalidName));
    }
}

#[test]
fn given_private_name_when_debugged_then_name_is_redacted() {
    let private = name("Private device");
    let policy = IdentityPolicy::new(private.clone(), &[ConsumerKind::Mdns]);
    for value in [
        format!("{private:?}"),
        format!("{policy:?}"),
        format!("{:?}", policy.snapshot()),
    ] {
        assert!(!value.contains("Private device"));
        assert!(!value.contains("Private"));
    }
}

#[test]
fn given_independent_owners_when_renamed_then_prior_snapshots_and_other_owner_stay_fixed() {
    let owner = IdentityPolicy::new(name("Original"), &[]);
    let other = IdentityPolicy::new(name("Other"), &[]);
    let original = owner.snapshot();
    assert_eq!(owner.set_override(Some(&name("New"))), Ok(2));
    assert_eq!(owner.set_override(Some(&name("New"))), Ok(2));
    assert_eq!(original.name.as_str(), "Original");
    assert_eq!(owner.snapshot().name.as_str(), "New");
    assert_eq!(other.snapshot().name.as_str(), "Other");
    assert_eq!(owner.set_override(None), Ok(3));
    assert_eq!(owner.snapshot().name, original.name);
    assert_eq!(owner.applied(3), Ok(true));
    owner.current.send_modify(|value| value.revision = u64::MAX);
    assert_eq!(owner.set_override(None), Ok(u64::MAX));
    assert_eq!(
        owner.set_override(Some(&name("New"))),
        Err(IdentityError::Exhausted)
    );
    assert_eq!(owner.snapshot().name, original.name);
}

#[tokio::test]
async fn given_required_consumers_when_acknowledgements_arrive_then_exact_revision_completes() {
    let owner = IdentityPolicy::new(name("Original"), &[ConsumerKind::Mdns, ConsumerKind::Gatt]);
    assert_eq!(owner.applied(0), Err(IdentityError::Consumer));
    assert_eq!(owner.applied(2), Err(IdentityError::Consumer));
    assert_eq!(owner.applied(1), Ok(false));
    let mut mdns = owner.subscribe(ConsumerKind::Mdns).unwrap();
    let mut gatt = owner.subscribe(ConsumerKind::Gatt).unwrap();
    assert!(owner.subscribe(ConsumerKind::Mdns).is_err());
    assert_eq!(mdns.snapshot().revision, 1);
    mdns.acknowledge(1).unwrap();
    assert_eq!(owner.applied(1), Ok(false));
    assert_eq!(gatt.acknowledge(0), Err(IdentityError::Consumer));
    assert_eq!(gatt.acknowledge(2), Err(IdentityError::Consumer));
    gatt.acknowledge(1).unwrap();
    owner
        .wait_applied(1, &CancellationToken::new())
        .await
        .unwrap();
    let revision = owner.set_override(Some(&name("Updated"))).unwrap();
    mdns.changed().await.unwrap();
    gatt.changed().await.unwrap();
    assert_eq!(mdns.snapshot(), gatt.snapshot());
    let stop = CancellationToken::new();
    let wait = owner.wait_applied(revision, &stop);
    pin_mut!(wait);
    assert!(poll!(&mut wait).is_pending());
    mdns.acknowledge(revision).unwrap();
    gatt.acknowledge(1).unwrap();
    assert!(poll!(&mut wait).is_pending());
    gatt.acknowledge(revision).unwrap();
    assert_eq!(poll!(&mut wait), Poll::Ready(Ok(())));
    assert_eq!(gatt.acknowledge(1), Err(IdentityError::Consumer));
    assert_eq!(owner.applied(1), Err(IdentityError::Superseded));
    let stop = CancellationToken::new();
    stop.cancel();
    assert_eq!(
        owner.wait_applied(revision, &stop).await,
        Err(IdentityError::Cancelled)
    );
}

#[tokio::test]
async fn given_waiting_apply_when_consumer_drops_or_revision_changes_then_wait_fails_without_timeout(
) {
    for supersede in [false, true] {
        let owner = IdentityPolicy::new(name("Original"), &[ConsumerKind::Mdns]);
        let consumer = owner.subscribe(ConsumerKind::Mdns).unwrap();
        let stop = CancellationToken::new();
        let wait = owner.wait_applied(1, &stop);
        pin_mut!(wait);
        assert!(poll!(&mut wait).is_pending());
        let expected = if supersede {
            owner.set_override(Some(&name("Other"))).unwrap();
            IdentityError::Superseded
        } else {
            IdentityError::Unavailable
        };
        drop(consumer);
        assert_eq!(wait.await, Err(expected));
    }
    let owner = IdentityPolicy::new(name("Original"), &[ConsumerKind::Mdns]);
    assert!(owner.subscribe(ConsumerKind::Gatt).is_err());
    let consumer = owner.subscribe(ConsumerKind::Mdns).unwrap();
    drop(consumer);
    assert!(owner.subscribe(ConsumerKind::Mdns).is_err());
    assert_eq!(owner.applied(1), Err(IdentityError::Unavailable));
}

#[tokio::test]
async fn given_pending_consumer_when_cancelled_or_source_closes_then_owned_waits_exit() {
    let owner = IdentityPolicy::new(name("Original"), &[ConsumerKind::Mdns]);
    let mut consumer = owner.subscribe(ConsumerKind::Mdns).unwrap();
    consumer.snapshot();
    let stop = CancellationToken::new();
    {
        let wait = owner.wait_applied(1, &stop);
        pin_mut!(wait);
        assert!(poll!(&mut wait).is_pending());
        stop.cancel();
        assert_eq!(wait.await, Err(IdentityError::Cancelled));
    }
    drop(owner);
    assert_eq!(consumer.changed().await, Err(IdentityError::Unavailable));
}
