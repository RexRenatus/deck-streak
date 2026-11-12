### Changed

- The TestFlight lane's signing step matches the provisioning profile's certificates to the keychain's identities by the SHA-256 the keychain prints, computes no SHA-1 of its own, and still refuses a profile not issued for the imported certificate.
