use super::*;
use crate::test_bus::Broker;
use zbus::zvariant::{Fd, Type};

#[test]
fn given_fixed_projection_when_layout_depth_properties_and_events_are_checked_then_output_is_bounded_and_actions_explicit(
) {
    // Arrange
    let state = State {
        projection: Projection {
            enabled: 0x187ff,
            full: false,
        },
        ..Default::default()
    };
    // Act / Assert
    assert_eq!(Layout::SIGNATURE.to_string(), "(ia{sv}av)");
    for depth in [-1, 0, 1, 2, i32::MAX] {
        assert_eq!(
            layout(0, depth, &[], state).unwrap().2.len(),
            if depth == 0 { 0 } else { ROWS.len() }
        );
        for id in 1..=17 {
            assert!(layout(id, depth, &[], state).unwrap().2.is_empty());
        }
    }
    for id in [-1, 18, i32::MAX] {
        assert!(layout(id, -1, &[], state).is_err());
    }
    assert!(layout(0, -2, &[], state).is_err());
    for names in [vec!["enabled".into()], vec!["unknown".into()]] {
        let result = layout(1, -1, &names, state).unwrap();
        assert!(result.1.keys().all(|name| names.contains(name)));
    }
    for names in [
        vec!["x".repeat(65)],
        vec!["".into()],
        vec!["valid".into(); 17],
    ] {
        assert!(layout(0, 1, &names, state).is_err());
    }
    let compact = i32::try_from(properties(6, state, &[]).remove("toggle-state").unwrap()).unwrap();
    let full = i32::try_from(properties(7, state, &[]).remove("toggle-state").unwrap()).unwrap();
    assert_eq!((compact, full), (1, 0));
    assert!(checked_event(0, "clicked", state).is_err());
    for id in (1..=11).chain([16, 17]) {
        assert_eq!(
            checked_event(id, "clicked", state).unwrap(),
            Action::from_id(id)
        );
    }
    for event in ["opened", "closed", "hovered"] {
        assert_eq!(checked_event(0, event, state).unwrap(), None);
    }
    assert!(checked_event(1, "accept", state).is_err());
    assert!(checked_event(1, "clicked", State::default()).is_err());
    let (tx, mut updates) = watch::channel(State::default());
    let handle = Handle { state: tx };
    handle.publish(state.projection);
    assert_eq!(updates.borrow_and_update().revision, 2);
    handle.publish(state.projection);
    assert!(!updates.has_changed().unwrap());
    handle.state.send_modify(|state| state.revision = u32::MAX);
    handle.publish(Projection {
        enabled: 0,
        full: true,
    });
    assert!(updates.borrow().exhausted);
}

#[tokio::test]
async fn given_real_menu_when_reads_clicks_groups_or_fd_abuse_arrive_then_order_capacity_and_rejected_state_are_preserved(
) {
    use std::os::fd::AsFd;
    // Arrange: own object on an isolated broker, no live tray or GTK.
    let broker = Broker::new();
    let server = broker.connection().await;
    let (state, _) = watch::channel(State {
        projection: Projection {
            enabled: 0x187ff,
            full: true,
        },
        ..Default::default()
    });
    let (commands, mut received) = mpsc::channel(8);
    server
        .object_server()
        .at(
            PATH,
            Menu {
                state: state.clone(),
                actions: commands,
                uid: rustix::process::geteuid().as_raw(),
            },
        )
        .await
        .unwrap();
    let client = broker.connection().await;
    let proxy = zbus::Proxy::new(
        &client,
        server.unique_name().unwrap().as_str(),
        PATH,
        INTERFACE,
    )
    .await
    .unwrap();
    // Act / Assert
    assert_eq!(proxy.get_property::<u32>("Version").await.unwrap(), 2);
    let (revision, tree): (u32, Layout) = proxy
        .call("GetLayout", &(0i32, -1i32, Vec::<String>::new()))
        .await
        .unwrap();
    assert_eq!(revision, 1);
    assert_eq!(tree.0, 0);
    assert_eq!(tree.2.len(), 17);
    let rows: Vec<i32> = tree
        .2
        .into_iter()
        .map(|node| {
            let row: Layout = node.try_into().unwrap();
            row.0
        })
        .collect();
    assert_eq!(rows, ROWS);
    for id in (1..=11).chain([16, 17]) {
        let label: OwnedValue = proxy.call("GetProperty", &(id, "label")).await.unwrap();
        assert_eq!(
            String::try_from(label).unwrap(),
            Action::from_id(id).unwrap().label()
        );
    }
    let props: Properties = proxy
        .call("GetGroupProperties", &(Vec::<i32>::new(), vec!["enabled"]))
        .await
        .unwrap();
    assert_eq!(props.len(), 18);
    assert!(props
        .iter()
        .all(|(_, props)| props.keys().all(|key| key == "enabled")));
    assert!(!proxy
        .call::<_, _, bool>("AboutToShow", &0i32)
        .await
        .unwrap());
    assert_eq!(
        proxy
            .call::<_, _, (Vec<i32>, Vec<i32>)>("AboutToShowGroup", &vec![0i32, 999])
            .await
            .unwrap(),
        (vec![], vec![999])
    );
    proxy
        .call::<_, _, ()>("Event", &(1i32, "opened", OwnedValue::from(0i32), 0u32))
        .await
        .unwrap();
    assert!(received.try_recv().is_err());
    for action in [Action::Text, Action::Image] {
        proxy
            .call::<_, _, ()>(
                "Event",
                &(action as i32, "clicked", OwnedValue::from(0i32), 0u32),
            )
            .await
            .unwrap();
        assert_eq!(received.recv().await.unwrap(), vec![action]);
        state.send_modify(|state| state.projection.enabled &= !action.bit());
        assert!(proxy
            .call::<_, _, ()>(
                "Event",
                &(action as i32, "clicked", OwnedValue::from(0i32), 0u32)
            )
            .await
            .is_err());
    }
    let bad: Vec<i32> = proxy
        .call(
            "EventGroup",
            &vec![
                (2i32, "clicked", OwnedValue::from(0i32), 0u32),
                (999, "clicked", OwnedValue::from(0i32), 0),
            ],
        )
        .await
        .unwrap();
    assert_eq!(bad, vec![2, 999]);
    assert!(received.try_recv().is_err());
    let good: Vec<i32> = proxy
        .call(
            "EventGroup",
            &vec![
                (2i32, "clicked", OwnedValue::from(0i32), 0u32),
                (7, "clicked", OwnedValue::from(0i32), 0),
                (8, "clicked", OwnedValue::from(0i32), 0),
            ],
        )
        .await
        .unwrap();
    assert!(good.is_empty());
    assert_eq!(
        received.recv().await.unwrap(),
        vec![Action::Files, Action::Full, Action::Settings]
    );
    state.send_modify(|state| state.projection.enabled &= !Action::Files.bit());
    assert!(proxy
        .call::<_, _, ()>("Event", &(2i32, "clicked", OwnedValue::from(0i32), 0u32))
        .await
        .is_err());
    for case in 0..8 {
        proxy
            .call::<_, _, ()>(
                "Event",
                &(
                    if case % 2 == 0 { 6i32 } else { 7 },
                    "clicked",
                    OwnedValue::from(0i32),
                    0u32,
                ),
            )
            .await
            .unwrap();
    }
    assert!(proxy
        .call::<_, _, ()>("Event", &(8i32, "clicked", OwnedValue::from(0i32), 0u32))
        .await
        .is_err());
    for case in 0..8 {
        assert_eq!(
            received.recv().await.unwrap(),
            vec![if case % 2 == 0 {
                Action::Compact
            } else {
                Action::Full
            }]
        );
    }
    let file = tempfile::NamedTempFile::new().unwrap();
    assert!(proxy
        .call::<_, _, ()>(
            "Event",
            &(
                8i32,
                "clicked",
                Value::from(Fd::from(file.as_file().as_fd())),
                0u32
            )
        )
        .await
        .is_err());
    assert!(proxy
        .call::<_, _, Vec<i32>>(
            "EventGroup",
            &vec![(8i32, "clicked", OwnedValue::from(0i32), 0u32); 17]
        )
        .await
        .is_err());
    assert!(proxy
        .call::<_, _, Properties>(
            "GetGroupProperties",
            &(vec![1i32; 33], Vec::<String>::new())
        )
        .await
        .is_err());
    assert!(proxy
        .call::<_, _, (Vec<i32>, Vec<i32>)>("AboutToShowGroup", &vec![1i32; 33])
        .await
        .is_err());
    server
        .object_server()
        .remove::<Menu, _>(PATH)
        .await
        .unwrap();
    let (commands, mut denied) = mpsc::channel(8);
    server
        .object_server()
        .at(
            PATH,
            Menu {
                state,
                actions: commands,
                uid: rustix::process::geteuid().as_raw().wrapping_add(1),
            },
        )
        .await
        .unwrap();
    for result in [
        proxy
            .call::<_, _, (u32, Layout)>("GetLayout", &(0i32, -1i32, Vec::<String>::new()))
            .await
            .map(|_| ()),
        proxy
            .call::<_, _, ()>("Event", &(8i32, "clicked", OwnedValue::from(0i32), 0u32))
            .await,
    ] {
        assert!(
            matches!(result, Err(zbus::Error::MethodError(name, _, _)) if name.as_str() == "org.freedesktop.DBus.Error.AccessDenied")
        );
    }
    assert!(denied.try_recv().is_err());
    assert!(received.try_recv().is_err());
    assert!(proxy
        .call::<_, _, (u32, Layout)>("GetLayout", &(0i32, -1i32, vec!["enabled"; 17]))
        .await
        .is_err());
    client.close().await.unwrap();
    server.close().await.unwrap();
}
