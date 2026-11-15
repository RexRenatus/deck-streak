The TestFlight lane's signing step now names the placed certificate's shape by one fixed class
before it reads it, gives openssl's could-not-read and ASN.1 refusals classes of their own, and
re-wraps the signing key under PBES2 with a SHA-256 MAC, importing the SHA-1 based form once only
when the keychain refuses the first.
