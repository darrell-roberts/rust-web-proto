# REST API with [Rocket](https://rocket.rs/)

## Features
* Create/Read/Update/Aggregate/Stream a resource(s) (User).
* Use Trait object to abstract Persistence layer.
* Use MongoDB persistence implementation for runtime and mock persitence implementation for unit testing.
* JSON data guard that validates deserialized types using the [validator crate](https://docs.rs/validator/latest/validator/index.html).
* SSL Server.
* SSL mutual TLS with MongoDB.
* JWT authorization