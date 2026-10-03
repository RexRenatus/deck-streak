### Changed

- The deploy tests' host stand-in runs only the commands the tests use and refuses any other by name; every deploy-script call goes through one helper that refuses an environment that does not name the elevation setting (#502).
