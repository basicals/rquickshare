use super::*;

#[test]
fn given_signal_sender_epoch_or_body_abuse_when_decoded_then_only_exact_authenticated_hints_are_accepted(
) {
    // Arrange
    let owner = ":1.99";
    let valid = Message::signal(PATH, INTERFACE, "StateChanged")
        .unwrap()
        .sender(owner)
        .unwrap()
        .build(&(format!("{:032x}", 123), 9u64))
        .unwrap();
    // Act / Assert
    assert_eq!(hint(&valid, owner), Ok((123, 9)));
    for (sender, path, interface, method, value, revision) in [
        (
            ":1.98",
            PATH,
            INTERFACE,
            "StateChanged",
            format!("{:032x}", 123),
            9,
        ),
        (
            owner,
            "/wrong",
            INTERFACE,
            "StateChanged",
            format!("{:032x}", 123),
            9,
        ),
        (
            owner,
            PATH,
            "wrong.Interface",
            "StateChanged",
            format!("{:032x}", 123),
            9,
        ),
        (owner, PATH, INTERFACE, "Wrong", format!("{:032x}", 123), 9),
        (
            owner,
            PATH,
            INTERFACE,
            "StateChanged",
            "bad epoch".into(),
            9,
        ),
        (
            owner,
            PATH,
            INTERFACE,
            "StateChanged",
            format!("{:032x}", 123),
            0,
        ),
    ] {
        let message = Message::signal(path, interface, method)
            .unwrap()
            .sender(sender)
            .unwrap()
            .build(&(value, revision))
            .unwrap();
        assert_eq!(hint(&message, owner), Err(ClientError::Protocol));
    }
    let file = tempfile::NamedTempFile::new().unwrap();
    let message = Message::signal(PATH, INTERFACE, "StateChanged")
        .unwrap()
        .sender(owner)
        .unwrap()
        .build(&Fd::from(file.as_file()))
        .unwrap();
    assert_eq!(hint(&message, owner), Err(ClientError::Protocol));
    for (sender, name, new, expected) in [
        ("org.freedesktop.DBus", SERVICE, owner, Ok(false)),
        ("org.freedesktop.DBus", SERVICE, "", Ok(true)),
        ("org.freedesktop.DBus", SERVICE, ":1.100", Ok(true)),
        (owner, SERVICE, "", Err(ClientError::Protocol)),
        (
            "org.freedesktop.DBus",
            "wrong.Service",
            "",
            Err(ClientError::Protocol),
        ),
    ] {
        let message = Message::signal(
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "NameOwnerChanged",
        )
        .unwrap()
        .sender(sender)
        .unwrap()
        .build(&(name, owner, new))
        .unwrap();
        assert_eq!(changed_owner(&message, owner), expected);
    }
}
