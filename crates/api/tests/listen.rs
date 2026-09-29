//! The API listens on loopback only: any other address refuses start by the setting's name, and
//! never echoes the address it was given (SPEC-025 A13, R6; ADR-007).

use std::net::SocketAddr;

use deck_streak_api::settings::LISTEN;
use deck_streak_api::{ApiError, ListenAddress};
use deck_streak_kernel::{Environment, SettingsError};

fn listen(value: &str) -> Result<ListenAddress, ApiError> {
    ListenAddress::from_env(&Environment::from_vars([(LISTEN, value)]))
}

#[test]
fn a_non_loopback_listen_address_is_refused_by_name() {
    // Addresses the whole network could reach (documentation ranges stand for real ones), and the
    // unspecified addresses, which listen on every interface.
    let exposed = [
        ("192.0.2.10:8080", "192.0.2.10"),
        ("198.51.100.7:443", "198.51.100.7"),
        ("[2001:db8::10]:8080", "2001:db8::10"),
        ("0.0.0.0:8080", "0.0.0.0"),
        ("[::]:8080", "[::]"),
    ];
    for (value, host) in exposed {
        let refused = listen(value);
        assert!(
            matches!(refused, Err(ApiError::NotLoopback { setting: LISTEN })),
            "{value} was not refused as a non-loopback address: {refused:?}"
        );
        let message = match &refused {
            Err(error) => error.to_string(),
            Ok(_) => String::new(),
        };
        assert!(message.contains(LISTEN), "{message}");
        assert!(
            !message.contains(host),
            "the refusal echoes the address: {message}"
        );
    }

    // Every loopback address starts: IPv4's whole 127.0.0.0/8, and IPv6's ::1.
    for value in ["127.0.0.1:8080", "127.0.0.2:8443", "[::1]:8080"] {
        let expected: SocketAddr = value.parse().expect("a socket address");
        let accepted = listen(value).ok().map(ListenAddress::socket_address);
        assert_eq!(accepted, Some(expected), "{value}");
    }

    // An unset address, and one that is not an address, are refused by the same name.
    let missing = ListenAddress::from_env(&Environment::default());
    assert!(
        matches!(
            missing,
            Err(ApiError::Settings(SettingsError::Missing {
                setting: LISTEN
            }))
        ),
        "{missing:?}"
    );
    let malformed = listen("localhost:8080");
    assert!(
        matches!(
            malformed,
            Err(ApiError::Settings(SettingsError::Malformed {
                setting: LISTEN,
                ..
            }))
        ),
        "{malformed:?}"
    );
}

#[test]
fn a_listen_address_displays_as_the_socket_address_it_holds() {
    for value in ["127.0.0.1:8080", "[::1]:8443"] {
        let address = listen(value).expect("a loopback address starts");
        assert_eq!(address.to_string(), value);
    }
}
